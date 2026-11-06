use std::collections::BTreeMap;
use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use ts_rs::TS;

use crate::progress::{Progress, ProgressSnapshot};
use crate::{CoreError, CoreResult};

const MAX_FINISHED_TASKS: usize = 30;

pub type TaskId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum TaskKind {
    InstallInstance,
    LaunchInstance,
    RepairInstance,
    InstallMod,
    UpdateMods,
    InstallPack,
    InstallOrreryShaders,
    InstallOrreryMenu,
    InstallModpack,
    ImportModpack,
    ImportInstance,
    SyncPacks,
    MoveData,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
#[ts(export)]
pub enum TaskStatus {
    Running,
    Completed,
    Failed { kind: String, message: String },
    Cancelled,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TaskView {
    pub id: TaskId,
    pub kind: TaskKind,
    pub subject: String,
    pub instance_id: Option<String>,
    pub status: TaskStatus,
    pub progress: ProgressSnapshot,
}

pub type TaskListener = Arc<dyn Fn(TaskView) + Send + Sync>;

struct Entry {
    kind: TaskKind,
    subject: String,
    instance_id: Option<String>,
    status: TaskStatus,
    progress: Arc<Progress>,
    emitted_revision: u64,
}

impl Entry {
    fn view(&self, id: TaskId) -> TaskView {
        TaskView {
            id,
            kind: self.kind,
            subject: self.subject.clone(),
            instance_id: self.instance_id.clone(),
            status: self.status.clone(),
            progress: self.progress.snapshot(),
        }
    }
}

pub struct TaskSpec {
    pub kind: TaskKind,
    pub subject: String,
    pub instance_id: Option<String>,
}

pub struct TaskRegistry {
    entries: Mutex<BTreeMap<TaskId, Entry>>,
    next_id: AtomicU64,
    listener: TaskListener,
}

impl TaskRegistry {
    pub fn new(listener: TaskListener) -> Arc<Self> {
        Arc::new(Self {
            entries: Mutex::new(BTreeMap::new()),
            next_id: AtomicU64::new(1),
            listener,
        })
    }

    pub fn spawn<F, Fut>(self: &Arc<Self>, spec: TaskSpec, job: F) -> CoreResult<TaskId>
    where
        F: FnOnce(Arc<Progress>) -> Fut,
        Fut: Future<Output = CoreResult<()>> + Send + 'static,
    {
        if let Some(instance_id) = &spec.instance_id
            && self.has_running(spec.kind, instance_id)
        {
            return Err(CoreError::InvalidInput(format!(
                "a {:?} task for instance {instance_id} is already running",
                spec.kind
            )));
        }

        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let progress = Arc::new(Progress::new());
        let entry = Entry {
            kind: spec.kind,
            subject: spec.subject,
            instance_id: spec.instance_id,
            status: TaskStatus::Running,
            progress: progress.clone(),
            emitted_revision: progress.revision(),
        };
        let view = entry.view(id);
        {
            let mut entries = self.lock();
            let mut running = entries
                .values()
                .filter(|entry| entry.status == TaskStatus::Running);
            if running
                .clone()
                .any(|entry| entry.kind == TaskKind::MoveData)
            {
                return Err(CoreError::InvalidInput(
                    "the data folder is being moved, try again afterwards".into(),
                ));
            }
            if spec.kind == TaskKind::MoveData && running.next().is_some() {
                return Err(CoreError::InvalidInput(
                    "wait for running tasks before moving the data folder".into(),
                ));
            }
            entries.insert(id, entry);
        }
        (self.listener)(view);

        let token = progress.cancel_token().clone();
        let future = job(progress);
        let registry = self.clone();
        tokio::spawn(async move {
            let result = tokio::select! {
                result = future => result,
                () = token.cancelled() => Err(CoreError::Cancelled),
            };
            registry.finish(id, result);
        });
        Ok(id)
    }

    pub fn list(&self) -> Vec<TaskView> {
        self.lock()
            .iter()
            .map(|(id, entry)| entry.view(*id))
            .collect()
    }

    pub fn cancel(&self, id: TaskId) -> CoreResult<()> {
        let entries = self.lock();
        let entry = entries
            .get(&id)
            .ok_or_else(|| CoreError::NotFound(format!("task {id}")))?;
        if entry.kind == TaskKind::MoveData {
            return Err(CoreError::InvalidInput(
                "moving the data folder cannot be cancelled".into(),
            ));
        }
        entry.progress.cancel();
        Ok(())
    }

    pub fn dismiss(&self, id: TaskId) {
        let mut entries = self.lock();
        if entries
            .get(&id)
            .is_some_and(|entry| entry.status != TaskStatus::Running)
        {
            entries.remove(&id);
        }
    }

    pub fn flush(&self) {
        let changed: Vec<TaskView> = {
            let mut entries = self.lock();
            entries
                .iter_mut()
                .filter(|(_, entry)| entry.status == TaskStatus::Running)
                .filter_map(|(id, entry)| {
                    let revision = entry.progress.revision();
                    (revision != entry.emitted_revision).then(|| {
                        entry.emitted_revision = revision;
                        entry.view(*id)
                    })
                })
                .collect()
        };
        for view in changed {
            (self.listener)(view);
        }
    }

    pub async fn run_ticker(self: Arc<Self>, interval: Duration) {
        let mut ticker = tokio::time::interval(interval);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            self.flush();
        }
    }

    fn has_running(&self, kind: TaskKind, instance_id: &str) -> bool {
        self.lock().values().any(|entry| {
            entry.kind == kind
                && entry.status == TaskStatus::Running
                && entry.instance_id.as_deref() == Some(instance_id)
        })
    }

    fn finish(&self, id: TaskId, result: CoreResult<()>) {
        let status = match result {
            Ok(()) => TaskStatus::Completed,
            Err(CoreError::Cancelled) => TaskStatus::Cancelled,
            Err(err) => {
                tracing::error!(task = id, error = %err, "task failed");
                TaskStatus::Failed {
                    kind: err.kind().to_string(),
                    message: err.to_string(),
                }
            }
        };
        let view = {
            let mut entries = self.lock();
            let Some(entry) = entries.get_mut(&id) else {
                return;
            };
            entry.status = status;
            entry.emitted_revision = entry.progress.revision();
            let view = entry.view(id);
            prune_finished(&mut entries);
            view
        };
        (self.listener)(view);
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, BTreeMap<TaskId, Entry>> {
        self.entries.lock().unwrap_or_else(|e| e.into_inner())
    }
}

fn prune_finished(entries: &mut BTreeMap<TaskId, Entry>) {
    let finished: Vec<TaskId> = entries
        .iter()
        .filter(|(_, entry)| entry.status != TaskStatus::Running)
        .map(|(id, _)| *id)
        .collect();
    if finished.len() > MAX_FINISHED_TASKS {
        for id in &finished[..finished.len() - MAX_FINISHED_TASKS] {
            entries.remove(id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::progress::Stage;

    fn recording_registry() -> (Arc<TaskRegistry>, Arc<Mutex<Vec<TaskView>>>) {
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = events.clone();
        let registry = TaskRegistry::new(Arc::new(move |view| sink.lock().unwrap().push(view)));
        (registry, events)
    }

    fn spec(instance: &str) -> TaskSpec {
        TaskSpec {
            kind: TaskKind::InstallInstance,
            subject: "Install".into(),
            instance_id: Some(instance.into()),
        }
    }

    async fn wait_for_status(registry: &TaskRegistry, id: TaskId) -> TaskStatus {
        for _ in 0..200 {
            let view = registry.list().into_iter().find(|v| v.id == id).unwrap();
            if view.status != TaskStatus::Running {
                return view.status;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        panic!("task {id} did not finish");
    }

    #[tokio::test]
    async fn completed_task_emits_start_progress_and_finish() {
        let (registry, events) = recording_registry();
        let (release_tx, release_rx) = tokio::sync::oneshot::channel::<()>();
        let id = registry
            .spawn(spec("a"), |progress| async move {
                progress.set_stage(Stage::DownloadingAssets);
                progress.add_totals(2, 0);
                progress.add_items(1);
                release_rx.await.ok();
                Ok(())
            })
            .unwrap();

        tokio::time::sleep(Duration::from_millis(20)).await;
        registry.flush();
        release_tx.send(()).unwrap();
        assert_eq!(wait_for_status(&registry, id).await, TaskStatus::Completed);

        let events = events.lock().unwrap();
        assert_eq!(events.first().unwrap().status, TaskStatus::Running);
        assert!(events.iter().any(|v| v.progress.items_done == 1));
        assert_eq!(events.last().unwrap().status, TaskStatus::Completed);
    }

    #[tokio::test]
    async fn cancel_stops_a_pending_task() {
        let (registry, _) = recording_registry();
        let id = registry
            .spawn(spec("a"), |_| std::future::pending::<CoreResult<()>>())
            .unwrap();
        registry.cancel(id).unwrap();
        assert_eq!(wait_for_status(&registry, id).await, TaskStatus::Cancelled);
    }

    #[tokio::test]
    async fn duplicate_task_for_same_instance_is_rejected() {
        let (registry, _) = recording_registry();
        registry
            .spawn(spec("a"), |_| std::future::pending::<CoreResult<()>>())
            .unwrap();
        let duplicate = registry.spawn(spec("a"), |_| async { Ok(()) });
        assert!(matches!(duplicate, Err(CoreError::InvalidInput(_))));
        assert!(registry.spawn(spec("b"), |_| async { Ok(()) }).is_ok());
    }

    #[tokio::test]
    async fn failure_is_reported_with_kind() {
        let (registry, _) = recording_registry();
        let id = registry
            .spawn(spec("a"), |_| async {
                Err(CoreError::NotFound("version 9.9".into()))
            })
            .unwrap();
        match wait_for_status(&registry, id).await {
            TaskStatus::Failed { kind, .. } => assert_eq!(kind, "not_found"),
            other => panic!("unexpected status {other:?}"),
        }
    }
}
