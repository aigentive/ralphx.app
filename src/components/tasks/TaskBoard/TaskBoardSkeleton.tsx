/**
 * TaskBoardSkeleton - Loading placeholder for the task board
 */

const COLUMN_NAMES = ["Draft", "Ready", "In Progress", "In Review", "Done"];

export function TaskBoardSkeleton() {
  return (
    <div
      data-testid="task-board-skeleton"
      className="flex gap-4 overflow-x-auto p-4"
      style={{ backgroundColor: "var(--bg-base)" }}
    >
      {COLUMN_NAMES.map((_name, index) => (
        <div
          key={index}
          data-testid={`skeleton-column-${index}`}
          className="flex-shrink-0 w-72 rounded-lg"
          style={{ backgroundColor: "var(--bg-surface)" }}
        >
          {/* Column header */}
          <div
            data-testid={`skeleton-header-${index}`}
            className="p-3 border-b animate-pulse"
            style={{ borderColor: "var(--border-subtle)" }}
          >
            <div
              className="h-5 w-24 rounded"
              style={{ backgroundColor: "var(--bg-elevated)" }}
            />
          </div>

          {/* Card placeholders */}
          <div className="p-2 space-y-2">
            {[0, 1, 2].slice(0, (index % 3) + 1).map((cardIndex) => (
              <div
                key={cardIndex}
                data-testid={`skeleton-card-${index}-${cardIndex}`}
                className="p-3 rounded-md animate-pulse"
                style={{ backgroundColor: "var(--bg-elevated)" }}
              >
                <div
                  className="h-4 w-3/4 rounded mb-2"
                  style={{ backgroundColor: "var(--bg-hover)" }}
                />
                <div
                  className="h-3 w-1/2 rounded"
                  style={{ backgroundColor: "var(--bg-hover)" }}
                />
              </div>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}
