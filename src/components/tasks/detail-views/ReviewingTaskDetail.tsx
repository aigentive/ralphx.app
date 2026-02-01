/**
 * ReviewingTaskDetail - macOS Tahoe-inspired AI review in progress view
 *
 * Shows animated review progress with step indicator and clean layout.
 */

import { Loader2, Bot, CheckCircle2, Circle, Sparkles } from "lucide-react";
import {
  SectionTitle,
  DetailCard,
  StatusBanner,
  StatusPill,
  TwoColumnLayout,
} from "./shared";
import type { Task } from "@/types/task";

interface ReviewingTaskDetailProps {
  task: Task;
  isHistorical?: boolean;
}

type ReviewStepStatus = "completed" | "active" | "pending";

interface ReviewStep {
  label: string;
  status: ReviewStepStatus;
}

/**
 * ReviewStepItem - Individual step with native-feeling progress indicator
 */
function ReviewStepItem({ label, status }: ReviewStep) {
  return (
    <div className="flex items-center gap-3 py-2.5">
      {/* Status icon */}
      <div className="relative">
        {status === "completed" && (
          <CheckCircle2 className="w-5 h-5" style={{ color: "#34c759" }} />
        )}
        {status === "active" && (
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
              ? "#64d2ff"
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
function ReviewStepsCard({ isHistorical }: { isHistorical?: boolean }) {
  const steps: ReviewStep[] = isHistorical
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
    <DetailCard variant={isHistorical ? "success" : "info"}>
      <div className="divide-y divide-white/5">
        {steps.map((step, index) => (
          <ReviewStepItem key={index} {...step} />
        ))}
      </div>
    </DetailCard>
  );
}

export function ReviewingTaskDetail({ task, isHistorical }: ReviewingTaskDetailProps) {
  return (
    <TwoColumnLayout
      description={task.description}
      testId="reviewing-task-detail"
    >
      {/* Status Banner */}
      <StatusBanner
        icon={isHistorical ? CheckCircle2 : Bot}
        title={isHistorical ? "AI Review Completed" : "AI Review in Progress"}
        subtitle={isHistorical ? "Review has finished" : "Analyzing changes and running checks"}
        variant={isHistorical ? "success" : "info"}
        animated={!isHistorical}
        badge={
          <StatusPill
            icon={isHistorical ? CheckCircle2 : Sparkles}
            label={isHistorical ? "Done" : "Analyzing"}
            variant={isHistorical ? "success" : "info"}
            animated={!isHistorical}
            size="md"
          />
        }
      />

      {/* Review Steps */}
      <section data-testid="reviewing-steps-section">
        <SectionTitle>Review Progress</SectionTitle>
        <ReviewStepsCard isHistorical={isHistorical === true} />
      </section>

      {/* Files Under Review - placeholder */}
      <section data-testid="files-under-review-empty">
        <SectionTitle muted>Files Under Review</SectionTitle>
        <p className="text-[12px] text-white/35 italic">
          File list will appear once review gathers context
        </p>
      </section>
    </TwoColumnLayout>
  );
}
