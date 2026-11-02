// Mirrors crates/nodeloom-proto. Newtype ids (NodeId, JobId, TaskId)
// serialize as their inner value, enums use the tags shown here.

export type NodeId = string;
export type RunnerKind = 'fake' | 'blender' | 'llm';
export type TaskState = 'pending' | 'running' | 'done' | 'failed';

export interface NodeInfo {
  id: NodeId;
  hostname: string;
  os: string;
  arch: string;
  cpu_cores: number;
  total_memory_bytes: number;
  runners: RunnerKind[];
  /** Models of the local LLM runner; absent without one. */
  llm_models?: string[];
}

export interface Metrics {
  cpu_percent: number;
  memory_used_bytes: number;
  memory_total_bytes: number;
  timestamp_ms: number;
}

export interface TaskView {
  task_id: number;
  node: NodeId | null;
  frame_start: number;
  frame_end: number;
  frames_done: number;
  state: TaskState;
  error: string | null;
}

export interface JobView {
  job_id: number;
  runner: RunnerKind;
  tasks: TaskView[];
  /** What frames produced so far: an LLM job's answers, by frame. */
  outputs: OutputView[];
}

export interface OutputView {
  frame: number;
  node: NodeId;
  prompt: string;
  text: string;
}

export interface NodeView {
  info: NodeInfo;
  metrics: Metrics | null;
  busy_tasks: number;
}

/** A machine waiting for approval. */
export interface PendingDevice {
  /** Identifies the request in the approve and ignore endpoints. */
  public_key: string;
  /** Six digits the machine also shows, e.g. "307 039". */
  code: string;
  info: NodeInfo;
  address: string | null;
}

/** An approved machine, connected or not. */
export interface PairedDevice {
  id: NodeId;
  /** Hostname at approval time. */
  name: string;
}

export interface ClusterSnapshot {
  /** Approved machines that are connected now. */
  nodes: NodeView[];
  jobs: JobView[];
  pending: PendingDevice[];
  devices: PairedDevice[];
  /** Long-document summaries, oldest first. Absent from older masters. */
  summaries?: SummaryView[];
}

export type SummaryState = 'running' | 'done' | 'failed';

/**
 * A long document summarized in rounds: its parts across machines as one
 * LLM job, then the part summaries combined (in more jobs while long).
 */
export interface SummaryView {
  summary_id: number;
  title: string;
  model: string;
  /** Characters in the document. */
  chars: number;
  /** Parts the document was cut into. */
  parts: number;
  /** One job per round; the first summarizes the parts. */
  jobs: number[];
  state: SummaryState;
  result: string | null;
  error: string | null;
}

/** Body of `POST /api/summaries`. */
export interface SubmitSummary {
  model: string;
  /** Shown in the dashboard and given to the model, e.g. a file name. */
  title: string;
  text: string;
}

export type TaskPayload =
  | { runner: 'fake'; frame_start: number; frame_end: number; millis_per_frame: number }
  | { runner: 'blender'; blend_file: string; frame_start: number; frame_end: number }
  | {
      runner: 'llm';
      model: string;
      prompts: string[];
      frame_start: number;
      frame_end: number;
      /** Number of `prompts[0]`; 1 when absent. Pieces carry only their own prompts. */
      first_prompt?: number;
      /** Longest answer in tokens; the runner's default when absent. */
      max_tokens?: number;
    };
