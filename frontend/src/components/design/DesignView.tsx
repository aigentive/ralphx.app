import { useEffect, useMemo, useState } from "react";

import { SeparatorLine } from "@/components/ui/ResizeHandle";
import { useProjects } from "@/hooks/useProjects";
import { buildDesignSystemFromResponse, type DesignSystem } from "./designSystems";
import { DesignComposerSurface } from "./DesignComposerSurface";
import { DesignSidebar } from "./DesignSidebar";
import { DesignStyleguidePane } from "./DesignStyleguidePane";
import {
  useCreateDesignSystem,
  useDesignSystemDetail,
  useProjectDesignSystems,
} from "./useProjectDesignSystems";

const DESIGN_SIDEBAR_WIDTH = 320;
const DESIGN_CHAT_MIN_WIDTH = 320;
const DESIGN_STYLEGUIDE_MIN_WIDTH = 360;
const DESIGN_STYLEGUIDE_DEFAULT_WIDTH = 520;

interface DesignViewProps {
  projectId: string;
  onCreateProject: () => void;
}

export function DesignView({ projectId }: DesignViewProps) {
  const { data: projects = [] } = useProjects();
  const [focusedProjectId, setFocusedProjectId] = useState<string | null>(projectId || null);
  const [selectedDesignSystemId, setSelectedDesignSystemId] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState("");
  const { groups } = useProjectDesignSystems(projects, { searchQuery });
  const createDesignSystem = useCreateDesignSystem();
  const allSystems = useMemo(
    () => groups.flatMap((group) => group.systems),
    [groups],
  );
  const selectedListDesignSystem =
    allSystems.find((system) => system.id === selectedDesignSystemId) ?? null;
  const selectedDetailQuery = useDesignSystemDetail(selectedDesignSystemId);
  const selectedDesignSystem = useMemo(() => {
    const detail = selectedDetailQuery.data;
    if (!detail) {
      return selectedListDesignSystem;
    }

    const project = projects.find((candidate) => candidate.id === detail.designSystem.primaryProjectId);
    if (!project) {
      return selectedListDesignSystem;
    }

    return buildDesignSystemFromResponse(project, detail.designSystem, {
      sources: detail.sources,
      conversationId: detail.conversation?.id ?? null,
    });
  }, [projects, selectedDetailQuery.data, selectedListDesignSystem]);

  useEffect(() => {
    if (projectId) {
      setFocusedProjectId(projectId);
    }
  }, [projectId]);

  useEffect(() => {
    if (selectedDesignSystemId && allSystems.some((system) => system.id === selectedDesignSystemId)) {
      return;
    }

    const preferred =
      allSystems.find((system) => system.primaryProjectId === focusedProjectId) ??
      allSystems[0] ??
      null;
    setSelectedDesignSystemId(preferred?.id ?? null);
  }, [allSystems, focusedProjectId, selectedDesignSystemId]);

  const selectDesignSystem = (system: DesignSystem) => {
    setFocusedProjectId(system.primaryProjectId);
    setSelectedDesignSystemId(system.id);
  };

  const selectPreferredDesignSystem = () => {
    const preferred =
      allSystems.find((system) => system.primaryProjectId === focusedProjectId) ??
      allSystems[0] ??
      null;
    setSelectedDesignSystemId(preferred?.id ?? null);
  };

  const createDraftDesignSystem = () => {
    const targetProject =
      projects.find((project) => project.id === focusedProjectId) ??
      projects.find((project) => project.id === projectId) ??
      projects[0] ??
      null;

    if (!targetProject || createDesignSystem.isPending) {
      return;
    }

    createDesignSystem.mutate(
      {
        primaryProjectId: targetProject.id,
        name: `${targetProject.name} Design System`,
        selectedPaths: [],
        sources: [],
      },
      {
        onSuccess: (response) => {
          setFocusedProjectId(response.designSystem.primaryProjectId);
          setSelectedDesignSystemId(response.designSystem.id);
        },
      },
    );
  };

  return (
    <div className="h-full min-h-0 flex overflow-hidden" data-testid="design-view">
      <div style={{ width: DESIGN_SIDEBAR_WIDTH, minWidth: DESIGN_SIDEBAR_WIDTH }}>
        <DesignSidebar
          groups={groups}
          focusedProjectId={focusedProjectId}
          selectedDesignSystemId={selectedDesignSystemId}
          searchQuery={searchQuery}
          onSearchQueryChange={setSearchQuery}
          onFocusProject={setFocusedProjectId}
          onSelectDesignSystem={selectDesignSystem}
          onNewDesignSystem={createDraftDesignSystem}
          onImportDesignSystem={selectPreferredDesignSystem}
        />
      </div>

      <div className="flex-1 min-w-0 h-full flex overflow-hidden">
        <div className="flex-1 min-w-0 h-full" style={{ minWidth: DESIGN_CHAT_MIN_WIDTH }}>
          <DesignComposerSurface
            selectedDesignSystem={selectedDesignSystem}
            onNewDesignSystem={createDraftDesignSystem}
            onImportDesignSystem={selectPreferredDesignSystem}
          />
        </div>

        <SeparatorLine />

        <div
          className="h-full shrink-0"
          style={{ width: DESIGN_STYLEGUIDE_DEFAULT_WIDTH, minWidth: DESIGN_STYLEGUIDE_MIN_WIDTH }}
        >
          <DesignStyleguidePane designSystem={selectedDesignSystem} />
        </div>
      </div>
    </div>
  );
}
