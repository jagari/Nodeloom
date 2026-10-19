// Mirrors crates/nodeloom-proto. Newtype ids (NodeId, JobId, TaskId)
// serialize as their inner value, enums use the tags shown here.

export type NodeId = string;
export type RunnerKind = 'fake' | 'blender';
export type TaskState = 'pending' | 'running' | 'done' | 'failed';

export interface NodeInfo {
  id: NodeId;
  hostname: string;
  os: string;
  arch: string;
  cpu_cores: number;
  total_memory_bytes: number;
  runners: RunnerKind[];
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
  tasks: TaskView[];
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
}

export type TaskPayload =
  | { runner: 'fake'; frame_start: number; frame_end: number; millis_per_frame: number }
  | { runner: 'blender'; blend_file: string; frame_start: number; frame_end: number };
