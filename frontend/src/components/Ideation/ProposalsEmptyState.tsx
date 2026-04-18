/**
 * ProposalsEmptyState - Empty state for proposals panel
 */

import { Lightbulb } from "lucide-react";
import { withAlpha } from "@/lib/theme-colors";
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
              border: "1.5px dashed var(--accent-border)",
              background: withAlpha("var(--accent-primary)", 2),
            }}
          >
            <div
              className="w-4 h-4 rounded border-[1.5px] border-dashed flex-shrink-0"
              style={{ borderColor: withAlpha("var(--accent-primary)", 40) }}
            />
            <div
              className="h-2 rounded-full flex-1"
              style={{
                background: "var(--overlay-weak)",
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
            background: "var(--status-warning-muted)",
            border: "1px solid var(--status-warning-border)",
          }}
        >
          <Lightbulb className="w-5 h-5" style={{ color: "var(--status-warning)" }} />
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
