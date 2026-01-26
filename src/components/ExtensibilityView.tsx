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
  Plus,
  Edit,
  Copy,
  Trash2,
  List,
  LayoutGrid,
  ArrowUpDown,
  FileText,
  FileJson,
  FileCode,
  Image,
  File,
  Zap,
  Target,
  Telescope,
  Microscope,
  Sliders,
  Rocket,
  Loader2,
} from "lucide-react";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
  TooltipProvider,
} from "@/components/ui/tooltip";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useMethodologies } from "@/hooks/useMethodologies";
import { useMethodologyActivation } from "@/hooks/useMethodologyActivation";
import type { MethodologyResponse } from "@/lib/api/methodologies";
import type {
  MethodologyExtension,
  MethodologyPhase,
  MethodologyTemplate,
} from "@/types/methodology";
import type { WorkflowSchema } from "@/types/workflow";
import type { Artifact, ArtifactBucket } from "@/types/artifact";
import type { ResearchDepthPreset, ResearchDepth, ResearchBrief } from "@/types/research";
import { RESEARCH_PRESET_INFO } from "@/types/research";

// ============================================================================
// Types
// ============================================================================

type TabId = "workflows" | "artifacts" | "research" | "methodologies";
type ViewMode = "grid" | "list";
type SortBy = "name" | "date" | "size" | "type";
type DepthSelection = ResearchDepthPreset | "custom";

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

/** Get file type icon based on artifact type or extension */
function getFileIcon(type: string) {
  switch (type.toLowerCase()) {
    case "markdown":
    case "md":
      return FileText;
    case "json":
      return FileJson;
    case "code":
    case "ts":
    case "tsx":
    case "js":
    case "jsx":
    case "rs":
      return FileCode;
    case "image":
    case "png":
    case "jpg":
    case "jpeg":
    case "svg":
      return Image;
    default:
      return File;
  }
}

/** Get depth preset icon */
function getDepthIcon(preset: string) {
  switch (preset) {
    case "quick-scan":
      return Zap;
    case "standard":
      return Target;
    case "deep-dive":
      return Telescope;
    case "exhaustive":
      return Microscope;
    default:
      return Sliders;
  }
}

// ============================================================================
// Sub-components
// ============================================================================

/**
 * WorkflowsPanel - Workflow management with cards
 */
