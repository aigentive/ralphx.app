import { useState } from "react";
import { Cpu, TriangleAlert } from "lucide-react";

import { Separator } from "@/components/ui/separator";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { ErrorBanner, SectionCard } from "./SettingsView.shared";
import {
  type Harness,
  type IdeationHarnessLaneView,
  type IdeationLane,
} from "@/api/ideation-harness";
import { useIdeationHarnessSettings } from "@/hooks/useIdeationHarnessSettings";
import { selectActiveProject, useProjectStore } from "@/stores/projectStore";

const HARNESS_OPTIONS: {
  value: Harness;
  label: string;
  description: string;
}[] = [
  {
    value: "claude",
    label: "Claude",
    description: "Current default runtime with full task pipeline support",
  },
  {
    value: "codex",
    label: "Codex",
    description: "Phase-1 ideation harness using Codex exec and native subagents",
  },
];

const LANE_META: Record<
  IdeationLane,
  { label: string; description: string }
> = {
  ideation_primary: {
    label: "Primary Ideation",
    description: "Orchestrator and ideation lead lane",
  },
  ideation_verifier: {
    label: "Verification",
    description: "Plan verifier lane",
  },
  ideation_subagent: {
    label: "Ideation Subagents",
    description: "Specialists spawned during ideation",
  },
  ideation_verifier_subagent: {
    label: "Verifier Subagents",
    description: "Critics and specialists spawned during verification",
  },
};

function defaultsForHarness(
  lane: IdeationLane,
  harness: Harness,
): {
  harness: Harness;
  model: string | null;
  effort: string | null;
  approvalPolicy: string | null;
  sandboxMode: string | null;
  fallbackHarness: Harness | null;
} {
  if (harness === "claude") {
    return {
      harness,
      model: null,
      effort: null,
      approvalPolicy: null,
      sandboxMode: null,
      fallbackHarness: null,
    };
  }

  if (lane === "ideation_primary") {
    return {
      harness,
      model: "gpt-5.4",
      effort: "xhigh",
      approvalPolicy: "on-request",
      sandboxMode: "workspace-write",
      fallbackHarness: "claude",
    };
  }

  if (lane === "ideation_verifier") {
    return {
      harness,
      model: "gpt-5.4-mini",
      effort: "medium",
      approvalPolicy: "on-request",
      sandboxMode: "workspace-write",
      fallbackHarness: "claude",
    };
  }

  return {
    harness,
    model: "gpt-5.4-mini",
    effort: "medium",
    approvalPolicy: null,
    sandboxMode: null,
    fallbackHarness: "claude",
  };
}

function availabilityCopy(lane: IdeationHarnessLaneView): string {
  if (lane.fallbackActivated && lane.configuredHarness) {
    return `Configured ${lane.configuredHarness}, using ${lane.effectiveHarness} until the requested harness is available.`;
  }

  if (lane.error) {
    return lane.error;
  }

  if (
    lane.configuredHarness === "codex" &&
    lane.missingCoreExecFeatures.length > 0
  ) {
    return `Missing Codex features: ${lane.missingCoreExecFeatures.join(", ")}.`;
  }

  if (lane.binaryFound && lane.binaryPath) {
    return `${lane.effectiveHarness} detected at ${lane.binaryPath}.`;
  }

  return `${lane.effectiveHarness} is the current effective harness for this lane.`;
}

