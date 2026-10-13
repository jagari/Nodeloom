//! Cluster state, pairing and scheduling. Everything here is synchronous and
//! lock-based so it can be unit-tested without a network.

use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    net::IpAddr,
    sync::{Mutex, MutexGuard},
    time::Duration,
};

use nodeloom_core::{
    auth::tokens_match, identity::PublicKey, split::split_frames, validate::validate_payload,
};
use nodeloom_proto::{
    ClusterSnapshot, JobId, JobView, MasterMessage, Metrics, NodeId, NodeInfo, NodeView,
    PairedDevice, PendingDevice, RunnerKind, TaskId, TaskPayload, TaskSpec, TaskState, TaskView,
};
use tokio::sync::{broadcast, mpsc, oneshot};

use crate::devices::{ApprovedDevice, DeviceStore, Devices, StoreError};

/// Jobs kept in memory; the oldest are dropped first.
const MAX_JOBS: usize = 100;

/// Reason sent to a machine whose key was removed from the cluster.
pub const REMOVED: &str = "this machine was removed from the cluster; run \
                           `nodeloom-agent reset-identity` on it and approve it again";
const IGNORED: &str = "the cluster owner ignored this machine; it can ask again after \
                       the master restarts";

pub type NodeTx = mpsc::UnboundedSender<MasterMessage>;

/// Limits that keep unapproved machines from crowding the dashboard, and
/// how long a silent pending connection is kept.
#[derive(Debug, Clone)]
pub struct Settings {
    pub max_pending: usize,
    /// One address may hold this many requests; a newer one replaces the
    /// oldest, so one noisy host cannot lock everyone else out.
    pub max_pending_per_address: usize,
    /// Ignored keys remembered until restart; the oldest are forgotten.
    pub max_ignored: usize,
    pub ping_interval: Duration,
    /// A pending connection that sends nothing (not even a pong) for this
    /// long is dropped, so dead peers do not linger in the list.
    pub pending_idle_timeout: Duration,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            max_pending: 32,
            max_pending_per_address: 2,
            max_ignored: 256,
            ping_interval: Duration::from_secs(15),
            pending_idle_timeout: Duration::from_secs(45),
        }
    }
}

/// What to do with a machine that proved its key.
pub enum Admission<'a> {
    Approved,
    /// It sent the right admin token; approve it without asking.
    ApproveWithToken,
    /// Wait for a person. Dropping the ticket withdraws the request.
    Pending(PendingTicket<'a>, oneshot::Receiver<Decision>),
    Rejected(String),
}

#[derive(Debug, PartialEq, Eq)]
pub enum Decision {
    Approved,
    Rejected(String),
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PairError {
    #[error("no such machine")]
    NotFound,
    #[error("{0}")]
    Conflict(String),
    #[error("could not save the device list: {0}")]
    Storage(String),
}

impl From<StoreError> for PairError {
    fn from(e: StoreError) -> Self {
        PairError::Storage(e.to_string())
    }
}

struct PendingEntry {
    seq: u64,
    info: NodeInfo,
    code: String,
    address: Option<IpAddr>,
    decision: oneshot::Sender<Decision>,
}

impl PendingEntry {
    fn decide(self, decision: Decision) {
        // The connection may already be gone; nothing to tell then.
        let _ = self.decision.send(decision);
    }
}

/// Keeps a pending request alive; dropping it removes the request.
pub struct PendingTicket<'a> {
    cluster: &'a Cluster,
    public_key: String,
    seq: u64,
}

