/**
 * IdeationSettingsPanel - Configuration for ideation plan workflow
 *
 * Features:
 * - Plan Workflow Mode radio group (Required/Optional/Parallel)
 * - Require explicit approval checkbox (disabled when not in Required mode)
 * - Suggest plans for complex features checkbox
 * - Auto-link proposals to session plan checkbox
 * - Verification gate controls (requireVerificationForProposals, requireVerificationForAccept)
 * - Collapsible External Session Overrides subsection (3-state inherit/on/off selects)
 * - Follows SettingsView pattern with SettingRow and shadcn components
 */

import { useState } from "react";
import { Lightbulb, ChevronDown, ChevronRight } from "lucide-react";
import { Card } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { cn } from "@/lib/utils";
import { useIdeationSettings } from "@/hooks/useIdeationSettings";
import { useUiStore } from "@/stores/uiStore";
import type { IdeationPlanMode, ExternalIdeationOverrides } from "@/types/ideation-config";

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
// Checkbox Setting Row
// ============================================================================

interface CheckboxSettingRowProps {
  id: string;
  label: string;
  description: string;
  checked: boolean;
  disabled: boolean;
  onChange: (checked: boolean) => void;
  isSubSetting?: boolean;
}

function CheckboxSettingRow({
  id,
  label,
  description,
  checked,
  disabled,
  onChange,
  isSubSetting = false,
}: CheckboxSettingRowProps) {
  return (
    <SettingRow
      id={id}
      label={label}
      description={description}
      isSubSetting={isSubSetting}
      isDisabled={disabled}
    >
      <Checkbox
        id={id}
        data-testid={id}
        checked={checked}
        onCheckedChange={onChange}
        disabled={disabled}
        aria-describedby={`${id}-desc`}
        className="data-[state=checked]:bg-[var(--accent-primary)] data-[state=checked]:border-[var(--accent-primary)]"
      />
    </SettingRow>
  );
}

// ============================================================================
// 3-State Override Select
// ============================================================================

type OverrideValue = "inherit" | "on" | "off";

const OVERRIDE_OPTIONS: { value: OverrideValue; label: string; description: string }[] = [
  { value: "inherit", label: "Inherit", description: "Use base policy" },
  { value: "on", label: "On", description: "Always enforce" },
  { value: "off", label: "Off", description: "Always bypass" },
];

function boolToOverride(value: boolean | null): OverrideValue {
  if (value === null) return "inherit";
  return value ? "on" : "off";
}

function overrideToBool(value: OverrideValue): boolean | null {
  if (value === "inherit") return null;
  return value === "on";
}

interface OverrideSelectRowProps {
  id: string;
  label: string;
  description: string;
  value: boolean | null;
  disabled: boolean;
  onChange: (value: boolean | null) => void;
}

