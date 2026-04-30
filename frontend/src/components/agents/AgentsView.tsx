import { AgentsShellLayout } from "./AgentsShellLayout";
import { AgentsConversationMainRegion } from "./AgentsConversationMainRegion";
import { AgentsConversationSideRegions } from "./AgentsConversationSideRegions";
import { useAgentsViewController } from "./useAgentsViewController";

interface AgentsViewProps {
  projectId: string;
  onCreateProject: () => void;
}

export function AgentsView({
  projectId,
  onCreateProject,
}: AgentsViewProps) {
  const {
    mainRegionProps,
    shellProps,
    sideRegionProps,
  } = useAgentsViewController({
    projectId,
    onCreateProject,
  });

  return (
    <AgentsShellLayout {...shellProps}>
      <AgentsConversationMainRegion {...mainRegionProps} />
      <AgentsConversationSideRegions {...sideRegionProps} />
    </AgentsShellLayout>
  );
}
