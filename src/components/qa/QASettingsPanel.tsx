/**
 * QASettingsPanel - Settings panel for QA configuration
 *
 * Displays:
 * - Global QA toggle
 * - Auto-QA checkboxes (UI tasks, API tasks)
 * - QA phases toggles (prep, browser testing)
 * - Browser testing URL input
 */

import { useState, useCallback, useId, useEffect, useRef } from "react";
import { useQASettings } from "@/hooks/useQA";

// ============================================================================
// Toggle Component
// ============================================================================

interface ToggleProps {
  id: string;
  checked: boolean;
  disabled: boolean;
  onChange: () => void;
  "aria-describedby": string | undefined;
}

function Toggle({
  id,
  checked,
  disabled = false,
  onChange,
  "aria-describedby": ariaDescribedBy,
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
    <div className={`flex items-start justify-between gap-4 py-3 ${indented ? "ml-6" : ""}`}>
      <div className="flex-1 min-w-0">
        <label
          htmlFor={id}
          className={`text-sm font-medium ${
            disabled ? "text-[--text-muted]" : "text-[--text-primary]"
          }`}
        >
          {label}
        </label>
        {description && (
          <p id={descId} className="mt-0.5 text-xs text-[--text-muted]">
            {description}
          </p>
        )}
      </div>
      <Toggle
        id={id}
        checked={checked}
        disabled={disabled ?? false}
        onChange={onChange}
        aria-describedby={descId}
      />
    </div>
  );
}

// ============================================================================
// Skeleton Component
// ============================================================================

function QASettingsSkeleton() {
  return (
    <div data-testid="qa-settings-skeleton" className="animate-pulse space-y-4">
      <div className="h-6 w-32 rounded bg-[--bg-hover]" />
      <div className="space-y-3">
        <div className="flex justify-between">
          <div className="h-5 w-48 rounded bg-[--bg-hover]" />
          <div className="h-6 w-11 rounded-full bg-[--bg-hover]" />
        </div>
        <div className="flex justify-between">
          <div className="h-5 w-40 rounded bg-[--bg-hover]" />
          <div className="h-6 w-11 rounded-full bg-[--bg-hover]" />
        </div>
        <div className="flex justify-between">
          <div className="h-5 w-44 rounded bg-[--bg-hover]" />
          <div className="h-6 w-11 rounded-full bg-[--bg-hover]" />
        </div>
      </div>
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
      <h3 className="text-lg font-medium text-[--text-primary]">QA Settings</h3>

      {error && (
        <div className="p-3 rounded bg-[--status-error] bg-opacity-10 text-[--status-error] text-sm">
          {error}
        </div>
      )}

      <div className="divide-y divide-[--border-subtle]">
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
          description="Automatically enable QA for tasks in UI-related categories (ui, component, feature)."
          checked={settings.auto_qa_for_ui_tasks}
          disabled={isSubSettingsDisabled}
          onChange={() => updateSettings({ auto_qa_for_ui_tasks: !settings.auto_qa_for_ui_tasks })}
          indented
        />

        {/* Auto-QA for API Tasks */}
        <SettingRow
          id="auto-qa-api-toggle"
          label="Auto-QA for API Tasks"
          description="Automatically enable QA for tasks in API-related categories (api, backend, endpoint)."
          checked={settings.auto_qa_for_api_tasks}
          disabled={isSubSettingsDisabled}
          onChange={() => updateSettings({ auto_qa_for_api_tasks: !settings.auto_qa_for_api_tasks })}
          indented
        />

        {/* QA Prep Phase */}
        <SettingRow
          id="qa-prep-toggle"
          label="QA Prep Phase"
          description="Enable background QA preparation that generates acceptance criteria while tasks execute."
          checked={settings.qa_prep_enabled}
          disabled={isSubSettingsDisabled}
          onChange={() => updateSettings({ qa_prep_enabled: !settings.qa_prep_enabled })}
          indented
        />

        {/* Browser Testing */}
        <SettingRow
          id="browser-testing-toggle"
          label="Browser Testing"
          description="Enable browser-based visual verification using agent-browser."
          checked={settings.browser_testing_enabled}
          disabled={isSubSettingsDisabled}
          onChange={() => updateSettings({ browser_testing_enabled: !settings.browser_testing_enabled })}
          indented
        />

        {/* Browser Testing URL */}
        <div className="py-3 ml-6">
          <label
            htmlFor="browser-testing-url-input"
            className={`block text-sm font-medium ${
              isUrlDisabled ? "text-[--text-muted]" : "text-[--text-primary]"
            }`}
          >
            Browser Testing URL
          </label>
          <p id={`${baseId}-url-desc`} className="mt-0.5 text-xs text-[--text-muted]">
            URL of your dev server for browser testing (e.g., http://localhost:1420).
          </p>
          <input
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
            className={`mt-2 block w-full rounded-md px-3 py-2 text-sm bg-[--bg-elevated] border border-[--border-subtle] text-[--text-primary] placeholder-[--text-muted] focus:outline-none focus:ring-2 focus:ring-[--accent-primary] focus:border-transparent ${
              isUrlDisabled ? "opacity-50 cursor-not-allowed" : ""
            }`}
          />
        </div>
      </div>
    </div>
  );
}
