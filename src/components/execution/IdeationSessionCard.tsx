/**
 * IdeationSessionCard - Compact row for a running ideation session
 *
 * Line 1: chat icon | title | "Ideation" badge
 * Line 2: elapsed time · team mode badge
 */

import { MessageSquare, Loader2, Pause } from "lucide-react";
import type { RunningIdeationSession } from "@/api/running-processes";
import { useEffect, useState } from "react";

interface IdeationSessionCardProps {
  session: RunningIdeationSession;
}

function formatElapsedTime(seconds: number | null): string {
  if (seconds === null) return "\u2014";
  const mins = Math.floor(seconds / 60);
  const secs = seconds % 60;
  if (mins === 0) return `${secs}s`;
  return `${mins}m ${secs}s`;
}

export function IdeationSessionCard({ session }: IdeationSessionCardProps) {
  const [elapsedTime, setElapsedTime] = useState(session.elapsedSeconds);

  useEffect(() => {
    if (session.elapsedSeconds === null) return;
    setElapsedTime(session.elapsedSeconds);
    const interval = setInterval(() => {
      setElapsedTime((prev) => (prev !== null ? prev + 1 : null));
    }, 1000);
    return () => clearInterval(interval);
  }, [session.elapsedSeconds, session.sessionId]);

  return (
    <div
      data-testid={`ideation-card-${session.sessionId}`}
      className="px-2 py-1.5 rounded-md hover:bg-white/[0.04] transition-colors"
    >
      {/* Line 1: Icon | Title | Badge */}
      <div className="flex items-center gap-2">
        {session.isGenerating ? (
          <Loader2
            className="w-3.5 h-3.5 animate-spin shrink-0"
            style={{ color: "hsl(14 100% 60%)" }}
          />
        ) : (
          <Pause
            className="w-3.5 h-3.5 shrink-0"
            style={{ color: "hsl(220 10% 45%)" }}
          />
        )}
        <span
          className="flex-1 text-xs font-medium truncate min-w-0"
          style={{ color: "hsl(220 10% 88%)" }}
          title={session.title}
        >
          {session.title}
        </span>
        <span
          className="text-[10px] font-medium px-1.5 py-0.5 rounded shrink-0"
          style={{
            color: "hsl(14 100% 60%)",
            backgroundColor: "hsla(14 100% 60% / 0.15)",
          }}
        >
          Ideation
        </span>
      </div>

      {/* Line 2: Elapsed · Team mode */}
      <div
        className="flex items-center gap-1.5 mt-0.5 pl-[22px] text-[11px] min-w-0"
        style={{ color: "hsl(220 10% 50%)" }}
      >
        <MessageSquare className="w-3 h-3 shrink-0" style={{ color: "hsl(220 10% 40%)" }} />
        <span className="shrink-0 tabular-nums">
          {formatElapsedTime(elapsedTime)}
        </span>
        {session.teamMode && (
          <>
            <span className="shrink-0" style={{ color: "hsl(220 10% 30%)" }}>
              ·
            </span>
            <span
              className="text-[10px] font-medium px-1 rounded shrink-0"
              style={{
                color: "hsl(220 10% 65%)",
                backgroundColor: "hsla(220 10% 65% / 0.15)",
              }}
            >
              {session.teamMode}
            </span>
          </>
        )}
      </div>
    </div>
  );
}
