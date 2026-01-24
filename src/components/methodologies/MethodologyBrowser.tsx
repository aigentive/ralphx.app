/**
 * MethodologyBrowser - Displays list of available methodologies
 *
 * Features:
 * - Methodology cards with name, description, phase/agent counts
 * - Active methodology badge
 * - Activate/Deactivate buttons
 * - Click to select/view details
 */

import type { MethodologyExtension } from "@/types/methodology";

// ============================================================================
// Types
// ============================================================================

interface MethodologyBrowserProps {
  methodologies: MethodologyExtension[];
  onActivate: (methodologyId: string) => void;
  onDeactivate: (methodologyId: string) => void;
  onSelect: (methodologyId: string) => void;
}

// ============================================================================
// Component
// ============================================================================

export function MethodologyBrowser({ methodologies, onActivate, onDeactivate, onSelect }: MethodologyBrowserProps) {
  if (methodologies.length === 0) {
    return (
      <div data-testid="methodology-browser" className="p-4 rounded" style={{ backgroundColor: "var(--bg-surface)" }}>
        <p className="text-sm text-center" style={{ color: "var(--text-muted)" }}>No methodologies available</p>
      </div>
    );
  }

  return (
    <div data-testid="methodology-browser" className="p-3 rounded space-y-2" style={{ backgroundColor: "var(--bg-surface)" }}>
      {methodologies.map((methodology) => (
        <div key={methodology.id} data-testid="methodology-card" data-active={methodology.isActive ? "true" : "false"}
          onClick={() => onSelect(methodology.id)} role="button" tabIndex={0} aria-label={methodology.name}
          onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") onSelect(methodology.id); }}
          className="w-full p-3 rounded border text-left cursor-pointer"
          style={{
            backgroundColor: "var(--bg-base)",
            borderColor: methodology.isActive ? "var(--accent-primary)" : "var(--border-subtle)",
          }}>
          {/* Header */}
          <div className="flex items-center justify-between mb-1">
            <div className="flex items-center gap-2">
              <span className="text-sm font-medium" style={{ color: "var(--text-primary)" }}>{methodology.name}</span>
              {methodology.isActive && (
                <span data-testid="active-badge" className="text-xs px-1.5 py-0.5 rounded"
                  style={{ backgroundColor: "var(--bg-hover)", color: "var(--status-success)" }}>Active</span>
              )}
            </div>
            {methodology.isActive ? (
              <button data-testid="deactivate-button" aria-label={`Deactivate ${methodology.name}`}
                onClick={(e) => { e.stopPropagation(); onDeactivate(methodology.id); }}
                className="text-xs px-2 py-1 rounded" style={{ backgroundColor: "var(--bg-hover)", color: "var(--text-muted)" }}>
                Deactivate
              </button>
            ) : (
              <button data-testid="activate-button" aria-label={`Activate ${methodology.name}`}
                onClick={(e) => { e.stopPropagation(); onActivate(methodology.id); }}
                className="text-xs px-2 py-1 rounded" style={{ backgroundColor: "var(--accent-primary)", color: "var(--text-on-accent)" }}>
                Activate
              </button>
            )}
          </div>

          {/* Description */}
          <p className="text-xs mb-2" style={{ color: "var(--text-secondary)" }}>{methodology.description}</p>

          {/* Stats */}
          <div className="flex gap-3 text-xs" style={{ color: "var(--text-muted)" }}>
            <span data-testid="phase-count">{methodology.phases.length} phases</span>
            <span data-testid="agent-count">{methodology.agentProfiles.length} agents</span>
          </div>
        </div>
      ))}
    </div>
  );
}
