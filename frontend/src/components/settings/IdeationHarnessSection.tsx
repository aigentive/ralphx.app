import { useState } from "react";
import { Cpu, TriangleAlert } from "lucide-react";

import { Input } from "@/components/ui/input";
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
  EXECUTION_LANES,
  type Harness,
  type AgentHarnessLaneView,
  type AgentLane,
  type KnownHarness,
  IDEATION_LANES,
} from "@/api/ideation-harness";
import { useAgentHarnessSettings } from "@/hooks/useIdeationHarnessSettings";
import { selectActiveProject, useProjectStore } from "@/stores/projectStore";

const HARNESS_OPTIONS: {
  value: KnownHarness;
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
    description: "Provider-neutral Codex harness with solo ideation and lane-level execution routing",
  },
];

const EFFORT_OPTIONS = [
  { value: "inherit", label: "Inherit" },
  { value: "low", label: "Low" },
  { value: "medium", label: "Medium" },
  { value: "high", label: "High" },
  { value: "xhigh", label: "XHigh" },
] as const;

const APPROVAL_POLICY_OPTIONS = [
  { value: "inherit", label: "Inherit" },
  { value: "untrusted", label: "Untrusted" },
  { value: "on-request", label: "On Request" },
  { value: "never", label: "Never" },
] as const;

const SANDBOX_MODE_OPTIONS = [
  { value: "inherit", label: "Inherit" },
  { value: "read-only", label: "Read Only" },
  { value: "workspace-write", label: "Workspace Write" },
  { value: "danger-full-access", label: "Danger Full Access" },
] as const;

const CODEX_LOCKED_APPROVAL_POLICY = "never";
const CODEX_LOCKED_SANDBOX_MODE = "danger-full-access";
const CODEX_MCP_REQUIREMENT_COPY =
  "Temporarily locked for Codex: RalphX MCP tools currently require Never approval and Danger Full Access.";

const LANE_META: Record<
  AgentLane,
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
  execution_worker: {
    label: "Execution Worker",
    description: "Primary task execution lane",
  },
  execution_reviewer: {
    label: "Execution Reviewer",
    description: "AI review lane after execution completes",
  },
  execution_reexecutor: {
    label: "Execution Re-executor",
    description: "Follow-up execution lane after review requests changes",
  },
  execution_merger: {
    label: "Execution Merger",
    description: "Merge-conflict and merge completion lane",
  },
};

const LANE_GROUPS: {
  id: "ideation" | "execution";
  title: string;
  description: string;
  lanes: AgentLane[];
}[] = [
  {
    id: "ideation",
    title: "Ideation",
    description: "Orchestration, verification, and subagent lanes used during planning.",
    lanes: IDEATION_LANES,
  },
  {
    id: "execution",
    title: "Execution Pipeline",
    description: "Task execution, review, re-execution, and merge lanes.",
    lanes: EXECUTION_LANES,
  },
];

type HarnessSectionScope = "ideation" | "execution";

function defaultsForHarness(
  lane: AgentLane,
  harness: KnownHarness,
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
      approvalPolicy: CODEX_LOCKED_APPROVAL_POLICY,
      sandboxMode: CODEX_LOCKED_SANDBOX_MODE,
      fallbackHarness: null,
    };
  }

  if (lane === "ideation_verifier") {
    return {
      harness,
      model: "gpt-5.4-mini",
      effort: "medium",
      approvalPolicy: CODEX_LOCKED_APPROVAL_POLICY,
      sandboxMode: CODEX_LOCKED_SANDBOX_MODE,
      fallbackHarness: null,
    };
  }

  if (EXECUTION_LANES.includes(lane)) {
    return {
      harness,
      model: "gpt-5.4",
      effort: "xhigh",
      approvalPolicy: CODEX_LOCKED_APPROVAL_POLICY,
      sandboxMode: CODEX_LOCKED_SANDBOX_MODE,
      fallbackHarness: null,
    };
  }

  return {
    harness,
    model: "gpt-5.4-mini",
    effort: "medium",
    approvalPolicy: CODEX_LOCKED_APPROVAL_POLICY,
    sandboxMode: CODEX_LOCKED_SANDBOX_MODE,
    fallbackHarness: null,
  };
}

