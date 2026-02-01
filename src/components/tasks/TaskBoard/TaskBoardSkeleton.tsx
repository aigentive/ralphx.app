/**
 * TaskBoardSkeleton - Loading placeholder for the task board
 *
 * Design: macOS Tahoe (2025) - clean, flat, minimal
 */

const COLUMN_COUNT = 5;

export function TaskBoardSkeleton() {
  return (
    <div
      data-testid="task-board-skeleton"
      className="flex gap-3 overflow-x-auto p-4 flex-1"
      style={{ background: "hsl(220 10% 8%)" }}
    >
      {/* Left spacer */}
      <div className="w-4 flex-shrink-0" aria-hidden="true" />

      {Array.from({ length: COLUMN_COUNT }).map((_, index) => (
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
              style={{ background: "hsl(220 10% 18%)", maxWidth: "70px" }}
            />
            <div
              className="h-2.5 w-4 rounded animate-pulse"
              style={{ background: "hsl(220 10% 15%)" }}
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
                style={{ background: "hsl(220 10% 12%)" }}
              >
                <div
                  className="h-3 w-4/5 rounded mb-2"
                  style={{ background: "hsl(220 10% 18%)" }}
                />
                <div
                  className="h-2.5 w-3/5 rounded"
                  style={{ background: "hsl(220 10% 16%)" }}
                />
              </div>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}
