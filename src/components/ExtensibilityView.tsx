/**
 * ExtensibilityView - Settings/configuration for workflows, artifacts, research, methodologies
 *
 * Features:
 * - Tab navigation (Workflows, Artifacts, Research, Methodologies)
 * - Each tab renders respective browser/editor components
 * - Accessible tab implementation
 * - Integrated methodology activation with app state
 */

import { useState, useCallback, useMemo } from "react";
import { WorkflowEditor } from "@/components/workflows/WorkflowEditor";
import { ArtifactBrowser } from "@/components/artifacts/ArtifactBrowser";
import { ResearchLauncher } from "@/components/research/ResearchLauncher";
import { MethodologyBrowser } from "@/components/methodologies/MethodologyBrowser";
import { useMethodologies } from "@/hooks/useMethodologies";
import { useMethodologyActivation } from "@/hooks/useMethodologyActivation";
import type { MethodologyResponse } from "@/lib/api/methodologies";
import type { MethodologyExtension, MethodologyPhase, MethodologyTemplate } from "@/types/methodology";

// ============================================================================
// Types
// ============================================================================

type TabId = "workflows" | "artifacts" | "research" | "methodologies";

interface Tab {
  id: TabId;
  label: string;
}

// ============================================================================
// Constants
// ============================================================================

const TABS: Tab[] = [
  { id: "workflows", label: "Workflows" },
  { id: "artifacts", label: "Artifacts" },
  { id: "research", label: "Research" },
  { id: "methodologies", label: "Methodologies" },
];

// ============================================================================
// Helpers
// ============================================================================

/** Convert API response (snake_case) to MethodologyExtension (camelCase) for UI */
function convertMethodologyResponse(response: MethodologyResponse): MethodologyExtension {
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
    workflow: { id: response.workflow_id, name: response.workflow_name, columns: [], isDefault: false },
    phases,
    templates,
    isActive: response.is_active,
    createdAt: response.created_at,
  };
}

// ============================================================================
// Component
// ============================================================================

export function ExtensibilityView() {
  const [activeTab, setActiveTab] = useState<TabId>("workflows");
  const panelId = "extensibility-panel";

  // Fetch methodologies
  const { data: methodologiesData } = useMethodologies();
  const { activate, deactivate } = useMethodologyActivation();

  // Convert API responses to UI-compatible format
  const methodologies = useMemo<MethodologyExtension[]>(() => {
    if (!methodologiesData) return [];
    return methodologiesData.map(convertMethodologyResponse);
  }, [methodologiesData]);

  // Handlers for methodology actions
  const handleActivate = useCallback(async (methodologyId: string) => {
    try {
      await activate(methodologyId);
    } catch {
      // Error is handled by the hook (shows notification)
    }
  }, [activate]);

  const handleDeactivate = useCallback(async (methodologyId: string) => {
    try {
      await deactivate(methodologyId);
    } catch {
      // Error is handled by the hook (shows notification)
    }
  }, [deactivate]);

  const handleSelectMethodology = useCallback((_methodologyId: string) => {
    // TODO: Show methodology details panel when selected
  }, []);

  return (
    <div data-testid="extensibility-view" className="flex flex-col h-full" style={{ backgroundColor: "var(--bg-base)" }}>
      {/* Tab Navigation */}
      <div data-testid="tab-navigation" role="tablist" className="flex border-b px-4" style={{ borderColor: "var(--border-subtle)" }}>
        {TABS.map((tab) => (
          <button
            key={tab.id}
            data-testid={`tab-${tab.id}`}
            role="tab"
            aria-selected={activeTab === tab.id}
            aria-controls={panelId}
            onClick={() => setActiveTab(tab.id)}
            className="px-4 py-2 text-sm font-medium border-b-2 -mb-px transition-colors"
            style={{
              color: activeTab === tab.id ? "var(--text-primary)" : "var(--text-muted)",
              borderColor: activeTab === tab.id ? "var(--accent-primary)" : "transparent",
            }}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {/* Tab Panel */}
      <div id={panelId} role="tabpanel" className="flex-1 overflow-auto p-4">
        {activeTab === "workflows" && <WorkflowEditor onSave={() => {}} onCancel={() => {}} />}
        {activeTab === "artifacts" && <ArtifactBrowser buckets={[]} artifacts={[]} selectedBucketId={null} selectedArtifactId={null} onSelectBucket={() => {}} onSelectArtifact={() => {}} />}
        {activeTab === "research" && <ResearchLauncher onLaunch={() => {}} onCancel={() => {}} />}
        {activeTab === "methodologies" && (
          <MethodologyBrowser
            methodologies={methodologies}
            onActivate={handleActivate}
            onDeactivate={handleDeactivate}
            onSelect={handleSelectMethodology}
          />
        )}
      </div>
    </div>
  );
}
