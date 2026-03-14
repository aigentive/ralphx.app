// Reconciliation handler methods — split into merge + execution submodules.
//
// - merge.rs: merge-specific reconcilers (Merging, PendingMerge, MergeIncomplete, MergeConflict)
// - execution.rs: all other reconcilers + orchestration + apply_recovery_decision

mod execution;
mod merge;

#[cfg(test)]
mod execution_tests;
