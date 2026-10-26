//! The Nodeloom master: pairs machines, tracks agents, splits jobs across
//! them and serves the cluster state to UI clients.

pub mod devices;
pub mod server;
pub mod state;
pub mod ui;

use std::{io, path::Path};

use nodeloom_core::{identity::random_hex, private_file};

pub use devices::DeviceStore;
pub use server::{router, serve};
pub use state::{Cluster, PairError, Settings, SubmitError};

const ADMIN_TOKEN_FILE: &str = "admin-token";

/// The dashboard's admin token, kept in `dir` so it survives restarts.
/// Returns the token and whether it was created just now. `rotate` replaces
/// an existing token, logging out every dashboard that used the old one.
pub fn load_admin_token(dir: &Path, rotate: bool) -> io::Result<(String, bool)> {
    private_file::create_private_dir(dir)?;
    let path = dir.join(ADMIN_TOKEN_FILE);
    if !rotate {
        match std::fs::read_to_string(&path) {
            Ok(text) if !text.trim().is_empty() => return Ok((text.trim().to_owned(), false)),
            Ok(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("{} is empty; use --rotate-admin-token", path.display()),
                ));
            }
            Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
            Err(_) => {}
        }
    }
    let token = random_hex(32).map_err(io::Error::other)?;
    if rotate {
        private_file::replace(&path, token.as_bytes())?;
    } else if let Err(e) = private_file::write_new(&path, token.as_bytes()) {
        // Another master started at the same moment and won; use its token.
        if e.kind() == io::ErrorKind::AlreadyExists {
            return load_admin_token(dir, false);
        }
        return Err(e);
    }
    Ok((token, true))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admin_token_is_created_once_kept_and_rotated_on_request() {
        let dir = tempfile::tempdir().unwrap();
        let (first, created) = load_admin_token(dir.path(), false).unwrap();
        assert!(created);
        assert_eq!(first.len(), 64);
        assert_eq!(
            load_admin_token(dir.path(), false).unwrap(),
            (first.clone(), false)
        );
        let (rotated, created) = load_admin_token(dir.path(), true).unwrap();
        assert!(created);
        assert_ne!(rotated, first);
        assert_eq!(load_admin_token(dir.path(), false).unwrap().0, rotated);
    }

    #[test]
    fn empty_token_file_is_an_error_not_a_blank_password() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(ADMIN_TOKEN_FILE), "\n").unwrap();
        assert!(load_admin_token(dir.path(), false).is_err());
    }
}
