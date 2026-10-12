//! Renders a frame range with a local Blender installation (`blender -b`).
//!
//! Frames are written under `<work_dir>/renders/`; collecting them from the
//! agents is not implemented yet.

use std::{
    collections::VecDeque,
    ffi::OsString,
    path::{Path, PathBuf},
    process::Stdio,
};

use async_trait::async_trait;
use nodeloom_core::{Runner, RunnerError, validate::validate_payload};
use nodeloom_proto::{RunnerKind, TaskPayload};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
    sync::mpsc,
};

pub struct BlenderRunner {
    executable: PathBuf,
    work_dir: PathBuf,
}

impl BlenderRunner {
    pub fn new(executable: impl Into<PathBuf>, work_dir: impl Into<PathBuf>) -> Self {
        Self {
            executable: executable.into(),
            work_dir: work_dir.into(),
        }
    }

    /// Looks for Blender in the usual install locations, then on `PATH`.
    pub fn detect(work_dir: impl Into<PathBuf>) -> Option<Self> {
        let work_dir = work_dir.into();
        well_known_locations()
            .into_iter()
            .find(|p| p.is_file())
            .or_else(|| find_on_path("blender"))
            .map(|exe| Self::new(exe, work_dir))
    }

    pub fn executable(&self) -> &Path {
        &self.executable
    }
}

fn well_known_locations() -> Vec<PathBuf> {
    vec![
        PathBuf::from("/Applications/Blender.app/Contents/MacOS/Blender"),
        PathBuf::from("C:/Program Files/Blender Foundation/Blender/blender.exe"),
        PathBuf::from("/usr/bin/blender"),
        PathBuf::from("/snap/bin/blender"),
    ]
}

fn find_on_path(name: &str) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    std::env::split_paths(&paths).find_map(|dir| {
        let plain = dir.join(name);
        if plain.is_file() {
            return Some(plain);
        }
        let exe = dir.join(format!("{name}.exe"));
        exe.is_file().then_some(exe)
    })
}

/// Builds the command line. The `.blend` path is joined onto `work_dir`, so
/// it is always absolute and can never be mistaken for a Blender option.
/// `--disable-autoexec` stops scripts embedded in the file from running.
pub fn build_args(work_dir: &Path, payload: &TaskPayload) -> Result<Vec<OsString>, RunnerError> {
    let TaskPayload::Blender {
        blend_file,
        frame_start,
        frame_end,
    } = payload
    else {
        return Err(RunnerError::InvalidTask("not a blender task".into()));
    };
    validate_payload(payload).map_err(RunnerError::InvalidTask)?;

    let mut output = work_dir.join("renders").join("frame_").into_os_string();
    output.push("####");

    Ok(vec![
        "--background".into(),
        "--disable-autoexec".into(),
        work_dir.join(blend_file).into_os_string(),
        "--render-output".into(),
        output,
        "--frame-start".into(),
        frame_start.to_string().into(),
        "--frame-end".into(),
        frame_end.to_string().into(),
        "--render-anim".into(),
    ])
}

#[async_trait]
impl Runner for BlenderRunner {
    fn kind(&self) -> RunnerKind {
        RunnerKind::Blender
    }

