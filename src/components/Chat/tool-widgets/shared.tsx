/**
 * Shared widget primitives for tool call widgets.
 * WidgetCard, WidgetHeader, GradientFade, Badge.
 */

import React, { useState, useCallback } from "react";
import { ChevronRight } from "lucide-react";

// ============================================================================
// Types
// ============================================================================

export interface WidgetCardProps {
  /** Header content rendered inside the clickable header area */
  header: React.ReactNode;
  /** Body content (collapsed with gradient fade by default) */
  children: React.ReactNode;
  /** Start expanded? Default false */
  defaultExpanded?: boolean;
  /** Compact mode — smaller padding, text */
  compact?: boolean;
  /** Additional className for the outer card */
  className?: string;
}

export type BadgeVariant = "muted" | "success" | "accent" | "error" | "blue";

// ============================================================================
// Badge
// ============================================================================

const BADGE_STYLES: Record<BadgeVariant, { bg: string; color: string }> = {
  muted: { bg: "hsl(220 10% 18%)", color: "hsl(220 10% 45%)" },
  success: { bg: "hsla(145 60% 45% / 0.10)", color: "#34c759" },
  accent: { bg: "hsla(14 100% 60% / 0.10)", color: "hsl(14 100% 60%)" },
  error: { bg: "hsla(0 70% 55% / 0.10)", color: "#ff453a" },
  blue: { bg: "hsla(220 60% 50% / 0.12)", color: "hsl(220 60% 50%)" },
};

export function Badge({
  children,
  variant = "muted",
  className = "",
}: {
  children: React.ReactNode;
  variant?: BadgeVariant;
  className?: string;
}) {
  const s = BADGE_STYLES[variant];
  return (
    <span
      className={`text-[9.5px] px-1.5 py-px rounded-md font-medium flex-shrink-0 whitespace-nowrap ${className}`}
      style={{ background: s.bg, color: s.color }}
    >
      {children}
    </span>
  );
}

// ============================================================================
// GradientFade
// ============================================================================

export function GradientFade({ visible }: { visible: boolean }) {
  return (
    <div
      className="absolute bottom-0 left-0 right-0 h-9 pointer-events-none transition-opacity duration-200"
      style={{
        background: "linear-gradient(to bottom, transparent, hsl(220 10% 12%))",
        opacity: visible ? 1 : 0,
      }}
    />
  );
}

// ============================================================================
// WidgetCard
// ============================================================================

export function WidgetCard({
  header,
  children,
  defaultExpanded = false,
  compact = false,
  className = "",
}: WidgetCardProps) {
  const [isOpen, setIsOpen] = useState(defaultExpanded);

  const toggle = useCallback(() => setIsOpen((v) => !v), []);

  return (
    <div
      className={`rounded-[10px] overflow-hidden ${className}`}
      style={{
        background: "hsl(220 10% 12%)",
        border: "1px solid hsl(220 10% 15%)",
      }}
    >
      {/* Header */}
      <button
        type="button"
        onClick={toggle}
        className={`w-full flex items-center gap-[7px] ${compact ? "px-2 py-[5px] min-h-[28px]" : "px-[10px] py-[7px] min-h-[32px]"} cursor-pointer select-none transition-colors duration-200 hover:bg-[hsl(220_10%_16%)]`}
      >
        <ChevronRight
          size={10}
          className="flex-shrink-0 transition-transform duration-200"
          style={{
            color: "hsl(220 10% 45%)",
            transform: isOpen ? "rotate(90deg)" : "rotate(0deg)",
          }}
        />
        {header}
      </button>

      {/* Collapsible body */}
      <div
        className="relative overflow-hidden transition-[max-height] duration-200 ease-out"
        style={{ maxHeight: isOpen ? 2000 : compact ? 52 : 73 }}
      >
        <div
          className={`${compact ? "px-2 pb-1.5 pt-1.5" : "px-[10px] pb-2 pt-2"}`}
          style={{ borderTop: "1px solid hsl(220 10% 15%)" }}
        >
          {children}
        </div>
        <GradientFade visible={!isOpen} />
      </div>
    </div>
  );
}
