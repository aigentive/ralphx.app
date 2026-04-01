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
      {/* Full-page container - same bg as Kanban */}
      <div
        data-testid="task-creation-overlay-backdrop"
        className="absolute inset-0 z-40 flex"
        style={{
          backgroundColor: "hsl(220 10% 8%)", /* Same as Kanban background */
        }}
        onClick={handleBackdropClick}
      >
        {/* Content area - full width, no boxing */}
        <div
          data-testid="task-creation-overlay"
          className="flex-1 flex flex-col"
          onClick={(e) => e.stopPropagation()}
        >
          {/* Header - minimal, integrated */}
          <div
            className="px-6 h-14 shrink-0 flex items-center justify-between"
            style={{
              borderBottom: "1px solid hsla(220 10% 100% / 0.06)",
            }}
          >
            <h2
              data-testid="task-creation-overlay-title"
              style={{
                fontSize: "15px",
                fontWeight: 600,
                color: "hsl(220 10% 90%)",
                letterSpacing: "-0.02em",
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
              style={{ color: "hsl(220 10% 50%)" }}
              className="hover:bg-[hsla(220_10%_100%/0.05)]"
            >
              <X className="w-4 h-4" />
            </Button>
          </div>

          {/* Scrollable Content with Form - full width */}
          <ScrollArea className="flex-1">
            <div className="px-6 py-6">
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
