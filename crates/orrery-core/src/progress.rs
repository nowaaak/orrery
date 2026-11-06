use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::Serialize;
use tokio_util::sync::CancellationToken;
use ts_rs::TS;

use crate::{CoreError, CoreResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Stage {
    Preparing,
    FetchingMetadata,
    DownloadingClient,
    DownloadingLibraries,
    DownloadingAssets,
    InstallingJava,
    InstallingLoader,
    ExtractingNatives,
    Verifying,
    Authenticating,
    Launching,
    DownloadingMods,
    ExtractingFiles,
    MovingData,
    Finished,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ProgressSnapshot {
    pub stage: Stage,
    pub items_done: u64,
    pub items_total: u64,
    pub bytes_done: u64,
    pub bytes_total: u64,
}

#[derive(Debug)]
pub struct Progress {
    stage: Mutex<Stage>,
    items_done: AtomicU64,
    items_total: AtomicU64,
    bytes_done: AtomicU64,
    bytes_total: AtomicU64,
    revision: AtomicU64,
    cancel: CancellationToken,
}

impl Default for Progress {
    fn default() -> Self {
        Self::new()
    }
}

impl Progress {
    pub fn new() -> Self {
        Self {
            stage: Mutex::new(Stage::Preparing),
            items_done: AtomicU64::new(0),
            items_total: AtomicU64::new(0),
            bytes_done: AtomicU64::new(0),
            bytes_total: AtomicU64::new(0),
            revision: AtomicU64::new(0),
            cancel: CancellationToken::new(),
        }
    }

    pub fn set_stage(&self, stage: Stage) {
        *self.stage.lock().unwrap_or_else(|e| e.into_inner()) = stage;
        self.items_done.store(0, Ordering::Relaxed);
        self.items_total.store(0, Ordering::Relaxed);
        self.bytes_done.store(0, Ordering::Relaxed);
        self.bytes_total.store(0, Ordering::Relaxed);
        self.bump();
    }

    pub fn add_totals(&self, items: u64, bytes: u64) {
        self.items_total.fetch_add(items, Ordering::Relaxed);
        self.bytes_total.fetch_add(bytes, Ordering::Relaxed);
        self.bump();
    }

    pub fn add_items(&self, items: u64) {
        self.items_done.fetch_add(items, Ordering::Relaxed);
        self.bump();
    }

    pub fn add_bytes(&self, bytes: u64) {
        self.bytes_done.fetch_add(bytes, Ordering::Relaxed);
        self.bump();
    }

    pub fn remove_bytes(&self, bytes: u64) {
        let _ = self
            .bytes_done
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |done| {
                Some(done.saturating_sub(bytes))
            });
        self.bump();
    }

    pub fn snapshot(&self) -> ProgressSnapshot {
        ProgressSnapshot {
            stage: *self.stage.lock().unwrap_or_else(|e| e.into_inner()),
            items_done: self.items_done.load(Ordering::Relaxed),
            items_total: self.items_total.load(Ordering::Relaxed),
            bytes_done: self.bytes_done.load(Ordering::Relaxed),
            bytes_total: self.bytes_total.load(Ordering::Relaxed),
        }
    }

    pub fn revision(&self) -> u64 {
        self.revision.load(Ordering::Relaxed)
    }

    pub fn cancel_token(&self) -> &CancellationToken {
        &self.cancel
    }

    pub fn cancel(&self) {
        self.cancel.cancel();
    }

    pub fn check_cancelled(&self) -> CoreResult<()> {
        if self.cancel.is_cancelled() {
            Err(CoreError::Cancelled)
        } else {
            Ok(())
        }
    }

    fn bump(&self) {
        self.revision.fetch_add(1, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_change_resets_counters_and_bumps_revision() {
        let progress = Progress::new();
        progress.add_totals(10, 1000);
        progress.add_items(3);
        progress.add_bytes(300);
        let before = progress.revision();

        progress.set_stage(Stage::DownloadingAssets);

        let snapshot = progress.snapshot();
        assert_eq!(snapshot.stage, Stage::DownloadingAssets);
        assert_eq!(snapshot.items_done, 0);
        assert_eq!(snapshot.bytes_total, 0);
        assert!(progress.revision() > before);
    }

    #[test]
    fn cancellation_is_reported() {
        let progress = Progress::new();
        assert!(progress.check_cancelled().is_ok());
        progress.cancel();
        assert!(matches!(
            progress.check_cancelled(),
            Err(CoreError::Cancelled)
        ));
    }
}
