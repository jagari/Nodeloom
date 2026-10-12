use std::{net::SocketAddr, sync::Arc};

use anyhow::{Context, Result};
use clap::Parser;
use nodeloom_core::{discovery, metrics::hostname};
use nodeloom_master::{Cluster, generate_token, serve};
use tokio::net::TcpListener;
use tracing::{info, warn};

#[derive(Parser)]
#[command(version, about = "Nodeloom master: schedules jobs across agents")]
struct Args {
    /// Address to listen on.
    #[arg(long, env = "NODELOOM_BIND", default_value = "0.0.0.0:7878")]
    bind: SocketAddr,

    /// Pairing token. A random one is generated and printed when omitted.
    #[arg(long, env = "NODELOOM_TOKEN")]
    token: Option<String>,

    /// Do not advertise this master on the local network.
    #[arg(long)]
    no_mdns: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
    let args = Args::parse();

    let generated = args.token.is_none();
    let token = args.token.unwrap_or_else(generate_token);
    let cluster = Arc::new(Cluster::new(token.clone()));

    let listener = TcpListener::bind(args.bind)
        .await
        .with_context(|| format!("binding {}", args.bind))?;
    let addr = listener.local_addr()?;
    info!("listening on {addr}");
    if generated {
        info!("pairing token: {token}");
    }

    let _advertisement = if args.no_mdns {
        None
    } else {
        match discovery::advertise(
            &format!("nodeloom-{}", hostname()),
            &hostname(),
            addr.port(),
        ) {
            Ok(ad) => Some(ad),
            Err(e) => {
                warn!("mDNS advertisement failed ({e}); agents must use --master");
                None
            }
        }
    };

    tokio::select! {
        result = serve(listener, cluster) => result.context("server stopped")?,
        _ = tokio::signal::ctrl_c() => info!("shutting down"),
    }
    Ok(())
}
