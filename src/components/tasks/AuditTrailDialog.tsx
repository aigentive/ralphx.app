/**
 * AuditTrailDialog - Two-column audit trail dialog (thin orchestrator).
 * Left: AuditTrailSidebar (phase navigation). Right: EventCard timeline.
 */

import { useState, useMemo } from "react";
import { ScrollText, X, Loader2 } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { useAuditTrail } from "@/hooks/useAuditTrail";
import { AuditTrailSidebar } from "./audit-trail/AuditTrailSidebar";
import { EventCard } from "./audit-trail/EventCards";

// ============================================================================
// Props
// ============================================================================

interface AuditTrailDialogProps {
  taskId: string;
  isOpen: boolean;
  onClose: () => void;
}

// ============================================================================
// Helpers
// ============================================================================

function formatDateRange(entries: { timestamp: string }[]): string {
  if (entries.length === 0) return "";
  const first = entries[0];
  const last = entries[entries.length - 1];
  if (!first || !last) return "";
  const f = new Date(first.timestamp);
  const l = new Date(last.timestamp);
  const fmt = (d: Date) =>
    d.toLocaleDateString(undefined, { month: "short", day: "numeric", year: "numeric" });
  if (f.toDateString() === l.toDateString()) return fmt(f);
  return `${fmt(f)} \u2014 ${fmt(l)}`;
}

// ============================================================================
// Main Component
// ============================================================================

