import { Monitor, Moon, SunMedium, type LucideIcon } from "lucide-react";

import { cn } from "@/lib/utils";
import { useThemeStore, type ThemeName } from "@/stores/themeStore";

interface ThemeOption {
  value: ThemeName;
  label: string;
  shortLabel: string;
  description: string;
  icon: LucideIcon;
}

const THEME_OPTIONS: ThemeOption[] = [
  {
    value: "dark",
    label: "Dark",
    shortLabel: "Dark",
    description: "Default RalphX palette with subdued surfaces and warm orange accents.",
    icon: Moon,
  },
  {
    value: "light",
    label: "Light",
    shortLabel: "Light",
    description: "Warm off-white surfaces with the same orange accent family.",
    icon: SunMedium,
  },
  {
    value: "high-contrast",
    label: "High contrast",
    shortLabel: "Contrast",
    description: "Maximum-contrast black canvas with shape-first emphasis and stronger borders.",
    icon: Monitor,
  },
];

export function ThemeSelector({ className = "" }: { className?: string }) {
  const theme = useThemeStore((s) => s.theme);
  const setTheme = useThemeStore((s) => s.setTheme);

  return (
    <div
      className={cn(
        "inline-flex h-8 items-center gap-0.5 rounded-lg border p-0.5 shadow-sm shrink-0",
        className
      )}
      style={{
        background: "var(--bg-surface)",
        borderColor: "var(--border-default)",
      }}
      data-testid="theme-selector"
      role="radiogroup"
      aria-label="Theme"
    >
      {THEME_OPTIONS.map((option) => {
        const OptionIcon = option.icon;
        const isActive = option.value === theme;

        return (
          <button
            key={option.value}
            type="button"
            role="radio"
            aria-checked={isActive}
            aria-label={`${option.label} theme`}
            title={option.description}
            onClick={() => setTheme(option.value)}
            className={cn(
              "inline-flex h-7 items-center gap-1 rounded-[8px] px-2 text-[11px] font-medium transition-all duration-150 outline-none focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-[var(--accent-primary)]",
              !isActive && "hover:bg-[var(--overlay-faint)] hover:text-[var(--text-primary)]",
              isActive ? "shadow-sm" : ""
            )}
            style={{
              background: isActive ? "var(--accent-muted)" : "transparent",
              border: `1px solid ${isActive ? "var(--accent-border)" : "transparent"}`,
              color: isActive ? "var(--accent-primary)" : "var(--text-secondary)",
              boxShadow: isActive ? "var(--shadow-xs)" : "none",
            }}
            data-testid={`theme-option-${option.value}`}
          >
            <OptionIcon className="h-[13px] w-[13px] shrink-0" />
            <span className="leading-none">{option.shortLabel}</span>
          </button>
        );
      })}
    </div>
  );
}
