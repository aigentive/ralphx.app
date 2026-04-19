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
    bg: "var(--bg-surface)",
  },
  success: {
    bg: "var(--status-success-muted)",
  },
  warning: {
    bg: "var(--status-warning-muted)",
  },
  error: {
    bg: "var(--status-error-muted)",
  },
  info: {
    bg: "var(--status-info-muted)",
  },
  accent: {
    bg: "var(--accent-muted)",
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
