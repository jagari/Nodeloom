//! Resource sampling and machine description via `sysinfo`.

use std::time::{SystemTime, UNIX_EPOCH};

use nodeloom_proto::{Metrics, NodeId, NodeInfo, RunnerKind};
use sysinfo::System;

pub struct MetricsCollector {
    sys: System,
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsCollector {
    pub fn new() -> Self {
        let mut sys = System::new();
        // CPU usage is a delta between two refreshes, so prime it once.
        sys.refresh_cpu_usage();
        Self { sys }
    }

    pub fn sample(&mut self) -> Metrics {
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();
        Metrics {
            cpu_percent: self.sys.global_cpu_usage(),
            memory_used_bytes: self.sys.used_memory(),
            memory_total_bytes: self.sys.total_memory(),
            timestamp_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        }
    }
}

/// Describes this machine. `id` is supplied by the caller so it stays stable
/// across restarts (the agent persists it).
pub fn describe_node(id: NodeId, runners: Vec<RunnerKind>) -> NodeInfo {
    let mut sys = System::new();
    sys.refresh_memory();
    sys.refresh_cpu_list(sysinfo::CpuRefreshKind::nothing());
    NodeInfo {
        id,
        hostname: System::host_name().unwrap_or_else(|| "unknown".into()),
        os: System::long_os_version().unwrap_or_else(|| std::env::consts::OS.into()),
        arch: std::env::consts::ARCH.into(),
        cpu_cores: sys.cpus().len().max(1),
        total_memory_bytes: sys.total_memory(),
        runners,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_reports_plausible_values() {
        let mut c = MetricsCollector::new();
        let m = c.sample();
        assert!(m.memory_total_bytes > 0);
        assert!(m.memory_used_bytes <= m.memory_total_bytes);
        assert!((0.0..=100.0 * 1024.0).contains(&m.cpu_percent));
        assert!(m.timestamp_ms > 0);
    }

    #[test]
    fn describe_node_fills_fields() {
        let info = describe_node(NodeId("n1".into()), vec![RunnerKind::Fake]);
        assert_eq!(info.id.0, "n1");
        assert!(info.cpu_cores >= 1);
        assert!(!info.arch.is_empty());
    }
}
