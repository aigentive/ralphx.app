import { Zap } from "lucide-react";
import type { ExecutionSettings } from "@/types/settings";
import {
  NumberSettingRow,
  SectionCard,
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
    </SectionCard>
  );
}
