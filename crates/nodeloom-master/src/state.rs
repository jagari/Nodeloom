//! Cluster state and scheduling. Everything here is synchronous and
//! lock-based so it can be unit-tested without a network.

use std::{
    collections::{BTreeMap, HashMap},
    sync::{Mutex, MutexGuard},
};

use nodeloom_core::{split::split_frames, validate::validate_payload};
use nodeloom_proto::{
    ClusterSnapshot, JobId, JobView, MasterMessage, Metrics, NodeId, NodeInfo, NodeView,
    RunnerKind, TaskId, TaskPayload, TaskSpec, TaskState, TaskView,
};
use tokio::sync::{broadcast, mpsc};

/// Jobs kept in memory; the oldest are dropped first.
const MAX_JOBS: usize = 100;

pub type NodeTx = mpsc::UnboundedSender<MasterMessage>;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SubmitError {
    #[error("invalid job: {0}")]
    Invalid(String),
    #[error("no connected node can run {0:?} tasks")]
    NoCapableNodes(RunnerKind),
}

struct NodeConn {
    info: NodeInfo,
    metrics: Option<Metrics>,
    tx: NodeTx,
    generation: u64,
}

struct TaskEntry {
    spec: TaskSpec,
    node: Option<NodeId>,
    frames_done: u32,
    state: TaskState,
    error: Option<String>,
}

impl TaskEntry {
    fn view(&self) -> TaskView {
        let (frame_start, frame_end) = self.spec.payload.frames();
        TaskView {
            task_id: self.spec.task_id,
            node: self.node.clone(),
            frame_start,
            frame_end,
            frames_done: self.frames_done,
            state: self.state,
            error: self.error.clone(),
        }
    }

    fn requeue(&mut self) {
        self.state = TaskState::Pending;
        self.node = None;
        self.frames_done = 0;
    }
}

#[derive(Default)]
struct Inner {
    nodes: HashMap<NodeId, NodeConn>,
    jobs: BTreeMap<JobId, Vec<TaskEntry>>,
    next_job: u64,
    next_task: u64,
    next_generation: u64,
}

impl Inner {
    fn requeue_tasks_of(&mut self, node: &NodeId) {
        for task in self.jobs.values_mut().flatten() {
            if task.state == TaskState::Running && task.node.as_ref() == Some(node) {
                task.requeue();
            }
        }
    }

    /// Hands every pending task to the least-busy node able to run it.
    fn dispatch(&mut self) {
        let mut load: HashMap<NodeId, usize> = HashMap::new();
        for task in self.jobs.values().flatten() {
            if task.state == TaskState::Running
                && let Some(node) = &task.node
            {
                *load.entry(node.clone()).or_default() += 1;
            }
        }

        for task in self.jobs.values_mut().flatten() {
            if task.state != TaskState::Pending {
                continue;
            }
            let kind = task.spec.payload.kind();
            let Some(target) = self
                .nodes
                .values()
                .filter(|n| n.info.runners.contains(&kind))
                .min_by_key(|n| {
                    (
                        load.get(&n.info.id).copied().unwrap_or(0),
                        n.info.id.0.clone(),
                    )
                })
            else {
                continue;
            };
            if target
                .tx
                .send(MasterMessage::AssignTask(task.spec.clone()))
                .is_ok()
            {
                task.state = TaskState::Running;
                task.node = Some(target.info.id.clone());
                *load.entry(target.info.id.clone()).or_default() += 1;
            }
        }
    }

    fn prune(&mut self) {
        while self.jobs.len() > MAX_JOBS {
            // Prefer dropping the oldest job that is no longer active.
            let victim = self
                .jobs
                .iter()
                .find(|(_, tasks)| {
                    tasks
                        .iter()
                        .all(|t| matches!(t.state, TaskState::Done | TaskState::Failed))
                })
                .map(|(id, _)| *id);
            match victim {
                Some(id) => {
                    self.jobs.remove(&id);
                }
                None => break,
            }
        }
    }

    fn running_tasks(&self, node: &NodeId) -> usize {
        self.jobs
            .values()
            .flatten()
            .filter(|t| t.state == TaskState::Running && t.node.as_ref() == Some(node))
            .count()
    }

