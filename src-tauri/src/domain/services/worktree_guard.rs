// Worktree refcount guard — prevents worktree deletion while validation runs in it.
//
// Uses a global DashMap<PathBuf, AtomicU32> so any code path (including standalone
// functions without access to TaskServices) can check if a worktree is in use.
// RAII WorktreePermit auto-decrements on drop.

use dashmap::DashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, LazyLock};

/// Global worktree refcount registry.
/// Acquire a permit before running commands in a worktree;
/// check `is_in_use` before deleting one.
static WORKTREE_REFCOUNTS: LazyLock<DashMap<PathBuf, Arc<AtomicU32>>> = LazyLock::new(DashMap::new);

/// Acquire a permit indicating this worktree is in active use.
/// The permit is RAII — dropping it decrements the refcount.
/// Multiple permits can be held for the same worktree concurrently.
pub fn acquire_worktree_permit(path: &Path) -> WorktreePermit {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let refcount = WORKTREE_REFCOUNTS
        .entry(canonical.clone())
        .or_insert_with(|| Arc::new(AtomicU32::new(0)))
        .clone();
    refcount.fetch_add(1, Ordering::SeqCst);
    WorktreePermit {
        path: canonical,
        refcount,
    }
}

/// Check whether any permits are held for the given worktree path.
/// Returns true if at least one `WorktreePermit` is alive for this path.
pub fn is_worktree_in_use(path: &Path) -> bool {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    WORKTREE_REFCOUNTS
        .get(&canonical)
        .map(|rc| rc.load(Ordering::SeqCst) > 0)
        .unwrap_or(false)
}

/// RAII permit for worktree usage. Drop decrements the refcount.
pub struct WorktreePermit {
    path: PathBuf,
    refcount: Arc<AtomicU32>,
}

impl Drop for WorktreePermit {
    fn drop(&mut self) {
        let prev = self.refcount.fetch_sub(1, Ordering::SeqCst);
        // Clean up the DashMap entry when refcount reaches zero to prevent unbounded growth.
        if prev == 1 {
            WORKTREE_REFCOUNTS.remove(&self.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn acquire_and_drop_refcount() {
        let path = PathBuf::from("/tmp/test-worktree-guard-acquire");
        assert!(!is_worktree_in_use(&path));

        let permit1 = acquire_worktree_permit(&path);
        assert!(is_worktree_in_use(&path));

        let permit2 = acquire_worktree_permit(&path);
        assert!(is_worktree_in_use(&path));

        drop(permit1);
        assert!(is_worktree_in_use(&path)); // still held by permit2

        drop(permit2);
        assert!(!is_worktree_in_use(&path)); // all released
    }

    #[test]
    fn cleanup_removes_dashmap_entry() {
        let path = PathBuf::from("/tmp/test-worktree-guard-cleanup");
        let permit = acquire_worktree_permit(&path);
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        assert!(WORKTREE_REFCOUNTS.contains_key(&canonical));

        drop(permit);
        assert!(!WORKTREE_REFCOUNTS.contains_key(&canonical));
    }
}