function OverrideSelectRow({
  id,
  label,
  description,
  value,
  disabled,
  onChange,
}: OverrideSelectRowProps) {
  return (
    <SettingRow id={id} label={label} description={description} isSubSetting isDisabled={disabled}>
      <Select
        value={boolToOverride(value)}
        onValueChange={(v) => onChange(overrideToBool(v as OverrideValue))}
        disabled={disabled}
      >
        <SelectTrigger
          id={id}
          data-testid={id}
          aria-describedby={`${id}-desc`}
          className="w-[160px] bg-[var(--bg-surface)] border-[var(--border-default)] focus:ring-[var(--accent-primary)]"
        >
          <SelectValue placeholder="Select override" />
        </SelectTrigger>
        <SelectContent className="bg-[var(--bg-elevated)] border-[var(--border-default)]">
          {OVERRIDE_OPTIONS.map((opt) => (
            <SelectItem
              key={opt.value}
              value={opt.value}
              className="focus:bg-[var(--accent-muted)]"
            >
              <div className="flex flex-col">
                <span className="text-[var(--text-primary)]">{opt.label}</span>
                <span className="text-xs text-[var(--text-muted)]">{opt.description}</span>
              </div>
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </SettingRow>
  );
}

// ============================================================================
// Plan Mode Options
// ============================================================================

const PLAN_MODE_OPTIONS: {
  value: IdeationPlanMode;
  label: string;
  description: string;
}[] = [
  {
    value: "required",
    label: "Required",
    description: "Plan must be created before proposals",
  },
  {
    value: "optional",
    label: "Optional (Default)",
    description: "Plan suggested for complex features",
  },
  {
    value: "parallel",
    label: "Parallel",
    description: "Plan and proposals created together",
  },
];

// ============================================================================
// IdeationSettingsPanel Component
// ============================================================================

export function IdeationSettingsPanel() {
  const { settings, updateSettings, isUpdating } = useIdeationSettings();
  const autoAcceptPlans = useUiStore((s) => s.autoAcceptPlans);
  const setAutoAcceptPlans = useUiStore((s) => s.setAutoAcceptPlans);
  const [showExternalOverrides, setShowExternalOverrides] = useState(false);

  const handlePlanModeChange = (mode: string) => {
    updateSettings({
      ...settings,
      planMode: mode as IdeationPlanMode,
    });
  };

  const handleRequirePlanApprovalChange = (checked: boolean) => {
    updateSettings({
      ...settings,
      requirePlanApproval: checked,
    });
  };

  const handleSuggestPlansChange = (checked: boolean) => {
    updateSettings({
      ...settings,
      suggestPlansForComplex: checked,
    });
  };

  const handleAutoLinkProposalsChange = (checked: boolean) => {
    updateSettings({
      ...settings,
      autoLinkProposals: checked,
    });
  };

  const handleRequireAcceptForFinalizeChange = (checked: boolean) => {
    updateSettings({
      ...settings,
      requireAcceptForFinalize: checked,
    });
  };

  const handleRequireVerificationForProposalsChange = (checked: boolean) => {
    updateSettings({
      ...settings,
      requireVerificationForProposals: checked,
    });
  };

  const handleRequireVerificationForAcceptChange = (checked: boolean) => {
    updateSettings({
      ...settings,
      requireVerificationForAccept: checked,
    });
  };

  const handleExternalOverrideChange = (
    field: keyof ExternalIdeationOverrides,
    value: boolean | null
  ) => {
    updateSettings({
      ...settings,
      externalOverrides: {
        ...settings.externalOverrides,
        [field]: value,
      },
    });
  };

  const isRequirePlanApprovalDisabled = settings.planMode !== "required";

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
          <Lightbulb className="w-[18px] h-[18px] text-[var(--accent-primary)]" />
        </div>
        <div>
          <h3 className="text-sm font-semibold tracking-tight text-[var(--text-primary)]">
            Ideation
          </h3>
          <p className="text-xs text-[var(--text-muted)] mt-0.5">
            Configure implementation plan workflow
          </p>
        </div>
      </div>
      <Separator className="my-4 bg-[var(--border-subtle)]" />
      <div className="px-5 pb-5 space-y-1">
        {/* Plan Workflow Mode */}
        <SettingRow
          id="plan-workflow-mode"
          label="Plan Workflow Mode"
          description="Control when implementation plans are created"
          isDisabled={isUpdating}
        >
          <RadioGroup
            value={settings.planMode}
            onValueChange={handlePlanModeChange}
            disabled={isUpdating}
            className="flex flex-col gap-2"
          >
            {PLAN_MODE_OPTIONS.map((option) => (
              <div key={option.value} className="flex items-center gap-2">
                <RadioGroupItem
                  value={option.value}
                  id={`plan-mode-${option.value}`}
                  data-testid={`plan-mode-${option.value}`}
                  className="border-[var(--border-default)] text-[var(--accent-primary)]"
                />
                <Label
                  htmlFor={`plan-mode-${option.value}`}
                  className="text-xs text-[var(--text-primary)] cursor-pointer"
                >
                  {option.label}
                </Label>
              </div>
            ))}
          </RadioGroup>
        </SettingRow>

        {/* Require explicit approval (disabled when not in Required mode) */}
        <CheckboxSettingRow
          id="require-plan-approval"
          label="Require explicit approval"
          description="User must click 'Approve Plan' before creating proposals (in Required mode)"
          checked={settings.requirePlanApproval}
          disabled={isUpdating || isRequirePlanApprovalDisabled}
          onChange={handleRequirePlanApprovalChange}
          isSubSetting
        />

        {/* Suggest plans for complex features */}
        <CheckboxSettingRow
          id="suggest-plans-for-complex"
          label="Suggest plans for complex features"
          description="When in Optional mode, prompt user for complex features"
          checked={settings.suggestPlansForComplex}
          disabled={isUpdating}
          onChange={handleSuggestPlansChange}
        />

        {/* Auto-link proposals to session plan */}
        <CheckboxSettingRow
          id="auto-link-proposals"
          label="Auto-link proposals to session plan"
          description="Automatically set plan reference when creating proposals"
          checked={settings.autoLinkProposals}
          disabled={isUpdating}
          onChange={handleAutoLinkProposalsChange}
        />

        {/* Require agent confirmation before finalizing proposals */}
        <CheckboxSettingRow
          id="require-accept-for-finalize"
          label="Require confirmation before finalizing"
          description="Pause finalize_proposals for user Accept/Reject before tasks are created"
          checked={settings.requireAcceptForFinalize}
          disabled={isUpdating}
          onChange={handleRequireAcceptForFinalizeChange}
        />

        {/* Require verification before accepting proposals */}
        <CheckboxSettingRow
          id="require-verification-for-accept"
          label="Require verification before accepting"
          description="Plan must pass adversarial verification before proposals can be accepted"
          checked={settings.requireVerificationForAccept}
          disabled={isUpdating}
          onChange={handleRequireVerificationForAcceptChange}
        />

        {/* Require verification before creating proposals */}
        <CheckboxSettingRow
          id="require-verification-for-proposals"
          label="Require verification before proposals"
          description="Plan must pass adversarial verification before proposals can be created"
          checked={settings.requireVerificationForProposals}
          disabled={isUpdating}
          onChange={handleRequireVerificationForProposalsChange}
        />

        {/* Auto-accept all finalization dialogs (in-memory only) */}
        <CheckboxSettingRow
          id="auto-accept-plans"
          label="Auto-accept ideation plans"
          description="Automatically accept all pending finalize confirmations without showing the dialog (resets on app restart)"
          checked={autoAcceptPlans}
          disabled={false}
          onChange={setAutoAcceptPlans}
        />

        {/* External Session Overrides — collapsible subsection */}
        <div className="pt-1">
          <button
            type="button"
            data-testid="external-overrides-toggle"
            onClick={() => setShowExternalOverrides((v) => !v)}
            className="flex items-center gap-2 w-full py-2 text-left text-xs font-semibold uppercase tracking-wider text-[var(--text-muted)] hover:text-[var(--text-secondary)] transition-colors"
          >
            {showExternalOverrides ? (
              <ChevronDown className="w-3.5 h-3.5" />
            ) : (
              <ChevronRight className="w-3.5 h-3.5" />
            )}
            External Session Overrides
          </button>
          {showExternalOverrides && (
            <div className="space-y-1 mt-1">
              <OverrideSelectRow
                id="ext-override-verification-for-accept"
                label="Verification for accept"
                description="Override verification-before-accept gate for external sessions"
                value={settings.externalOverrides.requireVerificationForAccept}
                disabled={isUpdating}
                onChange={(v) =>
                  handleExternalOverrideChange("requireVerificationForAccept", v)
                }
              />
              <OverrideSelectRow
                id="ext-override-verification-for-proposals"
                label="Verification for proposals"
                description="Override verification-before-proposals gate for external sessions"
                value={settings.externalOverrides.requireVerificationForProposals}
                disabled={isUpdating}
                onChange={(v) =>
                  handleExternalOverrideChange("requireVerificationForProposals", v)
                }
              />
              <OverrideSelectRow
                id="ext-override-accept-for-finalize"
                label="Accept before finalizing"
                description="Override accept-before-finalize gate for external sessions"
                value={settings.externalOverrides.requireAcceptForFinalize}
                disabled={isUpdating}
                onChange={(v) =>
                  handleExternalOverrideChange("requireAcceptForFinalize", v)
                }
              />
            </div>
          )}
        </div>
      </div>
    </Card>
  );
}
