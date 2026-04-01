/**
 * StatusBanner - macOS Tahoe-inspired status header
 *
 * Tahoe principles:
 * - NO decorative borders
 * - Flat background with subtle color tint
 * - NO glows (except completion celebration - handled separately)
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

// Flat backgrounds only - no borders, no glows
const VARIANT_CONFIG: Record<BannerVariant, {
  bgColor: string;
  iconBg: string;
  iconColor: string;
  titleColor: string;
}> = {
  success: {
    bgColor: "hsla(145 60% 45% / 0.12)",
    iconBg: "hsla(145 60% 45% / 0.18)",
    iconColor: "#34c759",
    titleColor: "#30d158",
  },
  warning: {
    bgColor: "hsla(35 100% 50% / 0.12)",
    iconBg: "hsla(35 100% 50% / 0.18)",
    iconColor: "#ff9f0a",
    titleColor: "#ffd60a",
  },
  error: {
    bgColor: "hsla(0 70% 55% / 0.12)",
    iconBg: "hsla(0 70% 55% / 0.18)",
    iconColor: "#ff453a",
    titleColor: "#ff6961",
  },
  info: {
    bgColor: "hsla(217 90% 55% / 0.12)",
    iconBg: "hsla(217 90% 55% / 0.18)",
    iconColor: "#0a84ff",
    titleColor: "#64d2ff",
  },
  accent: {
    bgColor: "hsla(14 100% 60% / 0.12)",
    iconBg: "hsla(14 100% 60% / 0.18)",
    iconColor: "#ff6b35",
    titleColor: "#ff8050",
  },
  neutral: {
    bgColor: "hsla(220 10% 50% / 0.12)",
    iconBg: "hsla(220 10% 50% / 0.18)",
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
      className="relative flex items-center gap-3.5 px-4 py-3.5 rounded-xl overflow-hidden"
      style={{
        backgroundColor: config.bgColor,
      }}
    >
      {/* Icon container - flat, no glow */}
      <div
        className="flex items-center justify-center w-9 h-9 rounded-xl shrink-0"
        style={{
          backgroundColor: config.iconBg,
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
            style={{ color: "hsl(220 10% 55%)" }}
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
