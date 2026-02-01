/**
 * Test page for BlockReasonDialog visual testing
 * Renders the dialog in isolation with controls to test all states
 */

import { useState } from "react";
import { BlockReasonDialog } from "@/components/tasks/BlockReasonDialog";
import { Button } from "@/components/ui/button";

export function BlockReasonDialogTestPage() {
  const [isOpen, setIsOpen] = useState(false);
  const [taskTitle, setTaskTitle] = useState("Implement user authentication");
  const [lastConfirmedReason, setLastConfirmedReason] = useState<string | undefined>();

  return (
    <div className="p-8 space-y-4">
      <h1 className="text-2xl font-bold">BlockReasonDialog Test Page</h1>

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

        <Button
          data-testid="open-dialog-button"
          onClick={() => setIsOpen(true)}
        >
          Open Dialog
        </Button>

        {lastConfirmedReason !== undefined && (
          <div className="mt-4 p-4 bg-[var(--bg-elevated)] rounded">
            <p className="text-sm font-medium mb-2">Last Confirmed Reason:</p>
            <p className="text-sm text-[var(--text-secondary)]">
              {lastConfirmedReason || "(no reason provided)"}
            </p>
          </div>
        )}
      </div>

      <BlockReasonDialog
        isOpen={isOpen}
        onClose={() => setIsOpen(false)}
        onConfirm={(reason) => {
          setLastConfirmedReason(reason);
          setIsOpen(false);
        }}
        taskTitle={taskTitle}
      />
    </div>
  );
}
