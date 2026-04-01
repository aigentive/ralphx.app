/**
 * DetailCard - macOS Tahoe-inspired card
 *
 * Tahoe principles:
 * - NO decorative borders
 * - Flat background with subtle color differentiation
 * - Minimal shadows (only for floating elements)
 */

import type { ReactNode } from "react";

type CardVariant = "default" | "success" | "warning" | "error" | "info" | "accent";

interface DetailCardProps {
  children: ReactNode;
  variant?: CardVariant;
  className?: string;
  noPadding?: boolean;
}

// Flat backgrounds only - no borders, no glows
const VARIANT_STYLES: Record<CardVariant, { bg: string }> = {
  default: {
    bg: "hsl(220 10% 12%)",
  },
  success: {
    bg: "hsla(145 60% 45% / 0.08)",
  },
  warning: {
    bg: "hsla(35 100% 50% / 0.08)",
  },
  error: {
    bg: "hsla(0 70% 55% / 0.08)",
  },
  info: {
    bg: "hsla(217 90% 55% / 0.08)",
  },
  accent: {
    bg: "hsla(14 100% 60% / 0.08)",
  },
};

export function DetailCard({
  children,
  variant = "default",
  className = "",
  noPadding = false,
}: DetailCardProps) {
  const styles = VARIANT_STYLES[variant];

  return (
    <div
      className={`rounded-xl ${className}`}
      style={{
        backgroundColor: styles.bg,
        padding: noPadding ? undefined : "14px 16px",
      }}
    >
      {children}
    </div>
  );
}
