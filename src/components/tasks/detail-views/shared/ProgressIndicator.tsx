/**
 * ProgressIndicator - macOS Tahoe-inspired progress bar
 *
 * Features:
 * - Rounded track with soft shadows
 * - Animated fill with subtle gradient
 * - Optional step counter
 */

interface ProgressIndicatorProps {
  percentComplete: number;
  completedSteps?: number;
  totalSteps?: number;
  variant?: "accent" | "success" | "info";
}

const VARIANT_COLORS = {
  accent: {
    fill: "linear-gradient(90deg, #ff6b35 0%, #ff8050 100%)",
    glow: "rgba(255, 107, 53, 0.4)",
  },
  success: {
    fill: "linear-gradient(90deg, #34c759 0%, #30d158 100%)",
    glow: "rgba(52, 199, 89, 0.4)",
  },
  info: {
    fill: "linear-gradient(90deg, #0a84ff 0%, #64d2ff 100%)",
    glow: "rgba(10, 132, 255, 0.4)",
  },
};

export function ProgressIndicator({
  percentComplete,
  completedSteps,
  totalSteps,
  variant = "accent",
}: ProgressIndicatorProps) {
  const colors = VARIANT_COLORS[variant];
  const showSteps = completedSteps !== undefined && totalSteps !== undefined && totalSteps > 0;

  return (
    <div className="space-y-2.5">
      {/* Labels row */}
      <div className="flex items-center justify-between">
        {showSteps && (
          <span className="text-[12px] text-white/50">
            Step <span className="text-white/80 font-medium">{completedSteps}</span> of{" "}
            <span className="text-white/80 font-medium">{totalSteps}</span>
          </span>
        )}
        <span
          className="text-[13px] font-semibold ml-auto tabular-nums"
          style={{ color: "#ff8050" }}
        >
          {Math.round(percentComplete)}%
        </span>
      </div>

      {/* Progress track */}
      <div
        className="relative h-2 rounded-full overflow-hidden"
        style={{
          backgroundColor: "rgba(255,255,255,0.08)",
          boxShadow: "inset 0 1px 2px rgba(0,0,0,0.2)",
        }}
      >
        {/* Progress fill */}
        <div
          className="absolute inset-y-0 left-0 rounded-full transition-all duration-500 ease-out"
          style={{
            width: `${Math.max(0, Math.min(100, percentComplete))}%`,
            background: colors.fill,
            boxShadow: `0 0 12px ${colors.glow}`,
          }}
        >
          {/* Shine effect */}
          <div
            className="absolute inset-0 rounded-full"
            style={{
              background: "linear-gradient(180deg, rgba(255,255,255,0.25) 0%, transparent 50%)",
            }}
          />
        </div>
      </div>
    </div>
  );
}
