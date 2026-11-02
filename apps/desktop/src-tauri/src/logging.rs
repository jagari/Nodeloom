//! The log goes to a file as well as the terminal. A Windows GUI app has no
//! console, so without the file there is no way to see why it misbehaved.

use std::{
    fs::{File, OpenOptions},
    io,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

use tracing_subscriber::{EnvFilter, Layer, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Past this size the file is moved aside at the next start, so the log
/// keeps this run and the previous one without growing forever.
const MAX_BYTES: u64 = 5 * 1024 * 1024;
const NAME: &str = "desktop.log";
const PREVIOUS: &str = "desktop.old.log";

static FILE: OnceLock<PathBuf> = OnceLock::new();

/// Where this run logs to, if the file could be opened.
pub fn file() -> Option<&'static PathBuf> {
    FILE.get()
}

pub fn init() {
    let filter = || EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into());
    // Next to the master's and agent's data (see services::Dirs).
    let opened = dirs::data_local_dir()
        .map(|d| d.join("nodeloom").join("logs"))
        .and_then(|dir| open_in(&dir).ok());
    let file_layer = opened.map(|(path, file)| {
        let _ = FILE.set(path);
        fmt::layer()
            .with_ansi(false)
            .with_writer(Mutex::new(file))
            .with_filter(filter())
    });
    tracing_subscriber::registry()
        .with(fmt::layer().with_filter(filter()))
        .with(file_layer)
        .init();

    // A panic (for example a webview that cannot start) would otherwise only
    // reach a console that Windows does not show.
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!("panic: {info}");
        default(info);
    }));
    tracing::info!(
        "Nodeloom desktop {} on {} {}",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH
    );
}

fn open_in(dir: &Path) -> io::Result<(PathBuf, File)> {
    std::fs::create_dir_all(dir)?;
    let path = dir.join(NAME);
    if std::fs::metadata(&path).is_ok_and(|m| m.len() > MAX_BYTES) {
        std::fs::rename(&path, dir.join(PREVIOUS))?;
    }
    let file = OpenOptions::new().create(true).append(true).open(&path)?;
    Ok((path, file))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_large_log_is_moved_aside_and_a_small_one_kept() {
        let dir = tempfile::tempdir().unwrap();
        let logs = dir.path().join("logs");
        let (path, _) = open_in(&logs).unwrap();
        std::fs::write(&path, b"small").unwrap();
        open_in(&logs).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"small");

        std::fs::write(&path, vec![b'x'; MAX_BYTES as usize + 1]).unwrap();
        open_in(&logs).unwrap();
        assert_eq!(std::fs::metadata(&path).unwrap().len(), 0);
        assert!(logs.join(PREVIOUS).exists());
    }
}
