/**
 * IdeationModelSection — Settings section for configuring ideation agent model selection.
 *
 * Uses SectionCard (frosted glass pattern) from SettingsView.shared.tsx.
 * Shows global dropdowns and per-project override dropdowns.
 * Effective value hint shown only when value is `inherit`.
 */

import { useState } from "react";
import { Cpu } from "lucide-react";
import { Separator } from "@/components/ui/separator";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { SectionCard, ErrorBanner } from "./SettingsView.shared";
import { useIdeationModelSettings } from "@/hooks/useIdeationModelSettings";
import { useProjectStore, selectActiveProject } from "@/stores/projectStore";

// ============================================================================
// Constants
// ============================================================================

const MODEL_OPTIONS = [
  {
    value: "inherit",
    label: "Inherit",
    description: "Use default from configuration",
  },
  {
    value: "sonnet",
    label: "Sonnet",
    description: "Fast and capable",
  },
  {
    value: "opus",
    label: "Opus",
    description: "Most capable, highest cost",
  },
  {
    value: "haiku",
    label: "Haiku",
    description: "Fastest, lowest cost",
  },
] as const;

// ============================================================================
// Helpers
// ============================================================================

function formatSource(source: string): string {
  switch (source) {
    case "user":
      return "user override";
    case "global":
      return "global setting";
    case "yaml":
      return "YAML config";
    case "yaml_default":
      return "YAML default";
    default:
      return source;
  }
}

// ============================================================================
// ModelRow — Custom row with optional effective value hint
// ============================================================================

interface ModelRowProps {
  id: string;
  label: string;
  description: string;
  value: string;
  disabled: boolean;
  onChange: (value: string) => void;
  effectiveValue: string;
  effectiveSource: string;
  isPlaceholderData: boolean;
  isLast?: boolean;
}

