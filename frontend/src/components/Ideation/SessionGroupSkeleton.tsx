/**
 * SessionGroupSkeleton - Loading placeholder for expanded session groups
 *
 * Shows 3-4 PlanItem-shaped skeletons with pulse animation while session data loads.
 * Matches the PlanItem layout: icon + title/metadata content area + menu button.
 */

// Width variants to give natural-looking variety between items
const TITLE_WIDTHS = ["65%", "80%", "72%", "58%"];
const META_WIDTHS = ["40%", "32%", "45%", "36%"];

export function SessionGroupSkeleton({ count = 4 }: { count?: number }) {
  return (
    <div data-testid="session-group-skeleton" className="space-y-0.5">
      {Array.from({ length: count }).map((_, i) => (
        <div key={i} className="rounded-md" style={{ padding: "6px 8px" }}>
          <div className="flex items-center gap-2">
            {/* Icon placeholder — matches PlanItem's w-6 h-6 rounded-md icon */}
            <div
              className="w-6 h-6 rounded-md flex-shrink-0 animate-pulse"
              style={{
                background: "var(--bg-elevated)",
                animationDelay: `${i * 0.08}s`,
              }}
            />

            {/* Content area — title line + metadata line */}
            <div className="flex-1 min-w-0 flex flex-col gap-1.5">
              <div
                className="h-2.5 rounded animate-pulse"
                style={{
                  background: "var(--bg-hover)",
                  width: TITLE_WIDTHS[i % TITLE_WIDTHS.length],
                  animationDelay: `${i * 0.08 + 0.05}s`,
                }}
              />
              <div
                className="h-2 rounded animate-pulse"
                style={{
                  background: "var(--bg-elevated)",
                  width: META_WIDTHS[i % META_WIDTHS.length],
                  animationDelay: `${i * 0.08 + 0.1}s`,
                }}
              />
            </div>

            {/* Menu button placeholder — matches PlanItem's w-6 h-6 rounded button */}
            <div
              className="w-6 h-6 rounded flex-shrink-0 animate-pulse"
              style={{
                background: "var(--bg-elevated)",
                animationDelay: `${i * 0.08}s`,
              }}
            />
          </div>
        </div>
      ))}
    </div>
  );
}