impl Drop for PendingTicket<'_> {
    fn drop(&mut self) {
        let mut inner = self.cluster.lock();
        if inner.pending.get(&self.public_key).map(|p| p.seq) == Some(self.seq) {
            inner.pending.remove(&self.public_key);
            drop(inner);
            self.cluster.notify();
        }
    }
}

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
    /// Ends the session with this reason (sent as `Rejected`).
    kick: oneshot::Sender<String>,
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
    /// Changed only while holding `Cluster::store`, so saves never race.
    devices: Devices,
    /// Machines waiting for approval, by public key.
    pending: HashMap<String, PendingEntry>,
    next_pending: u64,
    ignored: VecDeque<String>,
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

    /// Once `id` is paired, anyone else waiting under that id is an impostor
    /// or a stale copy.
    fn reject_pending_claims(&mut self, id: &NodeId) {
        let clashing: Vec<String> = self
            .pending
            .iter()
            .filter(|(_, p)| p.info.id == *id)
            .map(|(k, _)| k.clone())
            .collect();
        for key in clashing {
            if let Some(entry) = self.pending.remove(&key) {
                entry.decide(Decision::Rejected(format!(
                    "another machine was approved as {id}"
                )));
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
    /// Held across a whole approve or remove (save included), always before
    /// `inner`. Disk writes happen here, never under `inner`.
    store: Mutex<DeviceStore>,
    changed: broadcast::Sender<()>,
    admin_token: String,
    settings: Settings,
}

impl Cluster {
    /// A cluster that remembers paired machines only in memory.
    pub fn new(admin_token: impl Into<String>) -> Self {
        Self::with_store(admin_token, DeviceStore::in_memory())
    }

    pub fn with_store(admin_token: impl Into<String>, mut store: DeviceStore) -> Self {
        let (changed, _) = broadcast::channel(16);
        let inner = Inner {
            devices: store.take_loaded(),
            ..Inner::default()
        };
        Self {
            inner: Mutex::new(inner),
            store: Mutex::new(store),
            changed,
            admin_token: admin_token.into(),
            settings: Settings::default(),
        }
    }

    pub fn with_settings(mut self, settings: Settings) -> Self {
        self.settings = settings;
        self
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    /// Token for the dashboard API (and, optionally, for agents to skip
    /// approval).
    pub fn admin_token(&self) -> &str {
        &self.admin_token
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

    fn lock_store(&self) -> MutexGuard<'_, DeviceStore> {
        self.store.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// Decides what happens to a machine that has proved `public_key`. A
    /// machine that must wait is put on the pending list in the same step,
    /// so nothing can approve or remove it in between.
    pub fn classify(
        &self,
        info: &NodeInfo,
        public_key: &PublicKey,
        token: Option<&str>,
        address: Option<IpAddr>,
    ) -> Admission<'_> {
        let key = public_key.as_hex();
        let mut inner = self.lock();
        let devices = &inner.devices;
        if devices.revoked.contains(key) {
            return Admission::Rejected(REMOVED.into());
        }
        match devices.approved.get(&info.id) {
            Some(d) if d.public_key == key => return Admission::Approved,
            Some(_) => {
                return Admission::Rejected(format!(
                    "node id {} is already paired with another key; remove it in the \
                     dashboard first",
                    info.id
                ));
            }
            None => {}
        }
        if let Some(other) = devices.id_of_key(key) {
            return Admission::Rejected(format!("this key is already paired as {other}"));
        }
        if let Some(token) = token {
            return if tokens_match(&self.admin_token, token) {
                Admission::ApproveWithToken
            } else {
                Admission::Rejected("wrong token".into())
            };
        }
        if inner.ignored.iter().any(|k| k == key) {
            return Admission::Rejected(IGNORED.into());
        }

        // The same machine reconnecting replaces its own older request.
        if let Some(old) = inner.pending.remove(key) {
            old.decide(Decision::Rejected(
                "replaced by a newer connection from this machine".into(),
            ));
        }
        if let Some(address) = address {
            let mut same: Vec<(u64, String)> = inner
                .pending
                .iter()
                .filter(|(_, p)| p.address == Some(address))
                .map(|(k, p)| (p.seq, k.clone()))
                .collect();
            same.sort();
            let excess =
                (same.len() + 1).saturating_sub(self.settings.max_pending_per_address.max(1));
            for (_, old) in same.into_iter().take(excess) {
                if let Some(old) = inner.pending.remove(&old) {
                    old.decide(Decision::Rejected(
                        "replaced by a newer request from the same address".into(),
                    ));
                }
            }
        }
        if inner.pending.len() >= self.settings.max_pending {
            return Admission::Rejected("too many machines are waiting for approval".into());
        }

        inner.next_pending += 1;
        let seq = inner.next_pending;
        let (decision, receiver) = oneshot::channel();
        inner.pending.insert(
            key.to_owned(),
            PendingEntry {
                seq,
                info: info.clone(),
                code: public_key.confirmation_code(),
                address,
                decision,
            },
        );
        drop(inner);
        self.notify();
        Admission::Pending(
            PendingTicket {
                cluster: self,
                public_key: key.to_owned(),
                seq,
            },
            receiver,
        )
    }

    /// Approves a waiting machine: saves it, then lets its connection in.
    pub fn approve(&self, public_key: &str) -> Result<(), PairError> {
        let store = self.lock_store();
        let (info, mut devices) = {
            let inner = self.lock();
            let entry = inner.pending.get(public_key).ok_or(PairError::NotFound)?;
            (entry.info.clone(), inner.devices.clone())
        };
        add_device(&mut devices, &info, public_key)?;
        store.save(&devices)?;

        let mut inner = self.lock();
        inner.devices = devices;
        if let Some(entry) = inner.pending.remove(public_key) {
            entry.decide(Decision::Approved);
        }
        inner.reject_pending_claims(&info.id);
        drop(inner);
        self.notify();
        Ok(())
    }

    /// Approves a machine that presented the admin token.
    pub fn approve_with_token(&self, info: &NodeInfo, public_key: &str) -> Result<(), PairError> {
        let store = self.lock_store();
        let mut devices = self.lock().devices.clone();
        if devices.is_approved(&info.id, public_key) {
            return Ok(());
        }
        add_device(&mut devices, info, public_key)?;
        store.save(&devices)?;
        let mut inner = self.lock();
        inner.devices = devices;
        inner.reject_pending_claims(&info.id);
        drop(inner);
        self.notify();
        Ok(())
    }

    /// Refuses a waiting machine until the master restarts.
    pub fn ignore(&self, public_key: &str) -> Result<(), PairError> {
        let mut inner = self.lock();
        let entry = inner
            .pending
            .remove(public_key)
            .ok_or(PairError::NotFound)?;
        entry.decide(Decision::Rejected(IGNORED.into()));
        inner.ignored.push_back(public_key.to_owned());
        while inner.ignored.len() > self.settings.max_ignored {
            inner.ignored.pop_front();
        }
        drop(inner);
        self.notify();
        Ok(())
    }

    /// Forgets a paired machine and refuses its key for good: disconnects it
    /// now and hands its tasks to other machines.
    pub fn remove_device(&self, id: &NodeId) -> Result<(), PairError> {
        let store = self.lock_store();
        let mut devices = self.lock().devices.clone();
        let removed = devices.approved.remove(id).ok_or(PairError::NotFound)?;
        devices.revoked.insert(removed.public_key);
        store.save(&devices)?;

        let mut inner = self.lock();
        inner.devices = devices;
        if let Some(conn) = inner.nodes.remove(id) {
            let _ = conn.kick.send(REMOVED.into());
        }
        inner.requeue_tasks_of(id);
        inner.dispatch();
        drop(inner);
        self.notify();
        Ok(())
    }

    /// Registers the connection of an approved machine and returns a
    /// generation number for [`Cluster::unregister_node`]. Approval is
    /// checked again under the same lock that removal takes, so a machine
    /// removed a moment ago cannot slip in.
    pub fn admit(
        &self,
        info: NodeInfo,
        public_key: &str,
        tx: NodeTx,
        kick: oneshot::Sender<String>,
    ) -> Result<u64, String> {
        let mut inner = self.lock();
        if !inner.devices.is_approved(&info.id, public_key) {
            return Err(if inner.devices.revoked.contains(public_key) {
                REMOVED.into()
            } else {
                "this machine is not approved".into()
            });
        }
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
                kick,
                generation,
            },
        );
        inner.dispatch();
        drop(inner);
        self.notify();
        Ok(generation)
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
        let mut pending: Vec<(u64, PendingDevice)> = inner
            .pending
            .iter()
            .map(|(key, p)| {
                (
                    p.seq,
                    PendingDevice {
                        public_key: key.clone(),
                        code: p.code.clone(),
                        info: p.info.clone(),
                        address: p.address.map(|a| a.to_string()),
                    },
                )
            })
            .collect();
        pending.sort_by_key(|(seq, _)| *seq);
        let devices = inner
            .devices
            .approved
            .values()
            .map(|d| PairedDevice {
                id: d.id.clone(),
                name: d.name.clone(),
            })
            .collect();
        ClusterSnapshot {
            nodes,
            jobs,
            pending: pending.into_iter().map(|(_, p)| p).collect(),
            devices,
        }
    }
}

/// Adds `info` as approved under `public_key`, unless its id or key is
/// already taken by something else.
fn add_device(devices: &mut Devices, info: &NodeInfo, public_key: &str) -> Result<(), PairError> {
    if devices.revoked.contains(public_key) {
        return Err(PairError::Conflict(REMOVED.into()));
    }
    if let Some(existing) = devices.approved.get(&info.id)
        && existing.public_key != public_key
    {
        return Err(PairError::Conflict(format!(
            "{} is already paired with another key; remove it first",
            info.id
        )));
    }
    if let Some(other) = devices.id_of_key(public_key)
        && *other != info.id
    {
        return Err(PairError::Conflict(format!(
            "this key is already paired as {other}"
        )));
    }
    devices.approved.insert(
        info.id.clone(),
        ApprovedDevice {
            id: info.id.clone(),
            name: info.hostname.clone(),
            public_key: public_key.to_owned(),
        },
    );
    Ok(())
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

    fn key_of(id: &str) -> String {
        format!("key-{id}")
    }

    /// Pairs `id` (if needed) and connects it.
    fn connect(
        cluster: &Cluster,
        id: &str,
        runners: &[RunnerKind],
    ) -> (u64, mpsc::UnboundedReceiver<MasterMessage>) {
        let info = node(id, runners);
        cluster.approve_with_token(&info, &key_of(id)).unwrap();
        let (tx, rx) = mpsc::unbounded_channel();
        let (kick, _) = oneshot::channel();
        (cluster.admit(info, &key_of(id), tx, kick).unwrap(), rx)
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

    // --- pairing ---

    use nodeloom_core::identity::DeviceKey;
    use std::net::Ipv4Addr;

    fn new_key() -> PublicKey {
        DeviceKey::generate().unwrap().public_key()
    }

    fn ip(last: u8) -> Option<IpAddr> {
        Some(IpAddr::V4(Ipv4Addr::new(192, 168, 1, last)))
    }

    fn pending_keys(cluster: &Cluster) -> Vec<String> {
        cluster
            .snapshot()
            .pending
            .into_iter()
            .map(|p| p.public_key)
            .collect()
    }

    #[test]
    fn unknown_machine_waits_and_approval_lets_it_in() {
        let cluster = Cluster::new("admin");
        let key = new_key();
        let info = node("a", &[RunnerKind::Fake]);
        let Admission::Pending(ticket, mut decision) = cluster.classify(&info, &key, None, ip(5))
        else {
            panic!("expected pending");
        };
        let snap = cluster.snapshot();
        assert_eq!(snap.pending.len(), 1);
        assert_eq!(snap.pending[0].code, key.confirmation_code());
        assert_eq!(snap.pending[0].address.as_deref(), Some("192.168.1.5"));
        assert!(snap.nodes.is_empty() && snap.devices.is_empty());

        // A pending machine is not a node: jobs cannot use it.
        assert_eq!(
            cluster.submit_job(fake(1, 2)).unwrap_err(),
            SubmitError::NoCapableNodes(RunnerKind::Fake)
        );

        cluster.approve(key.as_hex()).unwrap();
        assert_eq!(decision.try_recv(), Ok(Decision::Approved));
        drop(ticket);
        let snap = cluster.snapshot();
        assert!(snap.pending.is_empty());
        assert_eq!(snap.devices.len(), 1);
        assert!(matches!(
            cluster.classify(&info, &key, None, ip(5)),
            Admission::Approved
        ));
    }

    #[test]
    fn dropping_the_ticket_withdraws_the_request() {
        let cluster = Cluster::new("admin");
        let admission = cluster.classify(&node("a", &[]), &new_key(), None, None);
        assert_eq!(cluster.snapshot().pending.len(), 1);
        drop(admission);
        assert!(cluster.snapshot().pending.is_empty());
    }

    #[test]
    fn token_decides_without_waiting() {
        let cluster = Cluster::new("admin");
        let info = node("a", &[]);
        assert!(matches!(
            cluster.classify(&info, &new_key(), Some("admin"), None),
            Admission::ApproveWithToken
        ));
        assert!(matches!(
            cluster.classify(&info, &new_key(), Some("nope"), None),
            Admission::Rejected(r) if r == "wrong token"
        ));
        assert!(cluster.snapshot().pending.is_empty());
    }

    #[test]
    fn a_paired_id_with_another_key_is_rejected_and_cannot_be_approved() {
        let cluster = Cluster::new("admin");
        let info = node("a", &[]);
        cluster
            .approve_with_token(&info, new_key().as_hex())
            .unwrap();
        let impostor = new_key();
        assert!(matches!(
            cluster.classify(&info, &impostor, None, None),
            Admission::Rejected(_)
        ));
        assert!(matches!(
            cluster.classify(&info, &impostor, Some("admin"), None),
            Admission::Rejected(_)
        ));
        assert!(matches!(
            cluster.approve_with_token(&info, impostor.as_hex()),
            Err(PairError::Conflict(_))
        ));
    }

    #[test]
    fn approving_one_claimant_rejects_others_with_the_same_id() {
        let cluster = Cluster::new("admin");
        let info = node("a", &[]);
        let (real, fake_key) = (new_key(), new_key());
        let Admission::Pending(_t1, mut d1) = cluster.classify(&info, &real, None, ip(1)) else {
            panic!()
        };
        let Admission::Pending(_t2, mut d2) = cluster.classify(&info, &fake_key, None, ip(2))
        else {
            panic!()
        };
        cluster.approve(real.as_hex()).unwrap();
        assert_eq!(d1.try_recv(), Ok(Decision::Approved));
        assert!(matches!(d2.try_recv(), Ok(Decision::Rejected(_))));
        assert!(pending_keys(&cluster).is_empty());
        assert_eq!(cluster.approve(fake_key.as_hex()), Err(PairError::NotFound));
    }

    #[test]
    fn ignored_machine_is_refused_until_restart() {
        let cluster = Cluster::new("admin");
        let key = new_key();
        let info = node("a", &[]);
        let Admission::Pending(_t, mut d) = cluster.classify(&info, &key, None, None) else {
            panic!()
        };
        cluster.ignore(key.as_hex()).unwrap();
        assert!(matches!(d.try_recv(), Ok(Decision::Rejected(_))));
        assert!(matches!(
            cluster.classify(&info, &key, None, None),
            Admission::Rejected(_)
        ));
        assert!(pending_keys(&cluster).is_empty());
        assert_eq!(cluster.ignore(key.as_hex()), Err(PairError::NotFound));
    }

    #[test]
    fn same_machine_replaces_its_request_and_one_address_holds_two() {
        let cluster = Cluster::new("admin");
        let key = new_key();
        let Admission::Pending(_old, mut old) =
            cluster.classify(&node("a", &[]), &key, None, ip(9))
        else {
            panic!()
        };
        let _new = cluster.classify(&node("a", &[]), &key, None, ip(9));
        assert!(matches!(old.try_recv(), Ok(Decision::Rejected(_))));
        assert_eq!(pending_keys(&cluster), [key.as_hex()]);

        // Two more from the same address push the oldest out.
        let (k2, k3) = (new_key(), new_key());
        let _b = cluster.classify(&node("b", &[]), &k2, None, ip(9));
        let _c = cluster.classify(&node("c", &[]), &k3, None, ip(9));
        assert_eq!(pending_keys(&cluster), [k2.as_hex(), k3.as_hex()]);
        // Another address is unaffected.
        let k4 = new_key();
        let _d = cluster.classify(&node("d", &[]), &k4, None, ip(10));
        assert_eq!(pending_keys(&cluster).len(), 3);
    }

    #[test]
    fn pending_list_is_capped() {
        let cluster = Cluster::new("admin").with_settings(Settings {
            max_pending: 2,
            ..Settings::default()
        });
        let _a = cluster.classify(&node("a", &[]), &new_key(), None, ip(1));
        let _b = cluster.classify(&node("b", &[]), &new_key(), None, ip(2));
        assert!(matches!(
            cluster.classify(&node("c", &[]), &new_key(), None, ip(3)),
            Admission::Rejected(_)
        ));
    }

    #[test]
    fn removal_kicks_the_machine_requeues_its_work_and_burns_the_key() {
        let cluster = Cluster::new("admin");
        let a = node("a", &[RunnerKind::Fake]);
        let key = new_key();
        cluster.approve_with_token(&a, key.as_hex()).unwrap();
        let (tx, mut rx) = mpsc::unbounded_channel();
        let (kick, mut kicked) = oneshot::channel();
        cluster.admit(a.clone(), key.as_hex(), tx, kick).unwrap();
        let (_, mut b) = connect(&cluster, "b", &[RunnerKind::Fake]);
        cluster.submit_job(fake(1, 10)).unwrap();
        assert_eq!(assigned(&mut rx).len(), 1);
        assert_eq!(assigned(&mut b).len(), 1);

        cluster.remove_device(&a.id).unwrap();
        assert_eq!(kicked.try_recv(), Ok(REMOVED.to_owned()));
        assert_eq!(assigned(&mut b).len(), 1, "a's task moved to b");
        let snap = cluster.snapshot();
        assert_eq!(snap.nodes.len(), 1);
        assert_eq!(snap.devices.len(), 1);

        // The key is dead: no admit, no approval, not even with the token.
        let (tx, _rx) = mpsc::unbounded_channel();
        let (kick, _) = oneshot::channel();
        assert_eq!(
            cluster.admit(a.clone(), key.as_hex(), tx, kick),
            Err(REMOVED.to_owned())
        );
        assert!(matches!(
            cluster.classify(&a, &key, Some("admin"), None),
            Admission::Rejected(r) if r == REMOVED
        ));
        assert_eq!(cluster.remove_device(&a.id), Err(PairError::NotFound));
    }

    #[test]
    fn approvals_and_removals_persist() {
        let dir = tempfile::tempdir().unwrap();
        let open = || Cluster::with_store("admin", DeviceStore::open(dir.path()).unwrap());
        let (kept, gone) = (new_key(), new_key());
        let (a, b) = (node("a", &[]), node("b", &[]));
        {
            let cluster = open();
            cluster.approve_with_token(&a, kept.as_hex()).unwrap();
            let Admission::Pending(_t, _d) = cluster.classify(&b, &gone, None, None) else {
                panic!()
            };
            cluster.approve(gone.as_hex()).unwrap();
            cluster.remove_device(&b.id).unwrap();
        }
        let cluster = open();
        assert!(matches!(
            cluster.classify(&a, &kept, None, None),
            Admission::Approved
        ));
        assert!(matches!(
            cluster.classify(&b, &gone, None, None),
            Admission::Rejected(r) if r == REMOVED
        ));
        let names: Vec<_> = cluster
            .snapshot()
            .devices
            .into_iter()
            .map(|d| d.id.0)
            .collect();
        assert_eq!(names, ["a"]);
    }
}
