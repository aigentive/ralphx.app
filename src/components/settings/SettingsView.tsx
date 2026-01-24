/**
 * SettingsView - Configuration panel for project settings
 *
 * Features:
 * - Execution section: max_concurrent_tasks, auto_commit, pause_on_failure
 * - Model section: model selection, Opus upgrade option
 * - Review section: ai_review_enabled, auto_fix, require_human_review, max_fix_attempts
 * - Supervisor section: supervisor_enabled, loop_threshold, stuck_timeout
 * - Profile management: create/edit/delete custom profiles
 */

import { useState, useCallback } from "react";
import type {
  ProjectSettings,
  ExecutionSettings,
  ModelSettings,
  ProjectReviewSettings,
  SupervisorSettings,
} from "@/types/settings";
import type { Model } from "@/types/agent-profile";
import { DEFAULT_PROJECT_SETTINGS } from "@/types/settings";

// ============================================================================
// Constants
// ============================================================================

const MODEL_OPTIONS: { value: Model; label: string; description: string }[] = [
  {
    value: "haiku",
    label: "Claude Haiku 4.5",
    description: "Fastest, most cost-effective",
  },
  {
    value: "sonnet",
    label: "Claude Sonnet 4.5",
    description: "Best balance of speed and quality",
  },
  {
    value: "opus",
    label: "Claude Opus 4.5",
    description: "Most capable, best for complex tasks",
  },
];

// ============================================================================
// Toggle Component
// ============================================================================

interface ToggleProps {
  id: string;
  checked: boolean;
  disabled: boolean;
  onChange: () => void;
  ariaDescribedBy?: string;
}

function Toggle({
  id,
  checked,
  disabled,
  onChange,
  ariaDescribedBy,
}: ToggleProps) {
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        if (!disabled) onChange();
      }
    },
    [disabled, onChange]
  );

  return (
    <button
      type="button"
      role="switch"
      id={id}
      data-testid={id}
      aria-checked={checked}
      aria-disabled={disabled}
      aria-describedby={ariaDescribedBy}
      tabIndex={disabled ? -1 : 0}
      onClick={() => !disabled && onChange()}
      onKeyDown={handleKeyDown}
      className={`relative inline-flex h-6 w-11 flex-shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-[--accent-primary] focus:ring-offset-2 focus:ring-offset-[--bg-base] ${
        disabled ? "cursor-not-allowed opacity-50" : ""
      } ${checked ? "bg-[--accent-primary]" : "bg-[--bg-hover]"}`}
    >
      <span
        aria-hidden="true"
        className={`pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out ${
          checked ? "translate-x-5" : "translate-x-0"
        }`}
      />
    </button>
  );
}

// ============================================================================
// Setting Row Component
// ============================================================================

interface SettingRowProps {
  id: string;
  label: string;
  description: string;
  children: React.ReactNode;
}

function SettingRow({ id, label, description, children }: SettingRowProps) {
  return (
    <div className="flex items-start justify-between gap-4 py-3">
      <div className="flex-1 min-w-0">
        <label
          htmlFor={id}
          className="text-sm font-medium text-[--text-primary]"
        >
          {label}
        </label>
        <p id={`${id}-desc`} className="mt-0.5 text-xs text-[--text-muted]">
          {description}
        </p>
      </div>
      {children}
    </div>
  );
}

// ============================================================================
// Toggle Setting Row
// ============================================================================

interface ToggleSettingRowProps {
  id: string;
  label: string;
  description: string;
  checked: boolean;
  disabled: boolean;
  onChange: () => void;
}

function ToggleSettingRow({
  id,
  label,
  description,
  checked,
  disabled,
  onChange,
}: ToggleSettingRowProps) {
  return (
    <SettingRow id={id} label={label} description={description}>
      <Toggle
        id={id}
        checked={checked}
        disabled={disabled}
        onChange={onChange}
        ariaDescribedBy={`${id}-desc`}
      />
    </SettingRow>
  );
}

// ============================================================================
// Number Input Setting Row
// ============================================================================

interface NumberSettingRowProps {
  id: string;
  label: string;
  description: string;
  value: number;
  min: number;
  max: number;
  step: number;
  unit: string;
  disabled: boolean;
  onChange: (value: number) => void;
}