function availabilityCopy(lane: AgentHarnessLaneView): string {
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

function selectValue(value: string | null | undefined): string {
  return value ?? "inherit";
}

function fromSelectValue(value: string): string | null {
  return value === "inherit" ? null : value;
}

function baseLaneUpdate(lane: AgentHarnessLaneView) {
  return {
    lane: lane.lane,
    harness: lane.configuredHarness ?? lane.effectiveHarness,
    model: lane.row?.model ?? null,
    effort: lane.row?.effort ?? null,
    approvalPolicy: lane.row?.approvalPolicy ?? null,
    sandboxMode: lane.row?.sandboxMode ?? null,
    fallbackHarness: null,
  };
}

function HarnessRow({
  lane,
  disabled,
  onHarnessChange,
  onLaneChange,
  isLast = false,
}: {
  lane: AgentHarnessLaneView;
  disabled: boolean;
  onHarnessChange: (value: KnownHarness) => void;
  onLaneChange: (
    patch: Partial<{
      model: string | null;
      effort: string | null;
      approvalPolicy: string | null;
      sandboxMode: string | null;
      fallbackHarness: Harness | null;
    }>,
  ) => void;
  isLast?: boolean;
}) {
  const meta = LANE_META[lane.lane];
  const configuredHarness = lane.configuredHarness ?? lane.effectiveHarness;
  const showWarning =
    lane.fallbackActivated ||
    !!lane.error ||
    lane.missingCoreExecFeatures.length > 0;
  const modelKey = `${lane.lane}-${lane.row?.model ?? ""}-${configuredHarness}`;
  const showCodexControls = configuredHarness === "codex";
  const codexPolicyLocked = showCodexControls;

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
          <Select
            value={configuredHarness}
            onValueChange={(value) => onHarnessChange(value as KnownHarness)}
            disabled={disabled}
          >
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
        <div className="grid gap-2 pt-1 md:grid-cols-2">
          <div className="space-y-1">
            <p className="text-[11px] font-medium uppercase tracking-[0.08em] text-[var(--text-muted)]">
              Model
            </p>
            <Input
              key={modelKey}
              defaultValue={lane.row?.model ?? ""}
              placeholder={configuredHarness === "codex" ? "gpt-5.4" : "sonnet"}
              disabled={disabled}
              className="h-8 bg-[var(--bg-surface)] border-[var(--border-default)]"
              onBlur={(event) => {
                const nextValue = event.target.value.trim();
                const currentValue = lane.row?.model ?? "";
                if (nextValue === currentValue) {
                  return;
                }
                onLaneChange({ model: nextValue || null });
              }}
            />
          </div>
          <div className="space-y-1">
            <p className="text-[11px] font-medium uppercase tracking-[0.08em] text-[var(--text-muted)]">
              Effort
            </p>
            <Select
              value={selectValue(lane.row?.effort)}
              onValueChange={(value) => onLaneChange({ effort: fromSelectValue(value) })}
              disabled={disabled}
            >
              <SelectTrigger className="h-8 bg-[var(--bg-surface)] border-[var(--border-default)]">
                <SelectValue placeholder="Select effort" />
              </SelectTrigger>
              <SelectContent className="bg-[var(--bg-elevated)] border-[var(--border-default)]">
                {EFFORT_OPTIONS.map((option) => (
                  <SelectItem key={option.value} value={option.value}>
                    {option.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          {showCodexControls && (
            <>
              <div className="space-y-1">
                <p className="text-[11px] font-medium uppercase tracking-[0.08em] text-[var(--text-muted)]">
                  Approval
                </p>
                <Select
                  value={selectValue(lane.row?.approvalPolicy)}
                  onValueChange={(value) =>
                    onLaneChange({ approvalPolicy: fromSelectValue(value) })
                  }
                  disabled={disabled || codexPolicyLocked}
                >
                  <SelectTrigger
                    data-testid={`approval-${lane.lane}`}
                    className="h-8 bg-[var(--bg-surface)] border-[var(--border-default)]"
                  >
                    <SelectValue placeholder="Select approval policy" />
                  </SelectTrigger>
                  <SelectContent className="bg-[var(--bg-elevated)] border-[var(--border-default)]">
                    {APPROVAL_POLICY_OPTIONS.map((option) => (
                      <SelectItem key={option.value} value={option.value}>
                        {option.label}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
              <div className="space-y-1">
                <p className="text-[11px] font-medium uppercase tracking-[0.08em] text-[var(--text-muted)]">
                  Sandbox
                </p>
                <Select
                  value={selectValue(lane.row?.sandboxMode)}
                  onValueChange={(value) =>
                    onLaneChange({ sandboxMode: fromSelectValue(value) })
                  }
                  disabled={disabled || codexPolicyLocked}
                >
                  <SelectTrigger
                    data-testid={`sandbox-${lane.lane}`}
                    className="h-8 bg-[var(--bg-surface)] border-[var(--border-default)]"
                  >
                    <SelectValue placeholder="Select sandbox mode" />
                  </SelectTrigger>
                  <SelectContent className="bg-[var(--bg-elevated)] border-[var(--border-default)]">
                    {SANDBOX_MODE_OPTIONS.map((option) => (
                      <SelectItem key={option.value} value={option.value}>
                        {option.label}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
              <p className="text-[11px] text-[var(--text-muted)] md:col-span-2">
                {CODEX_MCP_REQUIREMENT_COPY}
              </p>
            </>
          )}
        </div>
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
  scope,
}: {
  title: string;
  projectId: string | null;
  projectName: string | null;
  scope: HarnessSectionScope;
}) {
  const [showError, setShowError] = useState(false);
  const {
    lanes,
    isPlaceholderData,
    updateLane,
    saveError,
  } = useAgentHarnessSettings(projectId);
  const isDisabled = projectId === null && title !== "Global Defaults";

  const handleHarnessChange = (lane: AgentLane, harness: KnownHarness) => {
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

  const handleLaneSettingsChange = (
    laneView: AgentHarnessLaneView,
    patch: Partial<{
      model: string | null;
      effort: string | null;
      approvalPolicy: string | null;
      sandboxMode: string | null;
      fallbackHarness: Harness | null;
    }>,
  ) => {
    if (isDisabled) {
      return;
    }

    setShowError(false);
    updateLane(
      {
        ...baseLaneUpdate(laneView),
        ...patch,
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
            ? scope === "execution"
              ? "Default harness policy for execution worker, reviewer, re-executor, and merger lanes."
              : "Default harness policy for ideation leads, verification, and specialist lanes."
            : projectId
              ? `Project overrides for ${projectName ?? "the active project"}.`
              : scope === "execution"
                ? "Select a project to override execution-pipeline agents for a specific project."
                : "Select a project to override ideation agents for a specific project."}
        </p>
      </div>

      {showError && saveError && (
        <ErrorBanner
          error={saveError.message ?? "Failed to save agent harness settings"}
          onDismiss={() => setShowError(false)}
        />
      )}

      <div className={isDisabled ? "opacity-50 pointer-events-none" : undefined}>
        {LANE_GROUPS.filter((group) => group.id === scope).map((group, groupIndex) => {
          const groupLanes = lanes.filter((lane) => group.lanes.includes(lane.lane));
          if (groupLanes.length === 0) {
            return null;
          }

          return (
            <div key={`${title}-${group.id}`}>
              {groupIndex > 0 && (
                <Separator className="my-4 bg-[var(--border-subtle)]" />
              )}
              <div className="px-2 pb-2">
                <h5 className="text-xs font-semibold uppercase tracking-[0.08em] text-[var(--text-secondary)]">
                  {group.title}
                </h5>
                <p className="mt-1 text-xs text-[var(--text-muted)]">
                  {group.description}
                </p>
              </div>
              {groupLanes.map((lane, index) => (
                <HarnessRow
                  key={`${title}-${lane.lane}`}
                  lane={lane}
                  disabled={isDisabled || isPlaceholderData}
                  onHarnessChange={(value) => handleHarnessChange(lane.lane, value)}
                  onLaneChange={(patch) => handleLaneSettingsChange(lane, patch)}
                  isLast={index === groupLanes.length - 1}
                />
              ))}
            </div>
          );
        })}
      </div>
    </div>
  );
}

function AgentHarnessSection({
  scope,
  title,
  description,
}: {
  scope: HarnessSectionScope;
  title: string;
  description: string;
}) {
  const activeProject = useProjectStore(selectActiveProject);
  const projectId = activeProject?.id ?? null;
  const projectName = activeProject?.name ?? null;

  return (
    <SectionCard
      icon={<Cpu className="w-5 h-5 text-[var(--accent-primary)]" />}
      title={title}
      description={description}
    >
      <div className="space-y-6">
        <HarnessSubsection
          title="Global Defaults"
          projectId={null}
          projectName={null}
          scope={scope}
        />

        <Separator className="bg-[var(--border-subtle)]" />

        <HarnessSubsection
          title="Project Overrides"
          projectId={projectId}
          projectName={projectName}
          scope={scope}
        />
      </div>
    </SectionCard>
  );
}

export function IdeationHarnessSection() {
  return (
    <AgentHarnessSection
      scope="ideation"
      title="Ideation Agents"
      description="Choose Claude or Codex for ideation leads, verification, and specialist lanes. Codex ideation still runs in solo mode, so these settings mainly control planning and verifier routing."
    />
  );
}

export function ExecutionHarnessSection() {
  return (
    <AgentHarnessSection
      scope="execution"
      title="Execution Pipeline Agents"
      description="Choose Claude or Codex for the worker, reviewer, re-executor, and merger lanes. These settings control the live execution pipeline, including Codex approval and sandbox behavior per lane."
    />
  );
}