    /// The task `task_id` if it is currently running on `node`. Agents may
    /// only change tasks assigned to them.
    fn running_task_mut(&mut self, node: &NodeId, task_id: TaskId) -> Option<&mut TaskEntry> {
        self.jobs.values_mut().flatten().find(|t| {
            t.spec.task_id == task_id
                && t.state == TaskState::Running
                && t.node.as_ref() == Some(node)
        })
    }
}

pub struct Cluster {
    inner: Mutex<Inner>,
    changed: broadcast::Sender<()>,
    token: String,
}

impl Cluster {
    pub fn new(token: impl Into<String>) -> Self {
        let (changed, _) = broadcast::channel(16);
        Self {
            inner: Mutex::new(Inner::default()),
            changed,
            token: token.into(),
        }
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    /// Fires whenever the snapshot changes.
    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.changed.subscribe()
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        // A panic while holding the lock cannot leave the maps half-updated in
        // a way later calls cannot handle, so keep serving.
        self.inner.lock().unwrap_or_else(|p| p.into_inner())
    }

    fn notify(&self) {
        let _ = self.changed.send(());
    }

    /// Registers a connected agent and returns a generation number that must
    /// be passed to [`Cluster::unregister_node`].
    pub fn register_node(&self, info: NodeInfo, tx: NodeTx) -> u64 {
        let mut inner = self.lock();
        let id = info.id.clone();
        // A reconnecting agent knows nothing about tasks of its old session.
        inner.requeue_tasks_of(&id);
        inner.next_generation += 1;
        let generation = inner.next_generation;
        inner.nodes.insert(
            id,
            NodeConn {
                info,
                metrics: None,
                tx,
                generation,
            },
        );
        inner.dispatch();
        drop(inner);
        self.notify();
        generation
    }

    /// Removes the node unless a newer connection has already replaced it.
    pub fn unregister_node(&self, id: &NodeId, generation: u64) {
        let mut inner = self.lock();
        if inner.nodes.get(id).map(|n| n.generation) != Some(generation) {
            return;
        }
        inner.nodes.remove(id);
        inner.requeue_tasks_of(id);
        inner.dispatch();
        drop(inner);
        self.notify();
    }

    pub fn update_metrics(&self, id: &NodeId, metrics: Metrics) {
        if let Some(node) = self.lock().nodes.get_mut(id) {
            node.metrics = Some(metrics);
        }
        self.notify();
    }

    pub fn task_progress(&self, node: &NodeId, task_id: TaskId, frames_done: u32) {
        let mut inner = self.lock();
        if let Some(task) = inner.running_task_mut(node, task_id) {
            task.frames_done = frames_done.min(task.spec.payload.frame_count());
        }
        drop(inner);
        self.notify();
    }

    pub fn task_done(&self, node: &NodeId, task_id: TaskId) {
        let mut inner = self.lock();
        if let Some(task) = inner.running_task_mut(node, task_id) {
            task.state = TaskState::Done;
            task.frames_done = task.spec.payload.frame_count();
        }
        drop(inner);
        self.notify();
    }

    pub fn task_failed(&self, node: &NodeId, task_id: TaskId, error: String) {
        let mut inner = self.lock();
        if let Some(task) = inner.running_task_mut(node, task_id) {
            task.state = TaskState::Failed;
            task.error = Some(error);
        }
        drop(inner);
        self.notify();
    }

    /// Splits `template`'s frame range over the connected nodes that can run
    /// it and dispatches the pieces.
    pub fn submit_job(&self, template: TaskPayload) -> Result<JobView, SubmitError> {
        validate_payload(&template).map_err(SubmitError::Invalid)?;
        let kind = template.kind();

        let mut inner = self.lock();
        let capable = inner
            .nodes
            .values()
            .filter(|n| n.info.runners.contains(&kind))
            .count();
        if capable == 0 {
            return Err(SubmitError::NoCapableNodes(kind));
        }

        let (start, end) = template.frames();
        let job_id = JobId(inner.next_job);
        inner.next_job += 1;

        let mut tasks = Vec::new();
        for (from, to) in split_frames(start, end, capable) {
            let task_id = TaskId(inner.next_task);
            inner.next_task += 1;
            tasks.push(TaskEntry {
                spec: TaskSpec {
                    job_id,
                    task_id,
                    payload: template.with_frames(from, to),
                },
                node: None,
                frames_done: 0,
                state: TaskState::Pending,
                error: None,
            });
        }
        inner.jobs.insert(job_id, tasks);
        inner.prune();
        inner.dispatch();

        let view = job_view(job_id, &inner.jobs[&job_id]);
        drop(inner);
        self.notify();
        Ok(view)
    }

