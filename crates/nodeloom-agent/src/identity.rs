//! This machine's identity: a node id and the Ed25519 key that proves it to
//! masters. Both live in one owner-only file so they cannot drift apart.

use std::{
    io,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use nodeloom_core::{identity::DeviceKey, private_file};
use nodeloom_proto::NodeId;
use serde::{Deserialize, Serialize};

const FILE_NAME: &str = "identity";

#[derive(Clone, Debug)]
pub struct Identity {
    pub node_id: NodeId,
    pub key: DeviceKey,
}

#[derive(Serialize, Deserialize)]
struct IdentityFile {
    node_id: String,
    secret_key: String,
}

impl Identity {
    pub fn generate() -> Result<Self> {
        Ok(Self {
            node_id: NodeId(uuid::Uuid::new_v4().to_string()),
            key: DeviceKey::generate()?,
        })
    }

    /// The six digits the dashboard shows for this machine.
    pub fn confirmation_code(&self) -> String {
        self.key.public_key().confirmation_code()
    }

    fn to_json(&self) -> Vec<u8> {
        let file = IdentityFile {
            node_id: self.node_id.0.clone(),
            secret_key: self.key.seed_hex(),
        };
        serde_json::to_vec_pretty(&file).expect("identity serializes")
    }
}

/// The local data directory, not the config one: on Windows the config
/// directory roams to other PCs, and an identity belongs to one machine.
pub fn default_state_dir() -> Result<PathBuf> {
    Ok(dirs::data_local_dir()
        .context("no local data directory on this platform; pass --state-dir")?
        .join("nodeloom"))
}

/// Reads the identity in `dir`, or `None` if there is none yet. A damaged
/// file is an error: quietly making a new identity would turn a paired
/// machine into a stranger.
pub fn load(dir: &Path) -> Result<Option<Identity>> {
    let path = dir.join(FILE_NAME);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
    };
    let parsed = serde_json::from_str::<IdentityFile>(&text)
        .map_err(anyhow::Error::from)
        .and_then(|f| {
            if f.node_id.is_empty() {
                bail!("empty node id");
            }
            Ok(Identity {
                node_id: NodeId(f.node_id),
                key: DeviceKey::from_seed_hex(&f.secret_key)?,
            })
        });
    parsed.map(Some).with_context(|| {
        format!(
            "{} is damaged; fix it, or run `nodeloom-agent reset-identity` and approve \
             this machine again",
            path.display()
        )
    })
}

pub fn load_or_create(dir: &Path) -> Result<Identity> {
    if let Some(identity) = load(dir)? {
        return Ok(identity);
    }
    private_file::create_private_dir(dir).with_context(|| format!("creating {}", dir.display()))?;
    let identity = Identity::generate()?;
    match private_file::write_new(&dir.join(FILE_NAME), &identity.to_json()) {
        Ok(()) => Ok(identity),
        // Another agent process created it first; use theirs.
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
            load(dir)?.context("identity vanished while starting")
        }
        Err(e) => Err(e).with_context(|| format!("writing {}", dir.display())),
    }
}

/// Replaces the identity with a new one. Masters see a new machine.
pub fn reset(dir: &Path) -> Result<Identity> {
    private_file::create_private_dir(dir).with_context(|| format!("creating {}", dir.display()))?;
    let identity = Identity::generate()?;
    private_file::replace(&dir.join(FILE_NAME), &identity.to_json())
        .with_context(|| format!("writing {}", dir.display()))?;
    Ok(identity)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn created_once_then_reused() {
        let dir = tempfile::tempdir().unwrap();
        let first = load_or_create(dir.path()).unwrap();
        let again = load_or_create(dir.path()).unwrap();
        assert_eq!(first.node_id, again.node_id);
        assert_eq!(first.key.seed_hex(), again.key.seed_hex());
        assert_eq!(first.confirmation_code(), again.confirmation_code());
    }

    #[test]
    fn load_never_creates_anything() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("state");
        assert!(load(&missing).unwrap().is_none());
        assert!(!missing.exists());
    }

    #[test]
    fn damaged_file_is_an_error_not_a_new_identity() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(FILE_NAME), "{\"node_id\":\"a\"}").unwrap();
        assert!(load_or_create(dir.path()).is_err());
        assert!(load(dir.path()).is_err());
    }

    #[test]
    fn reset_gives_a_new_id_key_and_code() {
        let dir = tempfile::tempdir().unwrap();
        let old = load_or_create(dir.path()).unwrap();
        let new = reset(dir.path()).unwrap();
        assert_ne!(old.node_id, new.node_id);
        assert_ne!(old.key.seed_hex(), new.key.seed_hex());
        assert_eq!(load(dir.path()).unwrap().unwrap().node_id, new.node_id);
    }
}
