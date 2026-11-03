//! Starting this machine's Ollama server when it is installed but not
//! running, so a machine joins with its models without anyone typing
//! `ollama serve`. The program and its arguments are fixed here, run without
//! a shell (AGENTS.md rule 3), and nothing about it comes from the network.

use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use tokio::{process::Command, time::Instant};
use tracing::{info, warn};

/// How long a freshly started server gets to answer. The first start on a
/// machine can take a few seconds while Ollama looks for GPUs.
const START_TIMEOUT: Duration = Duration::from_secs(20);
const POLL: Duration = Duration::from_millis(250);
const PROBE_TIMEOUT: Duration = Duration::from_secs(2);

/// What [`ensure_running`] found or did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Startup {
    /// Something already answers at the address (the Ollama app, a service,
    /// or a terminal); it is left alone.
    AlreadyRunning,
    /// Nodeloom started `ollama serve` and it answers now. It keeps running
    /// after Nodeloom exits, like the Ollama app's own server.
    Started(PathBuf),
    /// No Ollama program on this machine.
    NotInstalled,
    /// Ollama is installed but did not come up; the reason is logged.
    Failed(String),
}

/// Whether an Ollama server answers at `base`.
pub async fn answers(base: &str) -> bool {
    let client = reqwest::Client::new();
    match client
        .get(format!("{}/api/tags", base.trim_end_matches('/')))
        .timeout(PROBE_TIMEOUT)
        .send()
        .await
    {
        Ok(reply) => reply.status().is_success(),
        Err(_) => false,
    }
}

/// Makes sure Ollama answers at `base` (a loopback `http://host:port`),
/// starting the installed program if nothing does.
pub async fn ensure_running(base: &str) -> Startup {
    if answers(base).await {
        return Startup::AlreadyRunning;
    }
    match find_ollama(&candidates(), std::env::var_os("PATH")) {
        Some(exe) => start(&exe, base).await,
        None => Startup::NotInstalled,
    }
}

/// Runs `exe serve`, listening only on `base`'s address, and waits for it
/// to answer.
pub async fn start(exe: &Path, base: &str) -> Startup {
    let host = base
        .trim_end_matches('/')
        .trim_start_matches("http://")
        .to_owned();
    let mut command = Command::new(exe);
    command
        .arg("serve")
        // Loopback only, whatever the user's environment says: Nodeloom
        // talks to it on this address, and nothing else on the network
        // should reach it through us.
        .env("OLLAMA_HOST", &host)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        // No console window flashing up behind the app.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(e) => {
            let reason = format!("could not run {}: {e}", exe.display());
            warn!("{reason}");
            return Startup::Failed(reason);
        }
    };
    info!("started {} serve on {host}", exe.display());

    let deadline = Instant::now() + START_TIMEOUT;
    while Instant::now() < deadline {
        if answers(base).await {
            return Startup::Started(exe.to_owned());
        }
        // Exiting early usually means the port is taken by something that
        // is not Ollama, or the install is broken.
        if let Ok(Some(status)) = child.try_wait() {
            let reason = format!("{} serve exited ({status})", exe.display());
            warn!("{reason}");
            return Startup::Failed(reason);
        }
        tokio::time::sleep(POLL).await;
    }
    let reason = format!(
        "{} serve did not answer within {}s",
        exe.display(),
        START_TIMEOUT.as_secs()
    );
    warn!("{reason}");
    Startup::Failed(reason)
}

/// Where the installers put the program. Desktop apps start with a short
/// `PATH` (a macOS app sees no Homebrew), so these come first.
fn candidates() -> Vec<PathBuf> {
    let mut paths = vec![
        PathBuf::from("/Applications/Ollama.app/Contents/Resources/ollama"),
        PathBuf::from("/opt/homebrew/bin/ollama"),
        PathBuf::from("/usr/local/bin/ollama"),
        PathBuf::from("/usr/bin/ollama"),
    ];
    if let Some(home) = std::env::var_os("HOME") {
        paths.push(PathBuf::from(home).join("Applications/Ollama.app/Contents/Resources/ollama"));
    }
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        paths.push(PathBuf::from(local).join("Programs/Ollama/ollama.exe"));
    }
    paths
}

/// The first existing file among `candidates`, else `ollama` on `path`.
pub fn find_ollama(candidates: &[PathBuf], path: Option<OsString>) -> Option<PathBuf> {
    candidates
        .iter()
        .find(|p| p.is_file())
        .cloned()
        .or_else(|| {
            std::env::split_paths(&path?).find_map(|dir| {
                ["ollama", "ollama.exe"]
                    .iter()
                    .map(|name| dir.join(name))
                    .find(|p| p.is_file())
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_locations_come_before_path() {
        let dir = std::env::temp_dir().join(format!("nodeloom-ollama-{}", std::process::id()));
        let (installed, on_path) = (dir.join("app"), dir.join("bin"));
        std::fs::create_dir_all(&installed).unwrap();
        std::fs::create_dir_all(&on_path).unwrap();
        std::fs::write(on_path.join("ollama"), "").unwrap();
        let path = std::env::join_paths([&on_path]).ok();

        let app = installed.join("ollama");
        assert_eq!(
            find_ollama(std::slice::from_ref(&app), path.clone()),
            Some(on_path.join("ollama")),
            "nothing installed: PATH"
        );
        std::fs::write(&app, "").unwrap();
        assert_eq!(find_ollama(std::slice::from_ref(&app), path), Some(app));
        assert_eq!(find_ollama(&[dir.join("missing")], None), None);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[tokio::test]
    async fn a_running_server_is_left_alone() {
        use axum::{Json, Router, routing::get};
        let app = Router::new().route(
            "/api/tags",
            get(|| async { Json(serde_json::json!({ "models": [] })) }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        assert_eq!(ensure_running(&base).await, Startup::AlreadyRunning);
    }

    #[tokio::test]
    async fn a_program_that_will_not_start_is_reported() {
        let free = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", free.local_addr().unwrap());
        drop(free);
        let missing = std::env::temp_dir().join("nodeloom-no-such-ollama");
        assert!(matches!(
            start(&missing, &base).await,
            Startup::Failed(reason) if reason.contains("could not run")
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_program_that_exits_at_once_is_reported_quickly() {
        use std::os::unix::fs::PermissionsExt;
        let free = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", free.local_addr().unwrap());
        drop(free);
        let fake =
            std::env::temp_dir().join(format!("nodeloom-false-ollama-{}", std::process::id()));
        std::fs::write(&fake, "#!/bin/sh\nexit 3\n").unwrap();
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
        let begun = Instant::now();
        let result = start(&fake, &base).await;
        std::fs::remove_file(&fake).unwrap();
        assert!(matches!(result, Startup::Failed(reason) if reason.contains("exited")));
        assert!(begun.elapsed() < Duration::from_secs(5));
    }
}