    async fn run(
        &self,
        payload: &TaskPayload,
        progress: &mpsc::Sender<u32>,
    ) -> Result<(), RunnerError> {
        let args = build_args(&self.work_dir, payload)?;
        tokio::fs::create_dir_all(self.work_dir.join("renders")).await?;

        let mut child = Command::new(&self.executable)
            .args(&args)
            .current_dir(&self.work_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;

        let stdout = child.stdout.take().expect("stdout is piped");
        let stderr = child.stderr.take().expect("stderr is piped");

        let progress_task = async {
            let mut frames_done = 0u32;
            let mut lines = BufReader::new(stdout).lines();
            while let Some(line) = lines.next_line().await? {
                // Blender prints "Saved: '<path>'" once per rendered frame.
                if line.trim_start().starts_with("Saved:") {
                    frames_done += 1;
                    let _ = progress.send(frames_done).await;
                }
            }
            Ok::<_, std::io::Error>(())
        };
        let stderr_task = async {
            let mut tail = VecDeque::with_capacity(5);
            let mut lines = BufReader::new(stderr).lines();
            while let Some(line) = lines.next_line().await? {
                if tail.len() == 5 {
                    tail.pop_front();
                }
                tail.push_back(line);
            }
            Ok::<_, std::io::Error>(tail)
        };

        let (progress_result, tail) = tokio::join!(progress_task, stderr_task);
        progress_result?;
        let tail = tail?;
        let status = child.wait().await?;

        if status.success() {
            Ok(())
        } else {
            let detail = tail.into_iter().collect::<Vec<_>>().join("\n");
            Err(RunnerError::Failed(format!(
                "blender exited with {status}: {detail}"
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(file: &str, start: u32, end: u32) -> TaskPayload {
        TaskPayload::Blender {
            blend_file: file.into(),
            frame_start: start,
            frame_end: end,
        }
    }

    #[test]
    fn args_use_absolute_blend_path_and_disable_autoexec() {
        let work = Path::new("/work");
        let args = build_args(work, &task("shots/a.blend", 3, 9)).unwrap();
        let args: Vec<String> = args
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();

        assert!(args.contains(&"--disable-autoexec".to_string()));
        assert!(
            args.contains(
                &Path::new("/work")
                    .join("shots/a.blend")
                    .to_string_lossy()
                    .into_owned()
            )
        );
        let pos = |flag: &str| args.iter().position(|a| a == flag).unwrap();
        assert_eq!(args[pos("--frame-start") + 1], "3");
        assert_eq!(args[pos("--frame-end") + 1], "9");
        // --render-anim must come after the range and output flags.
        assert!(pos("--render-anim") > pos("--frame-end"));
        assert!(pos("--render-anim") > pos("--render-output"));
    }

    #[test]
    fn args_reject_unsafe_tasks() {
        let work = Path::new("/work");
        assert!(build_args(work, &task("../x.blend", 1, 2)).is_err());
        assert!(build_args(work, &task("x.blend", 5, 1)).is_err());
        let fake = TaskPayload::Fake {
            frame_start: 1,
            frame_end: 2,
            millis_per_frame: 0,
        };
        assert!(build_args(work, &fake).is_err());
    }

    #[cfg(unix)]
    mod with_stub {
        use super::*;
        use std::os::unix::fs::PermissionsExt;

        /// A stand-in "blender" that prints one `Saved:` line per frame.
        fn stub(dir: &Path, exit_code: i32) -> PathBuf {
            let path = dir.join("fake-blender.sh");
            let script = format!(
                "#!/bin/sh\n\
                 while [ $# -gt 0 ]; do\n\
                   case \"$1\" in --frame-start) s=$2;; --frame-end) e=$2;; esac\n\
                   shift\n\
                 done\n\
                 f=$s\n\
                 while [ \"$f\" -le \"$e\" ]; do echo \"Saved: 'frame_$f.png'\"; f=$((f+1)); done\n\
                 echo oops >&2\n\
                 exit {exit_code}\n"
            );
            std::fs::write(&path, script).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
            path
        }

        fn temp_dir(name: &str) -> PathBuf {
            let dir = std::env::temp_dir().join(format!("nodeloom-{name}-{}", std::process::id()));
            std::fs::create_dir_all(&dir).unwrap();
            dir
        }

        #[tokio::test]
        async fn counts_saved_lines_as_progress() {
            let dir = temp_dir("blender-ok");
            let runner = BlenderRunner::new(stub(&dir, 0), &dir);
            let (tx, mut rx) = mpsc::channel(16);
            runner.run(&task("a.blend", 1, 3), &tx).await.unwrap();
            drop(tx);
            let mut seen = Vec::new();
            while let Some(n) = rx.recv().await {
                seen.push(n);
            }
            assert_eq!(seen, vec![1, 2, 3]);
            std::fs::remove_dir_all(dir).ok();
        }

        #[tokio::test]
        async fn nonzero_exit_is_a_failure_with_stderr_tail() {
            let dir = temp_dir("blender-fail");
            let runner = BlenderRunner::new(stub(&dir, 3), &dir);
            let (tx, _rx) = mpsc::channel(16);
            let err = runner.run(&task("a.blend", 1, 1), &tx).await.unwrap_err();
            let msg = err.to_string();
            assert!(msg.contains("oops"), "{msg}");
            std::fs::remove_dir_all(dir).ok();
        }
    }
}
