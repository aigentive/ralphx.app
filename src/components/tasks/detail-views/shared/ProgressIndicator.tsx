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
  accent: "hsl(14 100% 60%)",
  success: "hsl(142 70% 45%)",
  info: "hsl(217 90% 55%)",
};

export function ProgressIndicator({
  percentComplete,
  completedSteps,
  totalSteps,
  variant = "accent",
}: ProgressIndicatorProps) {
  const color = VARIANT_COLORS[variant];
  const showSteps = completedSteps !== undefined && totalSteps !== undefined && totalSteps > 0;

  return (
    <div className="space-y-2">
      {/* Labels row */}
      <div className="flex items-center justify-between">
        {showSteps && (
          <span className="text-[11px]" style={{ color: "hsl(220 10% 50%)" }}>
            Step <span style={{ color: "hsl(220 10% 70%)" }}>{completedSteps}</span> of{" "}
            <span style={{ color: "hsl(220 10% 70%)" }}>{totalSteps}</span>
          </span>
        )}
        <span
          className="text-[11px] font-medium ml-auto tabular-nums"
          style={{ color }}
        >
          {Math.round(percentComplete)}%
        </span>
      </div>

      {/* Progress track */}
      <div
        className="relative h-1.5 rounded-full overflow-hidden"
        style={{ backgroundColor: "hsla(220 10% 100% / 0.08)" }}
      >
        {/* Progress fill - flat, no gradient */}
        <div
          className="absolute inset-y-0 left-0 rounded-full transition-all duration-500 ease-out"
          style={{
            width: `${Math.max(0, Math.min(100, percentComplete))}%`,
            backgroundColor: color,
          }}
        />
      </div>
    </div>
  );
}
