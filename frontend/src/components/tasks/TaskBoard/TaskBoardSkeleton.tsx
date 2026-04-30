/**
 * TaskBoardSkeleton - Loading placeholder for the task board
 *
 * Design: macOS Tahoe (2025) - clean, flat, minimal
 * Later columns (in_review, done) render as collapsed compact rails
 * to match the typical empty-column auto-collapse appearance.
 */

const COLUMN_COUNT = 5;
/** Columns at index 3+ render as collapsed strips (in_review, done) */
const COLLAPSED_FROM_INDEX = 3;

export function TaskBoardSkeleton() {
  return (
    <div
      data-testid="task-board-skeleton"
      className="flex gap-3 overflow-x-auto p-4 flex-1"
      style={{ background: "var(--bg-base)" }}
    >
      {/* Left spacer */}
      <div className="w-4 flex-shrink-0" aria-hidden="true" />

      {Array.from({ length: COLUMN_COUNT }).map((_, index) => {
        const isCollapsed = index >= COLLAPSED_FROM_INDEX;

        if (isCollapsed) {
          return (
            <div
              key={index}
              data-testid={`skeleton-column-${index}`}
              className="flex-shrink-0 flex flex-col items-center"
              style={{
                width: "128px",
                minWidth: "128px",
                maxWidth: "128px",
                paddingTop: "8px",
                paddingLeft: "10px",
                paddingRight: "10px",
                borderRight: index < COLUMN_COUNT - 1
                  ? "1px solid var(--overlay-weak)"
                  : undefined,
              }}
            >
              {/* Horizontal title placeholder */}
              <div
                className="animate-pulse rounded"
                style={{
                  width: "72px",
                  height: "10px",
                  background: "var(--bg-elevated)",
                }}
              />
              {/* Count placeholder */}
              <div
                className="animate-pulse rounded mt-2"
                style={{
                  width: "12px",
                  height: "8px",
                  background: "var(--bg-surface)",
                }}
              />
            </div>
          );
        }

        return (
          <div
            key={index}
            data-testid={`skeleton-column-${index}`}
            className="flex-shrink-0 flex flex-col"
            style={{ width: "280px" }}
          >
            {/* Column header - simple */}
            <div
              data-testid={`skeleton-header-${index}`}
              className="flex items-center gap-2 px-2 py-1.5 mb-1"
            >
              <div
                className="h-2.5 flex-1 rounded animate-pulse"
                style={{ background: "var(--overlay-weak)", maxWidth: "70px" }}
              />
              <div
                className="h-2.5 w-4 rounded animate-pulse"
                style={{ background: "var(--bg-elevated)" }}
              />
            </div>

            {/* Drop zone */}
            <div className="flex-1 p-1 space-y-1.5">
              {/* Card placeholders */}
              {Array.from({ length: (index % 3) + 1 }).map((_, cardIndex) => (
                <div
                  key={cardIndex}
                  data-testid={`skeleton-card-${index}-${cardIndex}`}
                  className="p-2.5 rounded-lg animate-pulse"
                  style={{ background: "var(--bg-surface)" }}
                >
                  <div
                    className="h-3 w-4/5 rounded mb-2"
                    style={{ background: "var(--bg-elevated)" }}
                  />
                  <div
                    className="h-2.5 w-3/5 rounded"
                    style={{ background: "var(--overlay-weak)" }}
                  />
                </div>
              ))}
            </div>
          </div>
        );
      })}
    </div>
  );
}
