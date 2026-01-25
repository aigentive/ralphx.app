/**
 * QASettingsPanel - Settings panel for QA configuration
 *
 * Premium design with shadcn Switch and Input:
 * - Global QA toggle
 * - Auto-QA checkboxes (UI tasks, API tasks)
 * - QA phases toggles (prep, browser testing)
 * - Browser testing URL input
 */

import { useState, useCallback, useId, useEffect, useRef } from "react";
import { FlaskConical } from "lucide-react";
import { Switch } from "@/components/ui/switch";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Card } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { cn } from "@/lib/utils";
import { useQASettings } from "@/hooks/useQA";

// ============================================================================
// Setting Row Component
// ============================================================================

interface SettingRowProps {
  id: string;
  label: string;
  description?: string;
  checked: boolean;
  disabled?: boolean;
  onChange: () => void;
  indented?: boolean;
}

function SettingRow({
  id,
  label,
  description,
  checked,
  disabled,
  onChange,
  indented = false,
}: SettingRowProps) {
  const descId = description ? `${id}-desc` : undefined;

  return (
    <div className={cn(
      "flex items-start justify-between gap-4 px-4 py-3",
      indented && "ml-6"
    )}>
      <div className="flex-1 min-w-0 max-w-xs">
        <Label
          htmlFor={id}
          className={cn(
            "text-sm font-medium leading-tight",
            disabled ? "text-[var(--text-muted)]" : "text-[var(--text-primary)]"
          )}
        >
          {label}
        </Label>
        {description && (
          <p
            id={descId}
            className="mt-0.5 text-xs text-[var(--text-muted)] leading-normal"
          >
            {description}
          </p>
        )}
      </div>
      <Switch
        id={id}
        data-testid={id}
        checked={checked}
        onCheckedChange={onChange}
        disabled={disabled}
        aria-describedby={descId}
        className={cn(
          "data-[state=checked]:bg-[var(--accent-primary)]",
          "focus-visible:ring-2 focus-visible:ring-[var(--accent-primary)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--bg-base)]"
        )}
      />
    </div>
  );
}

// ============================================================================
// Skeleton Component
// ============================================================================

function QASettingsSkeleton() {
  return (
    <div data-testid="qa-settings-skeleton" className="space-y-4">
      <div className="flex items-center gap-2 mb-4">
        <Skeleton className="h-5 w-5" />
        <Skeleton className="h-6 w-28" />
      </div>
      <Card className="border-[var(--border-subtle)]">
        <div className="divide-y divide-[var(--border-subtle)]">
          <div className="flex justify-between items-start px-4 py-3">
            <div className="space-y-1">
              <Skeleton className="h-4 w-32" />
              <Skeleton className="h-3 w-64" />
            </div>
            <Skeleton className="h-5 w-9 rounded-full" />
          </div>
          <div className="flex justify-between items-start px-4 py-3 ml-6">
            <div className="space-y-1">
              <Skeleton className="h-4 w-36" />
              <Skeleton className="h-3 w-56" />
            </div>
            <Skeleton className="h-5 w-9 rounded-full" />
          </div>
          <div className="flex justify-between items-start px-4 py-3 ml-6">
            <div className="space-y-1">
              <Skeleton className="h-4 w-32" />
              <Skeleton className="h-3 w-60" />
            </div>
            <Skeleton className="h-5 w-9 rounded-full" />
          </div>
        </div>
      </Card>
    </div>
  );
}

// ============================================================================
// Main Component
// ============================================================================

