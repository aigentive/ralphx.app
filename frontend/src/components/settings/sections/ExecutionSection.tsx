import { Zap } from "lucide-react";
import type { ExecutionSettings } from "@/types/settings";
import {
  NumberSettingRow,
  SectionCard,
  ToggleSettingRow,
} from "../SettingsView.shared";

interface ExecutionSectionProps {
  settings: ExecutionSettings;
  onChange: (settings: Partial<ExecutionSettings>) => void;
  disabled: boolean;
}

export default function ExecutionSection({
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
      <NumberSettingRow
        id="project-ideation-max"
        label="Project Ideation Cap"
        description="Maximum concurrent ideation and verification sessions for this project (0-10)"
        value={settings.project_ideation_max}
        min={0}
        max={10}
        step={1}
        unit=""
        disabled={disabled}
        onChange={(value) => onChange({ project_ideation_max: value })}
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
