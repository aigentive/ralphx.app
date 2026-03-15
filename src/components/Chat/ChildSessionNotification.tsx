/**
 * ChildSessionNotification - Shows "View Follow-up" link when child session is created
 *
 * Listens for ideation:child_session_created events and displays an inline notification
 * in the chat with a link to navigate to the child session.
 */

import { useState, useEffect } from "react";
import { useEventBus } from "@/providers/EventProvider";
import { ExternalLink } from "lucide-react";
import { Button } from "@/components/ui/button";
import { logger } from "@/lib/logger";

interface ChildSessionInfo {
  sessionId: string;
  parentSessionId: string;
  title: string;
  timestamp: number;
}

interface ChildSessionNotificationProps {
  /** Current session ID to filter notifications */
  sessionId: string;
  /** Callback when user clicks the "View Follow-up" button */
  onNavigateToSession: (sessionId: string) => void;
}

/**
 * Shows a notification when a child session is created for the current session
 */
export function ChildSessionNotification({
  sessionId,
  onNavigateToSession,
}: ChildSessionNotificationProps) {
  const [childSessions, setChildSessions] = useState<ChildSessionInfo[]>([]);
  const bus = useEventBus();

  // Listen for child session created events
  useEffect(() => {
    const unsubscribe = bus.subscribe<{
      sessionId: string;
      parentSessionId: string;
      title: string;
      purpose?: string;
    }>("ideation:child_session_created:local", (payload) => {
      logger.debug("[ChildSessionNotification] Received child session event:", payload);

      // Suppress notifications for verification children — they are background work
      if (payload.purpose === "verification") {
        return;
      }

      // Only show notification if this is a child of the current session
      if (payload.parentSessionId === sessionId) {
        setChildSessions((prev) => [
          ...prev,
          {
            sessionId: payload.sessionId,
            parentSessionId: payload.parentSessionId,
            title: payload.title,
            timestamp: Date.now(),
          },
        ]);
      }
    });

    return unsubscribe;
  }, [bus, sessionId]);

  // Clear notifications when session changes
  useEffect(() => {
    setChildSessions([]);
  }, [sessionId]);

  if (childSessions.length === 0) {
    return null;
  }

  return (
    <div className="space-y-2">
      {childSessions.map((child) => (
        <div
          key={child.sessionId}
          data-testid="child-session-notification"
          className="mx-4 mb-2 p-3 rounded-lg animate-[slide-in-bottom_300ms_ease-out]"
          style={{
            background: "hsla(220 10% 14% / 0.8)",
            border: "1px solid hsla(220 20% 100% / 0.08)",
            backdropFilter: "blur(8px)",
          }}
        >
          <div className="flex items-center justify-between gap-3">
            <div className="flex-1 min-w-0">
              <p
                className="text-sm font-medium mb-1"
                style={{ color: "hsl(220 10% 90%)" }}
              >
                Follow-up session created
              </p>
              <p
                className="text-xs truncate"
                style={{ color: "hsl(220 10% 60%)" }}
                title={child.title}
              >
                {child.title}
              </p>
            </div>
            <Button
              size="sm"
              onClick={() => onNavigateToSession(child.sessionId)}
              className="gap-2 h-7 text-xs shrink-0"
              style={{
                background: "hsla(14 100% 60% / 0.12)",
                border: "1px solid hsla(14 100% 60% / 0.2)",
                color: "hsl(14 100% 60%)",
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.background = "hsla(14 100% 60% / 0.18)";
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.background = "hsla(14 100% 60% / 0.12)";
              }}
            >
              <ExternalLink className="w-3.5 h-3.5" />
              View Follow-up
            </Button>
          </div>
        </div>
      ))}
    </div>
  );
}