export function QASettingsPanel() {
  const { settings, isLoading, isUpdating, error, updateSettings } = useQASettings();
  const [urlValue, setUrlValue] = useState(settings.browser_testing_url);
  const baseId = useId();
  const lastSettingsUrl = useRef(settings.browser_testing_url);

  // Sync local URL state when settings change from backend
  useEffect(() => {
    if (settings.browser_testing_url !== lastSettingsUrl.current) {
      setUrlValue(settings.browser_testing_url);
      lastSettingsUrl.current = settings.browser_testing_url;
    }
  }, [settings.browser_testing_url]);

  const handleUrlBlur = useCallback(() => {
    if (urlValue !== lastSettingsUrl.current) {
      updateSettings({ browser_testing_url: urlValue });
    }
  }, [urlValue, updateSettings]);

  const handleUrlKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Enter") {
        e.preventDefault();
        if (urlValue !== lastSettingsUrl.current) {
          updateSettings({ browser_testing_url: urlValue });
        }
      }
    },
    [urlValue, updateSettings]
  );

  const isDisabled = isUpdating;
  const isSubSettingsDisabled = isDisabled || !settings.qa_enabled;
  const isUrlDisabled = isSubSettingsDisabled || !settings.browser_testing_enabled;

  if (isLoading) {
    return <QASettingsSkeleton />;
  }

  return (
    <div className="space-y-4">
      {/* Section Header */}
      <div className="flex items-center gap-2">
        <FlaskConical className="w-5 h-5 text-[var(--text-secondary)]" />
        <h3 className="text-lg font-medium text-[var(--text-primary)]">
          QA Settings
        </h3>
      </div>

      {/* Error Banner */}
      {error && (
        <div className="p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-sm text-[var(--status-error)]">
          {error}
        </div>
      )}

      {/* Settings Card Container */}
      <Card className="border-[var(--border-subtle)] bg-[var(--bg-surface)]">
        <div className="divide-y divide-[var(--border-subtle)]">
          {/* Global QA Toggle */}
          <SettingRow
            id="qa-enabled-toggle"
            label="Enable QA System"
            description="Master toggle for the QA system. When disabled, no QA tests will run."
            checked={settings.qa_enabled}
            disabled={isDisabled}
            onChange={() => updateSettings({ qa_enabled: !settings.qa_enabled })}
          />

          {/* Auto-QA for UI Tasks */}
          <SettingRow
            id="auto-qa-ui-toggle"
            label="Auto-QA for UI Tasks"
            description="Automatically enable QA for tasks in UI-related categories."
            checked={settings.auto_qa_for_ui_tasks}
            disabled={isSubSettingsDisabled}
            onChange={() => updateSettings({ auto_qa_for_ui_tasks: !settings.auto_qa_for_ui_tasks })}
            indented
          />

          {/* Auto-QA for API Tasks */}
          <SettingRow
            id="auto-qa-api-toggle"
            label="Auto-QA for API Tasks"
            description="Automatically enable QA for tasks in API-related categories."
            checked={settings.auto_qa_for_api_tasks}
            disabled={isSubSettingsDisabled}
            onChange={() => updateSettings({ auto_qa_for_api_tasks: !settings.auto_qa_for_api_tasks })}
            indented
          />

          {/* QA Prep Phase */}
          <SettingRow
            id="qa-prep-toggle"
            label="QA Prep Phase"
            description="Generate acceptance criteria while tasks execute."
            checked={settings.qa_prep_enabled}
            disabled={isSubSettingsDisabled}
            onChange={() => updateSettings({ qa_prep_enabled: !settings.qa_prep_enabled })}
            indented
          />

          {/* Browser Testing */}
          <SettingRow
            id="browser-testing-toggle"
            label="Browser Testing"
            description="Enable browser-based visual verification."
            checked={settings.browser_testing_enabled}
            disabled={isSubSettingsDisabled}
            onChange={() => updateSettings({ browser_testing_enabled: !settings.browser_testing_enabled })}
            indented
          />

          {/* Browser Testing URL */}
          <div className="px-4 py-3 ml-6">
            <Label
              htmlFor="browser-testing-url-input"
              className={cn(
                "text-sm font-medium",
                isUrlDisabled ? "text-[var(--text-muted)]" : "text-[var(--text-primary)]"
              )}
            >
              Browser Testing URL
            </Label>
            <p
              id={`${baseId}-url-desc`}
              className="mt-0.5 text-xs text-[var(--text-muted)]"
            >
              URL of your dev server for browser testing.
            </p>
            <Input
              type="url"
              id="browser-testing-url-input"
              data-testid="browser-testing-url-input"
              aria-describedby={`${baseId}-url-desc`}
              value={urlValue}
              onChange={(e) => setUrlValue(e.target.value)}
              onBlur={handleUrlBlur}
              onKeyDown={handleUrlKeyDown}
              disabled={isUrlDisabled}
              placeholder="http://localhost:1420"
              className={cn(
                "mt-2 max-w-md",
                "bg-[var(--bg-elevated)] border-[var(--border-subtle)]",
                "focus-visible:ring-[var(--accent-primary)]",
                isUrlDisabled && "opacity-50 cursor-not-allowed"
              )}
            />
          </div>
        </div>
      </Card>
    </div>
  );
}
