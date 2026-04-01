/**
 * StatusPill - macOS Tahoe-inspired compact status indicator
 *
 * Features:
 * - Frosted vibrancy background
 * - Bold system colors
 * - Compact, pill-shaped design
 */

import type { LucideIcon } from "lucide-react";

type PillVariant = "success" | "warning" | "error" | "info" | "accent" | "neutral";

interface StatusPillProps {
  icon?: LucideIcon;
  label: string;
  variant: PillVariant;
  animated?: boolean;
  size?: "sm" | "md";
}

// macOS Tahoe system colors (dark mode)
const VARIANT_STYLES: Record<PillVariant, { bg: string; text: string }> = {
  success: {
    bg: "rgba(52, 199, 89, 0.18)",
    text: "#30d158",
  },
  warning: {
    bg: "rgba(255, 159, 10, 0.18)",
    text: "#ffd60a",
  },
  error: {
    bg: "rgba(255, 69, 58, 0.18)",
    text: "#ff6961",
  },
  info: {
    bg: "rgba(10, 132, 255, 0.18)",
    text: "#64d2ff",
  },
  accent: {
    bg: "rgba(255, 107, 53, 0.18)",
    text: "#ff8050",
  },
  neutral: {
    bg: "rgba(142, 142, 147, 0.18)",
    text: "#aeaeb2",
  },
};

export function StatusPill({
  icon: Icon,
  label,
  variant,
  animated = false,
  size = "sm",
}: StatusPillProps) {
  const styles = VARIANT_STYLES[variant];
  const sizeClasses = size === "sm"
    ? "px-2.5 py-1 text-[10px] gap-1.5"
    : "px-3 py-1.5 text-[11px] gap-2";
  const iconSize = size === "sm" ? "w-3 h-3" : "w-3.5 h-3.5";

  return (
    <div
      className={`inline-flex items-center ${sizeClasses} rounded-full font-semibold`}
      style={{
        backgroundColor: styles.bg,
        color: styles.text,
        backdropFilter: "blur(12px) saturate(150%)",
        WebkitBackdropFilter: "blur(12px) saturate(150%)",
      }}
    >
      {Icon && (
        <Icon
          className={`${iconSize} ${animated ? "animate-pulse" : ""}`}
          style={{ color: styles.text }}
        />
      )}
      <span className="tracking-tight">{label}</span>
    </div>
  );
}
