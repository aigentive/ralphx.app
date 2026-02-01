/**
 * StatusBanner - macOS Tahoe-inspired status header
 *
 * Features:
 * - Bold, high-contrast system colors (Tahoe style)
 * - Vibrancy material background
 * - Prominent icon with soft glow
 * - Clean typographic hierarchy
 */

import type { ReactNode } from "react";
import type { LucideIcon } from "lucide-react";

type BannerVariant = "success" | "warning" | "error" | "info" | "accent" | "neutral";

interface StatusBannerProps {
  icon: LucideIcon;
  title: string;
  subtitle?: string;
  badge?: ReactNode;
  variant: BannerVariant;
  animated?: boolean;
}

// macOS Tahoe system colors (dark mode)
const VARIANT_CONFIG: Record<BannerVariant, {
  bgColor: string;
  borderColor: string;
  iconBg: string;
  iconColor: string;
  titleColor: string;
}> = {
  success: {
    bgColor: "rgba(52, 199, 89, 0.12)",
    borderColor: "rgba(52, 199, 89, 0.25)",
    iconBg: "rgba(52, 199, 89, 0.2)",
    iconColor: "#34c759",
    titleColor: "#30d158",
  },
  warning: {
    bgColor: "rgba(255, 159, 10, 0.12)",
    borderColor: "rgba(255, 159, 10, 0.25)",
    iconBg: "rgba(255, 159, 10, 0.2)",
    iconColor: "#ff9f0a",
    titleColor: "#ffd60a",
  },
  error: {
    bgColor: "rgba(255, 69, 58, 0.12)",
    borderColor: "rgba(255, 69, 58, 0.25)",
    iconBg: "rgba(255, 69, 58, 0.2)",
    iconColor: "#ff453a",
    titleColor: "#ff6961",
  },
  info: {
    bgColor: "rgba(10, 132, 255, 0.12)",
    borderColor: "rgba(10, 132, 255, 0.25)",
    iconBg: "rgba(10, 132, 255, 0.2)",
    iconColor: "#0a84ff",
    titleColor: "#64d2ff",
  },
  accent: {
    bgColor: "rgba(255, 107, 53, 0.12)",
    borderColor: "rgba(255, 107, 53, 0.25)",
    iconBg: "rgba(255, 107, 53, 0.2)",
    iconColor: "#ff6b35",
    titleColor: "#ff8050",
  },
  neutral: {
    bgColor: "rgba(142, 142, 147, 0.12)",
    borderColor: "rgba(142, 142, 147, 0.2)",
    iconBg: "rgba(142, 142, 147, 0.2)",
    iconColor: "#8e8e93",
    titleColor: "#aeaeb2",
  },
};

export function StatusBanner({
  icon: Icon,
  title,
  subtitle,
  badge,
  variant,
  animated = false,
}: StatusBannerProps) {
  const config = VARIANT_CONFIG[variant];

  return (
    <div
      className="relative flex items-center gap-3.5 px-4 py-3.5 rounded-2xl overflow-hidden"
      style={{
        backgroundColor: config.bgColor,
        border: `0.5px solid ${config.borderColor}`,
        backdropFilter: "blur(20px) saturate(150%)",
        WebkitBackdropFilter: "blur(20px) saturate(150%)",
      }}
    >
      {/* Icon container with glow */}
      <div
        className="flex items-center justify-center w-9 h-9 rounded-xl shrink-0"
        style={{
          backgroundColor: config.iconBg,
          boxShadow: `0 0 16px ${config.iconColor}30`,
        }}
      >
        <Icon
          className={`w-5 h-5 ${animated ? "animate-pulse" : ""}`}
          style={{ color: config.iconColor }}
        />
      </div>

      {/* Content */}
      <div className="flex-1 min-w-0">
        <span
          className="text-[14px] font-semibold tracking-tight block"
          style={{ color: config.titleColor }}
        >
          {title}
        </span>
        {subtitle && (
          <span
            className="text-[12px] mt-0.5 block truncate"
            style={{ color: "rgba(255,255,255,0.5)" }}
          >
            {subtitle}
          </span>
        )}
      </div>

      {/* Badge slot */}
      {badge}
    </div>
  );
}
