use std::collections::{HashSet, VecDeque};
use std::time::Duration;

use chrono::{DateTime, Utc};
use tracing::{info, warn};

/// Number of hours after which a session is considered too stale to recover.
pub const RECOVERY_CUTOFF_HOURS: i64 = 24;

/// Priority ordering for recovery. Variants with lower discriminant values are
/// processed first (Verification before Ideation).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryPriority {
    /// PDM-172: verification agents — time-sensitive, must recover first.
    Verification,
    /// PDM-171: ideation agents — conversational, recover second.
    Ideation,
}

/// A single item waiting to be recovered in the queue.
#[derive(Debug, Clone)]
pub struct RecoveryItem {
    /// Context type string, e.g. `"ideation"` or `"verification"`.
    pub context_type: String,
    /// Session or context ID (used as the dedup key).
    pub context_id: String,
    /// Conversation ID needed for history replay.
    pub conversation_id: String,
    /// Recovery priority — determines queue insertion order.
    pub priority: RecoveryPriority,
    /// When the agent was last registered; used to enforce the 24-hour cutoff.
    pub started_at: DateTime<Utc>,
}

/// Summary returned by [`RecoveryQueue::process`].
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ProcessSummary {
    /// Number of sessions successfully handed to the callback.
    pub recovered: usize,
    /// Number of sessions skipped (older than 24 hours).
    pub skipped: usize,
    /// Number of sessions where the callback returned an error.
    pub failed: usize,
}

/// Staggered, deduplicating recovery queue shared by PDM-171 (ideation) and
/// PDM-172 (verification) agent recovery.
///
/// # Design
/// - Priority insertion: Verification items are inserted before Ideation items.
/// - Dedup: A `HashSet` prevents the same `context_id` from being enqueued twice.
/// - Stagger: `process()` waits `stagger_interval` between each live recovery call
///   to avoid API rate-limiting on restart.
/// - 24-hour cutoff: Items whose `started_at` is older than 24 hours are silently
///   skipped during processing.
///
/// # Concurrency
/// All fields are `Send + Sync`, so a `RecoveryQueue` can be moved into a
/// `tokio::spawn` closure without wrapping in `Arc<Mutex<>>`.
///
/// # Instantiation
/// Create as a **local variable** inside `StartupJobRunner::run()` and move it into
/// the `tokio::spawn` closure. Do not store in `AppState`.
pub struct RecoveryQueue {
    items: VecDeque<RecoveryItem>,
    /// Context IDs already enqueued — prevents double-enqueue.
    processing: HashSet<String>,
    stagger_interval: Duration,
}

impl RecoveryQueue {
    /// Create a new empty queue with the given stagger interval between recoveries.
    pub fn new(stagger_interval: Duration) -> Self {
        Self {
            items: VecDeque::new(),
            processing: HashSet::new(),
            stagger_interval,
        }
    }

    /// Enqueue `item` with priority-sorted insertion and dedup guard.
    ///
    /// Returns `true` if the item was accepted, `false` if the `context_id` was
    /// already in the queue (deduplicated).
    ///
    /// Insertion order:
    /// - `Verification` items are inserted immediately before the first `Ideation`
    ///   item (FIFO within the same priority tier).
    /// - `Ideation` items are appended to the back.
    pub fn enqueue(&mut self, item: RecoveryItem) -> bool {
        if self.processing.contains(&item.context_id) {
            return false;
        }
        self.processing.insert(item.context_id.clone());

        match item.priority {
            RecoveryPriority::Verification => {
                // Insert before the first Ideation item to maintain priority order.
                let pos = self
                    .items
                    .iter()
                    .position(|i| i.priority == RecoveryPriority::Ideation)
                    .unwrap_or(self.items.len());
                self.items.insert(pos, item);
            }
            RecoveryPriority::Ideation => {
                self.items.push_back(item);
            }
        }
        true
    }

    /// Dequeue and return the next item (highest priority first), or `None` if empty.
    pub fn dequeue(&mut self) -> Option<RecoveryItem> {
        self.items.pop_front()
    }

    /// Returns the number of items currently in the queue.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns `true` if the queue contains no items.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Process all queued items with stagger pacing.
    ///
    /// For each item:
    /// 1. Items older than [`RECOVERY_CUTOFF_HOURS`] are skipped immediately
    ///    (no stagger consumed).
    /// 2. `tokio::time::interval` paces live recoveries — the first tick fires
    ///    immediately; subsequent ticks wait `stagger_interval`.
    /// 3. Callback errors are logged as warnings and do **not** halt the queue.
    ///
    /// Returns a [`ProcessSummary`] with counts of recovered / skipped / failed.
    pub async fn process<F, Fut, E>(&mut self, mut callback: F) -> ProcessSummary
    where
        F: FnMut(RecoveryItem) -> Fut,
        Fut: std::future::Future<Output = Result<(), E>>,
        E: std::fmt::Display,
    {
        let mut summary = ProcessSummary::default();

        if self.is_empty() {
            return summary;
        }

        let cutoff = Utc::now() - chrono::Duration::hours(RECOVERY_CUTOFF_HOURS);
        let mut interval = tokio::time::interval(self.stagger_interval);

        while let Some(item) = self.dequeue() {
            if item.started_at < cutoff {
                warn!(
                    context_id = %item.context_id,
                    started_at = %item.started_at,
                    "Skipping recovery for stale session (older than {}h)",
                    RECOVERY_CUTOFF_HOURS
                );
                summary.skipped += 1;
                continue;
            }

            // Capture context_id before item is consumed by the callback.
            let context_id = item.context_id.clone();

            // Pace live recoveries; first tick is immediate.
            interval.tick().await;

            match callback(item).await {
                Ok(()) => {
                    summary.recovered += 1;
                }
                Err(e) => {
                    warn!(
                        context_id = %context_id,
                        error = %e,
                        "Session recovery failed, continuing queue"
                    );
                    summary.failed += 1;
                }
            }
        }

        info!(
            recovered = summary.recovered,
            skipped = summary.skipped,
            failed = summary.failed,
            "Recovery queue processing complete"
        );

        summary
    }
}