    pub fn snapshot(&self) -> ClusterSnapshot {
        let inner = self.lock();
        let mut nodes: Vec<NodeView> = inner
            .nodes
            .values()
            .map(|n| NodeView {
                info: n.info.clone(),
                metrics: n.metrics,
                busy_tasks: inner.running_tasks(&n.info.id),
            })
            .collect();
        nodes.sort_by(|a, b| a.info.id.0.cmp(&b.info.id.0));
        let jobs = inner
            .jobs
            .iter()
            .map(|(id, tasks)| job_view(*id, tasks))
            .collect();
        ClusterSnapshot { nodes, jobs }
    }
}

fn job_view(job_id: JobId, tasks: &[TaskEntry]) -> JobView {
    JobView {
        job_id,
        tasks: tasks.iter().map(TaskEntry::view).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str, runners: &[RunnerKind]) -> NodeInfo {
        NodeInfo {
            id: NodeId(id.into()),
            hostname: id.into(),
            os: "test".into(),
            arch: "test".into(),
            cpu_cores: 4,
            total_memory_bytes: 1 << 30,
            runners: runners.to_vec(),
        }
    }

    fn fake(start: u32, end: u32) -> TaskPayload {
        TaskPayload::Fake {
            frame_start: start,
            frame_end: end,
            millis_per_frame: 0,
        }
    }

    fn connect(
        cluster: &Cluster,
        id: &str,
        runners: &[RunnerKind],
    ) -> (u64, mpsc::UnboundedReceiver<MasterMessage>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (cluster.register_node(node(id, runners), tx), rx)
    }

    fn assigned(rx: &mut mpsc::UnboundedReceiver<MasterMessage>) -> Vec<TaskSpec> {
        let mut out = Vec::new();
        while let Ok(MasterMessage::AssignTask(spec)) = rx.try_recv() {
            out.push(spec);
        }
        out
    }

    #[test]
    fn job_is_split_across_capable_nodes_only() {
        let cluster = Cluster::new("t");
        let (_, mut a) = connect(&cluster, "a", &[RunnerKind::Fake]);
        let (_, mut b) = connect(&cluster, "b", &[RunnerKind::Fake]);
        let (_, mut c) = connect(&cluster, "c", &[RunnerKind::Blender]);

        let job = cluster.submit_job(fake(1, 10)).unwrap();
        assert_eq!(job.tasks.len(), 2);

        let (ta, tb) = (assigned(&mut a), assigned(&mut b));
        assert_eq!((ta.len(), tb.len()), (1, 1));
        assert!(assigned(&mut c).is_empty());
        // Together the tasks cover 1..=10 exactly once.
        let mut ranges = [ta[0].payload.frames(), tb[0].payload.frames()];
        ranges.sort();
        assert_eq!(ranges, [(1, 5), (6, 10)]);
    }

    #[test]
    fn submit_without_capable_node_fails() {
        let cluster = Cluster::new("t");
        connect(&cluster, "a", &[RunnerKind::Blender]);
        assert_eq!(
            cluster.submit_job(fake(1, 3)).unwrap_err(),
            SubmitError::NoCapableNodes(RunnerKind::Fake)
        );
    }

    #[test]
    fn invalid_job_is_rejected() {
        let cluster = Cluster::new("t");
        connect(&cluster, "a", &[RunnerKind::Fake]);
        assert!(matches!(
            cluster.submit_job(fake(9, 1)),
            Err(SubmitError::Invalid(_))
        ));
    }

    #[test]
    fn progress_and_completion_are_tracked_per_node() {
        let cluster = Cluster::new("t");
        let (_, mut a) = connect(&cluster, "a", &[RunnerKind::Fake]);
        cluster.submit_job(fake(1, 4)).unwrap();
        let task = assigned(&mut a).remove(0);

        cluster.task_progress(&NodeId("a".into()), task.task_id, 2);
        let t = &cluster.snapshot().jobs[0].tasks[0];
        assert_eq!((t.state, t.frames_done), (TaskState::Running, 2));

        cluster.task_done(&NodeId("a".into()), task.task_id);
        let t = &cluster.snapshot().jobs[0].tasks[0];
        assert_eq!((t.state, t.frames_done), (TaskState::Done, 4));
    }

    #[test]
    fn a_node_cannot_touch_another_nodes_task() {
        let cluster = Cluster::new("t");
        let (_, mut a) = connect(&cluster, "a", &[RunnerKind::Fake]);
        connect(&cluster, "b", &[RunnerKind::Blender]);
        cluster.submit_job(fake(1, 4)).unwrap();
        let task = assigned(&mut a).remove(0);

        cluster.task_done(&NodeId("b".into()), task.task_id);
        cluster.task_failed(&NodeId("b".into()), task.task_id, "x".into());
        assert_eq!(
            cluster.snapshot().jobs[0].tasks[0].state,
            TaskState::Running
        );
    }

    #[test]
    fn progress_is_clamped_to_task_size() {
        let cluster = Cluster::new("t");
        let (_, mut a) = connect(&cluster, "a", &[RunnerKind::Fake]);
        cluster.submit_job(fake(1, 4)).unwrap();
        let task = assigned(&mut a).remove(0);
        cluster.task_progress(&NodeId("a".into()), task.task_id, 9999);
        assert_eq!(cluster.snapshot().jobs[0].tasks[0].frames_done, 4);
    }

    #[test]
    fn tasks_of_a_disconnected_node_move_to_another_node() {
        let cluster = Cluster::new("t");
        let (gen_a, mut a) = connect(&cluster, "a", &[RunnerKind::Fake]);
        let (_, mut b) = connect(&cluster, "b", &[RunnerKind::Fake]);
        cluster.submit_job(fake(1, 10)).unwrap();
        assert_eq!(assigned(&mut a).len(), 1);
        assert_eq!(assigned(&mut b).len(), 1);

        cluster.unregister_node(&NodeId("a".into()), gen_a);

        // a's task was re-sent to b, which now has both.
        let moved = assigned(&mut b);
        assert_eq!(moved.len(), 1);
        let snap = cluster.snapshot();
        assert_eq!(snap.nodes.len(), 1);
        assert!(
            snap.jobs[0]
                .tasks
                .iter()
                .all(|t| t.node == Some(NodeId("b".into())) && t.state == TaskState::Running)
        );
    }

    #[test]
    fn stale_unregister_does_not_remove_a_reconnected_node() {
        let cluster = Cluster::new("t");
        let (old_gen, _old_rx) = connect(&cluster, "a", &[RunnerKind::Fake]);
        let (_new_gen, _new_rx) = connect(&cluster, "a", &[RunnerKind::Fake]);
        cluster.unregister_node(&NodeId("a".into()), old_gen);
        assert_eq!(cluster.snapshot().nodes.len(), 1);
    }

    #[test]
    fn reconnect_requeues_tasks_of_the_old_session() {
        let cluster = Cluster::new("t");
        let (_, mut old) = connect(&cluster, "a", &[RunnerKind::Fake]);
        cluster.submit_job(fake(1, 4)).unwrap();
        assert_eq!(assigned(&mut old).len(), 1);

        let (_, mut fresh) = connect(&cluster, "a", &[RunnerKind::Fake]);
        assert_eq!(assigned(&mut fresh).len(), 1, "task must be re-sent");
    }

    #[test]
    fn pending_tasks_start_when_a_node_returns() {
        let cluster = Cluster::new("t");
        let (gen_a, _a) = connect(&cluster, "a", &[RunnerKind::Fake]);
        cluster.submit_job(fake(1, 4)).unwrap();
        cluster.unregister_node(&NodeId("a".into()), gen_a);
        assert_eq!(
            cluster.snapshot().jobs[0].tasks[0].state,
            TaskState::Pending
        );

        let (_, mut again) = connect(&cluster, "a", &[RunnerKind::Fake]);
        assert_eq!(assigned(&mut again).len(), 1);
    }

    #[test]
    fn old_finished_jobs_are_pruned() {
        let cluster = Cluster::new("t");
        let (_, mut a) = connect(&cluster, "a", &[RunnerKind::Fake]);
        let id = NodeId("a".into());
        for _ in 0..(MAX_JOBS + 5) {
            cluster.submit_job(fake(1, 1)).unwrap();
            for spec in assigned(&mut a) {
                cluster.task_done(&id, spec.task_id);
            }
        }
        let snap = cluster.snapshot();
        assert_eq!(snap.jobs.len(), MAX_JOBS);
        assert_eq!(snap.jobs.last().unwrap().job_id, JobId(MAX_JOBS as u64 + 4));
    }
}
