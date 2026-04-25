import {
  type MouseEvent as ReactMouseEvent,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { toast } from "sonner";

import { ResizeHandle } from "@/components/ui/ResizeHandle";
import { useProjects } from "@/hooks/useProjects";
import { extractErrorMessage } from "@/lib/errors";
import type {
  CreateDesignSystemInput,
  ExportDesignSystemPackageResponse,
  ImportDesignSystemPackageInput,
} from "@/lib/tauri";
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
const DESIGN_CHAT_MIN_WIDTH = 360;
const DESIGN_STYLEGUIDE_MIN_WIDTH = 420;
const DESIGN_STYLEGUIDE_DEFAULT_WIDTH = 640;
const DESIGN_STYLEGUIDE_WIDTH_STORAGE_KEY = "ralphx-design-styleguide-width";

interface DesignViewProps {
  projectId: string;
  onCreateProject: () => void;
}

interface StyleguideGenerationResult {
  itemCount: number;
  caveatCount: number;
  schemaVersionId: string | null;
}

function buildExportFileName(designSystem: DesignSystem): string {
  const slug = designSystem.name
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 48);
  const safeSlug = slug || "design-system";
  return `ralphx-design-system-${safeSlug}.zip`;
}

export function DesignView({ projectId }: DesignViewProps) {
  const { data: projects = [] } = useProjects();
  const [focusedProjectId, setFocusedProjectId] = useState<string | null>(projectId || null);
  const [selectedDesignSystemId, setSelectedDesignSystemId] = useState<string | null>(null);
  const [isSourceComposerOpen, setIsSourceComposerOpen] = useState(false);
  const [isPackageImportOpen, setIsPackageImportOpen] = useState(false);
  const [exportPackageResult, setExportPackageResult] =
    useState<ExportDesignSystemPackageResponse | null>(null);
  const [isSavingExportPackage, setIsSavingExportPackage] = useState(false);
  const [styleguideGenerationResult, setStyleguideGenerationResult] =
    useState<StyleguideGenerationResult | null>(null);
  const [styleguidePanelWidth, setStyleguidePanelWidth] = useState(() => {
    if (typeof window === "undefined") {
      return DESIGN_STYLEGUIDE_DEFAULT_WIDTH;
    }
    const saved = window.localStorage.getItem(DESIGN_STYLEGUIDE_WIDTH_STORAGE_KEY);
    if (!saved) {
      return DESIGN_STYLEGUIDE_DEFAULT_WIDTH;
    }
    const parsed = Number.parseInt(saved, 10);
    if (Number.isNaN(parsed)) {
      return DESIGN_STYLEGUIDE_DEFAULT_WIDTH;
    }
    return Math.max(DESIGN_STYLEGUIDE_MIN_WIDTH, parsed);
  });
  const [isStyleguideResizing, setIsStyleguideResizing] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const workspaceRef = useRef<HTMLDivElement>(null);
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
  const createDesignSystemError = createDesignSystem.error
    ? extractErrorMessage(createDesignSystem.error, "Failed to create design system")
    : null;

  useEffect(() => {
    if (projectId) {
      setFocusedProjectId(projectId);
    }
  }, [projectId]);

  useEffect(() => {
    setExportPackageResult(null);
    setStyleguideGenerationResult(null);
  }, [selectedDesignSystemId]);

  useEffect(() => {
    window.localStorage.setItem(
      DESIGN_STYLEGUIDE_WIDTH_STORAGE_KEY,
      String(styleguidePanelWidth),
    );
  }, [styleguidePanelWidth]);

  const handleStyleguideResizeStart = useCallback((event: ReactMouseEvent) => {
    event.preventDefault();
    setIsStyleguideResizing(true);
  }, []);

  const handleStyleguideResizeReset = useCallback((event: ReactMouseEvent) => {
    event.preventDefault();
    setStyleguidePanelWidth(DESIGN_STYLEGUIDE_DEFAULT_WIDTH);
  }, []);

  useEffect(() => {
    if (!isStyleguideResizing) {
      return;
    }

    const handleMouseMove = (event: MouseEvent) => {
      const container = workspaceRef.current;
      if (!container) {
        return;
      }
      const rect = container.getBoundingClientRect();
      const maxStyleguideWidth = Math.max(
        DESIGN_STYLEGUIDE_MIN_WIDTH,
        rect.width - DESIGN_CHAT_MIN_WIDTH,
      );
      const nextWidth = rect.right - event.clientX;
      setStyleguidePanelWidth(
        Math.max(DESIGN_STYLEGUIDE_MIN_WIDTH, Math.min(maxStyleguideWidth, nextWidth)),
      );
    };

    const handleMouseUp = () => setIsStyleguideResizing(false);

    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);

    return () => {
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
    };
  }, [isStyleguideResizing]);

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
    createDesignSystem.reset();
    setIsSourceComposerOpen(true);
  };

  const setSourceComposerOpen = (open: boolean) => {
    if (!open) {
      createDesignSystem.reset();
    }
    setIsSourceComposerOpen(open);
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
        onError: (error) => {
          const message = extractErrorMessage(error, "Failed to create design system");
          toast.error("Failed to create design system", {
            description: message,
          });
        },
      },
    );
  };

  const generateSelectedStyleguide = () => {
    if (!selectedDesignSystem || generateStyleguide.isPending) {
      return;
    }
    generateStyleguide.mutate(selectedDesignSystem.id, {
      onSuccess: (response) => {
        const caveatCount = response.items.filter((item) => item.confidence === "low").length;
        const rowLabel = response.items.length === 1 ? "row" : "rows";
        const caveatLabel = caveatCount === 1 ? "caveat" : "caveats";
        let description = "The Design agent will publish the styleguide when source analysis is complete.";
        if (response.items.length > 0) {
          description = `${response.items.length} existing review ${rowLabel}`;
          if (caveatCount > 0) {
            description = `${description}, ${caveatCount} ${caveatLabel}`;
          }
        }
        setStyleguideGenerationResult({
          itemCount: response.items.length,
          caveatCount,
          schemaVersionId: response.schemaVersionId ?? null,
        });
        toast.success("Design is analyzing selected sources", {
          description,
        });
      },
      onError: (error) => {
        const message = extractErrorMessage(error, "Failed to generate styleguide");
        toast.error("Failed to generate styleguide", {
          description: message,
        });
      },
    });
  };

  const exportSelectedPackage = async () => {
    if (!selectedDesignSystem || exportPackage.isPending || isSavingExportPackage) {
      return;
    }
    setIsSavingExportPackage(true);
    try {
      const savePath = await save({
        filters: [{ name: "RalphX Design Package", extensions: ["zip"] }],
        defaultPath: buildExportFileName(selectedDesignSystem),
      });

      if (savePath === null) {
        return;
      }

      exportPackage.mutate({
        designSystemId: selectedDesignSystem.id,
        destinationPath: savePath,
      }, {
        onSuccess: (response) => {
          setExportPackageResult(response);
          toast.success("Design package exported", {
            description: response.filePath
              ? "Saved the exported styleguide and schema package."
              : `Artifact ${response.artifactId.slice(0, 8)} is ready.`,
          });
        },
        onError: (error) => {
          const message = extractErrorMessage(error, "Failed to export design package");
          toast.error("Failed to export design package", {
            description: message,
          });
        },
      });
    } catch (error) {
      const message = extractErrorMessage(error, "Failed to choose export destination");
      toast.error("Failed to choose export destination", {
        description: message,
      });
    } finally {
      setIsSavingExportPackage(false);
    }
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

      <div ref={workspaceRef} className="flex-1 min-w-0 h-full flex overflow-hidden">
        <div className="flex-1 min-w-0 h-full" style={{ minWidth: DESIGN_CHAT_MIN_WIDTH }}>
          <DesignComposerSurface
            selectedDesignSystem={selectedDesignSystem}
            onNewDesignSystem={openSourceComposer}
            onImportDesignSystem={openPackageImport}
          />
        </div>

        <ResizeHandle
          isResizing={isStyleguideResizing}
          onMouseDown={handleStyleguideResizeStart}
          onDoubleClick={handleStyleguideResizeReset}
          testId="design-styleguide-resize-handle"
        />

        <div
          className="h-full shrink-0"
          style={{
            width: styleguidePanelWidth,
            minWidth: DESIGN_STYLEGUIDE_MIN_WIDTH,
            maxWidth: `calc(100% - ${DESIGN_CHAT_MIN_WIDTH}px)`,
            transition: isStyleguideResizing ? "none" : "width 150ms ease-out",
          }}
          data-testid="design-styleguide-resizable-pane"
        >
          <DesignStyleguidePane
            designSystem={selectedDesignSystem}
            isGeneratingStyleguide={generateStyleguide.isPending}
            isExportingPackage={exportPackage.isPending || isSavingExportPackage}
            exportPackage={exportPackageResult}
            generationResult={styleguideGenerationResult}
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
        createError={createDesignSystemError}
        onOpenChange={setSourceComposerOpen}
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
