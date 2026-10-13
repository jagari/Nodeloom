//! Checks applied to every task before it is scheduled (master) and again
//! before it runs (agent), so a bad task is rejected on both sides. Also the
//! checks on what an unpaired machine may tell the master about itself.

use std::{
    collections::HashSet,
    path::{Component, Path},
};

use nodeloom_proto::{NodeInfo, TaskPayload};

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

/// Longest node id, hostname, OS or architecture string a machine may send.
pub const MAX_NODE_TEXT: usize = 64;

/// Checks the self-description in a `Hello`. It arrives before the machine
/// is approved and ends up in every dashboard, so it must be short and free
/// of control or invisible formatting characters (no fake "approve me"
/// text, no right-to-left tricks in a hostname).
pub fn validate_node_info(info: &NodeInfo) -> Result<(), String> {
    let id = &info.id.0;
    if id.is_empty()
        || id.len() > MAX_NODE_TEXT
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(format!(
            "node id must be 1-{MAX_NODE_TEXT} characters of A-Z, a-z, 0-9, '-' or '_'"
        ));
    }
    for (field, value) in [
        ("hostname", &info.hostname),
        ("os", &info.os),
        ("arch", &info.arch),
    ] {
        if value.chars().count() > MAX_NODE_TEXT {
            return Err(format!("{field} is longer than {MAX_NODE_TEXT} characters"));
        }
        if value.chars().any(is_hidden_char) {
            return Err(format!("{field} contains control or formatting characters"));
        }
    }
    let unique: HashSet<_> = info.runners.iter().collect();
    if unique.len() != info.runners.len() {
        return Err("runners are listed more than once".into());
    }
    Ok(())
}

/// Control characters plus every Unicode default-ignorable code point
/// (renders as nothing: bidi controls, zero-width characters, variation
/// selectors, Hangul fillers, tag characters, the BOM, soft hyphen) and the
/// interlinear annotation marks. Any of these lets two names that look the
/// same compare different.
pub fn is_hidden_char(c: char) -> bool {
    c.is_control()
        || matches!(c,
            '\u{00AD}' | '\u{034F}' | '\u{061C}'
            | '\u{115F}'..='\u{1160}'
            | '\u{17B4}'..='\u{17B5}'
            | '\u{180B}'..='\u{180F}'
            | '\u{200B}'..='\u{200F}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2060}'..='\u{206F}'
            | '\u{3164}'
            | '\u{FE00}'..='\u{FE0F}'
            | '\u{FEFF}'
            | '\u{FFA0}'
            | '\u{FFF0}'..='\u{FFFB}'
            | '\u{1BCA0}'..='\u{1BCA3}'
            | '\u{1D173}'..='\u{1D17A}'
            | '\u{E0000}'..='\u{E0FFF}')
}

/// Makes this machine's own description pass [`validate_node_info`]: drops
/// hidden characters and shortens long values. A long hostname is usually a
/// fully qualified name (`laptop.dept.example.edu`), so its first label is
/// kept before anything is cut.
pub fn fit_node_info(mut info: NodeInfo) -> NodeInfo {
    fn clean(text: &str) -> String {
        text.chars()
            .filter(|c| !is_hidden_char(*c))
            .collect::<String>()
            .trim()
            .to_owned()
    }
    fn cut(text: String) -> String {
        text.chars().take(MAX_NODE_TEXT).collect()
    }
    let mut hostname = clean(&info.hostname);
    if hostname.chars().count() > MAX_NODE_TEXT
        && let Some((label, _)) = hostname.split_once('.')
        && !label.is_empty()
    {
        hostname = label.to_owned();
    }
    info.hostname = cut(hostname);
    info.os = cut(clean(&info.os));
    info.arch = cut(clean(&info.arch));
    info
}

#[cfg(test)]
mod tests {
    use super::*;
    use nodeloom_proto::{NodeId, RunnerKind};

    fn info(id: &str, hostname: &str) -> NodeInfo {
        NodeInfo {
            id: NodeId(id.into()),
            hostname: hostname.into(),
            os: "macOS 15".into(),
            arch: "aarch64".into(),
            cpu_cores: 8,
            total_memory_bytes: 1 << 34,
            runners: vec![RunnerKind::Fake],
        }
    }

    #[test]
    fn accepts_ordinary_machines() {
        let uuid = "2f1c3a4e-5b6d-4e7f-8a9b-0c1d2e3f4a5b";
        assert!(validate_node_info(&info(uuid, "Jun's MacBook Pro")).is_ok());
        assert!(validate_node_info(&info("pi_5", "raspberrypi")).is_ok());
        assert!(validate_node_info(&info("a", "연구실-데스크탑")).is_ok());
    }

    #[test]
    fn rejects_bad_ids_and_hidden_or_long_text() {
        for id in ["", "a b", "a\nb", "../x", &"x".repeat(65)] {
            assert!(validate_node_info(&info(id, "host")).is_err(), "{id:?}");
        }
        for host in [
            "evil\u{202E}kooBcaM",
            "zero\u{200B}width",
            "tab\there",
            &"h".repeat(65),
        ] {
            assert!(validate_node_info(&info("a", host)).is_err(), "{host:?}");
        }
        for invisible in [
            '\u{E0020}',
            '\u{FE0F}',
            '\u{3164}',
            '\u{115F}',
            '\u{180B}',
            '\u{034F}',
        ] {
            let host = format!("Jun-MacBook{invisible}");
            assert!(validate_node_info(&info("a", &host)).is_err(), "{host:?}");
        }
        let mut twice = info("a", "host");
        twice.runners = vec![RunnerKind::Fake, RunnerKind::Fake];
        assert!(validate_node_info(&twice).is_err());
    }

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

    #[test]
    fn own_description_is_fitted_to_pass_validation() {
        let fqdn = format!("{}.dept.example.edu", "a".repeat(50));
        assert!(fqdn.chars().count() > MAX_NODE_TEXT);
        let fitted = fit_node_info(info("a", &fqdn));
        assert_eq!(fitted.hostname, "a".repeat(50));
        assert!(validate_node_info(&fitted).is_ok());

        let mut odd = info("a", " lab\u{200B}-pc\n");
        odd.os = "x".repeat(100);
        let fitted = fit_node_info(odd);
        assert_eq!(fitted.hostname, "lab-pc");
        assert_eq!(fitted.os.chars().count(), MAX_NODE_TEXT);
        assert!(validate_node_info(&fitted).is_ok());

        let label_too_long = fit_node_info(info("a", &"b".repeat(80)));
        assert_eq!(label_too_long.hostname.chars().count(), MAX_NODE_TEXT);
    }
}
