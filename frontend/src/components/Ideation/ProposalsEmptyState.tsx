/**
 * ProposalsEmptyState - Empty state for proposals panel
 */

import { Lightbulb } from "lucide-react";
import { TabEmptyState } from "./TabEmptyState";

interface ProposalsEmptyStateProps {
  /** Called when user clicks the drop hint to browse for a file */
  onBrowse?: () => void;
}

function ProposalsMockVisual() {
  return (
    <div className="w-full">
      {/* Mock task cards */}
      <div className="space-y-2 mb-5">
        {[0.4, 0.25, 0.15].map((opacity, i) => (
          <div
            key={i}
            className="flex items-center gap-3 p-3 rounded-lg"
            style={{
              opacity,
              border: "1.5px dashed hsla(14 100% 60% / 0.25)",
              background: "hsla(14 100% 60% / 0.02)",
            }}
          >
            <div
              className="w-4 h-4 rounded border-[1.5px] border-dashed flex-shrink-0"
              style={{ borderColor: "hsla(14 100% 60% / 0.4)" }}
            />
            <div
              className="h-2 rounded-full flex-1"
              style={{
                background: "hsla(220 10% 100% / 0.06)",
                maxWidth: `${70 - i * 15}%`,
              }}
            />
          </div>
        ))}
      </div>

      {/* Central icon */}
      <div className="flex justify-center">
        <div
          className="w-12 h-12 rounded-xl flex items-center justify-center"
          style={{
            background: "hsla(45 93% 50% / 0.12)",
            border: "1px solid hsla(45 93% 50% / 0.25)",
          }}
        >
          <Lightbulb className="w-5 h-5" style={{ color: "hsl(45 93% 55%)" }} />
        </div>
      </div>
    </div>
  );
}

export function ProposalsEmptyState({ onBrowse }: ProposalsEmptyStateProps) {
  return (
    <div data-testid="proposals-empty-state">
      <TabEmptyState
        icon={<ProposalsMockVisual />}
        heading="No proposals yet"
        description="Ideas from the conversation will appear here as task proposals"
        {...(onBrowse !== undefined && { onBrowse })}
      />
    </div>
  );
}
