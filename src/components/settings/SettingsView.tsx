/**
 * SettingsView - Premium configuration panel for project settings
 *
 * Features:
 * - Glass effect header with Settings icon and saving indicator
 * - Section cards (Execution, Model, Review, Supervisor) with gradient borders
 * - shadcn Switch, Input, Select for form controls
 * - Master toggle → sub-settings disabled pattern
 * - Lucide icons: Settings, Zap, Brain, FileSearch, Shield, Loader2, AlertCircle, X
 */

import { useState, useCallback } from "react";
import {
  Settings,
  Zap,
  Brain,
  FileSearch,
  Shield,
  Loader2,
  AlertCircle,
  X,
} from "lucide-react";
import { Card } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import { Switch } from "@/components/ui/switch";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Skeleton } from "@/components/ui/skeleton";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
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
// Saving Indicator Component
// ============================================================================

function SavingIndicator() {
  return (
    <div className="flex items-center gap-2 px-3 py-1 rounded-full bg-[var(--bg-elevated)] text-[var(--text-muted)] text-sm">
      <Loader2 className="w-3.5 h-3.5 animate-spin" />
      <span>Saving...</span>
    </div>
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
  isSubSetting?: boolean;
  isDisabled?: boolean;
}

function SettingRow({
  id,
  label,
  description,
  children,
  isSubSetting = false,
  isDisabled = false,
}: SettingRowProps) {
  return (
    <div
      className={cn(
        "flex items-start justify-between py-3 border-b border-[var(--border-subtle)] last:border-0 -mx-2 px-2 rounded-md transition-colors",
        !isDisabled && "hover:bg-[rgba(45,45,45,0.3)]",
        isDisabled && "opacity-50"
      )}
    >
      <div
        className={cn(
          "flex-1 min-w-0 pr-4",
          isSubSetting && "pl-4 border-l-2 border-[var(--border-subtle)]"
        )}
      >
        <Label
          htmlFor={id}
          className="text-sm font-medium text-[var(--text-primary)]"
        >
          {label}
        </Label>
        <p id={`${id}-desc`} className="text-xs text-[var(--text-muted)] mt-0.5">
          {description}
        </p>
      </div>
      <div className="shrink-0">{children}</div>
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
  isSubSetting?: boolean;
}

function ToggleSettingRow({
  id,
  label,
  description,
  checked,
  disabled,
  onChange,
  isSubSetting = false,
}: ToggleSettingRowProps) {
  return (
    <SettingRow
      id={id}
      label={label}
      description={description}
      isSubSetting={isSubSetting}
      isDisabled={disabled}
    >
      <Switch
        id={id}
        data-testid={id}
        checked={checked}
        onCheckedChange={onChange}
        disabled={disabled}
        aria-describedby={`${id}-desc`}
        className="data-[state=checked]:bg-[var(--accent-primary)]"
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
  isSubSetting?: boolean;
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
  isSubSetting = false,
}: NumberSettingRowProps) {
  return (
    <SettingRow
      id={id}
      label={label}
      description={description}
      isSubSetting={isSubSetting}
      isDisabled={disabled}
    >
      <div className="flex items-center gap-2">
        <Input
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
          className="w-20 text-right bg-[var(--bg-surface)] border-[var(--border-default)] focus:border-[var(--accent-primary)] focus:ring-[var(--accent-primary)] [appearance:textfield] [&::-webkit-outer-spin-button]:appearance-none [&::-webkit-inner-spin-button]:appearance-none"
        />
        {unit && (
          <span className="text-xs text-[var(--text-muted)]">{unit}</span>
        )}
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
    <SettingRow id={id} label={label} description={description} isDisabled={disabled}>
      <Select
        value={value}
        onValueChange={onChange}
        disabled={disabled}
      >
        <SelectTrigger
          id={id}
          data-testid={id}
          aria-describedby={`${id}-desc`}
          className="w-[200px] bg-[var(--bg-surface)] border-[var(--border-default)] focus:ring-[var(--accent-primary)]"
        >
          <SelectValue placeholder="Select model" />
        </SelectTrigger>
        <SelectContent className="bg-[var(--bg-elevated)] border-[var(--border-default)]">
          {options.map((opt) => (
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
    </SettingRow>
  );
}

// ============================================================================
// Section Card Component
// ============================================================================

interface SectionCardProps {
  icon: React.ReactNode;
  title: string;
  description: string;
  children: React.ReactNode;
}

function SectionCard({ icon, title, description, children }: SectionCardProps) {
  return (
    <Card
      className={cn(
        "bg-[var(--bg-elevated)] border-[var(--border-default)] shadow-[var(--shadow-xs)]",
        // Gradient border technique
        "border border-transparent",
        "[background:linear-gradient(var(--bg-elevated),var(--bg-elevated))_padding-box,linear-gradient(180deg,rgba(255,255,255,0.08)_0%,rgba(255,255,255,0.02)_100%)_border-box]"
      )}
    >
      <div className="flex items-start gap-3 p-5 pb-0">
        <div className="p-2 rounded-lg bg-[var(--accent-muted)] shrink-0">
          {icon}
        </div>
        <div>
          <h3 className="text-sm font-semibold tracking-tight text-[var(--text-primary)]">
            {title}
          </h3>
          <p className="text-xs text-[var(--text-muted)] mt-0.5">{description}</p>
        </div>
      </div>
      <Separator className="my-4 bg-[var(--border-subtle)]" />
      <div className="px-5 pb-5 space-y-1">{children}</div>
    </Card>
  );
}

// ============================================================================
// Skeleton Component
// ============================================================================

function SettingsSkeleton() {
  return (
    <div
      data-testid="settings-skeleton"
      className="p-6 space-y-6 max-w-[720px] mx-auto"
    >
      {[1, 2, 3, 4].map((i) => (
        <Card key={i} className="p-5 bg-[var(--bg-elevated)] border-[var(--border-default)]">
          <div className="flex items-center gap-3 mb-4">
            <Skeleton className="w-9 h-9 rounded-lg" />
            <div className="space-y-2">
              <Skeleton className="h-4 w-24" />
              <Skeleton className="h-3 w-40" />
            </div>
          </div>
          <Separator className="my-4" />
          <div className="space-y-4">
            {[1, 2, 3].map((j) => (
              <div key={j} className="flex justify-between items-center">
                <div className="space-y-1">
                  <Skeleton className="h-4 w-32" />
                  <Skeleton className="h-3 w-48" />
                </div>
                <Skeleton className="h-6 w-11 rounded-full" />
              </div>
            ))}
          </div>
        </Card>
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
// Error Banner Component
// ============================================================================

interface ErrorBannerProps {
  error: string;
  onDismiss: () => void;
}

function ErrorBanner({ error, onDismiss }: ErrorBannerProps) {
  return (
    <div className="mx-6 mt-4 p-3 rounded-lg bg-[rgba(239,68,68,0.1)] border border-[rgba(239,68,68,0.3)] flex items-center gap-3">
      <AlertCircle className="w-4 h-4 text-[var(--status-error)] shrink-0" />
      <p className="text-sm text-[var(--status-error)] flex-1">{error}</p>
      <Button
        variant="ghost"
        size="icon"
        onClick={onDismiss}
        className="h-6 w-6 hover:bg-[rgba(239,68,68,0.2)]"
      >
        <X className="w-4 h-4 text-[var(--status-error)]" />
      </Button>
    </div>
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
        backgroundColor: "var(--bg-surface)",
        backgroundImage:
          "radial-gradient(ellipse at top right, rgba(255,107,53,0.02) 0%, var(--bg-surface) 40%)",
      }}
    >
      {/* Header with glass effect */}
      <div className="flex items-center justify-between px-6 py-4 backdrop-blur-md bg-[rgba(26,26,26,0.85)] border-b border-[var(--border-subtle)]">
        <div className="flex items-center gap-3">
          <div className="p-2 rounded-lg bg-[var(--accent-muted)]">
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
        </div>
      </ScrollArea>
    </div>
  );
}
