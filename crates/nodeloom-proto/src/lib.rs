//! Wire types shared by the Nodeloom agent, master and UI clients.
//!
//! Tasks are a closed enum ([`TaskPayload`]) rather than free-form commands:
//! an agent only ever runs work it understands, never an arbitrary command
//! line received over the network.

use serde::{Deserialize, Serialize};

/// Bumped whenever a message changes in a way older peers cannot parse.
pub const PROTOCOL_VERSION: u32 = 1;

/// mDNS service type the master advertises and agents browse for.
pub const MDNS_SERVICE_TYPE: &str = "_nodeloom._tcp.local.";

/// Default TCP port of the master's HTTP/WebSocket API.
pub const DEFAULT_PORT: u16 = 7878;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub String);

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunnerKind {
    /// Test runner that only sleeps; never touches the machine.
    Fake,
    Blender,
}

/// Static facts about a machine, sent once when an agent connects.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeInfo {
    pub id: NodeId,
    pub hostname: String,
    pub os: String,
    pub arch: String,
    pub cpu_cores: usize,
    pub total_memory_bytes: u64,
    pub runners: Vec<RunnerKind>,
}

/// A point-in-time resource sample.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Metrics {
    pub cpu_percent: f32,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct JobId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TaskId(pub u64);

/// What a task does. Each variant maps to exactly one runner.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "runner", rename_all = "snake_case")]
pub enum TaskPayload {
    /// Sleeps `millis_per_frame` for every frame in the range.
    Fake {
        frame_start: u32,
        frame_end: u32,
        millis_per_frame: u64,
    },
    /// Renders frames `frame_start..=frame_end` of `blend_file`.
    /// `blend_file` is relative to the agent's working directory.
    Blender {
        blend_file: String,
        frame_start: u32,
        frame_end: u32,
    },
}

impl TaskPayload {
    pub fn kind(&self) -> RunnerKind {
        match self {
            TaskPayload::Fake { .. } => RunnerKind::Fake,
            TaskPayload::Blender { .. } => RunnerKind::Blender,
        }
    }

    /// Inclusive frame range covered by the task.
    pub fn frames(&self) -> (u32, u32) {
        match self {
            TaskPayload::Fake {
                frame_start,
                frame_end,
                ..
            }
            | TaskPayload::Blender {
                frame_start,
                frame_end,
                ..
            } => (*frame_start, *frame_end),
        }
    }

    /// Copy of this payload restricted to the inclusive range `start..=end`.
    pub fn with_frames(&self, start: u32, end: u32) -> TaskPayload {
        let mut copy = self.clone();
        match &mut copy {
            TaskPayload::Fake {
                frame_start,
                frame_end,
                ..
            }
            | TaskPayload::Blender {
                frame_start,
                frame_end,
                ..
            } => {
                *frame_start = start;
                *frame_end = end;
            }
        }
        copy
    }

    pub fn frame_count(&self) -> u32 {
        let (start, end) = self.frames();
        end.saturating_sub(start) + 1
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskSpec {
    pub job_id: JobId,
    pub task_id: TaskId,
    pub payload: TaskPayload,
}

/// Messages an agent sends to the master.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentMessage {
    /// First message on every connection.
    Hello {
        protocol: u32,
        /// Pairing token; the master closes the connection if it is wrong.
        token: String,
        info: NodeInfo,
    },
    Metrics(Metrics),
    TaskProgress {
        task_id: TaskId,
        frames_done: u32,
        frames_total: u32,
    },
    TaskDone {
        task_id: TaskId,
    },
    TaskFailed {
        task_id: TaskId,
        error: String,
    },
}

/// Messages the master sends to an agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MasterMessage {
    Welcome,
    Rejected { reason: String },
    AssignTask(TaskSpec),
    CancelTask { task_id: TaskId },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    Pending,
    Running,
    Done,
    Failed,
}

/// Snapshot of one task, as shown to UI clients.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskView {
    pub task_id: TaskId,
    pub node: Option<NodeId>,
    pub frame_start: u32,
    pub frame_end: u32,
    pub frames_done: u32,
    pub state: TaskState,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobView {
    pub job_id: JobId,
    pub tasks: Vec<TaskView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeView {
    pub info: NodeInfo,
    pub metrics: Option<Metrics>,
    pub busy_tasks: usize,
}

/// Full cluster state served to UI clients.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClusterSnapshot {
    pub nodes: Vec<NodeView>,
    pub jobs: Vec<JobView>,
}

/// Request body for creating a job through the master's HTTP API.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubmitJob {
    /// Task template; its frame range is the range to split across nodes.
    pub payload: TaskPayload,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_message_roundtrips_as_tagged_json() {
        let msg = AgentMessage::TaskProgress {
            task_id: TaskId(3),
            frames_done: 2,
            frames_total: 10,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"task_progress""#));
        assert_eq!(serde_json::from_str::<AgentMessage>(&json).unwrap(), msg);
    }

    #[test]
    fn unknown_runner_is_rejected_not_executed() {
        let json = r#"{"runner":"shell","command":"rm -rf /"}"#;
        assert!(serde_json::from_str::<TaskPayload>(json).is_err());
    }

    #[test]
    fn with_frames_keeps_other_fields() {
        let p = TaskPayload::Blender {
            blend_file: "a.blend".into(),
            frame_start: 1,
            frame_end: 100,
        };
        assert_eq!(
            p.with_frames(10, 20),
            TaskPayload::Blender {
                blend_file: "a.blend".into(),
                frame_start: 10,
                frame_end: 20,
            }
        );
    }

    #[test]
    fn frame_count_is_inclusive() {
        let p = TaskPayload::Fake {
            frame_start: 5,
            frame_end: 9,
            millis_per_frame: 0,
        };
        assert_eq!(p.frame_count(), 5);
    }
}
