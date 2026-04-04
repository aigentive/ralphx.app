import { Shield } from "lucide-react";
import type { SupervisorSettings } from "@/types/settings";
import {
  NumberSettingRow,
  SectionCard,
  ToggleSettingRow,
} from "../SettingsView.shared";

interface SupervisorSectionProps {
  settings: SupervisorSettings;
  onChange: (settings: Partial<SupervisorSettings>) => void;
  disabled: boolean;
}

export default function SupervisorSection({
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
