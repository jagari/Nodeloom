//! Files that hold secrets (device keys, the admin token) or trust decisions
//! (the master's paired-device list).
//!
//! On Unix they are created owner-only from the first byte, never written
//! world-readable and tightened afterwards. Windows has no mode bits; the
//! callers keep these files under the per-user local app-data folder, which
//! other users cannot read by default.

use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::Path,
};

fn options() -> OpenOptions {
    let mut options = OpenOptions::new();
    options.write(true);
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

fn write_all_synced(mut file: File, contents: &[u8]) -> io::Result<()> {
    file.write_all(contents)?;
    file.sync_all()
}

/// Writes a new owner-only file. Fails with `AlreadyExists` instead of
/// replacing, so two processes racing to create the same identity cannot
/// both win.
pub fn write_new(path: &Path, contents: &[u8]) -> io::Result<()> {
    let file = options().create_new(true).open(path)?;
    write_all_synced(file, contents).inspect_err(|_| {
        let _ = fs::remove_file(path);
    })
}

/// Replaces `path` atomically: readers see either the old or the new file,
/// never a half-written one, even if the process dies mid-write.
pub fn replace(path: &Path, contents: &[u8]) -> io::Result<()> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = Path::new(&tmp);
    // A leftover from a crash may have other permissions; start fresh so
    // the mode below applies.
    match fs::remove_file(tmp) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
        _ => {}
    }
    write_all_synced(options().create_new(true).open(tmp)?, contents)?;
    fs::rename(tmp, path)
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