function NumberSettingRow({
  id,
  label,
  description,
  value,
  min,
  max,
  step,
  unit,
  disabled,
  onChange,
}: NumberSettingRowProps) {
  return (
    <SettingRow id={id} label={label} description={description}>
      <div className="flex items-center gap-2">
        <input
          type="number"
          id={id}
          data-testid={id}
          aria-describedby={`${id}-desc`}
          value={value}
          min={min}
          max={max}
          step={step}
          disabled={disabled}
          onChange={(e) => {
            const val = parseInt(e.target.value, 10);
            if (!isNaN(val) && val >= min && val <= max) {
              onChange(val);
            }
          }}
          className={`w-20 px-2 py-1.5 text-sm rounded-md bg-[--bg-elevated] border border-[--border-subtle] text-[--text-primary] text-right focus:outline-none focus:ring-2 focus:ring-[--accent-primary] focus:border-transparent ${
            disabled ? "opacity-50 cursor-not-allowed" : ""
          }`}
        />
        {unit && <span className="text-sm text-[--text-muted]">{unit}</span>}
      </div>
    </SettingRow>
  );
}

// ============================================================================
// Select Setting Row
// ============================================================================

interface SelectOption<T extends string> {
  value: T;
  label: string;
  description: string;
}

interface SelectSettingRowProps<T extends string> {
  id: string;
  label: string;
  description: string;
  value: T;
  options: SelectOption<T>[];
  disabled: boolean;
  onChange: (value: T) => void;
}

function SelectSettingRow<T extends string>({
  id,
  label,
  description,
  value,
  options,
  disabled,
  onChange,
}: SelectSettingRowProps<T>) {
  return (
    <SettingRow id={id} label={label} description={description}>
      <select
        id={id}
        data-testid={id}
        aria-describedby={`${id}-desc`}
        value={value}
        disabled={disabled}
        onChange={(e) => onChange(e.target.value as T)}
        className={`w-48 px-3 py-1.5 text-sm rounded-md bg-[--bg-elevated] border border-[--border-subtle] text-[--text-primary] focus:outline-none focus:ring-2 focus:ring-[--accent-primary] focus:border-transparent ${
          disabled ? "opacity-50 cursor-not-allowed" : ""
        }`}
      >
        {options.map((opt) => (
          <option key={opt.value} value={opt.value}>
            {opt.label}
          </option>
        ))}
      </select>
    </SettingRow>
  );
}

// ============================================================================
// Section Component
// ============================================================================

interface SectionContainerProps {
  title: string;
  description: string;
  children: React.ReactNode;
}

function SectionContainer({ title, description, children }: SectionContainerProps) {
  return (
    <div
      className="rounded-lg border p-4"
      style={{
        backgroundColor: "var(--bg-elevated)",
        borderColor: "var(--border-subtle)",
      }}
    >
      <div className="mb-4">
        <h3 className="text-sm font-semibold text-[--text-primary]">{title}</h3>
        <p className="text-xs text-[--text-muted] mt-0.5">{description}</p>
      </div>
      <div className="divide-y divide-[--border-subtle]">{children}</div>
    </div>
  );
}

// ============================================================================
// Skeleton Component
// ============================================================================

