import { useEffect, useMemo, useState } from "react";

import { SeparatorLine } from "@/components/ui/ResizeHandle";
import { useProjects } from "@/hooks/useProjects";
import type { CreateDesignSystemInput, ImportDesignSystemPackageInput } from "@/lib/tauri";
import { buildDesignSystemFromResponse, type DesignSystem } from "./designSystems";
import { DesignComposerSurface } from "./DesignComposerSurface";
import { DesignPackageImportDialog } from "./DesignPackageImportDialog";
import { DesignSidebar } from "./DesignSidebar";
import { DesignSourceComposerDialog } from "./DesignSourceComposerDialog";
import { DesignStyleguidePane } from "./DesignStyleguidePane";
import {
  useCreateDesignSystem,
  useDesignSystemDetail,
  useDesignStyleguideItems,
  useDesignStyleguideViewModel,
  useExportDesignSystemPackage,
  useGenerateDesignSystemStyleguide,
  useImportDesignSystemPackage,
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
  const [isSourceComposerOpen, setIsSourceComposerOpen] = useState(false);
  const [isPackageImportOpen, setIsPackageImportOpen] = useState(false);
  const [exportPackageArtifactId, setExportPackageArtifactId] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState("");
  const { groups } = useProjectDesignSystems(projects, { searchQuery });
  const createDesignSystem = useCreateDesignSystem();
  const generateStyleguide = useGenerateDesignSystemStyleguide();
  const exportPackage = useExportDesignSystemPackage();
  const importPackage = useImportDesignSystemPackage();
  const allSystems = useMemo(
    () => groups.flatMap((group) => group.systems),
    [groups],
  );
  const selectedListDesignSystem =
    allSystems.find((system) => system.id === selectedDesignSystemId) ?? null;
  const selectedDetailQuery = useDesignSystemDetail(selectedDesignSystemId);
  const selectedStyleguideItemsQuery = useDesignStyleguideItems(selectedDesignSystemId);
  const selectedStyleguideViewModelQuery = useDesignStyleguideViewModel(selectedDesignSystemId);
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
      styleguideItems: selectedStyleguideItemsQuery.data ?? [],
      styleguideViewModel: selectedStyleguideViewModelQuery.data ?? null,
    });
  }, [
    projects,
    selectedDetailQuery.data,
    selectedListDesignSystem,
    selectedStyleguideItemsQuery.data,
    selectedStyleguideViewModelQuery.data,
  ]);

  useEffect(() => {
    if (projectId) {
      setFocusedProjectId(projectId);
    }
  }, [projectId]);

  useEffect(() => {
    setExportPackageArtifactId(null);
  }, [selectedDesignSystemId]);

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

  const openSourceComposer = () => {
    setIsSourceComposerOpen(true);
  };

  const openPackageImport = () => {
    setIsPackageImportOpen(true);
  };

  const createDraftDesignSystem = (input: CreateDesignSystemInput) => {
    if (createDesignSystem.isPending) {
      return;
    }
    createDesignSystem.mutate(
      input,
      {
        onSuccess: (response) => {
          setFocusedProjectId(response.designSystem.primaryProjectId);
          setSelectedDesignSystemId(response.designSystem.id);
          setIsSourceComposerOpen(false);
        },
      },
    );
  };

  const generateSelectedStyleguide = () => {
    if (!selectedDesignSystem || generateStyleguide.isPending) {
      return;
    }
    generateStyleguide.mutate(selectedDesignSystem.id);
  };

  const exportSelectedPackage = () => {
    if (!selectedDesignSystem || exportPackage.isPending) {
      return;
    }
    exportPackage.mutate(selectedDesignSystem.id, {
      onSuccess: (response) => {
        setExportPackageArtifactId(response.artifactId);
      },
    });
  };

  const importDesignPackage = (input: ImportDesignSystemPackageInput) => {
    if (importPackage.isPending) {
      return;
    }
    importPackage.mutate(input, {
      onSuccess: (response) => {
        setFocusedProjectId(response.designSystem.primaryProjectId);
        setSelectedDesignSystemId(response.designSystem.id);
        setIsPackageImportOpen(false);
      },
    });
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
          onNewDesignSystem={openSourceComposer}
          onImportDesignSystem={openPackageImport}
        />
      </div>

      <div className="flex-1 min-w-0 h-full flex overflow-hidden">
        <div className="flex-1 min-w-0 h-full" style={{ minWidth: DESIGN_CHAT_MIN_WIDTH }}>
          <DesignComposerSurface
            selectedDesignSystem={selectedDesignSystem}
            onNewDesignSystem={openSourceComposer}
            onImportDesignSystem={openPackageImport}
          />
        </div>

        <SeparatorLine />

        <div
          className="h-full shrink-0"
          style={{ width: DESIGN_STYLEGUIDE_DEFAULT_WIDTH, minWidth: DESIGN_STYLEGUIDE_MIN_WIDTH }}
        >
          <DesignStyleguidePane
            designSystem={selectedDesignSystem}
            isGeneratingStyleguide={generateStyleguide.isPending}
            isExportingPackage={exportPackage.isPending}
            exportPackageArtifactId={exportPackageArtifactId}
            onGenerateStyleguide={generateSelectedStyleguide}
            onExportPackage={exportSelectedPackage}
          />
        </div>
      </div>
      <DesignSourceComposerDialog
        isOpen={isSourceComposerOpen}
        projects={projects}
        focusedProjectId={focusedProjectId}
        isCreating={createDesignSystem.isPending}
        onOpenChange={setIsSourceComposerOpen}
        onCreate={createDraftDesignSystem}
      />
      <DesignPackageImportDialog
        isOpen={isPackageImportOpen}
        projects={projects}
        focusedProjectId={focusedProjectId}
        isImporting={importPackage.isPending}
        onOpenChange={setIsPackageImportOpen}
        onImport={importDesignPackage}
      />
    </div>
  );
}