function HarnessRow({
  lane,
  disabled,
  onChange,
  isLast = false,
}: {
  lane: IdeationHarnessLaneView;
  disabled: boolean;
  onChange: (value: Harness) => void;
  isLast?: boolean;
}) {
  const meta = LANE_META[lane.lane];
  const selectValue = lane.configuredHarness ?? lane.effectiveHarness;
  const showWarning =
    lane.fallbackActivated ||
    !!lane.error ||
    lane.missingCoreExecFeatures.length > 0;

  return (
    <div className={isLast ? undefined : "border-b border-[var(--border-subtle)]"}>
      <div className="flex items-start justify-between py-3 -mx-2 px-2 rounded-md transition-colors hover:bg-[rgba(45,45,45,0.3)]">
        <div className="flex-1 min-w-0 pr-4">
          <label
            htmlFor={`harness-${lane.lane}`}
            className="text-sm font-medium text-[var(--text-primary)]"
          >
            {meta.label}
          </label>
          <p className="text-xs text-[var(--text-muted)] mt-0.5">
            {meta.description}
          </p>
        </div>
        <div className="shrink-0">
          <Select value={selectValue} onValueChange={onChange} disabled={disabled}>
            <SelectTrigger
              id={`harness-${lane.lane}`}
              data-testid={`harness-${lane.lane}`}
              className="w-[180px] bg-[var(--bg-surface)] border-[var(--border-default)] focus:ring-[var(--accent-primary)]"
            >
              <SelectValue placeholder="Select harness" />
            </SelectTrigger>
            <SelectContent className="bg-[var(--bg-elevated)] border-[var(--border-default)]">
              {HARNESS_OPTIONS.map((option) => (
                <SelectItem
                  key={option.value}
                  value={option.value}
                  className="focus:bg-[var(--accent-muted)]"
                >
                  <div className="flex flex-col">
                    <span className="text-[var(--text-primary)]">{option.label}</span>
                    <span className="text-xs text-[var(--text-muted)]">
                      {option.description}
                    </span>
                  </div>
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      </div>
      <div className="pb-3 px-2 space-y-1">
        <p className="text-xs text-[var(--text-muted)]">
          Effective:{" "}
          <span className="text-[var(--text-secondary)]">{lane.effectiveHarness}</span>
        </p>
        <p
          className={[
            "text-xs",
            showWarning ? "text-[var(--warning)]" : "text-[var(--text-muted)]",
          ].join(" ")}
        >
          {showWarning && (
            <TriangleAlert className="inline-block w-3 h-3 mr-1 align-[-2px]" />
          )}
          {availabilityCopy(lane)}
        </p>
      </div>
    </div>
  );
}

function HarnessSubsection({
  title,
  projectId,
  projectName,
}: {
  title: string;
  projectId: string | null;
  projectName: string | null;
}) {
  const [showError, setShowError] = useState(false);
  const {
    lanes,
    isPlaceholderData,
    updateLane,
    saveError,
  } = useIdeationHarnessSettings(projectId);
  const isDisabled = projectId === null && title !== "Global Defaults";

  const handleHarnessChange = (lane: IdeationLane, harness: Harness) => {
    if (isDisabled) {
      return;
    }

    setShowError(false);
    updateLane(
      {
        lane,
        ...defaultsForHarness(lane, harness),
      },
      { onError: () => setShowError(true) },
    );
  };

  return (
    <div>
      <div className="mb-3">
        <h4 className="text-sm font-semibold text-[var(--text-primary)]">{title}</h4>
        <p className="text-xs text-[var(--text-muted)] mt-1">
          {title === "Global Defaults"
            ? "Harness selection for ideation lanes. These overrides take precedence over the legacy ideation model and effort screens."
            : projectId
              ? `Project overrides for ${projectName ?? "the active project"}.`
              : "Select a project to override harnesses for specific ideation lanes."}
        </p>
      </div>

      {showError && saveError && (
        <ErrorBanner
          error={saveError.message ?? "Failed to save ideation harness settings"}
          onDismiss={() => setShowError(false)}
        />
      )}

      <div className={isDisabled ? "opacity-50 pointer-events-none" : undefined}>
        {lanes.map((lane, index) => (
          <HarnessRow
            key={`${title}-${lane.lane}`}
            lane={lane}
            disabled={isDisabled || isPlaceholderData}
            onChange={(value) => handleHarnessChange(lane.lane, value)}
            isLast={index === lanes.length - 1}
          />
        ))}
      </div>
    </div>
  );
}

export function IdeationHarnessSection() {
  const activeProject = useProjectStore(selectActiveProject);
  const projectId = activeProject?.id ?? null;
  const projectName = activeProject?.name ?? null;

  return (
    <SectionCard
      icon={<Cpu className="w-5 h-5 text-[var(--accent-primary)]" />}
      title="Ideation Harnesses"
      description="Choose Claude or Codex per ideation lane. Codex currently runs in solo mode with Codex-native subagents."
    >
      <div className="space-y-6">
        <HarnessSubsection
          title="Global Defaults"
          projectId={null}
          projectName={null}
        />

        <Separator className="bg-[var(--border-subtle)]" />

        <HarnessSubsection
          title="Project Overrides"
          projectId={projectId}
          projectName={projectName}
        />
      </div>
    </SectionCard>
  );
}
