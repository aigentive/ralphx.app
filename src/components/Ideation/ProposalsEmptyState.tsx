/**
 * ProposalsEmptyState - Empty state for proposals panel
 */

import { Lightbulb, FileDown } from "lucide-react";

interface ProposalsEmptyStateProps {
  /** Called when user clicks the drop hint to browse for a file */
  onBrowse?: () => void;
}

export function ProposalsEmptyState({ onBrowse }: ProposalsEmptyStateProps) {
  return (
    <div
      data-testid="proposals-empty-state"
      className="flex flex-col items-center pt-[20%] h-full p-6"
    >
      <div className="w-full max-w-[280px]">
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

        {/* Central icon - flat style */}
        <div className="flex justify-center mb-4">
          <div
            className="w-12 h-12 rounded-xl flex items-center justify-center relative"
            style={{
              background: "hsla(45 93% 50% / 0.12)",
              border: "1px solid hsla(45 93% 50% / 0.25)",
            }}
          >
            <Lightbulb className="w-5 h-5" style={{ color: "hsl(45 93% 55%)" }} />
          </div>
        </div>

        {/* Text */}
        <div className="text-center">
          <h3 className="text-sm font-semibold mb-1.5 tracking-tight" style={{ color: "hsl(220 10% 90%)" }}>
            No proposals yet
          </h3>
          <p className="text-xs leading-relaxed" style={{ color: "hsl(220 10% 60%)" }}>
            Ideas from the conversation will appear here as task proposals
          </p>
        </div>

        {/* Visual hint */}
        <div className="flex justify-center mt-5">
          <div
            className="flex items-center gap-2 px-3 py-1.5 rounded-full"
            style={{
              background: "hsla(220 10% 100% / 0.03)",
              border: "1px solid hsla(220 10% 100% / 0.06)",
            }}
          >
            <svg
              width="14"
              height="14"
              viewBox="0 0 14 14"
              fill="none"
              style={{ color: "hsl(220 10% 50%)" }}
            >
              <path
                d="M12 7H2m0 0l3-3m-3 3l3 3"
                stroke="currentColor"
                strokeWidth="1.5"
                strokeLinecap="round"
                strokeLinejoin="round"
              />
            </svg>
            <span className="text-[10px] uppercase tracking-wider" style={{ color: "hsl(220 10% 50%)" }}>
              From chat
            </span>
          </div>
        </div>

        {/* Divider with "or" */}
        <div className="flex items-center gap-3 mt-6">
          <div
            className="flex-1 h-px"
            style={{ background: "hsla(220 10% 100% / 0.08)" }}
          />
          <span className="text-[10px] uppercase tracking-wider" style={{ color: "hsl(220 10% 50%)" }}>
            or
          </span>
          <div
            className="flex-1 h-px"
            style={{ background: "hsla(220 10% 100% / 0.08)" }}
          />
        </div>

        {/* Drop hint - clickable to open file browser */}
        <button
          data-testid="drop-hint"
          type="button"
          onClick={onBrowse}
          className="w-full flex flex-col items-center gap-2 mt-4 p-3 rounded-lg transition-colors cursor-pointer focus:outline-none"
          style={{ color: "hsl(220 10% 50%)" }}
          onMouseEnter={(e) => {
            e.currentTarget.style.background = "hsla(220 10% 100% / 0.03)";
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.background = "transparent";
          }}
        >
          <FileDown className="w-4 h-4" />
          <p className="text-xs text-center leading-relaxed">
            Drag a markdown file here
            <br />
            or <span style={{ color: "hsla(14 100% 60% / 0.8)" }} className="underline underline-offset-2">click to browse</span>
          </p>
        </button>
      </div>
    </div>
  );
}
