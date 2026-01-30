/**
 * MessageListSkeleton - Loading placeholder for chat message list
 *
 * Shows animated skeleton bubbles that mimic a conversation layout.
 * Used when conversations are loading or switching contexts.
 */

export function MessageListSkeleton() {
  return (
    <div
      data-testid="chat-panel-loading"
      className="flex flex-col gap-3 p-3 h-full justify-end"
    >
      {/* Skeleton message bubbles - mimics a conversation */}
      {/* User message skeleton */}
      <div className="flex justify-end">
        <div
          className="skeleton-shimmer rounded-[10px_10px_4px_10px] h-10 w-48"
          style={{ background: "rgba(255,107,53,0.08)" }}
        />
      </div>
      {/* Assistant message skeleton - longer */}
      <div className="flex items-start gap-2">
        <div className="skeleton-shimmer w-4 h-4 rounded-full shrink-0 mt-1" />
        <div className="flex flex-col gap-1.5">
          <div
            className="skeleton-shimmer rounded-[10px_10px_10px_4px] h-8 w-64"
            style={{ border: "1px solid rgba(255,255,255,0.04)" }}
          />
          <div
            className="skeleton-shimmer rounded-[10px_10px_10px_4px] h-8 w-40"
            style={{ border: "1px solid rgba(255,255,255,0.04)" }}
          />
        </div>
      </div>
      {/* Another user message */}
      <div className="flex justify-end">
        <div
          className="skeleton-shimmer rounded-[10px_10px_4px_10px] h-8 w-32"
          style={{ background: "rgba(255,107,53,0.08)" }}
        />
      </div>
      {/* Assistant typing indicator skeleton */}
      <div className="flex items-start gap-2">
        <div className="skeleton-shimmer w-4 h-4 rounded-full shrink-0 mt-1" />
        <div
          className="skeleton-shimmer rounded-[10px_10px_10px_4px] h-10 w-56"
          style={{ border: "1px solid rgba(255,255,255,0.04)" }}
        />
      </div>
    </div>
  );
}
