/**
 * VerificationPanel — Verification tab content for the ideation middle panel.
 *
 * Assembles VerificationBadge, VerificationGapList, and VerificationHistory
 * with empty state (Verify First / Skip Verification CTAs) and action buttons
 * relocated from PlanDisplay (Revert & Skip, Re-verify Plan, Address Gaps).
 *
 * Design: macOS Tahoe style, warm orange accent (#ff6b35), SF Pro, no purple/blue.
 */

import { useState, useCallback, useRef, useEffect } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
  ShieldCheck,
  SkipForward,
  RotateCcw,
  Wand2,
  ShieldAlert,
  ChevronDown,
  History,
  AlertCircle,
} from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { VerificationBadge } from "./VerificationBadge";
import { VerificationGapList } from "./VerificationGapList";
import { VerificationHistory } from "./VerificationHistory";
import { ideationApi, type SessionWithDataResponse } from "@/api/ideation";
import { ideationKeys } from "@/hooks/useIdeation";
import { chatApi } from "@/api/chat";
import { useChildSessionStatus } from "@/hooks/useChildSessionStatus";
import { useIdeationStore } from "@/stores/ideationStore";
import { useUiStore } from "@/stores/uiStore";
import { useChatStore } from "@/stores/chatStore";
import { buildStoreKey } from "@/lib/chat-context-registry";
import { getModelLabel } from "@/lib/model-utils";
import type { IdeationSession, VerificationStatus } from "@/types/ideation";

// ============================================================================
// Types
// ============================================================================

interface VerificationPanelProps {
  session: IdeationSession;
}

interface VerificationRunEntry {
  id: string;
  runNumber: number;
  createdAt: string;
  /** Status derived from parent session for current run, or session title for others */
  status?: VerificationStatus;
}

// ============================================================================
// Helpers
// ============================================================================

function statusLabel(status: VerificationStatus | undefined): string {
  switch (status) {
    case "verified":
    case "imported_verified":
      return "Verified";
    case "needs_revision":
      return "Needs revision";
    case "reviewing":
      return "In progress";
    case "skipped":
      return "Skipped";
    default:
      return "Completed";
  }
}

function formatRelativeTime(dateStr: string): string {
  const diff = Date.now() - new Date(dateStr).getTime();
  const minutes = Math.floor(diff / 60_000);
  if (minutes < 1) return "just now";
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  if (days < 7) return `${days}d ago`;
  return new Date(dateStr).toLocaleDateString(undefined, { month: "short", day: "numeric" });
}

// ============================================================================
// VerificationRunPicker sub-component
// ============================================================================

interface VerificationRunPickerProps {
  runs: VerificationRunEntry[];
  activeRunId: string | null;
  currentStatus: VerificationStatus;
  currentRound?: number;
  maxRounds?: number;
  onSelect: (runId: string) => void;
}

