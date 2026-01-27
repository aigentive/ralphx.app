/**
 * TaskCreationOverlay - Inline task creation panel for split-screen layout
 *
 * Similar to TaskDetailOverlay but for creating new tasks.
 * Reuses TaskCreationForm for the actual form.
 *
 * Design spec: specs/design/refined-studio-patterns.md
 */

import { useCallback, useEffect } from "react";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Button } from "@/components/ui/button";
import { TaskCreationForm } from "./TaskCreationForm";
import { useUiStore } from "@/stores/uiStore";
import { X } from "lucide-react";

// ============================================================================
// Main Component
// ============================================================================

interface TaskCreationOverlayProps {
  projectId: string;
}

export function TaskCreationOverlay({ projectId }: TaskCreationOverlayProps) {
  const taskCreationContext = useUiStore((s) => s.taskCreationContext);
  const closeTaskCreation = useUiStore((s) => s.closeTaskCreation);

  // Close overlay on Escape key
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        closeTaskCreation();
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [closeTaskCreation]);

  // Handle backdrop click
  const handleBackdropClick = useCallback(
    (event: React.MouseEvent<HTMLDivElement>) => {
      // Only close if clicking the backdrop itself, not its children
      if (event.target === event.currentTarget) {
        closeTaskCreation();
      }
    },
    [closeTaskCreation]
  );

  // Don't render if not open
  if (!taskCreationContext) {
    return null;
  }

  return (
    <>
      {/* Backdrop with blur */}
      <div
        data-testid="task-creation-overlay-backdrop"
        className="absolute inset-0 z-40"
        style={{
          backgroundColor: "rgba(0, 0, 0, 0.6)",
          backdropFilter: "blur(8px)",
          WebkitBackdropFilter: "blur(8px)",
        }}
        onClick={handleBackdropClick}
      >
        {/* Overlay content */}
        <div
          data-testid="task-creation-overlay"
          className="absolute inset-6 flex flex-col rounded-xl overflow-hidden"
          style={{
            background: "linear-gradient(180deg, rgba(24,24,24,0.98) 0%, rgba(18,18,18,0.99) 100%)",
            border: "1px solid rgba(255,255,255,0.08)",
            boxShadow:
              "0 8px 16px rgba(0,0,0,0.4), 0 16px 32px rgba(0,0,0,0.3), 0 0 0 1px rgba(255,255,255,0.03)",
          }}
          onClick={(e) => e.stopPropagation()} // Prevent backdrop click
        >
          {/* Header - Glass effect */}
          <div
            className="px-5 pt-5 pb-4 shrink-0 backdrop-blur-sm flex items-center justify-between"
            style={{
              borderBottom: "1px solid rgba(255,255,255,0.06)",
              background: "linear-gradient(180deg, rgba(26,26,26,0.95) 0%, transparent 100%)",
            }}
          >
            <h2
              data-testid="task-creation-overlay-title"
              className="text-base font-semibold text-white/90"
              style={{
                letterSpacing: "-0.02em",
                lineHeight: "1.3",
              }}
            >
              Create Task
            </h2>

            {/* Close button */}
            <Button
              variant="ghost"
              size="icon-sm"
              onClick={closeTaskCreation}
              data-testid="task-creation-overlay-close"
              aria-label="Close"
              className="hover:bg-white/5"
            >
              <X className="w-4 h-4" />
            </Button>
          </div>

          {/* Scrollable Content with Form */}
          <ScrollArea className="flex-1">
            <div className="px-6 py-4">
              <TaskCreationForm
                projectId={taskCreationContext.projectId || projectId}
                {...(taskCreationContext.defaultTitle !== undefined && { defaultTitle: taskCreationContext.defaultTitle })}
                onSuccess={closeTaskCreation}
                onCancel={closeTaskCreation}
              />
            </div>
          </ScrollArea>
        </div>
      </div>
    </>
  );
}
