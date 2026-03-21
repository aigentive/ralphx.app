import { TabEmptyState } from "./TabEmptyState";

function PlanDocumentIcon() {
  return (
    <div
      className="w-10 h-12 rounded-md flex flex-col justify-start pt-2 px-2 gap-1.5"
      data-testid="plan-document-mock"
      style={{
        background: "hsla(220 10% 100% / 0.04)",
        border: "1px solid hsla(220 10% 100% / 0.08)",
      }}
    >
      {/* Title line */}
      <div
        className="h-1 rounded-full w-full"
        style={{ background: "hsla(220 10% 100% / 0.25)" }}
      />
      {/* Section lines */}
      <div
        className="h-px rounded-full w-3/4"
        style={{ background: "hsla(220 10% 100% / 0.12)" }}
      />
      <div
        className="h-px rounded-full w-full"
        style={{ background: "hsla(220 10% 100% / 0.12)" }}
      />
      <div
        className="h-px rounded-full w-5/6"
        style={{ background: "hsla(220 10% 100% / 0.12)" }}
      />
      <div
        className="h-px rounded-full w-2/3"
        style={{ background: "hsla(220 10% 100% / 0.12)" }}
      />
    </div>
  );
}

interface PlanEmptyStateProps {
  onBrowse?: () => void;
}

export function PlanEmptyState({ onBrowse }: PlanEmptyStateProps) {
  return (
    <div data-testid="plan-empty-state">
      <TabEmptyState
        icon={<PlanDocumentIcon />}
        heading="No plan yet"
        description="The implementation plan will appear here when created from the conversation"
        {...(onBrowse !== undefined && { onBrowse })}
      />
    </div>
  );
}
