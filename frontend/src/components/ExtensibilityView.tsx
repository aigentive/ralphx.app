/**
 * ExtensibilityView - Tabbed interface for Workflows, Artifacts, Research, Methodologies
 *
 * Design: macOS Tahoe Liquid Glass
 * - Frosted glass panels
 * - Ambient orange glow background
 * - Flat translucent cards
 */

import { useState, useCallback, useMemo } from "react";
import {
  Workflow,
  FileBox,
  Search,
  BookOpen,
} from "lucide-react";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  TooltipProvider,
} from "@/components/ui/tooltip";
import { useMethodologies } from "@/hooks/useMethodologies";
import { useMethodologyActivation } from "@/hooks/useMethodologyActivation";
import type { MethodologyResponse } from "@/lib/api/methodologies";
import type {
  MethodologyExtension,
  MethodologyPhase,
  MethodologyTemplate,
} from "@/types/methodology";
import { MethodologiesPanel } from "@/components/extensibility";
import { WorkflowsPanel, ArtifactsPanel, ResearchPanel } from "./ExtensibilityView.panels";

// ============================================================================
// Types
// ============================================================================

type TabId = "workflows" | "artifacts" | "research" | "methodologies";

// ============================================================================
// Helpers
// ============================================================================

/** Convert API response (snake_case) to MethodologyExtension (camelCase) for UI */
function convertMethodologyResponse(
  response: MethodologyResponse
): MethodologyExtension {
  const phases: MethodologyPhase[] = response.phases.map((p) => ({
    id: p.id,
    name: p.name,
    order: p.order,
    description: p.description ?? undefined,
    agentProfiles: p.agent_profiles,
    columnIds: p.column_ids,
  }));

  const templates: MethodologyTemplate[] = response.templates.map((t) => ({
    artifactType: t.artifact_type,
    templatePath: t.template_path,
    name: t.name ?? undefined,
    description: t.description ?? undefined,
  }));

  return {
    id: response.id,
    name: response.name,
    description: response.description ?? undefined,
    agentProfiles: response.agent_profiles,
    skills: response.skills,
    workflow: {
      id: response.workflow_id,
      name: response.workflow_name,
      columns: [],
      isDefault: false,
    },
    phases,
    templates,
    isActive: response.is_active,
    createdAt: response.created_at,
  };
}

// ============================================================================
// Main Component
// ============================================================================

export function ExtensibilityView() {
  const [activeTab, setActiveTab] = useState<TabId>("workflows");

  // Fetch methodologies
  const { data: methodologiesData } = useMethodologies();
  const { activate, deactivate } = useMethodologyActivation();

  // Convert API responses to UI-compatible format
  const methodologies = useMemo<MethodologyExtension[]>(() => {
    if (!methodologiesData) return [];
    return methodologiesData.map(convertMethodologyResponse);
  }, [methodologiesData]);

  // Handlers for methodology actions
  const handleActivate = useCallback(
    async (methodologyId: string) => {
      try {
        await activate(methodologyId);
      } catch {
        // Error is handled by the hook (shows notification)
      }
    },
    [activate]
  );

  const handleDeactivate = useCallback(
    async (methodologyId: string) => {
      try {
        await deactivate(methodologyId);
      } catch {
        // Error is handled by the hook (shows notification)
      }
    },
    [deactivate]
  );

  return (
    <TooltipProvider>
      <div
        data-testid="extensibility-view"
        className="flex flex-col h-full"
        style={{
          background: `
            radial-gradient(ellipse 80% 50% at 20% 0%, rgba(255,107,53,0.06) 0%, transparent 50%),
            radial-gradient(ellipse 60% 40% at 80% 100%, rgba(255,107,53,0.03) 0%, transparent 50%),
            var(--bg-base)
          `,
        }}
      >
        <Tabs
        value={activeTab}
        onValueChange={(v) => setActiveTab(v as TabId)}
        className="h-full flex flex-col"
      >
        {/* Tab Navigation */}
        <TabsList
          data-testid="tab-navigation"
          className="h-11 w-full justify-start gap-1 rounded-none border-b px-4 bg-transparent"
          style={{ borderColor: "var(--border-subtle)" }}
        >
          <TabsTrigger
            data-testid="tab-workflows"
            value="workflows"
            className="gap-2 px-4 py-2.5 rounded-none border-b-2 -mb-px data-[state=active]:border-[--accent-primary] data-[state=inactive]:border-transparent data-[state=active]:bg-transparent data-[state=inactive]:bg-transparent data-[state=active]:text-[--text-primary] data-[state=inactive]:text-[--text-muted] data-[state=active]:shadow-none transition-all duration-200"
          >
            <Workflow className="w-4 h-4" />
            Workflows
          </TabsTrigger>
          <TabsTrigger
            data-testid="tab-artifacts"
            value="artifacts"
            className="gap-2 px-4 py-2.5 rounded-none border-b-2 -mb-px data-[state=active]:border-[--accent-primary] data-[state=inactive]:border-transparent data-[state=active]:bg-transparent data-[state=inactive]:bg-transparent data-[state=active]:text-[--text-primary] data-[state=inactive]:text-[--text-muted] data-[state=active]:shadow-none transition-all duration-200"
          >
            <FileBox className="w-4 h-4" />
            Artifacts
          </TabsTrigger>
          <TabsTrigger
            data-testid="tab-research"
            value="research"
            className="gap-2 px-4 py-2.5 rounded-none border-b-2 -mb-px data-[state=active]:border-[--accent-primary] data-[state=inactive]:border-transparent data-[state=active]:bg-transparent data-[state=inactive]:bg-transparent data-[state=active]:text-[--text-primary] data-[state=inactive]:text-[--text-muted] data-[state=active]:shadow-none transition-all duration-200"
          >
            <Search className="w-4 h-4" />
            Research
          </TabsTrigger>
          <TabsTrigger
            data-testid="tab-methodologies"
            value="methodologies"
            className="gap-2 px-4 py-2.5 rounded-none border-b-2 -mb-px data-[state=active]:border-[--accent-primary] data-[state=inactive]:border-transparent data-[state=active]:bg-transparent data-[state=inactive]:bg-transparent data-[state=active]:text-[--text-primary] data-[state=inactive]:text-[--text-muted] data-[state=active]:shadow-none transition-all duration-200"
          >
            <BookOpen className="w-4 h-4" />
            Methodologies
          </TabsTrigger>
        </TabsList>

        {/* Tab Content */}
        <ScrollArea className="flex-1">
          <div className="p-6">
            <TabsContent value="workflows" className="mt-0">
              <WorkflowsPanel />
            </TabsContent>
            <TabsContent value="artifacts" className="mt-0 h-[calc(100vh-200px)]">
              <ArtifactsPanel />
            </TabsContent>
            <TabsContent value="research" className="mt-0">
              <ResearchPanel />
            </TabsContent>
            <TabsContent value="methodologies" className="mt-0">
              <MethodologiesPanel
                methodologies={methodologies}
                onActivate={handleActivate}
                onDeactivate={handleDeactivate}
              />
            </TabsContent>
          </div>
        </ScrollArea>
      </Tabs>
      </div>
    </TooltipProvider>
  );
}
