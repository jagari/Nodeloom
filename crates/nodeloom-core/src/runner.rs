//! The [`Runner`] abstraction: one implementation per kind of work.

use std::{collections::HashMap, sync::Arc, time::Duration};

use async_trait::async_trait;
use nodeloom_proto::{RunnerKind, TaskPayload};
use tokio::sync::mpsc;

#[derive(Debug, thiserror::Error)]
pub enum RunnerError {
    #[error("runner {0:?} is not available on this node")]
    Unavailable(RunnerKind),
    #[error("invalid task: {0}")]
    InvalidTask(String),
    #[error("task failed: {0}")]
    Failed(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[async_trait]
pub trait Runner: Send + Sync {
    fn kind(&self) -> RunnerKind;

    /// Executes `payload`, sending the number of frames completed so far on
    /// `progress` after each frame. Dropping the returned future must stop the
    /// work (runners spawning processes use `kill_on_drop`).
    async fn run(
        &self,
        payload: &TaskPayload,
        progress: &mpsc::Sender<u32>,
    ) -> Result<(), RunnerError>;
}

/// The runners available on one node.
#[derive(Clone, Default)]
pub struct RunnerSet {
    runners: HashMap<RunnerKind, Arc<dyn Runner>>,
}

impl RunnerSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with(mut self, runner: Arc<dyn Runner>) -> Self {
        self.runners.insert(runner.kind(), runner);
        self
    }

    pub fn kinds(&self) -> Vec<RunnerKind> {
        let mut kinds: Vec<_> = self.runners.keys().copied().collect();
        kinds.sort_by_key(|k| format!("{k:?}"));
        kinds
    }

    pub fn get(&self, kind: RunnerKind) -> Result<Arc<dyn Runner>, RunnerError> {
        self.runners
            .get(&kind)
            .cloned()
            .ok_or(RunnerError::Unavailable(kind))
    }
}

/// Test runner: sleeps per frame and reports progress. Never touches the
/// machine, so it is safe to leave enabled in integration tests.
pub struct FakeRunner;

#[async_trait]
impl Runner for FakeRunner {
    fn kind(&self) -> RunnerKind {
        RunnerKind::Fake
    }

    async fn run(
        &self,
        payload: &TaskPayload,
        progress: &mpsc::Sender<u32>,
    ) -> Result<(), RunnerError> {
        let TaskPayload::Fake {
            frame_start,
            frame_end,
            millis_per_frame,
        } = payload
        else {
            return Err(RunnerError::InvalidTask("not a fake task".into()));
        };
        if frame_start > frame_end {
            return Err(RunnerError::InvalidTask("empty frame range".into()));
        }
        for done in 1..=(frame_end - frame_start + 1) {
            tokio::time::sleep(Duration::from_millis(*millis_per_frame)).await;
            // A closed receiver just means nobody is listening any more.
            let _ = progress.send(done).await;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fake_runner_reports_every_frame() {
        let (tx, mut rx) = mpsc::channel(16);
        let payload = TaskPayload::Fake {
            frame_start: 1,
            frame_end: 4,
            millis_per_frame: 0,
        };
        FakeRunner.run(&payload, &tx).await.unwrap();
        drop(tx);
        let mut seen = Vec::new();
        while let Some(n) = rx.recv().await {
            seen.push(n);
        }
        assert_eq!(seen, vec![1, 2, 3, 4]);
    }

    #[tokio::test]
    async fn fake_runner_rejects_other_payloads() {
        let (tx, _rx) = mpsc::channel(1);
        let payload = TaskPayload::Blender {
            blend_file: "a.blend".into(),
            frame_start: 1,
            frame_end: 2,
        };
        assert!(matches!(
            FakeRunner.run(&payload, &tx).await,
            Err(RunnerError::InvalidTask(_))
        ));
    }

    #[test]
    fn runner_set_lookup() {
        let set = RunnerSet::new().with(Arc::new(FakeRunner));
        assert_eq!(set.kinds(), vec![RunnerKind::Fake]);
        assert!(set.get(RunnerKind::Fake).is_ok());
        assert!(matches!(
            set.get(RunnerKind::Blender),
            Err(RunnerError::Unavailable(RunnerKind::Blender))
        ));
    }
}