function ModelRow({
  id,
  label,
  description,
  value,
  disabled,
  onChange,
  effectiveValue,
  effectiveSource,
  isPlaceholderData,
  isLast = false,
}: ModelRowProps) {
  const showHint = value === "inherit" && !isPlaceholderData && !!effectiveValue;

  return (
    <div
      className={
        isLast
          ? undefined
          : "border-b border-[var(--border-subtle)]"
      }
    >
      <div
        className={[
          "flex items-start justify-between py-3 -mx-2 px-2 rounded-md transition-colors",
          !disabled ? "hover:bg-[rgba(45,45,45,0.3)]" : "opacity-50",
        ].join(" ")}
      >
        <div className="flex-1 min-w-0 pr-4">
          <label
            htmlFor={id}
            className="text-sm font-medium text-[var(--text-primary)]"
          >
            {label}
          </label>
          <p className="text-xs text-[var(--text-muted)] mt-0.5">{description}</p>
        </div>
        <div className="shrink-0">
          <Select value={value} onValueChange={onChange} disabled={disabled}>
            <SelectTrigger
              id={id}
              data-testid={id}
              className="w-[180px] bg-[var(--bg-surface)] border-[var(--border-default)] focus:ring-[var(--accent-primary)]"
            >
              <SelectValue placeholder="Select model" />
            </SelectTrigger>
            <SelectContent className="bg-[var(--bg-elevated)] border-[var(--border-default)]">
              {MODEL_OPTIONS.map((opt) => (
                <SelectItem
                  key={opt.value}
                  value={opt.value}
                  className="focus:bg-[var(--accent-muted)]"
                >
                  <div className="flex flex-col">
                    <span className="text-[var(--text-primary)]">{opt.label}</span>
                    <span className="text-xs text-[var(--text-muted)]">
                      {opt.description}
                    </span>
                  </div>
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      </div>
      {showHint && (
        <p className="text-xs text-[var(--text-muted)] pb-2 px-2">
          Effective: <span className="text-[var(--text-secondary)]">{effectiveValue}</span>{" "}
          (from {formatSource(effectiveSource)})
        </p>
      )}
    </div>
  );
}

// ============================================================================
// GlobalModelSubsection
// ============================================================================

function GlobalModelSubsection() {
  const [showError, setShowError] = useState(false);
  const { settings, isPlaceholderData, updateSettings, saveError } = useIdeationModelSettings(null);

  const handlePrimaryChange = (value: string) => {
    setShowError(false);
    updateSettings(
      { primaryModel: value },
      { onError: () => setShowError(true) }
    );
  };

  const handleVerifierChange = (value: string) => {
    setShowError(false);
    updateSettings(
      { verifierModel: value },
      { onError: () => setShowError(true) }
    );
  };

  return (
    <div>
      {showError && saveError && (
        <ErrorBanner
          error={saveError.message ?? "Failed to save model settings"}
          onDismiss={() => setShowError(false)}
        />
      )}
      <div className="space-y-0">
        <ModelRow
          id="global-primary-model"
          label="Primary Ideation Model"
          description="Model for orchestrator-ideation and team-lead agents"
          value={settings.primaryModel}
          disabled={false}
          onChange={handlePrimaryChange}
          effectiveValue={settings.effectivePrimaryModel}
          effectiveSource={settings.primaryModelSource}
          isPlaceholderData={isPlaceholderData}
        />
        <ModelRow
          id="global-verifier-model"
          label="Verification Model"
          description="Model for plan-verifier agent"
          value={settings.verifierModel}
          disabled={false}
          onChange={handleVerifierChange}
          effectiveValue={settings.effectiveVerifierModel}
          effectiveSource={settings.verifierModelSource}
          isPlaceholderData={isPlaceholderData}
          isLast
        />
      </div>
    </div>
  );
}

// ============================================================================
// ProjectModelSubsection
// ============================================================================

interface ProjectModelSubsectionProps {
  projectId: string | null;
  projectName: string | null;
}

function ProjectModelSubsection({
  projectId,
  projectName,
}: ProjectModelSubsectionProps) {
  const [showError, setShowError] = useState(false);
  const { settings, isPlaceholderData, updateSettings, saveError } = useIdeationModelSettings(projectId);
  const isDisabled = projectId === null;

  const handlePrimaryChange = (value: string) => {
    if (isDisabled) return;
    setShowError(false);
    updateSettings(
      { primaryModel: value },
      { onError: () => setShowError(true) }
    );
  };

  const handleVerifierChange = (value: string) => {
    if (isDisabled) return;
    setShowError(false);
    updateSettings(
      { verifierModel: value },
      { onError: () => setShowError(true) }
    );
  };

  return (
    <div>
      <div className="mb-3">
        <p className="text-xs font-semibold uppercase tracking-wider text-[var(--text-muted)]">
          {projectName ? `Project: ${projectName}` : "Project Override"}
        </p>
        {isDisabled && (
          <p className="text-xs text-[var(--text-muted)] mt-1">
            Select a project to configure per-project overrides
          </p>
        )}
      </div>
      {showError && saveError && (
        <ErrorBanner
          error={saveError.message ?? "Failed to save model settings"}
          onDismiss={() => setShowError(false)}
        />
      )}
      <div className="space-y-0">
        <ModelRow
          id="project-primary-model"
          label="Primary Ideation Model"
          description="Override for this project's ideation agents"
          value={settings.primaryModel}
          disabled={isDisabled}
          onChange={handlePrimaryChange}
          effectiveValue={settings.effectivePrimaryModel}
          effectiveSource={settings.primaryModelSource}
          isPlaceholderData={isPlaceholderData}
        />
        <ModelRow
          id="project-verifier-model"
          label="Verification Model"
          description="Override for this project's plan-verifier agent"
          value={settings.verifierModel}
          disabled={isDisabled}
          onChange={handleVerifierChange}
          effectiveValue={settings.effectiveVerifierModel}
          effectiveSource={settings.verifierModelSource}
          isPlaceholderData={isPlaceholderData}
          isLast
        />
      </div>
    </div>
  );
}

// ============================================================================
// IdeationModelSection — Main export
// ============================================================================

export function IdeationModelSection() {
  const activeProject = useProjectStore(selectActiveProject);

  return (
    <SectionCard
      icon={
        <Cpu className="w-[18px] h-[18px] text-[var(--accent-primary)]" />
      }
      title="Ideation Model"
      description="Configure AI model for ideation and verification agents"
    >
      <GlobalModelSubsection />
      <Separator className="my-4 bg-[var(--border-subtle)]" />
      <ProjectModelSubsection
        projectId={activeProject?.id ?? null}
        projectName={activeProject?.name ?? null}
      />
    </SectionCard>
  );
}
