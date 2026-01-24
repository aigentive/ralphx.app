/**
 * ExtensibilityView - Settings/configuration for workflows, artifacts, research, methodologies
 *
 * Features:
 * - Tab navigation (Workflows, Artifacts, Research, Methodologies)
 * - Each tab renders respective browser/editor components
 * - Accessible tab implementation
 */

import { useState } from "react";
import { WorkflowEditor } from "@/components/workflows/WorkflowEditor";
import { ArtifactBrowser } from "@/components/artifacts/ArtifactBrowser";
import { ResearchLauncher } from "@/components/research/ResearchLauncher";
import { MethodologyBrowser } from "@/components/methodologies/MethodologyBrowser";

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
// Component
// ============================================================================

export function ExtensibilityView() {
  const [activeTab, setActiveTab] = useState<TabId>("workflows");
  const panelId = "extensibility-panel";

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
        {activeTab === "methodologies" && <MethodologyBrowser methodologies={[]} onActivate={() => {}} onDeactivate={() => {}} onSelect={() => {}} />}
      </div>
    </div>
  );
}
