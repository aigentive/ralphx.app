/**
 * Test page for StopReasonDialog visual testing
 * Renders the dialog in isolation with controls to test all states
 */

import { useState } from "react";
import { StopReasonDialog } from "@/components/ui/StopReasonDialog";
import { Button } from "@/components/ui/button";
import type { InternalStatus } from "@/types/status";

export function StopReasonDialogTestPage() {
  const [isOpen, setIsOpen] = useState(false);
  const [taskTitle, setTaskTitle] = useState("Implement smart resume for stopped tasks");
  const [taskStatus, setTaskStatus] = useState<InternalStatus>("executing");
  const [lastAction, setLastAction] = useState<{ type: string; reason?: string } | undefined>();

  const statuses: InternalStatus[] = [
    "executing",
    "re_executing",
    "reviewing",
    "merging",
    "pending_merge",
    "qa_testing",
  ];

  return (
    <div className="p-8 space-y-4">
      <h1 className="text-2xl font-bold">StopReasonDialog Test Page</h1>

      <div className="space-y-4 bg-[var(--bg-surface)] p-6 rounded-lg border border-[var(--border-subtle)]">
        <div className="space-y-2">
          <label className="text-sm font-medium">Task Title (optional):</label>
          <input
            type="text"
            value={taskTitle}
            onChange={(e) => setTaskTitle(e.target.value)}
            className="w-full px-3 py-2 bg-[var(--bg-elevated)] border border-[var(--border-subtle)] rounded"
            placeholder="Enter task title..."
          />
        </div>

        <div className="space-y-2">
          <label className="text-sm font-medium">Task Status:</label>
          <select
            value={taskStatus}
            onChange={(e) => setTaskStatus(e.target.value as InternalStatus)}
            className="w-full px-3 py-2 bg-[var(--bg-elevated)] border border-[var(--border-subtle)] rounded"
          >
            {statuses.map((status) => (
              <option key={status} value={status}>
                {status}
              </option>
            ))}
          </select>
        </div>

        <Button
          data-testid="open-dialog-button"
          onClick={() => setIsOpen(true)}
        >
          Open Dialog
        </Button>

        {lastAction && (
          <div className="mt-4 p-4 bg-[var(--bg-elevated)] rounded">
            <p className="text-sm font-medium mb-2">Last Action:</p>
            <p className="text-sm text-[var(--text-secondary)]">
              <strong>{lastAction.type}</strong>
              {lastAction.reason !== undefined && (
                <span> - Reason: {lastAction.reason || "(no reason provided)"}</span>
              )}
            </p>
          </div>
        )}
      </div>

      <StopReasonDialog
        isOpen={isOpen}
        onClose={() => setIsOpen(false)}
        onConfirm={(reason) => {
          setLastAction({ type: "Confirmed", reason });
          setIsOpen(false);
        }}
        onSkip={() => {
          setLastAction({ type: "Skipped" });
          setIsOpen(false);
        }}
        taskTitle={taskTitle}
        taskStatus={taskStatus}
      />
    </div>
  );
}
