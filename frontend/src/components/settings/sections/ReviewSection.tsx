import { FileSearch } from "lucide-react";
import type { ProjectReviewSettings } from "@/types/settings";
import {
  NumberSettingRow,
  SectionCard,
  ToggleSettingRow,
} from "../SettingsView.shared";

interface ReviewSectionProps {
  settings: ProjectReviewSettings;
  onChange: (settings: Partial<ProjectReviewSettings>) => void;
  disabled: boolean;
}

export default function ReviewSection({ settings, onChange, disabled }: ReviewSectionProps) {
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
