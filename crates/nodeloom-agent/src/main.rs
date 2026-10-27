use std::{path::PathBuf, sync::Arc, time::Duration};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use nodeloom_agent::{AgentConfig, identity, run_forever};
use nodeloom_core::{FakeRunner, RunnerSet, discovery};
use nodeloom_proto::DEFAULT_PORT;
use nodeloom_runner_blender::BlenderRunner;
use tracing::{info, warn};

#[derive(Parser)]
#[command(
    version,
    about = "Nodeloom agent: runs tasks sent by a Nodeloom master"
)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,

    /// Where this machine's identity is kept.
    /// Default: the user's local data directory, under nodeloom.
    #[arg(long, env = "NODELOOM_STATE_DIR", global = true)]
    state_dir: Option<PathBuf>,

    #[command(flatten)]
    run: RunArgs,
}

#[derive(Subcommand)]
enum Command {
    /// Print the confirmation code the dashboard shows for this machine.
    PairingCode,
    /// Make a new identity. Masters will see a new machine to approve.
    ResetIdentity,
}

#[derive(clap::Args)]
struct RunArgs {
    /// Master address as host:port. Found automatically via mDNS when omitted.
    #[arg(long, env = "NODELOOM_MASTER")]
    master: Option<String>,

    /// The master's admin token, to join without approval in the dashboard.
    /// Needs --master, so the token never goes to a master found by discovery.
    #[arg(long, env = "NODELOOM_TOKEN")]
    token: Option<String>,

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
    let state_dir = match args.state_dir {
        Some(dir) => dir,
        None => identity::default_state_dir()?,
    };

    match args.command {
        Some(Command::PairingCode) => {
            match identity::load(&state_dir)? {
                Some(me) => println!("{}  (node {})", me.confirmation_code(), me.node_id),
                None => println!(
                    "No identity in {} yet; start nodeloom-agent once to create one.",
                    state_dir.display()
                ),
            }
            Ok(())
        }
        Some(Command::ResetIdentity) => {
            let me = identity::reset(&state_dir)?;
            println!(
                "New identity: node {}, confirmation code {}.\n\
                 Restart the agent and approve it in the dashboard. Remove the old entry \
                 there if it is still listed.",
                me.node_id,
                me.confirmation_code()
            );
            Ok(())
        }
        None => run(args.run, state_dir).await,
    }
}

async fn run(args: RunArgs, state_dir: PathBuf) -> Result<()> {
    // Checked here rather than with clap's `requires`, so an exported
    // NODELOOM_TOKEN does not break the subcommands.
    if args.token.as_deref().is_some_and(|t| t.trim().is_empty()) {
        bail!("--token / NODELOOM_TOKEN is empty; unset it to pair by approval in the dashboard");
    }
    if args.token.is_some() && args.master.is_none() {
        bail!(
            "--token (or NODELOOM_TOKEN) needs --master: the token is never sent to a master \
             found by discovery. Leave it out to pair by approval in the dashboard."
        );
    }
    if args.token.is_some()
        && let Some(host) = args.master.as_deref().map(host_of)
        && may_resolve_by_multicast(host)
    {
        // Such names may be resolved by multicast DNS or LLMNR, which anyone
        // on the network can answer, so the token could reach an impostor.
        warn!(
            "--master {host} may be resolved by multicast DNS; prefer an IP address when using \
             --token, or pair by approval instead"
        );
    }
    let me = identity::load_or_create(&state_dir)?;
    info!(
        "this is node {} (confirmation code {}; identity in {})",
        me.node_id,
        me.confirmation_code(),
        state_dir.display()
    );

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
        identity: me,
        token: args.token,
        runners,
        metrics_interval: Duration::from_secs(args.metrics_interval.max(1)),
        status: None,
    })
    .await
}

/// The host part of `host:port`, `[v6]:port` or a bare host.
fn host_of(master: &str) -> &str {
    if let Some(rest) = master.strip_prefix('[') {
        return rest.split(']').next().unwrap_or(rest);
    }
    match master.rsplit_once(':') {
        Some((host, port)) if !host.contains(':') && port.parse::<u16>().is_ok() => host,
        _ => master,
    }
}

/// Names the OS may look up with multicast DNS (`.local`) or LLMNR (single
/// labels), which anyone on the network can answer.
fn may_resolve_by_multicast(host: &str) -> bool {
    let name = host.trim_end_matches('.').to_ascii_lowercase();
    // A link-local IPv6 address may carry a zone: fe80::1%en0.
    let address = name.split('%').next().unwrap_or(&name);
    if address.parse::<std::net::IpAddr>().is_ok() {
        return false;
    }
    name.ends_with(".local") || (!name.contains('.') && name != "localhost")
}

#[cfg(test)]
mod tests {
    use super::{host_of, may_resolve_by_multicast};

    #[test]
    fn multicast_names_are_spotted_in_any_spelling() {
        for name in [
            "mac.local",
            "MAC.LOCAL",
            "Juns-MacBook.Local",
            "mac.local.",
            "studio",
        ] {
            assert!(may_resolve_by_multicast(name), "{name}");
        }
        for name in [
            "192.168.1.5",
            "fe80::1%en0",
            "localhost",
            "master.example.edu",
        ] {
            assert!(!may_resolve_by_multicast(name), "{name}");
        }
    }

    #[test]
    fn host_of_strips_the_port() {
        assert_eq!(host_of("mac.local:7878"), "mac.local");
        assert_eq!(host_of("192.168.1.5"), "192.168.1.5");
        assert_eq!(host_of("[fe80::1]:7878"), "fe80::1");
        assert_eq!(host_of("fe80::1"), "fe80::1");
    }
}
