import { Brain } from "lucide-react";
import type { ModelSettings } from "@/types/settings";
import {
  MODEL_OPTIONS,
  SectionCard,
  SelectSettingRow,
  ToggleSettingRow,
} from "../SettingsView.shared";

interface ModelSectionProps {
  settings: ModelSettings;
  onChange: (settings: Partial<ModelSettings>) => void;
  disabled: boolean;
}

export default function ModelSection({ settings, onChange, disabled }: ModelSectionProps) {
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
