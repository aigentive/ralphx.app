/**
 * IdeationSettingsPanel - Configuration for ideation plan workflow
 *
 * Features:
 * - Plan Workflow Mode radio group (Required/Optional/Parallel)
 * - Require explicit approval checkbox (disabled when not in Required mode)
 * - Suggest plans for complex features checkbox
 * - Auto-link proposals to session plan checkbox
 * - Follows SettingsView pattern with SettingRow and shadcn components
 */

import { Lightbulb } from "lucide-react";
import { Card } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import { cn } from "@/lib/utils";
import { useIdeationSettings } from "@/hooks/useIdeationSettings";
import type { IdeationPlanMode } from "@/types/ideation-config";

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
      </div>
    </Card>
  );
}
