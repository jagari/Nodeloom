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
        };
        save(&path, &chosen).unwrap();
        assert_eq!(load(&path), chosen);
        save(&path, &Settings::default()).unwrap();
        assert_eq!(load(&path).role, None);
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