function SettingsSkeleton() {
  return (
    <div data-testid="settings-skeleton" className="animate-pulse space-y-6 p-4">
      {[1, 2, 3, 4].map((i) => (
        <div key={i} className="rounded-lg bg-[--bg-elevated] p-4 space-y-4">
          <div className="h-5 w-32 rounded bg-[--bg-hover]" />
          <div className="space-y-3">
            {[1, 2, 3].map((j) => (
              <div key={j} className="flex justify-between">
                <div className="h-4 w-48 rounded bg-[--bg-hover]" />
                <div className="h-6 w-11 rounded-full bg-[--bg-hover]" />
              </div>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}

// ============================================================================
// Section Components
// ============================================================================

interface ExecutionSectionProps {
  settings: ExecutionSettings;
  onChange: (settings: Partial<ExecutionSettings>) => void;
  disabled: boolean;
}

function ExecutionSection({ settings, onChange, disabled }: ExecutionSectionProps) {
  return (
    <SectionContainer
      title="Execution"
      description="Control task execution behavior and concurrency"
    >
      <NumberSettingRow
        id="max-concurrent-tasks"
        label="Max Concurrent Tasks"
        description="Maximum number of tasks to run simultaneously (1-10)"
        value={settings.max_concurrent_tasks}
        min={1}
        max={10}
        step={1}
        unit=""
        disabled={disabled}
        onChange={(value) => onChange({ max_concurrent_tasks: value })}
      />
      <ToggleSettingRow
        id="auto-commit"
        label="Auto Commit"
        description="Automatically commit changes after each completed task"
        checked={settings.auto_commit}
        disabled={disabled}
        onChange={() => onChange({ auto_commit: !settings.auto_commit })}
      />
      <ToggleSettingRow
        id="pause-on-failure"
        label="Pause on Failure"
        description="Stop the task queue when a task fails"
        checked={settings.pause_on_failure}
        disabled={disabled}
        onChange={() => onChange({ pause_on_failure: !settings.pause_on_failure })}
      />
      <ToggleSettingRow
        id="review-before-destructive"
        label="Review Before Destructive"
        description="Insert review point before tasks that delete files or modify configs"
        checked={settings.review_before_destructive}
        disabled={disabled}
        onChange={() => onChange({ review_before_destructive: !settings.review_before_destructive })}
      />
    </SectionContainer>
  );
}

interface ModelSectionProps {
  settings: ModelSettings;
  onChange: (settings: Partial<ModelSettings>) => void;
  disabled: boolean;
}

function ModelSection({ settings, onChange, disabled }: ModelSectionProps) {
  return (
    <SectionContainer title="Model" description="Configure AI model selection">
      <SelectSettingRow
        id="model-selection"
        label="Default Model"
        description="Model to use for task execution"
        value={settings.model}
        options={MODEL_OPTIONS}
        disabled={disabled}
        onChange={(value) => onChange({ model: value })}
      />
      <ToggleSettingRow
        id="allow-opus-upgrade"
        label="Allow Opus Upgrade"
        description="Automatically upgrade to Opus for complex tasks"
        checked={settings.allow_opus_upgrade}
        disabled={disabled}
        onChange={() => onChange({ allow_opus_upgrade: !settings.allow_opus_upgrade })}
      />
    </SectionContainer>
  );
}

interface ReviewSectionProps {
  settings: ProjectReviewSettings;
  onChange: (settings: Partial<ProjectReviewSettings>) => void;
  disabled: boolean;
}

function ReviewSection({ settings, onChange, disabled }: ReviewSectionProps) {
  const isSubSettingsDisabled = disabled || !settings.ai_review_enabled;

  return (
    <SectionContainer title="Review" description="Configure code review automation">
      <ToggleSettingRow
        id="ai-review-enabled"
        label="Enable AI Review"
        description="Automatically review completed tasks with AI"
        checked={settings.ai_review_enabled}
        disabled={disabled}
        onChange={() => onChange({ ai_review_enabled: !settings.ai_review_enabled })}
      />
      <ToggleSettingRow
        id="ai-review-auto-fix"
        label="Auto Create Fix Tasks"
        description="Automatically create fix tasks when review fails"
        checked={settings.ai_review_auto_fix}
        disabled={isSubSettingsDisabled}
        onChange={() => onChange({ ai_review_auto_fix: !settings.ai_review_auto_fix })}
      />
      <ToggleSettingRow
        id="require-fix-approval"
        label="Require Fix Approval"
        description="Require human approval before executing AI-proposed fix tasks"
        checked={settings.require_fix_approval}
        disabled={isSubSettingsDisabled}
        onChange={() => onChange({ require_fix_approval: !settings.require_fix_approval })}
      />
      <ToggleSettingRow
        id="require-human-review"
        label="Require Human Review"
        description="Require human review even after AI approval"
        checked={settings.require_human_review}
        disabled={isSubSettingsDisabled}
        onChange={() => onChange({ require_human_review: !settings.require_human_review })}
      />
      <NumberSettingRow
        id="max-fix-attempts"
        label="Max Fix Attempts"
        description="Maximum times AI can propose fixes before moving to backlog"
        value={settings.max_fix_attempts}
        min={1}
        max={10}
        step={1}
        unit=""
        disabled={isSubSettingsDisabled}
        onChange={(value) => onChange({ max_fix_attempts: value })}
      />
    </SectionContainer>
  );
}

interface SupervisorSectionProps {
  settings: SupervisorSettings;
  onChange: (settings: Partial<SupervisorSettings>) => void;
  disabled: boolean;
}

function SupervisorSection({ settings, onChange, disabled }: SupervisorSectionProps) {
  const isSubSettingsDisabled = disabled || !settings.supervisor_enabled;

  return (
    <SectionContainer
      title="Supervisor"
      description="Configure watchdog monitoring for stuck or looping agents"
    >
      <ToggleSettingRow
        id="supervisor-enabled"
        label="Enable Supervisor"
        description="Enable watchdog monitoring for agent execution"
        checked={settings.supervisor_enabled}
        disabled={disabled}
        onChange={() => onChange({ supervisor_enabled: !settings.supervisor_enabled })}
      />
      <NumberSettingRow
        id="loop-threshold"
        label="Loop Threshold"
        description="Number of identical tool calls before loop detection"
        value={settings.loop_threshold}
        min={2}
        max={10}
        step={1}
        unit=""
        disabled={isSubSettingsDisabled}
        onChange={(value) => onChange({ loop_threshold: value })}
      />
      <NumberSettingRow
        id="stuck-timeout"
        label="Stuck Timeout"
        description="Seconds without progress before stuck detection"
        value={settings.stuck_timeout}
        min={60}
        max={1800}
        step={30}
        unit="seconds"
        disabled={isSubSettingsDisabled}
        onChange={(value) => onChange({ stuck_timeout: value })}
      />
    </SectionContainer>
  );
}

// ============================================================================
// Main Component
// ============================================================================

export interface SettingsViewProps {
  /** Initial settings (if undefined, uses defaults) */
  initialSettings?: ProjectSettings;
  /** Whether to show loading state */
  isLoading?: boolean;
  /** Whether settings are being saved */
  isSaving?: boolean;
  /** Error message to display */
  error?: string | null;
  /** Callback when settings change */
  onSettingsChange?: (settings: ProjectSettings) => void;
}

export function SettingsView({
  initialSettings,
  isLoading = false,
  isSaving = false,
  error = null,
  onSettingsChange,
}: SettingsViewProps) {
  const [settings, setSettings] = useState<ProjectSettings>(
    initialSettings ?? DEFAULT_PROJECT_SETTINGS
  );

  const handleExecutionChange = useCallback(
    (changes: Partial<ExecutionSettings>) => {
      setSettings((prev) => {
        const updated = {
          ...prev,
          execution: { ...prev.execution, ...changes },
        };
        onSettingsChange?.(updated);
        return updated;
      });
    },
    [onSettingsChange]
  );

  const handleModelChange = useCallback(
    (changes: Partial<ModelSettings>) => {
      setSettings((prev) => {
        const updated = {
          ...prev,
          model: { ...prev.model, ...changes },
        };
        onSettingsChange?.(updated);
        return updated;
      });
    },
    [onSettingsChange]
  );

  const handleReviewChange = useCallback(
    (changes: Partial<ProjectReviewSettings>) => {
      setSettings((prev) => {
        const updated = {
          ...prev,
          review: { ...prev.review, ...changes },
        };
        onSettingsChange?.(updated);
        return updated;
      });
    },
    [onSettingsChange]
  );

  const handleSupervisorChange = useCallback(
    (changes: Partial<SupervisorSettings>) => {
      setSettings((prev) => {
        const updated = {
          ...prev,
          supervisor: { ...prev.supervisor, ...changes },
        };
        onSettingsChange?.(updated);
        return updated;
      });
    },
    [onSettingsChange]
  );

  if (isLoading) {
    return <SettingsSkeleton />;
  }

  return (
    <div
      data-testid="settings-view"
      className="flex flex-col h-full"
      style={{ backgroundColor: "var(--bg-surface)" }}
    >
      {/* Header */}
      <div
        className="flex items-center justify-between px-4 py-3 border-b"
        style={{ borderColor: "var(--border-subtle)" }}
      >
        <div>
          <h2
            className="text-lg font-semibold"
            style={{ color: "var(--text-primary)" }}
          >
            Settings
          </h2>
          <p className="text-sm" style={{ color: "var(--text-muted)" }}>
            Configure project behavior
          </p>
        </div>
        {isSaving && (
          <span
            className="text-sm px-3 py-1 rounded-md"
            style={{
              backgroundColor: "var(--bg-elevated)",
              color: "var(--text-muted)",
            }}
          >
            Saving...
          </span>
        )}
      </div>

      {/* Error Message */}
      {error && (
        <div
          className="mx-4 mt-4 p-3 rounded-md"
          style={{
            backgroundColor: "rgba(239, 68, 68, 0.1)",
            color: "var(--status-error)",
          }}
        >
          <p className="text-sm">{error}</p>
        </div>
      )}

      {/* Settings Sections */}
      <div className="flex-1 overflow-y-auto p-4 space-y-4">
        <ExecutionSection
          settings={settings.execution}
          onChange={handleExecutionChange}
          disabled={isSaving}
        />
        <ModelSection
          settings={settings.model}
          onChange={handleModelChange}
          disabled={isSaving}
        />
        <ReviewSection
          settings={settings.review}
          onChange={handleReviewChange}
          disabled={isSaving}
        />
        <SupervisorSection
          settings={settings.supervisor}
          onChange={handleSupervisorChange}
          disabled={isSaving}
        />
      </div>
    </div>
  );
}
