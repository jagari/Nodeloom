//! Wire types shared by the Nodeloom agent, master and UI clients.
//!
//! Tasks are a closed enum ([`TaskPayload`]) rather than free-form commands:
//! an agent only ever runs work it understands, never an arbitrary command
//! line received over the network.

use serde::{Deserialize, Serialize};

/// Bumped whenever a message changes in a way older peers cannot parse.
/// Version 2 replaced the shared pairing token with per-device keys.
pub const PROTOCOL_VERSION: u32 = 2;

/// mDNS service type the master advertises and agents browse for.
pub const MDNS_SERVICE_TYPE: &str = "_nodeloom._tcp.local.";

/// Default TCP port of the master's HTTP/WebSocket API.
pub const DEFAULT_PORT: u16 = 7878;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
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
    /// A local LLM (Ollama) answering a list of prompts.
    Llm,
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
    /// Models the local LLM runner can use, so LLM jobs only go to machines
    /// that have the model. Empty without an LLM runner.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub llm_models: Vec<String>,
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
    /// Asks a local LLM each prompt in `frame_start..=frame_end`; "frames"
    /// are 1-based prompt numbers, so a job splits like a render.
    /// `prompts[0]` is prompt number `first_prompt`: a piece carries only
    /// its own prompts, since a document's parts can be long.
    Llm {
        model: String,
        prompts: Vec<String>,
        frame_start: u32,
        frame_end: u32,
        #[serde(default = "first_prompt_default")]
        first_prompt: u32,
        /// Longest answer in tokens; the runner's default when absent.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max_tokens: Option<u32>,
    },
}

fn first_prompt_default() -> u32 {
    1
}

impl TaskPayload {
    pub fn kind(&self) -> RunnerKind {
        match self {
            TaskPayload::Fake { .. } => RunnerKind::Fake,
            TaskPayload::Blender { .. } => RunnerKind::Blender,
            TaskPayload::Llm { .. } => RunnerKind::Llm,
        }
    }

    /// The LLM model this task needs on the machine that runs it.
    pub fn llm_model(&self) -> Option<&str> {
        match self {
            TaskPayload::Llm { model, .. } => Some(model),
            _ => None,
        }
    }

    /// The prompt for `frame`, for an LLM task.
    pub fn prompt(&self, frame: u32) -> Option<&str> {
        match self {
            TaskPayload::Llm {
                prompts,
                first_prompt,
                ..
            } => frame
                .checked_sub(*first_prompt)
                .and_then(|i| prompts.get(i as usize))
                .map(String::as_str),
            _ => None,
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
            }
            | TaskPayload::Llm {
                frame_start,
                frame_end,
                ..
            } => (*frame_start, *frame_end),
        }
    }

    /// Copy of this payload restricted to the inclusive range `start..=end`.
    /// An LLM piece keeps only the prompts of that range.
    pub fn with_frames(&self, start: u32, end: u32) -> TaskPayload {
        if let TaskPayload::Llm {
            model, max_tokens, ..
        } = self
        {
            return TaskPayload::Llm {
                model: model.clone(),
                prompts: (start..=end)
                    .map_while(|frame| self.prompt(frame).map(str::to_owned))
                    .collect(),
                frame_start: start,
                frame_end: end,
                first_prompt: start,
                max_tokens: *max_tokens,
            };
        }
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
            }
            | TaskPayload::Llm {
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
///
/// A connection opens with `Hello`, the master answers `Challenge`, the agent
/// answers `Proof`, and the master decides: `Welcome`, `Pending` (waiting for
/// someone to approve the machine in the dashboard) or `Rejected`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentMessage {
    /// First message on every connection.
    Hello {
        protocol: u32,
        /// The machine's Ed25519 public key, 64 lowercase hex characters.
        public_key: String,
        info: NodeInfo,
        /// The master's admin token, to skip approval in the dashboard.
        /// Optional; agents only send it to a master given explicitly.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        token: Option<String>,
    },
    /// Answer to [`MasterMessage::Challenge`]: the challenge signed with the
    /// machine's key, as hex.
    Proof {
        signature: String,
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
    /// What one frame produced: an LLM task's answer to prompt `frame`.
    TaskOutput {
        task_id: TaskId,
        frame: u32,
        text: String,
    },
}

/// Messages the master sends to an agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MasterMessage {
    /// Random hex the agent must sign, so a recorded handshake cannot be
    /// replayed.
    Challenge {
        nonce: String,
    },
    Welcome,
    /// The machine is not approved yet. The connection stays open and the
    /// master sends `Welcome` or `Rejected` once someone decides.
    Pending,
    Rejected {
        reason: String,
    },
    AssignTask(TaskSpec),
    CancelTask {
        task_id: TaskId,
    },
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
    pub runner: RunnerKind,
    pub tasks: Vec<TaskView>,
    /// Results collected so far, by frame: an LLM job's answers.
    #[serde(default)]
    pub outputs: Vec<OutputView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutputView {
    pub frame: u32,
    pub node: NodeId,
    pub prompt: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeView {
    pub info: NodeInfo,
    pub metrics: Option<Metrics>,
    pub busy_tasks: usize,
}

/// A machine waiting for approval, as shown in the dashboard.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PendingDevice {
    /// Identifies the request in the approve and ignore endpoints.
    pub public_key: String,
    /// Six digits the machine also shows; compare before approving.
    pub code: String,
    pub info: NodeInfo,
    /// IP address the request came from, when known.
    pub address: Option<String>,
}

/// An approved machine, whether or not it is connected right now.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PairedDevice {
    pub id: NodeId,
    /// Hostname at approval time.
    pub name: String,
}

