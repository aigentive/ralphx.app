/**
 * Keyboard shortcuts hook for the kanban board
 */

import { useEffect, useCallback } from "react";

interface UseKeyboardShortcutsOptions {
  selectedTaskId: string | null;
  onMove: (taskId: string, columnId: string) => void;
}

const SHORTCUTS: Record<string, string> = {
  p: "planned",
  b: "backlog",
  t: "todo",
};

/**
 * Hook for keyboard shortcuts in the task board
 *
 * Shortcuts:
 * - P: Move to Planned
 * - B: Move to Backlog
 * - T: Move to To-do
 */
export function useKeyboardShortcuts({
  selectedTaskId,
  onMove,
}: UseKeyboardShortcutsOptions): void {
  const handleKeyDown = useCallback(
    (event: KeyboardEvent) => {
      // Don't trigger shortcuts when typing in inputs
      const target = event.target as HTMLElement;
      if (
        target.tagName === "INPUT" ||
        target.tagName === "TEXTAREA" ||
        target.isContentEditable
      ) {
        return;
      }

      if (!selectedTaskId) return;

      const key = event.key.toLowerCase();

      // Check movement shortcuts
      const columnId = SHORTCUTS[key];
      if (columnId) {
        event.preventDefault();
        onMove(selectedTaskId, columnId);
        return;
      }

    },
    [selectedTaskId, onMove]
  );

  useEffect(() => {
    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [handleKeyDown]);
}
