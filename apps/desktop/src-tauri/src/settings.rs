//! What the app remembers between runs. Whether it starts at login is kept
//! by the OS (the autostart plugin), not here.

use std::{io, path::Path};

use serde::{Deserialize, Serialize};

/// What this machine does in the cluster.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Runs the master (and an agent, so this machine works too).
    Master,
    /// Runs an agent that joins a master found on the network.
    Agent,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    /// `None` until the user picks one on the first screen.
    pub role: Option<Role>,
    /// `host:port` of the master to join, typed by the user when discovery
    /// does not find it. `None` means find it on the network.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub master: Option<String>,
}

/// Turns what someone typed ("192.168.0.12", "http://mac.local:7878/")
/// into `host:port`, the default port filled in.
pub fn master_address(input: &str) -> Result<String, String> {
    let mut rest = input.trim();
    for scheme in ["http://", "https://", "ws://", "wss://"] {
        if let Some(stripped) = rest.strip_prefix(scheme) {
            rest = stripped;
        }
    }
    let rest = rest.split('/').next().unwrap_or("");
    let invalid = || {
        format!(
            "\"{}\" is not an address like 192.168.0.12:7878",
            input.trim()
        )
    };
    if rest.is_empty() || rest.chars().any(|c| c.is_whitespace() || c == '@') {
        return Err(invalid());
    }
    let (host, port) = match rest.rsplit_once(':') {
        // A bare IPv6 address has colons but no port.
        Some((host, port)) if !host.contains(':') || host.ends_with(']') => {
            (host, port.parse::<u16>().map_err(|_| invalid())?)
        }
        _ => (rest, nodeloom_proto::DEFAULT_PORT),
    };
    if host.is_empty() || port == 0 {
        return Err(invalid());
    }
    Ok(format!("{host}:{port}"))
}

/// Reads the settings. A missing file is a first run. A damaged one is
/// treated the same way: the worst case is asking for the role again,
/// which beats an app that will not start.
pub fn load(path: &Path) -> Settings {
    match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|e| {
            tracing::warn!("ignoring unreadable {}: {e}", path.display());
            Settings::default()
        }),
        Err(e) => {
            if e.kind() != io::ErrorKind::NotFound {
                tracing::warn!("cannot read {}: {e}", path.display());
            }
            Settings::default()
        }
    }
}

pub fn save(path: &Path, settings: &Settings) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let json = serde_json::to_vec_pretty(settings).map_err(io::Error::other)?;
    // Written whole and renamed into place, so a crash never leaves half a file.
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_run_has_no_role_and_choices_are_kept() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("settings.json");
        assert_eq!(load(&path), Settings::default());

        let chosen = Settings {
            role: Some(Role::Agent),
            master: Some("192.168.0.12:7878".into()),
        };
        save(&path, &chosen).unwrap();
        assert_eq!(load(&path), chosen);
        save(&path, &Settings::default()).unwrap();
        assert_eq!(load(&path).role, None);
    }

    #[test]
    fn typed_master_addresses_become_host_and_port() {
        assert_eq!(
            master_address(" 192.168.0.12 ").unwrap(),
            "192.168.0.12:7878"
        );
        assert_eq!(
            master_address("192.168.0.12:9000").unwrap(),
            "192.168.0.12:9000"
        );
        assert_eq!(
            master_address("http://mac.local:7878/").unwrap(),
            "mac.local:7878"
        );
        assert_eq!(master_address("[fe80::1]:7878").unwrap(), "[fe80::1]:7878");
        for bad in [
            "",
            "   ",
            "192.168.0.12:port",
            "a b:7878",
            "host:0",
            "user@host",
            ":7878",
        ] {
            assert!(master_address(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn settings_from_before_the_address_still_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, br#"{"role":"master"}"#).unwrap();
        assert_eq!(
            load(&path),
            Settings {
                role: Some(Role::Master),
                master: None
            }
        );
    }

    #[test]
    fn a_damaged_file_means_asking_again() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, b"{ not json").unwrap();
        assert_eq!(load(&path), Settings::default());
        std::fs::write(&path, br#"{"role":"overlord"}"#).unwrap();
        assert_eq!(load(&path), Settings::default());
    }
}