function VerificationRunPicker({
  runs,
  activeRunId,
  currentStatus,
  currentRound,
  maxRounds,
  onSelect,
}: VerificationRunPickerProps) {
  const [isOpen, setIsOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  // Close on outside click
  useEffect(() => {
    if (!isOpen) return;
    const handler = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setIsOpen(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [isOpen]);

  // Close on Escape
  useEffect(() => {
    if (!isOpen) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") setIsOpen(false);
    };
    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  }, [isOpen]);

  const activeRun = runs.find((r) => r.id === activeRunId) ?? runs[0];
  const mostRecentRunId = runs[0]?.id;
  const isCurrentRun = activeRunId === mostRecentRunId || !activeRunId;

  // Build trigger label
  let triggerLabel: string;
  if (isCurrentRun && currentRound !== undefined && maxRounds !== undefined) {
    triggerLabel = `Current run (Round ${currentRound}/${maxRounds})`;
  } else if (isCurrentRun) {
    triggerLabel = "Current run";
  } else if (activeRun) {
    triggerLabel = `Run ${activeRun.runNumber}`;
  } else {
    triggerLabel = "Select run";
  }

  if (runs.length <= 1) {
    // Single run — just show a non-interactive label
    return (
      <div
        className="flex items-center gap-1.5 px-2 py-1 rounded-md"
        style={{ background: "hsla(220 10% 100% / 0.03)" }}
      >
        <History className="w-3 h-3 shrink-0" style={{ color: "hsl(220 10% 45%)" }} />
        <span className="text-[11px]" style={{ color: "hsl(220 10% 55%)" }}>
          {triggerLabel}
        </span>
      </div>
    );
  }

  return (
    <div ref={containerRef} className="relative">
      {/* Trigger button */}
      <button
        onClick={() => setIsOpen((v) => !v)}
        className="flex items-center gap-1.5 px-2 py-1 rounded-md transition-colors duration-150"
        style={{
          background: isOpen ? "hsla(220 10% 100% / 0.07)" : "hsla(220 10% 100% / 0.03)",
          border: "1px solid hsla(220 10% 100% / 0.08)",
        }}
        onMouseEnter={(e) => {
          if (!isOpen) e.currentTarget.style.background = "hsla(220 10% 100% / 0.06)";
        }}
        onMouseLeave={(e) => {
          if (!isOpen) e.currentTarget.style.background = "hsla(220 10% 100% / 0.03)";
        }}
        aria-haspopup="listbox"
        aria-expanded={isOpen}
        data-testid="verification-run-picker-trigger"
      >
        <History className="w-3 h-3 shrink-0" style={{ color: "hsl(220 10% 45%)" }} />
        <span className="text-[11px] font-medium" style={{ color: "hsl(220 10% 75%)" }}>
          {triggerLabel}
        </span>
        <ChevronDown
          className="w-3 h-3 shrink-0 transition-transform duration-150"
          style={{
            color: "hsl(220 10% 45%)",
            transform: isOpen ? "rotate(180deg)" : "rotate(0deg)",
          }}
        />
      </button>

      {/* Dropdown menu */}
      {isOpen && (
        <div
          role="listbox"
          data-testid="verification-run-picker-menu"
          className="absolute top-full left-0 mt-1 z-50 min-w-[220px] rounded-lg py-1 shadow-xl"
          style={{
            background: "hsl(220 10% 12%)",
            border: "1px solid hsla(220 10% 100% / 0.1)",
          }}
        >
          {runs.map((run) => {
            const isActive = run.id === activeRunId || (!activeRunId && run.id === mostRecentRunId);
            const isNewest = run.id === mostRecentRunId;
            const label = isNewest
              ? statusLabel(currentStatus)
              : statusLabel(run.status);

            return (
              <button
                key={run.id}
                role="option"
                aria-selected={isActive}
                onClick={() => {
                  onSelect(run.id);
                  setIsOpen(false);
                }}
                className="w-full text-left px-3 py-2 flex items-center justify-between gap-3 transition-colors duration-100"
                style={{
                  background: isActive ? "hsla(14 100% 60% / 0.08)" : "transparent",
                }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.background = isActive
                    ? "hsla(14 100% 60% / 0.12)"
                    : "hsla(220 10% 100% / 0.05)";
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.background = isActive
                    ? "hsla(14 100% 60% / 0.08)"
                    : "transparent";
                }}
                data-testid={`verification-run-option-${run.runNumber}`}
              >
                <div className="flex flex-col gap-0.5 min-w-0">
                  <span
                    className="text-[12px] font-medium truncate"
                    style={{ color: isActive ? "hsl(14 100% 65%)" : "hsl(220 10% 85%)" }}
                  >
                    Run {run.runNumber}
                    <span className="ml-1.5 text-[10px] font-normal" style={{ color: "hsl(220 10% 45%)" }}>— {label}</span>
                  </span>
                </div>
                <span
                  className="text-[10px] shrink-0"
                  style={{ color: "hsl(220 10% 40%)" }}
                >
                  {formatRelativeTime(run.createdAt)}
                </span>
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}

// ============================================================================
// Component
// ============================================================================

export function VerificationPanel({ session }: VerificationPanelProps) {
  const queryClient = useQueryClient();
  const [selectedGaps, setSelectedGaps] = useState<Set<number>>(new Set());
  // Stable time reference for stale detection — refreshes every 30s while in-progress
  const [nowMs, setNowMs] = useState(Date.now);

  const activeVerificationChildId = useIdeationStore(
    (s) => s.activeVerificationChildId[session.id] ?? null
  );
  const setActiveVerificationChildId = useIdeationStore(
    (s) => s.setActiveVerificationChildId
  );
  const lastVerificationChildId = useIdeationStore(
    (s) => s.lastVerificationChildId[session.id] ?? null
  );
  const setLastVerificationChildId = useIdeationStore(
    (s) => s.setLastVerificationChildId
  );
  const enqueuePendingVerification = useUiStore((s) => s.enqueuePendingVerification);

  // Poll child session status to get lastEffectiveModel for backfill hydration.
  const { lastEffectiveModel: childLastModel } = useChildSessionStatus(lastVerificationChildId);

  // Backfill effectiveModel store for the verification child session on page-load/reopen.
  // Uses the child's own store key (not the parent's) so the chat header shows the correct model.
  // Guard: skip if the store already has a value (live agent:run_started event wins).
  useEffect(() => {
    if (!lastVerificationChildId || !childLastModel) return;
    const storeKey = buildStoreKey("ideation", lastVerificationChildId);
    if (useChatStore.getState().effectiveModel[storeKey]) return;
    useChatStore.getState().setEffectiveModel(storeKey, {
      id: childLastModel,
      label: getModelLabel(childLastModel),
    });
  }, [lastVerificationChildId, childLastModel]);

  const verificationStatus = session.verificationStatus ?? "unverified";
  const hasPlan = !!session.planArtifactId;
  const isApproved = session.status === "accepted";
  const isInProgress = (session.verificationInProgress ?? false) || !!activeVerificationChildId;

  // Fetch full verification data — always fires when a plan exists (not gated on verificationStatus)
  // so that page-load hydration works even when the session cache still shows "unverified".
  const { data: verificationData } = useQuery({
    queryKey: ["verification", session.id],
    queryFn: async () => {
      try {
        return await ideationApi.verification.getStatus(session.id);
      } catch (err) {
        // 404 = no verification started yet — return null so the empty state renders correctly.
        if (err instanceof Error && err.message.includes("404")) return null;
        throw err;
      }
    },
    enabled: hasPlan && session.sessionPurpose !== "verification",
    staleTime: 30_000,
    retry: (failureCount: number, err: unknown) => {
      // Don't retry 404s — they mean no verification data exists
      if (err instanceof Error && err.message.includes("404")) return false;
      return failureCount < 2;
    },
    retryDelay: (attempt) => Math.min(1000 * 2 ** attempt, 10000),
  });

  // Fetch all verification child sessions for the history picker
  const { data: childSessions = [] } = useQuery({
    queryKey: ["childSessions", session.id, "verification"],
    queryFn: () => ideationApi.sessions.getChildren(session.id, "verification"),
    enabled: hasPlan && session.sessionPurpose !== "verification",
    staleTime: 4_000,
    refetchInterval: 10_000,
  });

  // Hydrate session query cache from verification API response on page load.
  // The session schema defaults verificationStatus to "unverified", so if the server
  // omits it the query gate would have blocked loading — this effect bootstraps the UI.
  useEffect(() => {
    if (!verificationData) return;
    if (verificationData.status === "unverified") return;
    // Only update if the session still shows the default (unverified); avoids overwriting
    // live event-driven updates that may have already set the correct status.
    if (verificationStatus !== "unverified") return;
    queryClient.setQueryData<SessionWithDataResponse | null>(
      ideationKeys.sessionWithData(session.id),
      (old) =>
        old
          ? {
              ...old,
              session: {
                ...old.session,
                verificationStatus: verificationData.status as VerificationStatus,
                verificationInProgress: verificationData.inProgress,
              },
            }
          : old
    );
  }, [verificationData, verificationStatus, session.id, queryClient]);

  // Stable boolean extracted from verificationData to prevent object-identity re-fires in the effect below.
  const apiInProgress = verificationData?.inProgress ?? false;

  // Auto-update verification child IDs when a new verification run appears.
  // lastVerificationChildId tracks the display reference (persists after agent terminates).
  // activeVerificationChildId is only set on first mount (when both are null) to avoid
  // fighting the termination null-clear from guardedTermination/watchdog guards.
  useEffect(() => {
    if (childSessions.length === 0) return;
    const sorted = [...childSessions].sort(
      (a, b) => new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime()
    );
    const latestId = sorted[0]?.id;
    if (!latestId) return;
    if (latestId !== lastVerificationChildId) {
      setLastVerificationChildId(session.id, latestId);
    }
    // Set activeVerificationChildId on first mount (both null) — prevents re-asserting after termination.
    // Also hydrate when API confirms verification is active AND a NEW child exists (not the just-terminated one).
    // Race-safety: latestId !== lastVerificationChildId guard prevents re-asserting terminated child
    // when stale verificationData?.inProgress is still true before cache invalidates.
    if (activeVerificationChildId === null &&
        (lastVerificationChildId === null ||
         (latestId !== lastVerificationChildId && apiInProgress))) {
      setActiveVerificationChildId(session.id, latestId);
    }
  }, [childSessions, activeVerificationChildId, lastVerificationChildId, apiInProgress, session.id, setActiveVerificationChildId, setLastVerificationChildId]);

  // Refresh nowMs every 30s while in-progress (stale detection clock)
  useEffect(() => {
    if (!isInProgress) return;
    const id = setInterval(() => { setNowMs(Date.now()); }, 30_000);
    return () => { clearInterval(id); };
  }, [isInProgress]);

  // Build sorted run entries (newest first for display, oldest = Run 1)
  const runEntries: VerificationRunEntry[] = [...childSessions]
    .sort((a, b) => new Date(a.createdAt).getTime() - new Date(b.createdAt).getTime())
    .map((child, index) => ({
      id: child.id,
      runNumber: index + 1,
      createdAt: child.createdAt,
    }))
    .reverse(); // newest first in dropdown

  const handleRunSelect = useCallback((runId: string) => {
    // Only update display reference — do NOT set activeVerificationChildId for historical runs
    // (prevents mount-then-unmount flicker from reverse-link cleanup)
    setLastVerificationChildId(session.id, runId);
  }, [session.id, setLastVerificationChildId]);

  // ── Action handlers ──────────────────────────────────────────────────────

  const handleTriggerVerification = useCallback(() => {
    enqueuePendingVerification(session.id);
  }, [session.id, enqueuePendingVerification]);

  const handleSkipVerification = useCallback(async () => {
    queryClient.setQueryData<SessionWithDataResponse | null>(
      ideationKeys.sessionWithData(session.id),
      (old) => old ? { ...old, session: { ...old.session, verificationStatus: "skipped" } } : old
    );
    try {
      await ideationApi.verification.skip(session.id);
      queryClient.invalidateQueries({ queryKey: ideationKeys.sessions() });
      queryClient.invalidateQueries({ queryKey: ideationKeys.sessionWithData(session.id) });
      queryClient.invalidateQueries({ queryKey: ["verification", session.id] });
    } catch (err) {
      queryClient.invalidateQueries({ queryKey: ideationKeys.sessionWithData(session.id) });
      console.error("Failed to skip verification:", err);
      toast.error("Failed to skip verification");
    }
  }, [session.id, queryClient]);

  const handleAddressGaps = useCallback(async () => {
    const gaps = verificationData?.gaps ?? [];
    const selected = [...selectedGaps];
    const descriptions =
      selected.length === 0
        ? gaps.map((g) => g.description)
        : selected.map((i) => gaps[i]!.description);

    const allGapCount = gaps.length;
    const isAll = descriptions.length === allGapCount || allGapCount === 0;
    const message = isAll
      ? "update the plan to address all verification gaps"
      : `update the plan to address these specific verification gaps:\n${descriptions.map((d, i) => `${i + 1}. ${d}`).join("\n")}`;

    try {
      await chatApi.sendAgentMessage("ideation", session.id, message);
    } catch (err) {
      console.error("Failed to address gaps:", err);
      toast.error("Failed to request gap resolution");
    }
  }, [session.id, verificationData?.gaps, selectedGaps]);

  // ── Derived state ────────────────────────────────────────────────────────

  const gaps = verificationData?.gaps ?? [];
  const rounds = verificationData?.rounds ?? [];
  const roundDetails = verificationData?.roundDetails ?? [];
  const gapScore = verificationData?.gapScore ?? (session.gapScore != null ? session.gapScore : undefined);
  const hasGaps = gaps.length > 0;
  const hasRounds = rounds.length > 0 || roundDetails.length > 0;
  const hasVerificationRunEvidence =
    childSessions.length > 0 ||
    activeVerificationChildId != null ||
    lastVerificationChildId != null;

  const isVerified = verificationStatus === "verified" || verificationStatus === "imported_verified";
  const isSkipped = verificationStatus === "skipped";
  const showSkipVerification = !isVerified && !isSkipped && !isApproved;
  const showAddressGaps =
    verificationStatus === "needs_revision" && !isInProgress && hasGaps;
  const showReVerify =
    !isInProgress &&
    (verificationStatus === "needs_revision" || (isVerified && hasGaps));

  // ── Stale detection (7B) ─────────────────────────────────────────────────
  // Heuristic: verification is stalled if in_progress for > maxRounds * 5 minutes
  // since the newest verification child session was created.
  // Component re-renders at least every 10s (refetchInterval) so no extra timer needed.
  const maxRoundsForStale = verificationData?.maxRounds ?? 5;
  const staleThresholdMs = maxRoundsForStale * 5 * 60 * 1000;
  const newestChildCreatedAt = runEntries[0]?.createdAt;
  const isStaleVerification =
    isInProgress &&
    newestChildCreatedAt != null &&
    nowMs - new Date(newestChildCreatedAt).getTime() > staleThresholdMs;

  // ── Empty state ──────────────────────────────────────────────────────────

  if (verificationStatus === "unverified" && !hasRounds && !hasVerificationRunEvidence) {
    return (
      <div
        data-testid="verification-empty-state"
        className="flex flex-1 items-center justify-center p-8 min-h-0"
      >
        <div className="flex flex-col items-center gap-5 max-w-xs text-center">
          {/* Shield icon */}
          <div
            className="w-14 h-14 rounded-2xl flex items-center justify-center"
            style={{
              background: "hsla(220 10% 100% / 0.04)",
              border: "1px solid hsla(220 10% 100% / 0.08)",
            }}
          >
            <ShieldAlert
              className="w-7 h-7"
              style={{ color: "hsl(220 10% 40%)" }}
            />
          </div>

          {/* Text */}
          <div className="space-y-1.5">
            <h3
              className="text-[14px] font-semibold"
              style={{ color: "hsl(220 10% 85%)" }}
            >
              No verification yet
            </h3>
            <p
              className="text-[12px] leading-relaxed"
              style={{ color: "hsl(220 10% 50%)" }}
            >
              Run the AI verification agent to check your plan for gaps and
              implementation risks before creating proposals.
            </p>
          </div>

          {/* CTAs */}
          <div className="flex flex-col gap-2 w-full">
            {hasPlan && (
              <Button
                onClick={handleTriggerVerification}
                data-testid="verify-first-button"
                className="h-8 gap-2 text-[12px] font-semibold w-full rounded-lg transition-colors duration-150"
                style={{
                  color: "hsl(14 100% 60%)",
                  background: "hsla(14 100% 60% / 0.1)",
                  border: "1px solid hsla(14 100% 60% / 0.2)",
                }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.background = "hsla(14 100% 60% / 0.15)";
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.background = "hsla(14 100% 60% / 0.1)";
                }}
              >
                <ShieldCheck className="w-3.5 h-3.5" />
                Verify First
              </Button>
            )}
            {showSkipVerification && (
              <Button
                variant="ghost"
                onClick={handleSkipVerification}
                data-testid="skip-verification-button"
                className="h-8 gap-2 text-[12px] font-medium w-full rounded-lg transition-colors duration-150"
                style={{
                  color: "hsl(220 10% 50%)",
                  background: "transparent",
                  border: "1px solid hsla(220 10% 100% / 0.06)",
                }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.background = "hsla(220 10% 100% / 0.05)";
                  e.currentTarget.style.color = "hsl(220 10% 70%)";
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.background = "transparent";
                  e.currentTarget.style.color = "hsl(220 10% 50%)";
                }}
              >
                <SkipForward className="w-3.5 h-3.5" />
                Skip Verification
              </Button>
            )}
          </div>
        </div>
      </div>
    );
  }

  // ── Main content ─────────────────────────────────────────────────────────

  return (
    <div
      data-testid="verification-panel-content"
      className="flex-1 overflow-y-auto p-4 space-y-4"
    >
      {/* History picker — shown when there are any child sessions */}
      {runEntries.length > 0 && (
        <div className="flex items-center justify-between gap-3">
          <VerificationRunPicker
            runs={runEntries}
            activeRunId={lastVerificationChildId ?? activeVerificationChildId}
            currentStatus={verificationStatus}
            {...(verificationData?.currentRound !== undefined && {
              currentRound: verificationData.currentRound,
            })}
            {...(verificationData?.maxRounds !== undefined && {
              maxRounds: verificationData.maxRounds,
            })}
            onSelect={handleRunSelect}
          />
        </div>
      )}

      {/* Stale verification warning (7B) — shown when in_progress for longer than maxRounds * 5 min */}
      {isStaleVerification && (
        <div
          data-testid="stale-verification-warning"
          className="flex items-start gap-2.5 rounded-lg p-3"
          style={{
            background: "hsla(38 100% 60% / 0.08)",
            border: "1px solid hsla(38 100% 60% / 0.2)",
          }}
        >
          <AlertCircle
            className="w-3.5 h-3.5 shrink-0 mt-0.5"
            style={{ color: "hsl(38 100% 60%)" }}
          />
          <div className="flex flex-col gap-2 min-w-0 flex-1">
            <div>
              <p className="text-[12px] font-medium" style={{ color: "hsl(38 100% 70%)" }}>
                Verification may be stalled
              </p>
              <p className="text-[11px] mt-0.5" style={{ color: "hsl(220 10% 55%)" }}>
                The verification agent has been running longer than expected. Try retrying the verification.
              </p>
            </div>
            <Button
              size="sm"
              onClick={handleTriggerVerification}
              data-testid="stale-retry-button"
              className="h-7 px-2.5 text-[11px] font-semibold gap-1.5 rounded-lg self-start transition-colors duration-150"
              style={{
                color: "hsl(38 100% 65%)",
                background: "hsla(38 100% 60% / 0.12)",
                border: "1px solid hsla(38 100% 60% / 0.25)",
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.background = "hsla(38 100% 60% / 0.18)";
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.background = "hsla(38 100% 60% / 0.12)";
              }}
            >
              <RotateCcw className="w-3 h-3" />
              Retry
            </Button>
          </div>
        </div>
      )}

      {/* Status header row */}
      <div className="flex items-center justify-between gap-3">
        <VerificationBadge
          status={verificationStatus}
          inProgress={isInProgress}
          {...(verificationData?.currentRound !== undefined && {
            currentRound: verificationData.currentRound,
          })}
          {...(verificationData?.maxRounds !== undefined && {
            maxRounds: verificationData.maxRounds,
          })}
          {...(verificationData?.convergenceReason !== undefined && {
            convergenceReason: verificationData.convergenceReason,
          })}
          onRetry={handleTriggerVerification}
        />

        {/* Secondary action buttons */}
        <div className="flex items-center gap-1.5">
          {showSkipVerification && (
            <Button
              variant="ghost"
              size="sm"
              onClick={handleSkipVerification}
              data-testid="skip-verification-button"
              className="h-7 px-2.5 text-[11px] font-medium gap-1.5 rounded-lg transition-colors duration-150"
              style={{
                color: "hsl(220 10% 55%)",
                background: "transparent",
                border: "1px solid hsla(220 10% 100% / 0.08)",
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.background = "hsla(220 10% 100% / 0.06)";
                e.currentTarget.style.color = "hsl(220 10% 75%)";
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.background = "transparent";
                e.currentTarget.style.color = "hsl(220 10% 55%)";
              }}
            >
              <SkipForward className="w-3 h-3" />
              Skip
            </Button>
          )}
        </div>
      </div>

      {/* Gap list */}
      {hasGaps && (
        <div
          className="rounded-lg p-3"
          style={{
            background: "hsla(220 10% 100% / 0.02)",
            border: "1px solid hsla(220 10% 100% / 0.06)",
          }}
        >
          <div
            className="text-[11px] font-semibold uppercase tracking-wider mb-3"
            style={{ color: "hsl(220 10% 50%)" }}
          >
            Verification Gaps
          </div>
          {isVerified && (
            <div
              className="text-[11px] mb-2"
              style={{ color: "hsl(220 10% 55%)" }}
            >
              Verified with acceptable gaps — no critical issues remain.
            </div>
          )}
          <VerificationGapList
            gaps={gaps}
            {...(rounds.length > 0 && { rounds })}
            {...(gapScore !== undefined && { gapScore })}
            selectable={showAddressGaps}
            selectedGaps={selectedGaps}
            onSelectionChange={setSelectedGaps}
          />
        </div>
      )}

      {/* Address Gaps button */}
      {showAddressGaps && (
        <Button
          size="sm"
          onClick={handleAddressGaps}
          data-testid="address-gaps-button"
          className="h-7 px-2.5 text-[11px] font-semibold gap-1.5 rounded-lg transition-colors duration-150"
          style={{
            color: "hsl(14 100% 60%)",
            background: "hsla(14 100% 60% / 0.1)",
            border: "1px solid hsla(14 100% 60% / 0.2)",
          }}
          onMouseEnter={(e) => {
            e.currentTarget.style.background = "hsla(14 100% 60% / 0.15)";
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.background = "hsla(14 100% 60% / 0.1)";
          }}
        >
          <Wand2 className="w-3 h-3" />
          {selectedGaps.size === 0
            ? "Address All Gaps"
            : `Address ${selectedGaps.size} Gap${selectedGaps.size !== 1 ? "s" : ""}`}
        </Button>
      )}

      {/* Re-verify Plan button */}
      {showReVerify && (
        <Button
          variant="ghost"
          size="sm"
          onClick={handleTriggerVerification}
          data-testid="re-verify-button"
          className="h-7 px-2.5 text-[11px] font-medium gap-1.5 rounded-lg transition-colors duration-150"
          style={{
            color: "hsl(220 10% 55%)",
            background: "transparent",
            border: "1px solid hsla(220 10% 100% / 0.08)",
          }}
          onMouseEnter={(e) => {
            e.currentTarget.style.background = "hsla(220 10% 100% / 0.06)";
            e.currentTarget.style.color = "hsl(220 10% 75%)";
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.background = "transparent";
            e.currentTarget.style.color = "hsl(220 10% 55%)";
          }}
        >
          <RotateCcw className="w-3 h-3" />
          Re-verify Plan
        </Button>
      )}

      {/* Round history */}
      {hasRounds && (
        <div
          className="rounded-lg p-3"
          style={{
            background: "hsla(220 10% 100% / 0.02)",
            border: "1px solid hsla(220 10% 100% / 0.06)",
          }}
        >
          <div
            className="text-[11px] font-semibold uppercase tracking-wider mb-3"
            style={{ color: "hsl(220 10% 50%)" }}
          >
            Verification History
          </div>
          <VerificationHistory
            rounds={rounds}
            roundDetails={roundDetails}
            {...(hasGaps && { currentGaps: gaps })}
            {...(gapScore !== undefined && { gapScore })}
            status={verificationStatus}
            {...(verificationData?.convergenceReason !== undefined && {
              convergenceReason: verificationData.convergenceReason,
            })}
          />
        </div>
      )}
    </div>
  );
}
