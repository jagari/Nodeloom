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

export interface ClusterSnapshot {
  nodes: NodeView[];
  jobs: JobView[];
}

export type TaskPayload =
  | { runner: 'fake'; frame_start: number; frame_end: number; millis_per_frame: number }
  | { runner: 'blender'; blend_file: string; frame_start: number; frame_end: number };
