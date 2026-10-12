use std::{path::PathBuf, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use clap::Parser;
use nodeloom_agent::{AgentConfig, run_forever};
use nodeloom_core::{FakeRunner, RunnerSet, discovery};
use nodeloom_proto::{DEFAULT_PORT, NodeId};
use nodeloom_runner_blender::BlenderRunner;
use tracing::{info, warn};

#[derive(Parser)]
#[command(
    version,
    about = "Nodeloom agent: runs tasks sent by a Nodeloom master"
)]
struct Args {
    /// Master address as host:port. Found automatically via mDNS when omitted.
    #[arg(long, env = "NODELOOM_MASTER")]
    master: Option<String>,

    /// Pairing token printed by the master.
    #[arg(long, env = "NODELOOM_TOKEN")]
    token: String,

    /// Directory holding project files and receiving renders.
    #[arg(long, env = "NODELOOM_WORK_DIR", default_value = ".")]
    work_dir: PathBuf,

    /// Enable the sleep-only test runner.
    #[arg(long)]
    enable_fake_runner: bool,

    /// Seconds between resource reports.
    #[arg(long, default_value_t = 1)]
    metrics_interval: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
    let args = Args::parse();

    let work_dir = std::fs::canonicalize(&args.work_dir)
        .with_context(|| format!("work dir {}", args.work_dir.display()))?;

    let mut runners = RunnerSet::new();
    match BlenderRunner::detect(&work_dir) {
        Some(blender) => {
            info!("blender found at {}", blender.executable().display());
            runners = runners.with(Arc::new(blender));
        }
        None => warn!("blender not found; this node cannot render blender tasks"),
    }
    if args.enable_fake_runner {
        runners = runners.with(Arc::new(FakeRunner));
    }

    let master = match args.master {
        Some(addr) => addr,
        None => {
            info!("looking for a master on the local network...");
            let (ip, port) = discovery::find_master(Duration::from_secs(30)).await?;
            format!("{ip}:{port}")
        }
    };
    let master_url = format!("ws://{master}/ws/agent");
    info!("master: {master_url} (default port {DEFAULT_PORT})");

    run_forever(AgentConfig {
        master_url,
        token: args.token,
        node_id: load_or_create_node_id()?,
        runners,
        metrics_interval: Duration::from_secs(args.metrics_interval.max(1)),
    })
    .await
}

/// A stable id per machine, kept in the user's config directory.
fn load_or_create_node_id() -> Result<NodeId> {
    let dir = dirs::config_dir()
        .context("no config directory on this platform")?
        .join("nodeloom");
    let path = dir.join("node-id");
    if let Ok(existing) = std::fs::read_to_string(&path) {
        let existing = existing.trim();
        if !existing.is_empty() {
            return Ok(NodeId(existing.to_owned()));
        }
    }
    std::fs::create_dir_all(&dir)?;
    let id = uuid::Uuid::new_v4().to_string();
    std::fs::write(&path, &id)?;
    Ok(NodeId(id))
}
