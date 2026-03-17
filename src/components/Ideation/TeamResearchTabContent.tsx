import { TeamResearchView } from "./TeamResearchView";
import type { TeamArtifactSummary } from "@/api/team";

interface TeamResearchTabContentProps {
  teamArtifacts: TeamArtifactSummary[];
  sessionId: string;
}

export function TeamResearchTabContent({ teamArtifacts, sessionId }: TeamResearchTabContentProps) {
  return (
    <div className="flex-1 overflow-y-auto p-4">
      <TeamResearchView artifacts={teamArtifacts} sessionId={sessionId} />
    </div>
  );
}
