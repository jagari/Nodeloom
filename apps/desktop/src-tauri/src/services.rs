//! The master and the agent, run inside the app rather than as separate
//! programs. They keep their data where the command-line tools do, so a
//! machine has one identity and one list of paired machines whichever way
//! it is started.

use std::{
    net::{Ipv4Addr, SocketAddr},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use anyhow::{Context, Result};
use nodeloom_agent::{AgentConfig, AgentStatus, identity, run_forever};
use nodeloom_core::{FakeRunner, RunnerSet, discovery, metrics::hostname};
use nodeloom_master::{Cluster, DeviceStore, dashboard_url, load_admin_token, serve};
use nodeloom_proto::DEFAULT_PORT;
use tokio::{net::TcpListener, sync::watch, task::JoinHandle};
use tracing::{info, warn};

use crate::settings::Role;

/// Where the master and the agent keep their files.
pub struct Dirs {
    pub master_data: PathBuf,
    pub agent_state: PathBuf,
}

impl Dirs {
    /// The command-line tools' defaults.
    pub fn standard() -> Result<Self> {
        let agent_state = identity::default_state_dir()?;
        Ok(Self {
            master_data: agent_state.join("master"),
            agent_state,
        })
    }
}

/// How to start the services; tests change the port and turn off mDNS.
pub struct Options {
    pub dirs: Dirs,
    pub port: u16,
    pub mdns: bool,
}

impl Options {
    pub fn standard() -> Result<Self> {
        Ok(Self {
            dirs: Dirs::standard()?,
            port: DEFAULT_PORT,
            mdns: true,
        })
    }
}

/// The master running in this app.
#[derive(Debug, Clone)]
pub struct MasterInfo {
    /// For this app's own window.
    pub local_url: String,
    /// For other machines on the network.
    pub lan_url: String,
    pub token: String,
}

/// Services started for a role; stopped when dropped.
pub struct Running {
    pub master: Option<MasterInfo>,
    pub agent: watch::Receiver<AgentStatus>,
    /// The code the dashboard shows for this machine.
    pub code: String,
    tasks: Vec<JoinHandle<()>>,
    _advertisement: Option<discovery::Advertisement>,
}

impl Drop for Running {
    fn drop(&mut self) {
        for task in &self.tasks {
            task.abort();
        }
    }
}

/// Starts what `role` needs. Fails only for problems the user has to fix
/// (the port is taken, a data file is damaged); a master that cannot be
/// found yet is a status, not an error.
pub async fn start(role: Role, options: Options) -> Result<Running> {
    let me = identity::load_or_create(&options.dirs.agent_state)?;
    let code = me.confirmation_code();
    let (status_tx, agent) = watch::channel(AgentStatus::Searching);
    // There is nothing else to run yet: Whisper (#26) will be the first
    // real plugin. The test runner only sleeps.
    let runners = RunnerSet::new().with(Arc::new(FakeRunner));
    let mut tasks = Vec::new();

    let (master, advertisement) = match role {
        Role::Master => {
            let dir = &options.dirs.master_data;
            let store = DeviceStore::open(dir)?;
            let bind = SocketAddr::from((Ipv4Addr::UNSPECIFIED, options.port));
            let listener = TcpListener::bind(bind).await.with_context(|| {
                format!(
                    "port {} is already in use; is another Nodeloom master running?",
                    options.port
                )
            })?;
            let addr = listener.local_addr()?;
            let (token, _) = load_admin_token(dir, false)
                .with_context(|| format!("admin token in {}", dir.display()))?;
            let cluster = Arc::new(Cluster::with_store(token.clone(), store));
            tasks.push(tokio::spawn(async move {
                if let Err(e) = serve(listener, cluster).await {
                    warn!("master stopped: {e}");
                }
            }));
            info!("master listening on {addr}");

            let advertisement = if options.mdns {
                discovery::advertise(
                    &format!("nodeloom-{}", hostname()),
                    &hostname(),
                    addr.port(),
                )
                .inspect_err(|e| warn!("mDNS advertisement failed ({e})"))
                .ok()
            } else {
                None
            };

            // This machine works too. It joins its own master with the
            // admin token over loopback, so it needs no approval.
            let local = format!("127.0.0.1:{}", addr.port());
            let config = AgentConfig {
                master_url: format!("ws://{local}/ws/agent"),
                identity: me,
                token: Some(token.clone()),
                runners,
                metrics_interval: Duration::from_secs(1),
                status: Some(status_tx),
            };
            tasks.push(tokio::spawn(async move {
                run_forever(config).await;
            }));
            let info = MasterInfo {
                local_url: format!("http://{local}/"),
                lan_url: dashboard_url(addr),
                token,
            };
            (Some(info), advertisement)
        }
        Role::Agent => {
            tasks.push(tokio::spawn(async move {
                let master = find_master_forever().await;
                run_forever(AgentConfig {
                    master_url: format!("ws://{master}/ws/agent"),
                    identity: me,
                    // Never a token here: the master was found by discovery,
                    // which anyone on the network can answer.
                    token: None,
                    runners,
                    metrics_interval: Duration::from_secs(1),
                    status: Some(status_tx),
                })
                .await;
            }));
            (None, None)
        }
    };

    Ok(Running {
        master,
        agent,
        code,
        tasks,
        _advertisement: advertisement,
    })
}

/// Unlike the command-line agent, which gives up after 30 seconds, the app
/// keeps looking: the master may simply not be switched on yet.
async fn find_master_forever() -> String {
    loop {
        match discovery::find_master(Duration::from_secs(30)).await {
            Ok((ip, port)) => return format!("{ip}:{port}"),
            Err(e) => {
                info!("no master yet ({e}); still looking");
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options_in(dir: &std::path::Path) -> Options {
        Options {
            dirs: Dirs {
                master_data: dir.join("master"),
                agent_state: dir.join("agent"),
            },
            port: 0,
            mdns: false,
        }
    }

    #[tokio::test]
    async fn a_master_machine_joins_its_own_cluster_without_approval() {
        let dir = tempfile::tempdir().unwrap();
        let mut running = start(Role::Master, options_in(dir.path())).await.unwrap();
        let master = running.master.clone().unwrap();
        assert!(master.local_url.starts_with("http://127.0.0.1:"));
        assert_eq!(master.token.len(), 64);

        let joined = tokio::time::timeout(
            Duration::from_secs(10),
            running
                .agent
                .wait_for(|s| matches!(s, AgentStatus::Joined { .. })),
        )
        .await;
        assert!(
            joined.is_ok_and(|r| r.is_ok()),
            "agent status: {:?}",
            *running.agent.borrow()
        );

        // The dashboard is bundled and the token opens the API.
        let page = reqwest::get(&master.local_url).await.unwrap();
        assert!(page.text().await.unwrap().contains("/assets/"));
        let state: serde_json::Value = reqwest::Client::new()
            .get(format!("{}api/state", master.local_url))
            .bearer_auth(&master.token)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(state["nodes"].as_array().unwrap().len(), 1);
        assert_eq!(state["pending"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn the_identity_and_admin_token_survive_a_restart() {
        let dir = tempfile::tempdir().unwrap();
        let first = start(Role::Master, options_in(dir.path())).await.unwrap();
        let (code, token) = (first.code.clone(), first.master.clone().unwrap().token);
        drop(first);
        let second = start(Role::Master, options_in(dir.path())).await.unwrap();
        assert_eq!(second.code, code);
        assert_eq!(second.master.clone().unwrap().token, token);
    }

    #[tokio::test]
    async fn a_taken_port_is_explained() {
        let dir = tempfile::tempdir().unwrap();
        let taken = std::net::TcpListener::bind("0.0.0.0:0").unwrap();
        let mut options = options_in(dir.path());
        options.port = taken.local_addr().unwrap().port();
        let error = start(Role::Master, options).await.err().unwrap();
        assert!(format!("{error:#}").contains("already in use"));
    }

    #[tokio::test]
    async fn an_agent_machine_starts_by_searching() {
        let dir = tempfile::tempdir().unwrap();
        let running = start(Role::Agent, options_in(dir.path())).await.unwrap();
        assert!(running.master.is_none());
        assert_eq!(*running.agent.borrow(), AgentStatus::Searching);
        assert_eq!(running.code.len(), 7, "ddd ddd");
    }
}