function WorkflowsPanel() {
  // Mock data for now - would come from API
  const workflows: WorkflowSchema[] = [
    {
      id: "default",
      name: "Default Kanban",
      description: "Standard development workflow",
      columns: [
        { id: "1", name: "Backlog", mapsTo: "backlog" },
        { id: "2", name: "In Progress", mapsTo: "executing" },
        { id: "3", name: "Review", mapsTo: "pending_review" },
        { id: "4", name: "Done", mapsTo: "approved" },
      ],
      isDefault: true,
    },
  ];

  const isEmpty = workflows.length === 0;

  return (
    <div data-testid="workflows-panel" className="space-y-4">
      {/* Header */}
      <div className="flex items-center justify-between">
        <h2
          className="text-lg font-semibold"
          style={{
            color: "var(--text-primary)",
            letterSpacing: "-0.02em",
          }}
        >
          Workflow Schemas
        </h2>
        <Button variant="secondary" size="sm" className="gap-1.5">
          <Plus className="w-4 h-4" />
          New Workflow
        </Button>
      </div>

      {/* Empty State */}
      {isEmpty ? (
        <div className="flex flex-col items-center justify-center py-16">
          <div
            className="w-16 h-16 rounded-xl flex items-center justify-center mb-4"
            style={{
              border: "2px dashed var(--border-subtle)",
            }}
          >
            <Workflow
              className="w-8 h-8"
              style={{ color: "var(--text-muted)" }}
            />
          </div>
          <p
            className="text-sm font-medium mb-1"
            style={{ color: "var(--text-secondary)" }}
          >
            No custom workflows yet
          </p>
          <p className="text-xs mb-4" style={{ color: "var(--text-muted)" }}>
            Create a workflow to organize tasks
          </p>
          <Button className="gap-1.5">
            <Plus className="w-4 h-4" />
            Create Workflow
          </Button>
        </div>
      ) : (
        /* Workflow Cards */
        <div className="space-y-3">
          {workflows.map((workflow) => (
            <Card
              key={workflow.id}
              data-testid="workflow-card"
              className="group transition-all duration-180 hover:-translate-y-px"
              style={{
                background: "rgba(255,255,255,0.04)",
                backdropFilter: "blur(20px)",
                WebkitBackdropFilter: "blur(20px)",
                border: "1px solid rgba(255,255,255,0.08)",
                boxShadow: "0 1px 3px rgba(0,0,0,0.12)",
              }}
            >
              <CardContent className="p-4">
                {/* Header Row */}
                <div className="flex items-center justify-between mb-2">
                  <div className="flex items-center gap-2">
                    {workflow.isDefault && (
                      <div
                        className="w-2 h-2 rounded-full"
                        style={{ backgroundColor: "var(--accent-primary)" }}
                      />
                    )}
                    <span
                      className="text-sm font-medium"
                      style={{ color: "var(--text-primary)" }}
                    >
                      {workflow.name}
                    </span>
                    {workflow.isDefault && (
                      <Badge variant="secondary" className="text-[10px]">
                        DEFAULT
                      </Badge>
                    )}
                  </div>
                  <div className="flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                    <Tooltip>
                      <TooltipTrigger asChild>
                        <Button
                          variant="ghost"
                          size="sm"
                          className="h-7 w-7 p-0"
                        >
                          <Edit className="w-4 h-4" />
                        </Button>
                      </TooltipTrigger>
                      <TooltipContent>Edit</TooltipContent>
                    </Tooltip>
                    <Tooltip>
                      <TooltipTrigger asChild>
                        <Button
                          variant="ghost"
                          size="sm"
                          className="h-7 w-7 p-0"
                        >
                          <Copy className="w-4 h-4" />
                        </Button>
                      </TooltipTrigger>
                      <TooltipContent>Duplicate</TooltipContent>
                    </Tooltip>
                    {!workflow.isDefault && (
                      <Tooltip>
                        <TooltipTrigger asChild>
                          <Button
                            variant="ghost"
                            size="sm"
                            className="h-7 w-7 p-0 text-red-400 hover:text-red-300"
                          >
                            <Trash2 className="w-4 h-4" />
                          </Button>
                        </TooltipTrigger>
                        <TooltipContent>Delete</TooltipContent>
                      </Tooltip>
                    )}
                  </div>
                </div>

                {/* Description */}
                <p
                  className="text-sm line-clamp-2 mb-2"
                  style={{ color: "var(--text-secondary)" }}
                >
                  {workflow.description}
                </p>

                {/* Metadata */}
                <div
                  className="text-xs flex items-center gap-2"
                  style={{ color: "var(--text-muted)" }}
                >
                  <span>{workflow.columns.length} columns</span>
                  <span>·</span>
                  <span>Created Jan 2026</span>
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      )}
    </div>
  );
}

/**
 * ArtifactsPanel - Browse artifacts by bucket with grid/list toggle
 */
function ArtifactsPanel() {
  const [viewMode, setViewMode] = useState<ViewMode>("grid");
  const [selectedBucket, setSelectedBucket] = useState<string | null>(null);
  const [_sortBy, setSortBy] = useState<SortBy>("name");
  const [searchQuery, setSearchQuery] = useState("");

  // Mock data - would come from API
  const buckets: ArtifactBucket[] = [
    { id: "all", name: "All", acceptedTypes: [], writers: [], readers: ["all"], isSystem: false },
    { id: "system", name: "System", acceptedTypes: ["context", "activity_log"], writers: ["system"], readers: ["all"], isSystem: true },
    { id: "prds", name: "PRDs", acceptedTypes: ["prd", "specification"], writers: ["orchestrator", "user"], readers: ["all"], isSystem: false },
    { id: "docs", name: "Docs", acceptedTypes: ["research_document", "design_doc"], writers: ["user"], readers: ["all"], isSystem: false },
  ];

  const artifacts: Artifact[] = [
    {
      id: "1",
      name: "PRD.md",
      type: "prd",
      content: { type: "file", path: "/docs/PRD.md" },
      metadata: { createdAt: "2026-01-01T00:00:00Z", createdBy: "user", version: 1 },
      derivedFrom: [],
      bucketId: "prds",
    },
    {
      id: "2",
      name: "Research Notes",
      type: "research_document",
      content: { type: "inline", text: "Research content here..." },
      metadata: { createdAt: "2026-01-01T00:00:00Z", createdBy: "deep-researcher", version: 1 },
      derivedFrom: [],
      bucketId: "docs",
    },
  ];

  const filteredArtifacts = artifacts.filter((a) => {
    if (selectedBucket && selectedBucket !== "all" && a.bucketId !== selectedBucket) {
      return false;
    }
    if (searchQuery && !a.name.toLowerCase().includes(searchQuery.toLowerCase())) {
      return false;
    }
    return true;
  });

  const bucketCounts = artifacts.reduce<Record<string, number>>((acc, a) => {
    if (a.bucketId) {
      acc[a.bucketId] = (acc[a.bucketId] ?? 0) + 1;
    }
    acc["all"] = (acc["all"] ?? 0) + 1;
    return acc;
  }, {});

  return (
    <div data-testid="artifacts-panel" className="flex h-full gap-4">
      {/* Bucket Sidebar */}
      <div
        className="w-48 flex-shrink-0 p-3 rounded-lg"
        style={{
          background: "rgba(255,255,255,0.03)",
          backdropFilter: "blur(20px)",
          WebkitBackdropFilter: "blur(20px)",
          border: "1px solid rgba(255,255,255,0.06)",
        }}
      >
        <h3
          className="text-xs font-medium uppercase tracking-wide mb-3"
          style={{ color: "var(--text-muted)" }}
        >
          Buckets
        </h3>
        <div className="space-y-1">
          {buckets.map((bucket) => {
            const isSelected = selectedBucket === bucket.id;
            const count = bucketCounts[bucket.id] ?? 0;
            return (
              <button
                key={bucket.id}
                data-testid="bucket-item"
                onClick={() => setSelectedBucket(bucket.id)}
                className="w-full flex items-center justify-between px-2 py-1.5 rounded text-sm transition-colors hover:bg-[--bg-hover]"
                style={{
                  backgroundColor: isSelected ? "var(--bg-hover)" : undefined,
                  color: isSelected
                    ? "var(--text-primary)"
                    : "var(--text-secondary)",
                }}
              >
                <div className="flex items-center gap-1.5">
                  <span className="truncate">{bucket.name}</span>
                  {bucket.isSystem && (
                    <Badge
                      variant="secondary"
                      className="text-[10px] px-1 py-0"
                    >
                      S
                    </Badge>
                  )}
                </div>
                <span className="text-xs" style={{ color: "var(--text-muted)" }}>
                  {count}
                </span>
              </button>
            );
          })}
        </div>
      </div>

      {/* Artifact Content */}
      <div className="flex-1 space-y-4">
        {/* Search & Filter Bar */}
        <div className="flex items-center gap-3">
          <div className="relative flex-1">
            <Search
              className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4"
              style={{ color: "var(--text-muted)" }}
            />
            <Input
              placeholder="Search artifacts..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="pl-9"
            />
          </div>
          <div className="flex items-center gap-1 p-1 rounded-md" style={{ backgroundColor: "var(--bg-surface)" }}>
            <Button
              variant={viewMode === "list" ? "secondary" : "ghost"}
              size="sm"
              className="h-7 w-7 p-0"
              onClick={() => setViewMode("list")}
            >
              <List className="w-4 h-4" />
            </Button>
            <Button
              variant={viewMode === "grid" ? "secondary" : "ghost"}
              size="sm"
              className="h-7 w-7 p-0"
              onClick={() => setViewMode("grid")}
            >
              <LayoutGrid className="w-4 h-4" />
            </Button>
          </div>
          <Select defaultValue="name" onValueChange={(v) => setSortBy(v as SortBy)}>
            <SelectTrigger className="w-auto gap-1.5">
              <ArrowUpDown className="w-4 h-4" />
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="name">Name</SelectItem>
              <SelectItem value="date">Date</SelectItem>
              <SelectItem value="size">Size</SelectItem>
              <SelectItem value="type">Type</SelectItem>
            </SelectContent>
          </Select>
        </div>

        {/* Artifact Display */}
        {!selectedBucket ? (
          <div
            className="flex flex-col items-center justify-center py-16"
            style={{ color: "var(--text-muted)" }}
          >
            <FileBox className="w-12 h-12 mb-3" />
            <p className="text-sm">Select a bucket to view artifacts</p>
          </div>
        ) : filteredArtifacts.length === 0 ? (
          <div
            className="flex flex-col items-center justify-center py-16"
            style={{ color: "var(--text-muted)" }}
          >
            <div
              className="w-16 h-16 rounded-xl flex items-center justify-center mb-4"
              style={{ border: "2px dashed var(--border-subtle)" }}
            >
              <FileBox className="w-8 h-8" />
            </div>
            <p className="text-sm">No artifacts in this bucket</p>
          </div>
        ) : viewMode === "grid" ? (
          <div className="grid grid-cols-4 gap-3">
            {filteredArtifacts.map((artifact) => {
              const IconComponent = getFileIcon(artifact.type);
              return (
                <Card
                  key={artifact.id}
                  data-testid="artifact-card"
                  className="group cursor-pointer transition-all duration-180 hover:-translate-y-px"
                  style={{
                    background: "rgba(255,255,255,0.04)",
                    backdropFilter: "blur(20px)",
                    WebkitBackdropFilter: "blur(20px)",
                    border: "1px solid rgba(255,255,255,0.08)",
                    boxShadow: "0 1px 3px rgba(0,0,0,0.12)",
                  }}
                >
                  <CardContent className="p-3 text-center">
                    <IconComponent
                      className="w-8 h-8 mx-auto mb-2"
                      style={{ color: "var(--text-secondary)" }}
                    />
                    <p
                      className="text-sm truncate"
                      style={{ color: "var(--text-primary)" }}
                    >
                      {artifact.name}
                    </p>
                    <p className="text-xs" style={{ color: "var(--text-muted)" }}>
                      {artifact.type}
                    </p>
                  </CardContent>
                </Card>
              );
            })}
          </div>
        ) : (
          <div className="space-y-1">
            {filteredArtifacts.map((artifact) => {
              const IconComponent = getFileIcon(artifact.type);
              return (
                <div
                  key={artifact.id}
                  data-testid="artifact-row"
                  className="flex items-center gap-3 px-3 py-2 rounded-md cursor-pointer transition-colors hover:bg-[--bg-hover]"
                >
                  <IconComponent
                    className="w-5 h-5"
                    style={{ color: "var(--text-secondary)" }}
                  />
                  <span
                    className="flex-1 text-sm"
                    style={{ color: "var(--text-primary)" }}
                  >
                    {artifact.name}
                  </span>
                  <Badge variant="secondary" className="text-[10px]">
                    {artifact.type}
                  </Badge>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}

/**
 * ResearchPanel - Launch and manage research sessions
 */
function ResearchPanel() {
  const [question, setQuestion] = useState("");
  const [context, setContext] = useState("");
  const [scope, setScope] = useState("");
  const [selectedPreset, setSelectedPreset] = useState<DepthSelection>("standard");
  const [customIterations, setCustomIterations] = useState(100);
  const [customTimeout, setCustomTimeout] = useState(4);
  const [isLaunching, setIsLaunching] = useState(false);

  const isValid = question.trim().length > 0;
  const isCustom = selectedPreset === "custom";

  const handleLaunch = useCallback(() => {
    const brief: ResearchBrief = {
      question,
      constraints: [],
      ...(context && { context }),
      ...(scope && { scope }),
    };
    const depth: ResearchDepth = isCustom
      ? {
          type: "custom",
          config: {
            maxIterations: customIterations,
            timeoutHours: customTimeout,
            checkpointInterval: Math.ceil(customIterations / 10),
          },
        }
      : { type: "preset", preset: selectedPreset as ResearchDepthPreset };

    setIsLaunching(true);
    // Simulate launch
    setTimeout(() => setIsLaunching(false), 2000);
    console.log("Launching research:", { brief, depth });
  }, [question, context, scope, isCustom, selectedPreset, customIterations, customTimeout]);

  // Recent sessions mock
  const recentSessions = [
    {
      id: "1",
      question: "Best practices for state management in React",
      status: "complete" as const,
      preset: "standard",
      iterations: 45,
      duration: "2h 15m",
      date: "Jan 24",
    },
  ];

  return (
    <div data-testid="research-panel" className="space-y-6">
      {/* Research Launcher Card */}
      <Card
        className="max-w-xl mx-auto"
        style={{
          background: "rgba(255,255,255,0.04)",
          backdropFilter: "blur(20px)",
          WebkitBackdropFilter: "blur(20px)",
          border: "1px solid rgba(255,255,255,0.08)",
          boxShadow: "0 1px 3px rgba(0,0,0,0.12)",
        }}
      >
        <CardContent className="p-6 space-y-5">
          <h2
            className="text-lg font-semibold text-center"
            style={{
              color: "var(--text-primary)",
              letterSpacing: "-0.02em",
            }}
          >
            Launch New Research
          </h2>

          {/* Question */}
          <div className="space-y-2">
            <Label htmlFor="question" className="font-medium">
              Research Question
            </Label>
            <Textarea
              id="question"
              data-testid="question-input"
              value={question}
              onChange={(e) => setQuestion(e.target.value)}
              placeholder="What do you want to research?"
              rows={3}
              disabled={isLaunching}
            />
          </div>

          {/* Context & Scope */}
          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-2">
              <Label
                htmlFor="context"
                className="text-xs"
                style={{ color: "var(--text-secondary)" }}
              >
                Context (optional)
              </Label>
              <Input
                id="context"
                data-testid="context-input"
                value={context}
                onChange={(e) => setContext(e.target.value)}
                placeholder="Background information"
                disabled={isLaunching}
              />
            </div>
            <div className="space-y-2">
              <Label
                htmlFor="scope"
                className="text-xs"
                style={{ color: "var(--text-secondary)" }}
              >
                Scope (optional)
              </Label>
              <Input
                id="scope"
                data-testid="scope-input"
                value={scope}
                onChange={(e) => setScope(e.target.value)}
                placeholder="Limit the scope"
                disabled={isLaunching}
              />
            </div>
          </div>

          {/* Depth Preset Selector */}
          <div className="space-y-3">
            <Label className="font-medium">Research Depth</Label>
            <div
              data-testid="depth-preset-selector"
              role="radiogroup"
              aria-label="Research depth"
              className="grid grid-cols-2 gap-2"
            >
              {RESEARCH_PRESET_INFO.map((info) => {
                const isSelected = selectedPreset === info.preset;
                const IconComponent = getDepthIcon(info.preset);
                return (
                  <button
                    key={info.preset}
                    type="button"
                    role="radio"
                    aria-checked={isSelected}
                    data-testid={`preset-${info.preset}`}
                    onClick={() => setSelectedPreset(info.preset)}
                    disabled={isLaunching}
                    className="p-3 rounded-lg text-left transition-all duration-150"
                    style={{
                      backgroundColor: "var(--bg-base)",
                      border: isSelected
                        ? "2px solid var(--accent-primary)"
                        : "1px solid var(--border-subtle)",
                    }}
                  >
                    <IconComponent
                      className="w-4 h-4 mb-1"
                      style={{
                        color: isSelected
                          ? "var(--accent-primary)"
                          : "var(--text-muted)",
                      }}
                    />
                    <div
                      className="text-sm font-medium"
                      style={{ color: "var(--text-primary)" }}
                    >
                      {info.name}
                    </div>
                    <div className="text-xs" style={{ color: "var(--text-muted)" }}>
                      {info.config.maxIterations} iter, {info.config.timeoutHours}h
                    </div>
                  </button>
                );
              })}
              <button
                type="button"
                role="radio"
                aria-checked={isCustom}
                data-testid="preset-custom"
                onClick={() => setSelectedPreset("custom")}
                disabled={isLaunching}
                className="p-3 rounded-lg text-left transition-all duration-150"
                style={{
                  backgroundColor: "var(--bg-base)",
                  border: isCustom
                    ? "2px solid var(--accent-primary)"
                    : "1px solid var(--border-subtle)",
                }}
              >
                <Sliders
                  className="w-4 h-4 mb-1"
                  style={{
                    color: isCustom
                      ? "var(--accent-primary)"
                      : "var(--text-muted)",
                  }}
                />
                <div
                  className="text-sm font-medium"
                  style={{ color: "var(--text-primary)" }}
                >
                  Custom
                </div>
                <div className="text-xs" style={{ color: "var(--text-muted)" }}>
                  Set your own limits
                </div>
              </button>
            </div>
          </div>

          {/* Custom Depth Inputs */}
          {isCustom && (
            <div className="grid grid-cols-2 gap-3 animate-in slide-in-from-top-2 duration-200">
              <div className="space-y-2">
                <Label
                  htmlFor="iterations"
                  className="text-xs"
                  style={{ color: "var(--text-secondary)" }}
                >
                  Max Iterations
                </Label>
                <Input
                  id="iterations"
                  data-testid="custom-iterations-input"
                  type="number"
                  value={customIterations}
                  onChange={(e) => setCustomIterations(Number(e.target.value))}
                  min={1}
                  disabled={isLaunching}
                />
              </div>
              <div className="space-y-2">
                <Label
                  htmlFor="timeout"
                  className="text-xs"
                  style={{ color: "var(--text-secondary)" }}
                >
                  Timeout (hours)
                </Label>
                <Input
                  id="timeout"
                  data-testid="custom-timeout-input"
                  type="number"
                  value={customTimeout}
                  onChange={(e) => setCustomTimeout(Number(e.target.value))}
                  min={0.5}
                  step={0.5}
                  disabled={isLaunching}
                />
              </div>
            </div>
          )}

          {/* Actions */}
          <div className="flex justify-end gap-2 pt-2">
            <Button
              data-testid="cancel-button"
              variant="ghost"
              disabled={isLaunching}
            >
              Cancel
            </Button>
            <Button
              data-testid="launch-button"
              onClick={handleLaunch}
              disabled={!isValid || isLaunching}
              className="gap-1.5"
            >
              {isLaunching ? (
                <>
                  <Loader2 className="w-4 h-4 animate-spin" />
                  Launching...
                </>
              ) : (
                <>
                  <Rocket className="w-4 h-4" />
                  Launch Research
                </>
              )}
            </Button>
          </div>
        </CardContent>
      </Card>

      {/* Recent Sessions */}
      {recentSessions.length > 0 && (
        <div className="max-w-xl mx-auto space-y-3">
          <h3
            className="text-sm font-medium"
            style={{ color: "var(--text-secondary)" }}
          >
            Recent Research Sessions
          </h3>
          {recentSessions.map((session) => (
            <Card
              key={session.id}
              data-testid="session-card"
              className="cursor-pointer transition-all duration-180 hover:-translate-y-px"
              style={{
                background: "rgba(255,255,255,0.04)",
                backdropFilter: "blur(20px)",
                WebkitBackdropFilter: "blur(20px)",
                border: "1px solid rgba(255,255,255,0.08)",
                boxShadow: "0 1px 3px rgba(0,0,0,0.12)",
              }}
            >
              <CardContent className="p-4">
                <div className="flex items-center justify-between mb-2">
                  <div className="flex items-center gap-2">
                    <Search
                      className="w-4 h-4"
                      style={{ color: "var(--text-muted)" }}
                    />
                    <span
                      className="text-sm font-medium truncate"
                      style={{ color: "var(--text-primary)" }}
                    >
                      {session.question}
                    </span>
                  </div>
                  <Badge
                    variant={
                      session.status === "complete" ? "default" : "secondary"
                    }
                    className={
                      session.status === "complete"
                        ? "bg-emerald-500/10 text-emerald-400"
                        : ""
                    }
                  >
                    {session.status === "complete" ? "Complete" : session.status}
                  </Badge>
                </div>
                <div
                  className="text-xs flex items-center gap-2"
                  style={{ color: "var(--text-muted)" }}
                >
                  <span>{session.preset}</span>
                  <span>·</span>
                  <span>{session.iterations} iterations</span>
                  <span>·</span>
                  <span>{session.duration}</span>
                  <span>·</span>
                  <span>{session.date}</span>
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      )}
    </div>
  );
}

/**
 * MethodologiesPanel - Methodology management with activation
 */
function MethodologiesPanel({
  methodologies,
  onActivate,
  onDeactivate,
}: {
  methodologies: MethodologyExtension[];
  onActivate: (id: string) => void;
  onDeactivate: (id: string) => void;
}) {
  const isEmpty = methodologies.length === 0;

  return (
    <div data-testid="methodologies-panel" className="space-y-4">
      {/* Header */}
      <div>
        <h2
          className="text-lg font-semibold"
          style={{
            color: "var(--text-primary)",
            letterSpacing: "-0.02em",
          }}
        >
          Development Methodologies
        </h2>
        <p className="text-sm" style={{ color: "var(--text-secondary)" }}>
          Choose how RalphX organizes work
        </p>
      </div>

      {/* Empty State */}
      {isEmpty ? (
        <div className="flex flex-col items-center justify-center py-16">
          <div
            className="w-16 h-16 rounded-xl flex items-center justify-center mb-4"
            style={{ border: "2px dashed var(--border-subtle)" }}
          >
            <BookOpen
              className="w-8 h-8"
              style={{ color: "var(--text-muted)" }}
            />
          </div>
          <p
            className="text-sm font-medium mb-1"
            style={{ color: "var(--text-secondary)" }}
          >
            No methodologies available
          </p>
          <p className="text-xs" style={{ color: "var(--text-muted)" }}>
            Configure methodologies in the plugin directory
          </p>
        </div>
      ) : (
        /* Methodology Cards */
        <div className="space-y-4">
          {methodologies.map((methodology) => (
            <Card
              key={methodology.id}
              data-testid="methodology-card"
              data-active={methodology.isActive ? "true" : "false"}
              className="cursor-pointer transition-all duration-180 hover:-translate-y-px"
              style={{
                background: methodology.isActive
                  ? "rgba(255,107,53,0.08)"
                  : "rgba(255,255,255,0.04)",
                backdropFilter: "blur(20px)",
                WebkitBackdropFilter: "blur(20px)",
                border: methodology.isActive
                  ? "1px solid rgba(255,107,53,0.25)"
                  : "1px solid rgba(255,255,255,0.08)",
                boxShadow: methodology.isActive
                  ? "0 0 0 1px rgba(255,107,53,0.15), 0 2px 8px rgba(0,0,0,0.15)"
                  : "0 1px 3px rgba(0,0,0,0.12)",
              }}
            >
              <CardContent className="p-5">
                {/* Header Row */}
                <div className="flex items-center justify-between mb-3">
                  <div className="flex items-center gap-3">
                    <div
                      className={`w-2.5 h-2.5 rounded-full ${
                        methodology.isActive ? "animate-pulse" : ""
                      }`}
                      style={{
                        backgroundColor: methodology.isActive
                          ? "var(--accent-primary)"
                          : "var(--border-subtle)",
                        boxShadow: methodology.isActive
                          ? "0 0 0 4px rgba(255, 107, 53, 0.1)"
                          : undefined,
                      }}
                    />
                    <span
                      className="text-base font-semibold"
                      style={{ color: "var(--text-primary)" }}
                    >
                      {methodology.name}
                    </span>
                    {methodology.isActive && (
                      <Badge className="bg-emerald-500/10 text-emerald-400 border-0">
                        ACTIVE
                      </Badge>
                    )}
                  </div>
                  {methodology.isActive ? (
                    <Button
                      data-testid="deactivate-button"
                      variant="ghost"
                      size="sm"
                      onClick={(e) => {
                        e.stopPropagation();
                        onDeactivate(methodology.id);
                      }}
                    >
                      Deactivate
                    </Button>
                  ) : (
                    <Button
                      data-testid="activate-button"
                      size="sm"
                      onClick={(e) => {
                        e.stopPropagation();
                        onActivate(methodology.id);
                      }}
                    >
                      Activate
                    </Button>
                  )}
                </div>

                {/* Description */}
                <p
                  className="text-sm leading-relaxed mb-3"
                  style={{ color: "var(--text-secondary)" }}
                >
                  {methodology.description}
                </p>

                {/* Stats */}
                <div
                  className="text-xs flex items-center gap-3"
                  style={{ color: "var(--text-muted)" }}
                >
                  <span data-testid="phase-count">
                    {methodology.phases.length} phases
                  </span>
                  <span>·</span>
                  <span data-testid="agent-count">
                    {methodology.agentProfiles.length} agents
                  </span>
                  <span>·</span>
                  <span>{methodology.workflow.name} workflow</span>
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      )}
    </div>
  );
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
