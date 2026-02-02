/**
 * SettingsView - Configuration panel for project settings
 *
 * Design: macOS Tahoe Liquid Glass
 * - Frosted glass header with backdrop-blur
 * - Flat translucent section cards
 * - Ambient orange glow background
 */

import { useState, useCallback } from "react";
import {
  Settings,
  Zap,
  Brain,
  FileSearch,
  Shield,
} from "lucide-react";
import { ScrollArea } from "@/components/ui/scroll-area";
import type {
  ProjectSettings,
  ExecutionSettings,
  ModelSettings,
  ProjectReviewSettings,
  SupervisorSettings,
} from "@/types/settings";
import { DEFAULT_PROJECT_SETTINGS } from "@/types/settings";
import { IdeationSettingsPanel } from "./IdeationSettingsPanel";
import { GitSettingsSection } from "./GitSettingsSection";
import {
  MODEL_OPTIONS,
  SavingIndicator,
  ToggleSettingRow,
  NumberSettingRow,
  SelectSettingRow,
  SectionCard,
  SettingsSkeleton,
  ErrorBanner,
} from "./SettingsView.shared";

// ============================================================================
// Section Components
// ============================================================================

interface ExecutionSectionProps {
  settings: ExecutionSettings;
  onChange: (settings: Partial<ExecutionSettings>) => void;
  disabled: boolean;
}

function ExecutionSection({
  settings,
  onChange,
  disabled,
}: ExecutionSectionProps) {
  return (
    <SectionCard
      icon={<Zap className="w-[18px] h-[18px] text-[var(--accent-primary)]" />}
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
        onChange={() =>
          onChange({
            review_before_destructive: !settings.review_before_destructive,
          })
        }
      />
    </SectionCard>
  );
}

interface ModelSectionProps {
  settings: ModelSettings;
  onChange: (settings: Partial<ModelSettings>) => void;
  disabled: boolean;
}

function ModelSection({ settings, onChange, disabled }: ModelSectionProps) {
  return (
    <SectionCard
      icon={<Brain className="w-[18px] h-[18px] text-[var(--accent-primary)]" />}
      title="Model"
      description="Configure AI model selection"
    >
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
        onChange={() =>
          onChange({ allow_opus_upgrade: !settings.allow_opus_upgrade })
        }
      />
    </SectionCard>
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
    <SectionCard
      icon={
        <FileSearch className="w-[18px] h-[18px] text-[var(--accent-primary)]" />
      }
      title="Review"
      description="Configure code review automation"
    >
      <ToggleSettingRow
        id="ai-review-enabled"
        label="Enable AI Review"
        description="Automatically review completed tasks with AI"
        checked={settings.ai_review_enabled}
        disabled={disabled}
        onChange={() =>
          onChange({ ai_review_enabled: !settings.ai_review_enabled })
        }
      />
      <ToggleSettingRow
        id="ai-review-auto-fix"
        label="Auto Create Fix Tasks"
        description="Automatically create fix tasks when review fails"
        checked={settings.ai_review_auto_fix}
        disabled={isSubSettingsDisabled}
        onChange={() =>
          onChange({ ai_review_auto_fix: !settings.ai_review_auto_fix })
        }
        isSubSetting
      />
      <ToggleSettingRow
        id="require-fix-approval"
        label="Require Fix Approval"
        description="Require human approval before executing AI-proposed fix tasks"
        checked={settings.require_fix_approval}
        disabled={isSubSettingsDisabled}
        onChange={() =>
          onChange({ require_fix_approval: !settings.require_fix_approval })
        }
        isSubSetting
      />
      <ToggleSettingRow
        id="require-human-review"
        label="Require Human Review"
        description="Require human review even after AI approval"
        checked={settings.require_human_review}
        disabled={isSubSettingsDisabled}
        onChange={() =>
          onChange({ require_human_review: !settings.require_human_review })
        }
        isSubSetting
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
        isSubSetting
      />
    </SectionCard>
  );
}

interface SupervisorSectionProps {
  settings: SupervisorSettings;
  onChange: (settings: Partial<SupervisorSettings>) => void;
  disabled: boolean;
}

function SupervisorSection({
  settings,
  onChange,
  disabled,
}: SupervisorSectionProps) {
  const isSubSettingsDisabled = disabled || !settings.supervisor_enabled;

  return (
    <SectionCard
      icon={<Shield className="w-[18px] h-[18px] text-[var(--accent-primary)]" />}
      title="Supervisor"
      description="Configure watchdog monitoring for stuck or looping agents"
    >
      <ToggleSettingRow
        id="supervisor-enabled"
        label="Enable Supervisor"
        description="Enable watchdog monitoring for agent execution"
        checked={settings.supervisor_enabled}
        disabled={disabled}
        onChange={() =>
          onChange({ supervisor_enabled: !settings.supervisor_enabled })
        }
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
        isSubSetting
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
        isSubSetting
      />
    </SectionCard>
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
  const [dismissedError, setDismissedError] = useState(false);

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

  const handleDismissError = useCallback(() => {
    setDismissedError(true);
  }, []);

  // Reset dismissed state when error changes
  const showError = error && !dismissedError;

  if (isLoading) {
    return <SettingsSkeleton />;
  }

  return (
    <div
      data-testid="settings-view"
      className="flex flex-col h-full"
      style={{
        background: `
          radial-gradient(ellipse 80% 50% at 20% 0%, rgba(255,107,53,0.06) 0%, transparent 50%),
          radial-gradient(ellipse 60% 40% at 80% 100%, rgba(255,107,53,0.03) 0%, transparent 50%),
          var(--bg-base)
        `,
      }}
    >
      {/* Header - Frosted Glass */}
      <div
        className="flex items-center justify-between px-6 py-4 border-b"
        style={{
          background: "rgba(18,18,18,0.85)",
          backdropFilter: "blur(20px)",
          WebkitBackdropFilter: "blur(20px)",
          borderColor: "rgba(255,255,255,0.06)",
        }}
      >
        <div className="flex items-center gap-3">
          <div
            className="p-2 rounded-lg"
            style={{
              background: "rgba(255,107,53,0.1)",
              border: "1px solid rgba(255,107,53,0.2)",
            }}
          >
            <Settings className="w-5 h-5 text-[var(--accent-primary)]" />
          </div>
          <div>
            <h2 className="text-lg font-semibold tracking-tight text-[var(--text-primary)]">
              Settings
            </h2>
            <p className="text-sm text-[var(--text-muted)]">
              Configure project behavior
            </p>
          </div>
        </div>
        {isSaving && <SavingIndicator />}
      </div>

      {/* Error Banner */}
      {showError && (
        <ErrorBanner error={error} onDismiss={handleDismissError} />
      )}

      {/* Settings Sections with ScrollArea */}
      <ScrollArea className="flex-1">
        <div className="p-6 space-y-6 max-w-[720px] mx-auto">
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
          <GitSettingsSection />
          <IdeationSettingsPanel />
        </div>
      </ScrollArea>
    </div>
  );
}
