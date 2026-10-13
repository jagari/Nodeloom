//! Paired machines, kept on disk so approvals survive a master restart.
//!
//! The file holds public keys only. Leaking it lets nobody connect, but
//! editing it would admit a machine, so it is written owner-only like the
//! secrets next to it.

use std::{
    collections::{BTreeMap, BTreeSet},
    io,
    path::{Path, PathBuf},
};

use nodeloom_core::{identity::PublicKey, private_file};
use nodeloom_proto::NodeId;
use serde::{Deserialize, Serialize};

const FILE_NAME: &str = "devices.json";
const FILE_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovedDevice {
    pub id: NodeId,
    /// Hostname when the machine was approved.
    pub name: String,
    pub public_key: String,
}

/// Approved machines by id, and keys that were removed for good.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Devices {
    pub approved: BTreeMap<NodeId, ApprovedDevice>,
    pub revoked: BTreeSet<String>,
}

impl Devices {
    pub fn is_approved(&self, id: &NodeId, public_key: &str) -> bool {
        !self.revoked.contains(public_key)
            && self
                .approved
                .get(id)
                .is_some_and(|d| d.public_key == public_key)
    }

    /// The id a key is already paired under, if any.
    pub fn id_of_key(&self, public_key: &str) -> Option<&NodeId> {
        self.approved
            .values()
            .find(|d| d.public_key == public_key)
            .map(|d| &d.id)
    }
}

#[derive(Serialize, Deserialize)]
struct DeviceFile {
    version: u32,
    approved: Vec<ApprovedDevice>,
    revoked: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("{path}: {source}")]
    Io { path: PathBuf, source: io::Error },
    #[error("{path} is damaged ({reason}); fix or move it away to start over")]
    Damaged { path: PathBuf, reason: String },
}

/// Where [`Devices`] are saved. In-memory stores (for tests) save nothing.
pub struct DeviceStore {
    path: Option<PathBuf>,
    loaded: Devices,
}

impl DeviceStore {
    pub fn in_memory() -> Self {
        Self {
            path: None,
            loaded: Devices::default(),
        }
    }

    /// Opens `<dir>/devices.json`, creating `dir` if needed. A missing file
    /// means no machine is paired yet. A damaged one is an error rather than
    /// an empty list: silently forgetting removals would let removed
    /// machines back in.
    pub fn open(dir: &Path) -> Result<Self, StoreError> {
        let io_err = |source| StoreError::Io {
            path: dir.to_owned(),
            source,
        };
        private_file::create_private_dir(dir).map_err(io_err)?;
        let path = dir.join(FILE_NAME);
        let loaded = match std::fs::read_to_string(&path) {
            Ok(text) => parse(&text).map_err(|reason| StoreError::Damaged {
                path: path.clone(),
                reason,
            })?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => Devices::default(),
            Err(source) => return Err(StoreError::Io { path, source }),
        };
        Ok(Self {
            path: Some(path),
            loaded,
        })
    }

    /// The devices read by [`DeviceStore::open`]; empty afterwards.
    pub fn take_loaded(&mut self) -> Devices {
        std::mem::take(&mut self.loaded)
    }

    pub fn save(&self, devices: &Devices) -> Result<(), StoreError> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        let file = DeviceFile {
            version: FILE_VERSION,
            approved: devices.approved.values().cloned().collect(),
            revoked: devices.revoked.iter().cloned().collect(),
        };
        let json = serde_json::to_vec_pretty(&file).expect("device list serializes");
        private_file::replace(path, &json).map_err(|source| StoreError::Io {
            path: path.clone(),
            source,
        })
    }
}

fn parse(text: &str) -> Result<Devices, String> {
    let file: DeviceFile = serde_json::from_str(text).map_err(|e| e.to_string())?;
    if file.version != FILE_VERSION {
        return Err(format!("unknown version {}", file.version));
    }
    let mut devices = Devices::default();
    for key in file.revoked {
        PublicKey::parse(&key).map_err(|e| format!("revoked key {key:?}: {e}"))?;
        devices.revoked.insert(key);
    }
    for device in file.approved {
        PublicKey::parse(&device.public_key).map_err(|e| format!("key of {}: {e}", device.id))?;
        if devices.id_of_key(&device.public_key).is_some() {
            return Err(format!("key of {} is listed twice", device.id));
        }
        if let Some(old) = devices.approved.insert(device.id.clone(), device) {
            return Err(format!("{} is listed twice", old.id));
        }
    }
    Ok(devices)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nodeloom_core::identity::DeviceKey;

    fn key() -> String {
        DeviceKey::generate()
            .unwrap()
            .public_key()
            .as_hex()
            .to_owned()
    }

    fn device(id: &str, public_key: &str) -> ApprovedDevice {
        ApprovedDevice {
            id: NodeId(id.into()),
            name: format!("{id}-host"),
            public_key: public_key.into(),
        }
    }

    #[test]
    fn missing_file_is_an_empty_list_and_saves_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = DeviceStore::open(dir.path()).unwrap();
        assert_eq!(store.take_loaded(), Devices::default());

        let mut devices = Devices::default();
        let (a, gone) = (key(), key());
        devices.approved.insert(NodeId("a".into()), device("a", &a));
        devices.revoked.insert(gone.clone());
        store.save(&devices).unwrap();

        let mut again = DeviceStore::open(dir.path()).unwrap();
        let loaded = again.take_loaded();
        assert_eq!(loaded, devices);
        assert!(loaded.is_approved(&NodeId("a".into()), &a));
        assert!(!loaded.is_approved(&NodeId("a".into()), &gone));
    }

    #[test]
    fn damaged_files_refuse_to_load() {
        let a = key();
        let cases = [
            "not json".to_owned(),
            r#"{"version":9,"approved":[],"revoked":[]}"#.to_owned(),
            r#"{"version":1,"approved":[{"id":"a","name":"a","public_key":"zz"}],"revoked":[]}"#
                .to_owned(),
            format!(
                r#"{{"version":1,"approved":[{{"id":"a","name":"a","public_key":"{a}"}},{{"id":"b","name":"b","public_key":"{a}"}}],"revoked":[]}}"#
            ),
        ];
        for text in cases {
            let dir = tempfile::tempdir().unwrap();
            std::fs::write(dir.path().join(FILE_NAME), &text).unwrap();
            assert!(
                matches!(
                    DeviceStore::open(dir.path()),
                    Err(StoreError::Damaged { .. })
                ),
                "{text}"
            );
        }
    }

    #[test]
    fn revoked_key_is_never_approved() {
        let a = key();
        let mut devices = Devices::default();
        devices.approved.insert(NodeId("a".into()), device("a", &a));
        devices.revoked.insert(a.clone());
        assert!(!devices.is_approved(&NodeId("a".into()), &a));
    }
}
