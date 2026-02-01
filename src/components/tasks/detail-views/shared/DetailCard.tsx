/**
 * DetailCard - macOS Tahoe-inspired material card
 *
 * Features:
 * - Vibrancy-style translucent background
 * - Prominent yet soft shadows (Tahoe signature)
 * - Large corner radius (16px - Tahoe standard)
 * - Subtle inner highlight for depth
 */

import type { ReactNode } from "react";

type CardVariant = "default" | "success" | "warning" | "error" | "info" | "accent";

interface DetailCardProps {
  children: ReactNode;
  variant?: CardVariant;
  className?: string;
  noPadding?: boolean;
}

const VARIANT_STYLES: Record<CardVariant, { border: string; glow: string }> = {
  default: {
    border: "rgba(255,255,255,0.08)",
    glow: "transparent",
  },
  success: {
    border: "rgba(52, 199, 89, 0.35)",
    glow: "rgba(52, 199, 89, 0.05)",
  },
  warning: {
    border: "rgba(255, 159, 10, 0.35)",
    glow: "rgba(255, 159, 10, 0.05)",
  },
  error: {
    border: "rgba(255, 69, 58, 0.35)",
    glow: "rgba(255, 69, 58, 0.05)",
  },
  info: {
    border: "rgba(10, 132, 255, 0.35)",
    glow: "rgba(10, 132, 255, 0.05)",
  },
  accent: {
    border: "rgba(255, 107, 53, 0.35)",
    glow: "rgba(255, 107, 53, 0.05)",
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
      className={`rounded-2xl transition-all duration-200 ${className}`}
      style={{
        backgroundColor: "rgba(30, 30, 30, 0.6)",
        backdropFilter: "blur(40px) saturate(180%)",
        WebkitBackdropFilter: "blur(40px) saturate(180%)",
        border: `0.5px solid ${styles.border}`,
        boxShadow: `
          0 0 0 0.5px rgba(0,0,0,0.3),
          0 2px 4px rgba(0,0,0,0.2),
          0 8px 24px rgba(0,0,0,0.25),
          inset 0 0.5px 0 rgba(255,255,255,0.06),
          0 0 60px ${styles.glow}
        `,
        padding: noPadding ? undefined : "16px",
      }}
    >
      {children}
    </div>
  );
}
