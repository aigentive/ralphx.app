import { ShieldCheck } from "lucide-react";
import { SolutionCritiqueAction } from "./SolutionCritiqueAction";

interface ReviewCritiquePreflightBannerProps {
  sessionId: string | null | undefined;
  taskId: string | null | undefined;
  taskTitle: string | null | undefined;
  compact?: boolean;
}

export function ReviewCritiquePreflightBanner({
  sessionId,
  taskId,
  taskTitle,
  compact = false,
}: ReviewCritiquePreflightBannerProps) {
  if (!sessionId || !taskId) return null;

  return (
    <div
      data-testid="review-critique-preflight"
      className="flex items-center justify-between gap-3 rounded-md border px-3 py-2"
      style={{
        background: "var(--overlay-faint)",
        borderColor: "var(--overlay-weak)",
      }}
    >
      <div className="flex min-w-0 items-center gap-2">
        <ShieldCheck className="h-4 w-4 shrink-0 text-text-primary/45" />
        <div className="min-w-0">
          <div className="text-[11px] font-semibold uppercase text-text-primary/40">
            Review Preflight
          </div>
          {!compact && (
            <div className="truncate text-[12px] text-text-primary/60">
              {taskTitle ?? "Task execution"}
            </div>
          )}
        </div>
      </div>
      <SolutionCritiqueAction
        sessionId={sessionId}
        target={{
          targetType: "task_execution",
          id: taskId,
          label: taskTitle ? `Task execution: ${taskTitle}` : "Task execution",
        }}
        label="Critique"
        size="xs"
      />
    </div>
  );
}