/// Full cluster state served to UI clients.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ClusterSnapshot {
    /// Approved machines that are connected now.
    pub nodes: Vec<NodeView>,
    pub jobs: Vec<JobView>,
    pub pending: Vec<PendingDevice>,
    pub devices: Vec<PairedDevice>,
    /// Long-document summaries, oldest first.
    #[serde(default)]
    pub summaries: Vec<SummaryView>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SummaryId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SummaryState {
    Running,
    Done,
    Failed,
}

/// A long document summarized in stages: its parts are summarized across
/// machines as one LLM job, then those summaries are combined (in more
/// jobs if they are still long) into one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SummaryView {
    pub summary_id: SummaryId,
    pub title: String,
    pub model: String,
    /// Characters in the document.
    pub chars: u32,
    /// Parts the document was cut into.
    pub parts: u32,
    /// The jobs so far, one per stage; the first summarizes the parts.
    pub jobs: Vec<JobId>,
    pub state: SummaryState,
    pub result: Option<String>,
    pub error: Option<String>,
}

/// Request body for summarizing a long text across the machines.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubmitSummary {
    pub model: String,
    /// Shown in the dashboard and given to the model, e.g. a file name.
    pub title: String,
    pub text: String,
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
    fn hello_omits_a_missing_token_and_handshake_variants_are_tagged() {
        let hello = AgentMessage::Hello {
            protocol: PROTOCOL_VERSION,
            public_key: "ab".into(),
            info: NodeInfo {
                id: NodeId("a".into()),
                hostname: "h".into(),
                os: "o".into(),
                arch: "x".into(),
                cpu_cores: 1,
                total_memory_bytes: 1,
                runners: vec![],
                llm_models: vec![],
            },
            token: None,
        };
        let json = serde_json::to_string(&hello).unwrap();
        assert!(!json.contains("token"));
        assert_eq!(serde_json::from_str::<AgentMessage>(&json).unwrap(), hello);
        assert_eq!(
            serde_json::to_string(&MasterMessage::Pending).unwrap(),
            r#"{"type":"pending"}"#
        );
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
    fn llm_frames_are_prompt_numbers() {
        let p = TaskPayload::Llm {
            model: "llama3.2:1b".into(),
            prompts: vec!["a".into(), "b".into(), "c".into()],
            frame_start: 1,
            frame_end: 3,
            first_prompt: 1,
            max_tokens: Some(50),
        };
        assert_eq!(p.kind(), RunnerKind::Llm);
        assert_eq!(p.llm_model(), Some("llama3.2:1b"));
        let piece = p.with_frames(2, 3);
        assert_eq!(piece.frames(), (2, 3));
        assert_eq!(piece.prompt(2), Some("b"));
        assert_eq!(piece.prompt(3), Some("c"));
        assert_eq!(piece.prompt(1), None, "not this piece's prompt");
        assert_eq!(piece.prompt(0), None);
        assert_eq!(piece.prompt(4), None);
        let TaskPayload::Llm {
            prompts,
            first_prompt,
            max_tokens,
            ..
        } = &piece
        else {
            unreachable!()
        };
        assert_eq!(prompts, &["b".to_owned(), "c".to_owned()], "only its share");
        assert_eq!((*first_prompt, *max_tokens), (2, Some(50)));
        // A piece of a piece still lines up.
        assert_eq!(piece.with_frames(3, 3).prompt(3), Some("c"));
        let json = serde_json::to_string(&piece).unwrap();
        assert!(json.contains(r#""runner":"llm""#));
        assert_eq!(serde_json::from_str::<TaskPayload>(&json).unwrap(), piece);
    }

    #[test]
    fn older_llm_payloads_start_at_prompt_one() {
        let json =
            r#"{"runner":"llm","model":"m","prompts":["a","b"],"frame_start":1,"frame_end":2}"#;
        let p: TaskPayload = serde_json::from_str(json).unwrap();
        assert_eq!(p.prompt(1), Some("a"));
        assert!(!serde_json::to_string(&p).unwrap().contains("max_tokens"));
        let snap: ClusterSnapshot =
            serde_json::from_str(r#"{"nodes":[],"jobs":[],"pending":[],"devices":[]}"#).unwrap();
        assert!(snap.summaries.is_empty());
    }

    #[test]
    fn machines_without_models_still_parse() {
        let json = r#"{"id":"a","hostname":"h","os":"o","arch":"x","cpu_cores":1,"total_memory_bytes":1,"runners":["fake"]}"#;
        let info: NodeInfo = serde_json::from_str(json).unwrap();
        assert!(info.llm_models.is_empty());
        assert!(!serde_json::to_string(&info).unwrap().contains("llm_models"));
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
