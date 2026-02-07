/**
 * ReviewingTaskDetail - macOS Tahoe-inspired AI review in progress view
 *
 * Shows animated review progress with step indicator and clean layout.
 */

import {
  Loader2,
  Bot,
  CheckCircle2,
  Circle,
  Sparkles,
  AlertTriangle,
  XCircle,
} from "lucide-react";
import {
  SectionTitle,
  DetailCard,
  StatusBanner,
  StatusPill,
  TwoColumnLayout,
} from "./shared";
import type { Task } from "@/types/task";
import { useTaskStateHistory } from "@/hooks/useReviews";
import type { ReviewNoteResponse } from "@/lib/tauri";

interface ReviewingTaskDetailProps {
  task: Task;
  isHistorical?: boolean;
  viewTimestamp?: string | undefined;
}

type ReviewStepStatus = "completed" | "active" | "pending";

interface ReviewStep {
  label: string;
  status: ReviewStepStatus;
}

/**
 * ReviewStepItem - Individual step with native-feeling progress indicator
 */
function ReviewStepItem({
  label,
  status,
  isHistorical,
}: ReviewStep & { isHistorical?: boolean }) {
  return (
    <div className="flex items-center gap-3 py-2.5">
      {/* Status icon */}
      <div className="relative">
        {status === "completed" && (
          <CheckCircle2 className="w-5 h-5" style={{ color: "#34c759" }} />
        )}
        {status === "active" && !isHistorical && (
          <div className="relative">
            <Loader2
              className="w-5 h-5 animate-spin"
              style={{ color: "#0a84ff" }}
            />
            {/* Glow effect */}
            <div
              className="absolute inset-0 rounded-full animate-pulse"
              style={{
                background: "radial-gradient(circle, rgba(10,132,255,0.3) 0%, transparent 70%)",
              }}
            />
          </div>
        )}
        {status === "active" && isHistorical && (
          <Circle className="w-5 h-5" style={{ color: "#64d2ff" }} />
        )}
        {status === "pending" && (
          <Circle
            className="w-5 h-5"
            style={{ color: "rgba(255,255,255,0.2)" }}
          />
        )}
      </div>

      {/* Label */}
      <span
        className="text-[13px] font-medium"
        style={{
          color:
            status === "completed"
              ? "rgba(255,255,255,0.6)"
              : status === "active"
              ? isHistorical
                ? "rgba(255,255,255,0.35)"
                : "#64d2ff"
              : "rgba(255,255,255,0.35)",
        }}
      >
        {label}
      </span>
    </div>
  );
}

/**
 * ReviewStepsCard - Shows all review steps with progress
 */
function ReviewStepsCard({
  isHistorical,
  mode,
  variant,
}: {
  isHistorical?: boolean;
  mode: "completed" | "in_progress";
  variant: "success" | "warning" | "error" | "info";
}) {
  const steps: ReviewStep[] =
    mode === "completed"
      ? [
          { label: "Gathering context", status: "completed" },
          { label: "Examining changes", status: "completed" },
          { label: "Running checks", status: "completed" },
          { label: "Generating feedback", status: "completed" },
        ]
      : [
          { label: "Gathering context", status: "completed" },
          { label: "Examining changes", status: "active" },
          { label: "Running checks", status: "pending" },
          { label: "Generating feedback", status: "pending" },
        ];

  return (
    <DetailCard variant={variant}>
      <div className="divide-y divide-white/5">
        {steps.map((step, index) => (
          <ReviewStepItem key={index} {...step} isHistorical={isHistorical === true} />
        ))}
      </div>
    </DetailCard>
  );
}

function findOutcomeForTimestamp(
  history: ReviewNoteResponse[],
  timestamp: string | undefined
): ReviewNoteResponse | null {
  if (!timestamp) return null;
  const target = new Date(timestamp).getTime();
  const sorted = [...history].sort(
    (a, b) => new Date(a.created_at).getTime() - new Date(b.created_at).getTime()
  );
  return sorted.find((entry) => new Date(entry.created_at).getTime() >= target) ?? null;
}

function getOutcomeConfig(outcome: ReviewNoteResponse | null) {
  if (!outcome) {
    return {
      title: "AI Review in Progress",
      subtitle: "Outcome not recorded",
      label: "In Progress",
      variant: "info" as const,
      icon: Bot,
      pillIcon: Sparkles,
      mode: "in_progress" as const,
    };
  }

  switch (outcome.outcome) {
    case "approved":
      return {
        title: "AI Review Completed",
        subtitle: "Outcome: Approved",
        label: "Approved",
        variant: "success" as const,
        icon: CheckCircle2,
        pillIcon: CheckCircle2,
        mode: "completed" as const,
      };
    case "changes_requested":
      return {
        title: "AI Review Completed",
        subtitle: "Outcome: Changes Requested",
        label: "Changes Requested",
        variant: "warning" as const,
        icon: AlertTriangle,
        pillIcon: AlertTriangle,
        mode: "completed" as const,
      };
    case "rejected":
      return {
        title: "AI Review Completed",
        subtitle: "Outcome: Rejected",
        label: "Rejected",
        variant: "error" as const,
        icon: XCircle,
        pillIcon: XCircle,
        mode: "completed" as const,
      };
    default:
      return {
        title: "AI Review in Progress",
        subtitle: "Outcome not recorded",
        label: "In Progress",
        variant: "info" as const,
        icon: Bot,
        pillIcon: Sparkles,
        mode: "in_progress" as const,
      };
  }
}

export function ReviewingTaskDetail({
  task,
  isHistorical,
  viewTimestamp,
}: ReviewingTaskDetailProps) {
  const { data: history = [] } = useTaskStateHistory(task.id, {
    enabled: isHistorical === true,
  });
  const outcome = isHistorical ? findOutcomeForTimestamp(history, viewTimestamp) : null;
  const outcomeConfig = isHistorical ? getOutcomeConfig(outcome) : null;
  return (
    <TwoColumnLayout
      description={task.description}
      testId="reviewing-task-detail"
    >
      {/* Status Banner */}
      <StatusBanner
        icon={isHistorical ? outcomeConfig?.icon ?? Bot : Bot}
        title={isHistorical ? outcomeConfig?.title ?? "AI Review in Progress" : "AI Review in Progress"}
        subtitle={
          isHistorical
            ? outcomeConfig?.subtitle ?? "Analyzing changes and running checks"
            : "Analyzing changes and running checks"
        }
        variant={isHistorical ? outcomeConfig?.variant ?? "info" : "info"}
        animated={!isHistorical}
        badge={
          <StatusPill
            icon={isHistorical ? outcomeConfig?.pillIcon ?? Sparkles : Sparkles}
            label={isHistorical ? outcomeConfig?.label ?? "In Progress" : "Analyzing"}
            variant={isHistorical ? outcomeConfig?.variant ?? "info" : "info"}
            animated={!isHistorical}
            size="md"
          />
        }
      />

      {/* Review Steps */}
      <section data-testid="reviewing-steps-section">
        <SectionTitle>Review Progress</SectionTitle>
        <ReviewStepsCard
          isHistorical={isHistorical === true}
          mode={isHistorical ? outcomeConfig?.mode ?? "in_progress" : "in_progress"}
          variant={isHistorical ? outcomeConfig?.variant ?? "info" : "info"}
        />
      </section>
    </TwoColumnLayout>
  );
}
