//! Checks applied to every task before it is scheduled (master) and again
//! before it runs (agent), so a bad task is rejected on both sides.

use std::path::{Component, Path};

use nodeloom_proto::TaskPayload;

/// Upper bound on frames in a single task; guards against absurd requests.
pub const MAX_FRAMES_PER_TASK: u32 = 1_000_000;

pub fn validate_payload(payload: &TaskPayload) -> Result<(), String> {
    let (start, end) = payload.frames();
    if start > end {
        return Err(format!("empty frame range {start}..={end}"));
    }
    if payload.frame_count() > MAX_FRAMES_PER_TASK {
        return Err(format!("more than {MAX_FRAMES_PER_TASK} frames"));
    }
    if let TaskPayload::Blender { blend_file, .. } = payload {
        validate_relative_path(blend_file)?;
        if !blend_file.ends_with(".blend") {
            return Err("blend_file must end with .blend".into());
        }
    }
    Ok(())
}

/// Accepts only plain relative paths with no `..`, root or drive prefix, so a
/// task cannot point outside the agent's working directory.
pub fn validate_relative_path(path: &str) -> Result<(), String> {
    if path.is_empty() {
        return Err("path is empty".into());
    }
    if path.contains('\0') {
        return Err("path contains NUL".into());
    }
    // Reject Windows-style separators and drive prefixes on every platform:
    // the same task may be executed by a Windows agent.
    if path.contains('\\') || path.as_bytes().get(1) == Some(&b':') {
        return Err("path must use '/' separators and no drive prefix".into());
    }
    for component in Path::new(path).components() {
        match component {
            Component::Normal(_) => {}
            _ => return Err("path must be relative without '..' or '.'".into()),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blender(file: &str) -> TaskPayload {
        TaskPayload::Blender {
            blend_file: file.into(),
            frame_start: 1,
            frame_end: 10,
        }
    }

    #[test]
    fn accepts_plain_relative_blend_paths() {
        assert!(validate_payload(&blender("scene.blend")).is_ok());
        assert!(validate_payload(&blender("projects/a/scene.blend")).is_ok());
    }

    #[test]
    fn rejects_escaping_and_absolute_paths() {
        for bad in [
            "../scene.blend",
            "a/../../scene.blend",
            "/etc/scene.blend",
            "./scene.blend",
            "C:/scene.blend",
            "a\\scene.blend",
            "",
            "scene.txt",
            "a\0.blend",
        ] {
            assert!(validate_payload(&blender(bad)).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn rejects_bad_ranges() {
        let p = TaskPayload::Fake {
            frame_start: 5,
            frame_end: 1,
            millis_per_frame: 0,
        };
        assert!(validate_payload(&p).is_err());
        let huge = TaskPayload::Fake {
            frame_start: 0,
            frame_end: MAX_FRAMES_PER_TASK,
            millis_per_frame: 0,
        };
        assert!(validate_payload(&huge).is_err());
    }
}
