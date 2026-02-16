/**
 * TeamArtifactChips - Horizontal row of clickable artifact pills
 *
 * Renders below PlanDisplay. Each chip shows type icon + name.
 * Active chip gets warm orange border + subtle orange bg tint.
 * macOS Tahoe glass-morphism, warm orange accent.
 */

import React from "react";
import { Microscope, BarChart3, FileText } from "lucide-react";
import { cn } from "@/lib/utils";
import type { TeamArtifactSummary } from "@/api/team";

// ============================================================================
// Types
// ============================================================================

export interface TeamArtifactChipsProps {
  artifacts: TeamArtifactSummary[];
  selectedArtifactId: string | null;
  onSelect: (artifactId: string) => void;
}

// ============================================================================
// Helpers
// ============================================================================

interface ArtifactTypeConfig {
  icon: React.ElementType;
  color: string;
}

const DEFAULT_CONFIG: ArtifactTypeConfig = { icon: FileText, color: "hsl(220 10% 50%)" };

const ARTIFACT_TYPE_CONFIG: Record<string, ArtifactTypeConfig> = {
  research: { icon: Microscope, color: "hsl(14 100% 60%)" },
  analysis: { icon: BarChart3, color: "hsl(174 60% 50%)" },
  summary: DEFAULT_CONFIG,
};

function getTypeConfig(artifactType: string): ArtifactTypeConfig {
  return ARTIFACT_TYPE_CONFIG[artifactType.toLowerCase()] ?? DEFAULT_CONFIG;
}

// ============================================================================
// Component
// ============================================================================

export const TeamArtifactChips = React.memo(function TeamArtifactChips({
  artifacts,
  selectedArtifactId,
  onSelect,
}: TeamArtifactChipsProps) {
  if (artifacts.length === 0) return null;

  return (
    <div className="mt-3">
      {/* Section header */}
      <span
        className="text-[11px] font-medium tracking-wide block mb-2"
        style={{ color: "hsl(220 10% 50%)" }}
      >
        Team Research &middot; {artifacts.length} artifact{artifacts.length !== 1 ? "s" : ""}
      </span>

      {/* Chips row */}
      <div className="flex flex-wrap gap-2">
        {artifacts.map((artifact) => {
          const isActive = artifact.id === selectedArtifactId;
          const config = getTypeConfig(artifact.artifact_type);
          const Icon = config.icon;

          return (
            <button
              key={artifact.id}
              data-testid={`artifact-chip-${artifact.id}`}
              onClick={() => onSelect(artifact.id)}
              className={cn(
                "flex items-center gap-1.5 h-7 px-2.5 rounded-full",
                "text-[12px] font-medium",
                "transition-all duration-150 ease-out",
                "cursor-pointer select-none",
              )}
              style={{
                background: isActive
                  ? "hsla(14 100% 60% / 0.06)"
                  : "hsla(220 10% 100% / 0.02)",
                border: isActive
                  ? "1px solid hsla(14 100% 60% / 0.35)"
                  : "1px solid hsla(220 10% 100% / 0.06)",
                color: isActive
                  ? "hsl(220 10% 90%)"
                  : "hsl(220 10% 65%)",
              }}
              onMouseEnter={(e) => {
                if (!isActive) {
                  e.currentTarget.style.background = "hsla(220 10% 100% / 0.04)";
                  e.currentTarget.style.borderColor = "hsla(220 10% 100% / 0.12)";
                }
              }}
              onMouseLeave={(e) => {
                if (!isActive) {
                  e.currentTarget.style.background = "hsla(220 10% 100% / 0.02)";
                  e.currentTarget.style.borderColor = "hsla(220 10% 100% / 0.06)";
                }
              }}
            >
              <Icon
                className="w-3.5 h-3.5 flex-shrink-0"
                style={{ color: isActive ? config.color : "hsl(220 10% 50%)" }}
              />
              <span className="truncate max-w-[140px]">
                {artifact.name}
              </span>
            </button>
          );
        })}
      </div>
    </div>
  );
});
