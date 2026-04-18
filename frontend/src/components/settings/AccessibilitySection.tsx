/**
 * AccessibilitySection — user-facing controls for theme, reduced motion, and
 * font scale. Spec: specs/design/accessibility.md
 */

import { Accessibility } from "lucide-react";
import { useThemeStore, type FontScale, type MotionPreference } from "@/stores/themeStore";
import {
  SectionCard,
  SettingRow,
  ToggleSettingRow,
  SelectSettingRow,
  type SelectOption,
} from "./SettingsView.shared";

const MOTION_OPTIONS: SelectOption<MotionPreference>[] = [
  {
    value: "system",
    label: "Follow system",
    description: "Use the OS prefers-reduced-motion setting",
  },
  {
    value: "reduce",
    label: "Always reduce",
    description: "Disable animations app-wide even if OS allows them",
  },
];

const FONT_SCALE_OPTIONS: SelectOption<FontScale>[] = [
  { value: "default", label: "Default (100%)", description: "Standard sizing" },
  { value: "lg", label: "Large (110%)", description: "Bumps root font size" },
  { value: "xl", label: "Extra large (125%)", description: "For low-vision comfort" },
];

export function AccessibilitySection() {
  const theme = useThemeStore((s) => s.theme);
  const motion = useThemeStore((s) => s.motion);
  const fontScale = useThemeStore((s) => s.fontScale);
  const toggleHighContrast = useThemeStore((s) => s.toggleHighContrast);
  const setMotion = useThemeStore((s) => s.setMotion);
  const setFontScale = useThemeStore((s) => s.setFontScale);

  return (
    <SectionCard
      icon={<Accessibility className="w-[18px] h-[18px] text-[var(--accent-primary)]" />}
      title="Accessibility"
      description="Contrast, motion, and typography preferences that apply across the entire app"
    >
      <ToggleSettingRow
        id="theme-high-contrast"
        label="High contrast mode"
        description="Maximum-contrast palette with thicker borders, shape-based status icons, and bright focus rings. Meets WCAG 2.1 AAA."
        checked={theme === "high-contrast"}
        disabled={false}
        onChange={toggleHighContrast}
      />

      <SelectSettingRow
        id="motion-preference"
        label="Motion"
        description="Control animations and transitions"
        value={motion}
        options={MOTION_OPTIONS}
        disabled={false}
        onChange={setMotion}
      />

      <SelectSettingRow
        id="font-scale"
        label="Font size"
        description="Root font size scale applied app-wide"
        value={fontScale}
        options={FONT_SCALE_OPTIONS}
        disabled={false}
        onChange={setFontScale}
      />

      <SettingRow
        id="theme-reference"
        label="Theme reference"
        description="See specs/design/themes/high-contrast.md for the full spec including contrast ratios and shape-over-color rules."
      >
        <span className="text-xs text-[var(--text-muted)]" />
      </SettingRow>
    </SectionCard>
  );
}
