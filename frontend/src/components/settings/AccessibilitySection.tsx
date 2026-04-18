/**
 * AccessibilitySection — user-facing controls for theme, reduced motion, and
 * font scale. Spec: specs/design/accessibility.md
 *
 * UI layout:
 *   - Theme selector (Dark / Light / High contrast)
 *   - Convenience "High contrast mode" switch that forces the HC theme
 *     (equivalent to picking it from the selector) and restores the previous
 *     base theme when toggled off
 *   - Motion preference (follow system / always reduce)
 *   - Font scale (default / lg / xl)
 */

import { Accessibility } from "lucide-react";
import {
  useThemeStore,
  type FontScale,
  type MotionPreference,
  type ThemeName,
} from "@/stores/themeStore";
import {
  SectionCard,
  SelectSettingRow,
  ToggleSettingRow,
  type SelectOption,
} from "./SettingsView.shared";

const THEME_OPTIONS: SelectOption<ThemeName>[] = [
  {
    value: "dark",
    label: "Dark (default)",
    description: "Warm-orange accent on blue-gray surfaces",
  },
  {
    value: "light",
    label: "Light",
    description: "Near-white surfaces with dark text — same accent family",
  },
  {
    value: "high-contrast",
    label: "High contrast",
    description: "WCAG AAA palette — yellow accent on pure black, thicker borders, shape-based status",
  },
];

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
  const setTheme = useThemeStore((s) => s.setTheme);
  const toggleHighContrast = useThemeStore((s) => s.toggleHighContrast);
  const setMotion = useThemeStore((s) => s.setMotion);
  const setFontScale = useThemeStore((s) => s.setFontScale);

  return (
    <SectionCard
      icon={<Accessibility className="w-[18px] h-[18px] text-[var(--accent-primary)]" />}
      title="Accessibility"
      description="Theme, motion, and typography preferences that apply across the entire app"
    >
      <SelectSettingRow
        id="theme-selector"
        label="Theme"
        description="Pick a base look for the app"
        value={theme}
        options={THEME_OPTIONS}
        disabled={false}
        onChange={setTheme}
      />

      <ToggleSettingRow
        id="theme-high-contrast"
        label="High contrast mode"
        description="Shortcut — forces the WCAG AAA high-contrast theme. Toggling off restores your previous theme choice."
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
    </SectionCard>
  );
}
