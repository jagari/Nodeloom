//! Files that hold secrets (device keys, the admin token) or trust decisions
//! (the master's paired-device list).
//!
//! On Unix they are created owner-only from the first byte, never written
//! world-readable and tightened afterwards. Windows has no mode bits; the
//! callers keep these files under the per-user local app-data folder, which
//! other users cannot read by default.

use std::{
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

fn options() -> OpenOptions {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options
}

/// Creates `dir` and missing parents, owner-only on Unix.
pub fn create_private_dir(dir: &Path) -> io::Result<()> {
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(dir)
}

/// A complete, synced, owner-only copy of `contents` next to `path`, under a
/// name no other writer uses. Readers never see it half-written because it
/// only appears under `path` through a link or rename.
fn write_temp(path: &Path, contents: &[u8]) -> io::Result<PathBuf> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let mut name = OsString::from(path.as_os_str());
    name.push(format!(
        ".tmp-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let tmp = PathBuf::from(name);
    let result = options().open(&tmp).and_then(|mut file: File| {
        file.write_all(contents)?;
        file.sync_all()
    });
    match result {
        Ok(()) => Ok(tmp),
        Err(e) => {
            let _ = fs::remove_file(&tmp);
            Err(e)
        }
    }
}

/// Makes a rename or link durable: on Unix the directory entry is only on
/// disk once the directory itself is synced. Windows cannot open
/// directories as files, and NTFS journals the rename.
fn sync_parent(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    if let Some(parent) = path.parent() {
        let dir = if parent.as_os_str().is_empty() {
            Path::new(".")
        } else {
            parent
        };
        File::open(dir)?.sync_all()?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

/// Writes a new owner-only file. Fails with `AlreadyExists` instead of
/// replacing, so two processes racing to create the same identity cannot
/// both win, and the loser never reads a half-written file: the content is
/// complete before the hard link makes it visible.
pub fn write_new(path: &Path, contents: &[u8]) -> io::Result<()> {
    let tmp = write_temp(path, contents)?;
    let linked = fs::hard_link(&tmp, path);
    let _ = fs::remove_file(&tmp);
    linked?;
    sync_parent(path)
}

/// Replaces `path` atomically: readers see either the old or the new file,
/// never a half-written one, even if the process or the machine dies
/// mid-write.
pub fn replace(path: &Path, contents: &[u8]) -> io::Result<()> {
    let tmp = write_temp(path, contents)?;
    if let Err(e) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(e);
    }
    sync_parent(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_new_refuses_to_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("secret");
        write_new(&path, b"one").unwrap();
        let err = write_new(&path, b"two").unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read(&path).unwrap(), b"one");
    }

    #[test]
    fn racing_writers_see_one_winner_and_never_an_empty_file() {
        for _ in 0..50 {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("identity");
            let barrier = std::sync::Barrier::new(2);
            let outcomes: Vec<_> = std::thread::scope(|scope| {
                let handles: Vec<_> = [b"first".as_slice(), b"second".as_slice()]
                    .into_iter()
                    .map(|contents| {
                        let (path, barrier) = (&path, &barrier);
                        scope.spawn(move || {
                            barrier.wait();
                            let result = write_new(path, contents);
                            // Whoever lost must find complete contents.
                            (result.is_ok(), fs::read(path).unwrap())
                        })
                    })
                    .collect();
                handles.into_iter().map(|h| h.join().unwrap()).collect()
            });
            assert_eq!(outcomes.iter().filter(|(won, _)| *won).count(), 1);
            for (_, seen) in outcomes {
                assert!(seen == b"first" || seen == b"second", "{seen:?}");
            }
            assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1, "temp left");
        }
    }

    #[test]
    fn replace_swaps_contents_and_leaves_no_temp_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("list.json");
        replace(&path, b"old").unwrap();
        replace(&path, b"new").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"new");
        let names: Vec<_> = fs::read_dir(dir.path()).unwrap().collect();
        assert_eq!(names.len(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn files_and_dirs_are_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("a/b");
        create_private_dir(&dir).unwrap();
        let mode = |p: &Path| fs::metadata(p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&dir), 0o700);
        write_new(&dir.join("k"), b"x").unwrap();
        replace(&dir.join("r"), b"x").unwrap();
        assert_eq!(mode(&dir.join("k")), 0o600);
        assert_eq!(mode(&dir.join("r")), 0o600);
    }
}
