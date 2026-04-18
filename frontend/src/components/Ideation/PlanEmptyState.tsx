import { TabEmptyState } from "./TabEmptyState";

function PlanDocumentIcon() {
  return (
    <div
      className="w-10 h-12 rounded-md flex flex-col justify-start pt-2 px-2 gap-1.5"
      data-testid="plan-document-mock"
      style={{
        background: "var(--overlay-faint)",
        border: "1px solid var(--overlay-weak)",
      }}
    >
      {/* Title line */}
      <div
        className="h-1 rounded-full w-full"
        style={{ background: "var(--border-default)" }}
      />
      {/* Section lines */}
      <div
        className="h-px rounded-full w-3/4"
        style={{ background: "var(--overlay-moderate)" }}
      />
      <div
        className="h-px rounded-full w-full"
        style={{ background: "var(--overlay-moderate)" }}
      />
      <div
        className="h-px rounded-full w-5/6"
        style={{ background: "var(--overlay-moderate)" }}
      />
      <div
        className="h-px rounded-full w-2/3"
        style={{ background: "var(--overlay-moderate)" }}
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