export function AuditTrailDialog({ taskId, isOpen, onClose }: AuditTrailDialogProps) {
  const { entries, phases, isLoading, isEmpty } = useAuditTrail(taskId, { enabled: isOpen });
  const [selectedPhaseId, setSelectedPhaseId] = useState<string | null>(null);

  const selectedPhase = useMemo(
    () => phases.find((p) => p.id === selectedPhaseId) ?? null,
    [phases, selectedPhaseId],
  );

  const filteredEntries = useMemo(() => {
    if (!selectedPhaseId) return entries;
    return entries.filter((e) => e.phaseId === selectedPhaseId);
  }, [entries, selectedPhaseId]);

  const dateRange = useMemo(() => formatDateRange(entries), [entries]);

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && onClose()}>
      <DialogContent
        data-testid="audit-trail-dialog"
        hideCloseButton
        className="p-0 gap-0 overflow-hidden flex flex-col max-w-[95vw] w-[95vw] h-[95vh]"
        style={{
          backgroundColor: "var(--bg-surface)",
          border: "1px solid rgba(255,255,255,0.08)",
        }}
      >
        {/* Glass Header */}
        <div
          className="flex items-center justify-between px-4 py-3 border-b shrink-0"
          style={{
            borderColor: "rgba(255,255,255,0.06)",
            background: "rgba(18,18,18,0.85)",
            backdropFilter: "blur(20px)",
          }}
        >
          <div className="flex items-center gap-3">
            <ScrollText className="w-5 h-5" style={{ color: "var(--accent-primary)" }} />
            <DialogTitle
              className="text-base font-semibold text-white/90"
              style={{ letterSpacing: "-0.02em" }}
            >
              Audit Trail
            </DialogTitle>
          </div>
          <Button
            data-testid="dialog-close"
            variant="ghost"
            size="icon"
            onClick={onClose}
            className="w-8 h-8 text-white/50 hover:text-white/80 hover:bg-white/10"
          >
            <X className="w-4 h-4" />
          </Button>
        </div>

        {/* Two-column body */}
        <div className="flex flex-1 min-h-0">
          {/* Left sidebar - 320px */}
          <div
            className="w-[320px] shrink-0 border-r overflow-hidden"
            style={{ borderColor: "rgba(255,255,255,0.06)" }}
          >
            <AuditTrailSidebar
              phases={phases}
              selectedPhaseId={selectedPhaseId}
              onPhaseSelect={setSelectedPhaseId}
              totalEvents={entries.length}
              dateRange={dateRange}
              isLoading={isLoading}
            />
          </div>

          {/* Right content - flex-1 */}
          <div className="flex-1 min-w-0 overflow-y-auto" data-testid="audit-trail-timeline">
            {isLoading && (
              <div className="flex justify-center py-16" data-testid="audit-trail-loading">
                <Loader2 className="w-6 h-6 animate-spin" style={{ color: "var(--text-muted)" }} />
              </div>
            )}

            {!isLoading && isEmpty && (
              <div
                className="flex flex-col items-center justify-center py-16 text-center"
                data-testid="audit-trail-empty"
              >
                <ScrollText
                  className="w-8 h-8 mb-2"
                  style={{ color: "var(--text-muted)", opacity: 0.5 }}
                />
                <p className="text-sm text-white/50">No audit events recorded yet</p>
                <p className="text-xs mt-1 text-white/30">
                  State transitions and activity events will appear here
                </p>
              </div>
            )}

            {!isLoading && !isEmpty && (
              <>
                {/* Phase header (when a phase is selected) */}
                {selectedPhase && (
                  <div
                    className="sticky top-0 z-10 px-4 py-2.5 border-b"
                    style={{
                      borderColor: "rgba(255,255,255,0.06)",
                      background: "rgba(18,18,18,0.95)",
                      backdropFilter: "blur(12px)",
                    }}
                  >
                    <span className="text-[12px] font-semibold text-white/80">
                      {selectedPhase.label}
                    </span>
                    <span className="text-[11px] text-white/40 ml-2">
                      {filteredEntries.length} events
                    </span>
                  </div>
                )}

                {/* Filtered view (phase selected) */}
                {selectedPhaseId && (
                  <div className="p-4 space-y-2">
                    {filteredEntries.map((entry) => (
                      <EventCard key={entry.id} entry={entry} />
                    ))}
                  </div>
                )}

                {/* Grouped view (no phase selected, phases exist) */}
                {!selectedPhaseId && phases.length > 0 && (
                  <div className="p-4 space-y-4">
                    {phases.map((phase) => {
                      const phaseEntries = entries.filter((e) => e.phaseId === phase.id);
                      if (phaseEntries.length === 0) return null;
                      return (
                        <div key={phase.id}>
                          <h4
                            className="text-[11px] font-semibold uppercase tracking-wider mb-2"
                            style={{ color: "var(--text-muted)" }}
                          >
                            {phase.label}
                          </h4>
                          <div className="space-y-2">
                            {phaseEntries.map((entry) => (
                              <EventCard key={entry.id} entry={entry} />
                            ))}
                          </div>
                        </div>
                      );
                    })}
                    {entries.filter((e) => !e.phaseId).length > 0 && (
                      <div>
                        <h4
                          className="text-[11px] font-semibold uppercase tracking-wider mb-2"
                          style={{ color: "var(--text-muted)" }}
                        >
                          Other
                        </h4>
                        <div className="space-y-2">
                          {entries
                            .filter((e) => !e.phaseId)
                            .map((entry) => (
                              <EventCard key={entry.id} entry={entry} />
                            ))}
                        </div>
                      </div>
                    )}
                  </div>
                )}

                {/* Flat view (no phases derived) */}
                {!selectedPhaseId && phases.length === 0 && (
                  <div className="p-4 space-y-2">
                    {entries.map((entry) => (
                      <EventCard key={entry.id} entry={entry} />
                    ))}
                  </div>
                )}
              </>
            )}
          </div>
        </div>

        {/* Glass Footer */}
        <div
          className="flex items-center justify-between px-4 py-3 border-t shrink-0"
          style={{
            borderColor: "rgba(255,255,255,0.06)",
            background: "rgba(18,18,18,0.85)",
            backdropFilter: "blur(20px)",
          }}
        >
          <span className="text-[12px] text-white/50">
            {entries.length} {entries.length === 1 ? "event" : "events"}
            {selectedPhase && ` \u00B7 Showing: ${selectedPhase.label}`}
          </span>
          <Button
            variant="ghost"
            onClick={onClose}
            className="text-[13px] text-white/60 hover:text-white/80 hover:bg-white/10"
          >
            Close
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}
