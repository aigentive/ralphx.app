/**
 * ProgressIndicator - macOS Tahoe-inspired progress bar
 *
 * Minimal design:
 * - No container/card wrapper
 * - Thin progress track (4px)
 * - Flat colors, no gradients or borders
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
    <div className="space-y-2.5">
      {/* Labels row */}
      <div className="flex items-center justify-between">
        {showSteps && (
          <span
            className="text-[12px]"
            style={{ color: "hsl(220 10% 55%)" }}
          >
            Step{" "}
            <span
              className="font-medium tabular-nums"
              style={{ color: "hsl(220 10% 80%)" }}
            >
              {completedSteps}
            </span>
            {" "}of{" "}
            <span
              className="font-medium tabular-nums"
              style={{ color: "hsl(220 10% 80%)" }}
            >
              {totalSteps}
            </span>
          </span>
        )}
        <span
          className="text-[12px] font-medium ml-auto tabular-nums"
          style={{ color }}
        >
          {Math.round(percentComplete)}%
        </span>
      </div>

      {/* Progress track - thin, minimal */}
      <div
        className="relative h-1 rounded-full overflow-hidden"
        style={{ backgroundColor: "hsl(220 10% 16%)" }}
      >
        {/* Progress fill */}
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
