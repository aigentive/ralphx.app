/**
 * TimeoutWarning - Banner warning when a bash tool call is approaching the agent timeout
 *
 * Displays when elapsed time since a bash tool call started exceeds 70% of the
 * effective timeout threshold. Context-aware thresholds: 600s (non-team) or
 * 3600s (team mode).
 */

import { useState, useEffect } from "react";
import { AlertTriangle, X } from "lucide-react";
import { Button } from "@/components/ui/button";

// Polling interval for elapsed time check (ms)
const CHECK_INTERVAL_MS = 5_000;

// Warning threshold: show at 70% of effective timeout
const WARNING_THRESHOLD_RATIO = 0.7;

export interface TimeoutWarningProps {
  /** Unix timestamp (ms) when the bash tool call started */
  toolCallStartTime: number;
  /** Effective timeout in ms: 600_000 (non-team) or 3_600_000 (team) */
  effectiveTimeoutMs: number;
  /** Called when user clicks the dismiss button */
  onDismiss: () => void;
}

export function TimeoutWarning({ toolCallStartTime, effectiveTimeoutMs, onDismiss }: TimeoutWarningProps) {
  const [showWarning, setShowWarning] = useState(false);
  const [elapsedSecs, setElapsedSecs] = useState(0);

  useEffect(() => {
    const check = () => {
      const elapsed = Date.now() - toolCallStartTime;
      const threshold = effectiveTimeoutMs * WARNING_THRESHOLD_RATIO;
      setElapsedSecs(Math.round(elapsed / 1000));
      if (elapsed >= threshold) {
        setShowWarning(true);
      }
    };

    // Check immediately
    check();
    const interval = setInterval(check, CHECK_INTERVAL_MS);
    return () => clearInterval(interval);
  }, [toolCallStartTime, effectiveTimeoutMs]);

  if (!showWarning) return null;

  const timeoutSecs = Math.round(effectiveTimeoutMs / 1000);

  return (
    <div
      data-testid="timeout-warning-banner"
      className="shrink-0 mx-3 mb-2 flex items-start gap-2 rounded-md px-3 py-2 text-sm"
      style={{
        backgroundColor: "hsla(38 80% 50% / 0.12)",
        border: "1px solid hsla(38 80% 50% / 0.3)",
        color: "hsl(38 80% 70%)",
      }}
    >
      <AlertTriangle className="w-4 h-4 mt-0.5 shrink-0" />
      <span className="flex-1">
        Bash tool running for {elapsedSecs}s — agent may time out at {timeoutSecs}s. Long-running commands (e.g.{" "}
        <code className="font-mono text-xs">cargo test | tail</code>) buffer output; this is expected.
      </span>
      <Button
        variant="ghost"
        size="icon-sm"
        onClick={onDismiss}
        aria-label="Dismiss timeout warning"
        className="shrink-0 -mt-0.5 -mr-1 h-6 w-6"
        style={{ color: "hsl(38 80% 60%)" }}
      >
        <X className="w-3.5 h-3.5" />
      </Button>
    </div>
  );
}
