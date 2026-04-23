import { Accessibility, ChevronDown, Moon, SunMedium, type LucideIcon } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
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
    icon: Accessibility,
  },
];

export function ThemeSelector({ className = "" }: { className?: string }) {
  const theme = useThemeStore((s) => s.theme);
  const setTheme = useThemeStore((s) => s.setTheme);
  const activeTheme =
    THEME_OPTIONS.find((option) => option.value === theme) ?? THEME_OPTIONS[0]!;

  const ActiveIcon = activeTheme.icon;

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          variant="ghost"
          className={cn(
            "inline-flex items-center gap-2 px-3 h-8 border border-[var(--border-default)] max-w-[180px] overflow-hidden",
            className
          )}
          data-testid="theme-selector-trigger"
          aria-label="Select theme"
        >
          <ActiveIcon className="w-4 h-4 text-[var(--text-secondary)] flex-shrink-0" />
          <span className="text-sm font-medium truncate">{activeTheme.shortLabel}</span>
          <ChevronDown className="w-3.5 h-3.5 text-[var(--text-muted)] flex-shrink-0" />
        </Button>
      </DropdownMenuTrigger>

      <DropdownMenuContent
        className="w-72 bg-[var(--bg-elevated)] border-[var(--border-default)]"
        align="end"
        sideOffset={8}
        data-testid="theme-selector-dropdown"
      >
        <DropdownMenuLabel className="text-xs uppercase tracking-wide text-[var(--text-muted)] px-3 py-2">
          Theme
        </DropdownMenuLabel>
        <DropdownMenuSeparator className="bg-[var(--border-subtle)]" />

        {THEME_OPTIONS.map((option) => {
          const OptionIcon = option.icon;
          const isActive = option.value === theme;

          return (
            <DropdownMenuItem
              key={option.value}
              className={cn(
                "flex items-start gap-3 px-3 py-3 cursor-pointer",
                isActive && "bg-[var(--accent-muted)]"
              )}
              onClick={() => setTheme(option.value)}
              data-testid={`theme-option-${option.value}`}
            >
              <OptionIcon
                className={cn(
                  "w-4 h-4 mt-0.5 flex-shrink-0",
                  isActive ? "text-[var(--accent-primary)]" : "text-[var(--text-secondary)]"
                )}
              />
              <div className="min-w-0">
                <div className="flex items-center gap-2">
                  <span className="text-sm font-medium text-[var(--text-primary)]">
                    {option.label}
                  </span>
                  {isActive ? (
                    <span className="text-[10px] font-semibold uppercase tracking-[0.12em] text-[var(--accent-primary)]">
                      Active
                    </span>
                  ) : null}
                </div>
                <p className="mt-1 text-xs leading-snug text-[var(--text-muted)]">
                  {option.description}
                </p>
              </div>
            </DropdownMenuItem>
          );
        })}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
