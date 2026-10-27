use std::{net::SocketAddr, path::PathBuf, sync::Arc};

use anyhow::{Context, Result, bail};
use clap::Parser;
use nodeloom_core::{discovery, metrics::hostname};
use nodeloom_master::{Cluster, DeviceStore, dashboard_url, load_admin_token, serve, ui};
use tokio::net::TcpListener;
use tracing::{info, warn};

#[derive(Parser)]
#[command(version, about = "Nodeloom master: schedules jobs across agents")]
struct Args {
    /// Address to listen on.
    #[arg(long, env = "NODELOOM_BIND", default_value = "0.0.0.0:7878")]
    bind: SocketAddr,

    /// Admin token for the dashboard. When omitted, one is created on first
    /// start and kept in the data directory.
    #[arg(long, env = "NODELOOM_TOKEN")]
    token: Option<String>,

    /// Replace the saved admin token with a new one.
    #[arg(long, conflicts_with = "token")]
    rotate_admin_token: bool,

    /// Where paired machines and the admin token are kept.
    /// Default: the user's local data directory, under nodeloom/master.
    #[arg(long, env = "NODELOOM_DATA_DIR")]
    data_dir: Option<PathBuf>,

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

    let data_dir = match args.data_dir {
        Some(dir) => dir,
        None => dirs::data_local_dir()
            .context("no local data directory on this platform; pass --data-dir")?
            .join("nodeloom")
            .join("master"),
    };
    let store = DeviceStore::open(&data_dir)?;
    // Bind before touching the admin token: a second master started with
    // --rotate-admin-token while the first still runs must fail without
    // rotating, or the file would hold a token the running master rejects.
    let listener = TcpListener::bind(args.bind)
        .await
        .with_context(|| format!("binding {}", args.bind))?;
    let addr = listener.local_addr()?;

    let dashboard = dashboard_url(addr);
    let token = match args.token {
        // clap passes an empty NODELOOM_TOKEN through; a blank admin token
        // would let anyone in.
        Some(token) if token.trim().is_empty() => {
            bail!("--token / NODELOOM_TOKEN is empty; unset it to use the saved admin token")
        }
        Some(token) => token,
        None => {
            let (token, created) = load_admin_token(&data_dir, args.rotate_admin_token)
                .with_context(|| format!("admin token in {}", data_dir.display()))?;
            let path = data_dir.join("admin-token");
            // Print a new token once; afterwards only say where it is, so
            // service logs do not keep a copy of a long-lived secret.
            if created {
                info!("admin token: {token} (saved to {})", path.display());
                if ui::EMBEDDED {
                    // The token rides in the fragment, which browsers never
                    // send to the server; the page reads it and drops it.
                    info!("open the dashboard: {dashboard}#token={token}");
                }
            } else {
                info!("admin token: see {}", path.display());
            }
            token
        }
    };
    let cluster = Arc::new(Cluster::with_store(token, store));
    info!(
        "listening on {addr}; paired machines in {}",
        data_dir.display()
    );
    if ui::EMBEDDED {
        info!("dashboard: {dashboard}");
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
