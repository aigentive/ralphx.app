/**
 * Unified branch display utility.
 * Replaces: abbreviateBranch(), shortBranch(), inline .split("/").pop()
 */

/** Strip "ralphx/<slug>/" prefix, truncate UUID to 8 chars, keep plan/main as-is */
export function formatBranchDisplay(branch: string): { short: string; full: string } {
  // Strip leading "ralphx/<slug>/" prefix (e.g., "ralphx/ralphx/task-...")
  const parts = branch.split("/");
  const name = parts.length >= 3 ? parts.slice(2).join("/") : parts.length === 2 ? (parts[1] ?? branch) : branch;

  // Truncate long UUIDs in task branch names: "task-9f7d52f0-9ce9-4c33-..." → "task-9f7d52f0"
  const short = name.replace(
    /^(task-[a-f0-9]{8})-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$/,
    "$1",
  );

  return { short, full: branch };
}
