import type { Project } from "@/types/project";
import type {
  DesignPersistedStyleguideItem,
  DesignStyleguideItemResponse,
  DesignStyleguideViewModelResponse,
  DesignSystemResponse,
  DesignSystemSourceResponse,
} from "@/api/design";

export type DesignSystemStatus =
  | "draft"
  | "analyzing"
  | "schema_ready"
  | "ready"
  | "updating"
  | "failed"
  | "archived";

export type DesignStyleguideGroupId =
  | "ui_kit"
  | "type"
  | "colors"
  | "spacing"
  | "components"
  | "brand";

export type DesignReviewState = "needs_review" | "approved" | "needs_work" | "stale";

export interface DesignSourceRef {
  projectId: string;
  path: string;
  line?: number;
}

export interface DesignStyleguideItem {
  id: string;
  itemId: string;
  group: DesignStyleguideGroupId;
  label: string;
  summary: string;
  previewArtifactId?: string;
  sourceRefs: DesignSourceRef[];
  confidence: "high" | "medium" | "low";
  approvalStatus: "needs_review" | "approved" | "needs_work";
  feedbackStatus: "none" | "open" | "in_progress" | "resolved" | "dismissed";
  reviewState: DesignReviewState;
  sourceStatus: "current" | "stale";
  isPersisted?: boolean;
}

export interface DesignStyleguideGroup {
  id: DesignStyleguideGroupId;
  label: string;
  items: DesignStyleguideItem[];
}

export interface DesignCaveat {
  id: string;
  severity: "info" | "warning" | "blocker";
  title: string;
  body: string;
  actionLabel?: string;
}

export interface DesignSystem {
  id: string;
  primaryProjectId: string;
  name: string;
  status: DesignSystemStatus;
  version: string;
  sourceCount: number;
  updatedAt: string;
  conversationId?: string | null;
  readySummary: string;
  caveats: DesignCaveat[];
  groups: DesignStyleguideGroup[];
}

export function buildMockDesignSystems(projects: Project[]): DesignSystem[] {
  return projects.map((project) => mockDesignSystem(project));
}

export function buildDesignSystemFromResponse(
  _project: Project,
  response: DesignSystemResponse,
  options: {
    sources?: DesignSystemSourceResponse[];
    conversationId?: string | null;
    styleguideItems?: DesignStyleguideItemResponse[];
    styleguideViewModel?: DesignStyleguideViewModelResponse | null;
  } = {},
): DesignSystem {
  const persistedViewModel = options.styleguideViewModel?.content;
  const groups = options.styleguideItems?.length
    ? buildStyleguideGroupsFromResponses(options.styleguideItems)
    : persistedViewModel?.groups.length
      ? buildStyleguideGroupsFromViewModel(persistedViewModel.groups)
    : [];
  const emptyStyleguide = emptyStyleguideModel(response);

  return {
    ...emptyStyleguide,
    id: response.id,
    primaryProjectId: response.primaryProjectId,
    name: response.name,
    status: response.status,
    version: response.currentSchemaVersionId ? response.currentSchemaVersionId.slice(0, 8) : "draft",
    sourceCount: response.sourceCount ?? options.sources?.length ?? 0,
    updatedAt: response.updatedAt,
    conversationId: options.conversationId ?? null,
    readySummary: persistedViewModel?.ready_summary ?? emptyStyleguide.readySummary,
    caveats: persistedViewModel?.caveats.length
      ? caveatsFromViewModel(persistedViewModel.caveats, groups)
      : emptyStyleguide.caveats,
    groups,
  };
}

function mockDesignSystem(project: Project): DesignSystem {
  return {
    id: `design-system-${project.id}`,
    primaryProjectId: project.id,
    name: `${project.name} Design System`,
    status: "ready",
    version: "0.1.0",
    sourceCount: 3,
    updatedAt: project.updatedAt,
    ...mockStyleguideModel(project.id),
  };
}

function mockStyleguideModel(projectId: string): Pick<DesignSystem, "readySummary" | "caveats" | "groups"> {
  return {
    readySummary: "Source-backed colors, type, components, and workspace layout patterns are ready for review.",
    caveats: [
      {
        id: "font-substitution",
        severity: "warning",
        title: "Missing brand font files",
        body: "System fonts are used until brand font assets are attached.",
        actionLabel: "Upload fonts",
      },
    ],
    groups: [
      {
        id: "colors",
        label: "Colors",
        items: [
          styleguideItem(projectId, {
            id: "primary-palette",
            itemId: "colors.primary_palette",
            group: "colors",
            label: "Primary palette",
            summary: "RalphX orange primary, hover, soft, and ring roles.",
            sourcePath: "specs/design/styleguide.md",
            confidence: "high",
          }),
        ],
      },
      {
        id: "components",
        label: "Components",
        items: [
          styleguideItem(projectId, {
            id: "buttons",
            itemId: "components.buttons",
            group: "components",
            label: "Buttons",
            summary: "Primary, secondary, ghost, icon, and loading button patterns.",
            sourcePath: "frontend/src/components/ui/button.tsx",
            confidence: "medium",
            approvalStatus: "needs_work",
            feedbackStatus: "open",
            reviewState: "needs_work",
          }),
          styleguideItem(projectId, {
            id: "composer",
            itemId: "components.composer",
            group: "components",
            label: "Composer",
            summary: "Compact chat composer with source-scoped actions.",
            sourcePath: "frontend/src/components/Chat/IntegratedChatPanel.tsx",
            confidence: "medium",
          }),
        ],
      },
      {
        id: "spacing",
        label: "Spacing",
        items: [
          styleguideItem(projectId, {
            id: "radii",
            itemId: "spacing.radii",
            group: "spacing",
            label: "Radii",
            summary: "8px controls with compact row and panel spacing.",
            sourcePath: "AGENTS.md",
            confidence: "medium",
          }),
        ],
      },
    ],
  };
}

function emptyStyleguideModel(
  response: DesignSystemResponse,
): Pick<DesignSystem, "readySummary" | "caveats" | "groups"> {
  const hasPublishedSchema = Boolean(response.currentSchemaVersionId);
  return {
    readySummary: hasPublishedSchema
      ? `${response.name} has no loaded styleguide rows yet.`
      : `${response.name} has no generated styleguide rows yet.`,
    caveats: [],
    groups: [],
  };
}

const GROUP_LABELS: Record<DesignStyleguideGroupId, string> = {
  ui_kit: "UI Kit",
  type: "Type",
  colors: "Colors",
  spacing: "Spacing",
  components: "Components",
  brand: "Brand",
};

function buildStyleguideGroupsFromResponses(
  responses: DesignStyleguideItemResponse[],
): DesignStyleguideGroup[] {
  const groups = new Map<DesignStyleguideGroupId, DesignStyleguideItem[]>();
  for (const response of responses) {
    const item = styleguideItemFromResponse(response);
    groups.set(item.group, [...(groups.get(item.group) ?? []), item]);
  }

  return Array.from(groups.entries()).map(([id, items]) => ({
    id,
    label: GROUP_LABELS[id],
    items,
  }));
}

function buildStyleguideGroupsFromViewModel(
  groups: DesignStyleguideViewModelResponse["content"]["groups"],
): DesignStyleguideGroup[] {
  return groups.map((group) => ({
    id: group.id,
    label: group.label || GROUP_LABELS[group.id],
    items: group.items.map(styleguideItemFromViewModel),
  }));
}

function styleguideItemFromResponse(response: DesignStyleguideItemResponse): DesignStyleguideItem {
  return {
    id: response.id,
    itemId: response.itemId,
    group: response.group,
    label: response.label,
    summary: response.summary,
    ...(response.previewArtifactId ? { previewArtifactId: response.previewArtifactId } : {}),
    sourceRefs: response.sourceRefs.map(sourceRefFromResponse),
    confidence: response.confidence,
    approvalStatus: response.approvalStatus,
    feedbackStatus: response.feedbackStatus,
    reviewState: reviewStateForResponse(response),
    sourceStatus: "current",
    isPersisted: true,
  };
}

function styleguideItemFromViewModel(item: DesignPersistedStyleguideItem): DesignStyleguideItem {
  const approvalStatus = item.approval_status ?? "needs_review";
  const feedbackStatus = item.feedback_status ?? "none";
  return {
    id: item.id,
    itemId: item.id,
    group: item.group,
    label: item.label,
    summary: item.summary,
    ...(item.preview_artifact_id ? { previewArtifactId: item.preview_artifact_id } : {}),
    sourceRefs: item.source_refs.map(sourceRefFromResponse),
    confidence: item.confidence ?? "medium",
    approvalStatus,
    feedbackStatus,
    reviewState: reviewStateForStatus(approvalStatus, feedbackStatus),
    sourceStatus: "current",
    isPersisted: true,
  };
}

function reviewStateForResponse(response: DesignStyleguideItemResponse): DesignReviewState {
  return reviewStateForStatus(response.approvalStatus, response.feedbackStatus);
}

function reviewStateForStatus(
  approvalStatus: DesignStyleguideItem["approvalStatus"],
  feedbackStatus: DesignStyleguideItem["feedbackStatus"],
): DesignReviewState {
  if (feedbackStatus === "open" || feedbackStatus === "in_progress") {
    return "needs_work";
  }
  return approvalStatus;
}

function sourceRefFromResponse(sourceRef: {
  project_id: string;
  path: string;
  line?: number | null | undefined;
}): DesignSourceRef {
  return {
    projectId: sourceRef.project_id,
    path: sourceRef.path,
    ...(sourceRef.line ? { line: sourceRef.line } : {}),
  };
}

const FALLBACK_SOURCE_CAVEAT_SUMMARY =
  "Only fallback source references matched this row; review before treating it as canonical.";

function caveatsFromViewModel(
  caveats: DesignStyleguideViewModelResponse["content"]["caveats"],
  groups: DesignStyleguideGroup[],
): DesignCaveat[] {
  return caveats.map((caveat, index) => ({
    id: caveat.id ?? caveat.item_id ?? `caveat-${index}`,
    severity: caveatSeverity(caveat.severity),
    title: caveat.title ?? caveatTitle(caveat.item_id, groups),
    body: caveat.body ?? caveatBody(caveat.summary, caveat.item_id, groups),
  }));
}

function caveatTitle(
  itemId: string | undefined,
  groups: DesignStyleguideGroup[],
): string {
  const item = findStyleguideItem(itemId, groups);
  return item ? `Source review needed: ${item.label}` : "Review caveat";
}

function caveatBody(
  summary: string | undefined,
  itemId: string | undefined,
  groups: DesignStyleguideGroup[],
): string {
  if (summary !== FALLBACK_SOURCE_CAVEAT_SUMMARY) {
    return summary ?? "";
  }

  const item = findStyleguideItem(itemId, groups);
  const label = item?.label ?? "This row";
  return `${label} used fallback source references because no direct source match was found in the selected paths. Review its sources before approving it.`;
}

function findStyleguideItem(
  itemId: string | undefined,
  groups: DesignStyleguideGroup[],
): DesignStyleguideItem | null {
  if (!itemId) {
    return null;
  }
  return groups
    .flatMap((group) => group.items)
    .find((item) => item.itemId === itemId || item.id === itemId) ?? null;
}

function caveatSeverity(value: string | undefined): DesignCaveat["severity"] {
  if (value === "blocker" || value === "warning" || value === "info") {
    return value;
  }
  return "warning";
}

function styleguideItem(
  projectId: string,
  input: {
    id: string;
    itemId: string;
    group: DesignStyleguideGroupId;
    label: string;
    summary: string;
    sourcePath: string;
    confidence: "high" | "medium" | "low";
    approvalStatus?: "needs_review" | "approved" | "needs_work";
    feedbackStatus?: "none" | "open" | "in_progress" | "resolved" | "dismissed";
    reviewState?: DesignReviewState;
  },
): DesignStyleguideItem {
  return {
    id: input.id,
    itemId: input.itemId,
    group: input.group,
    label: input.label,
    summary: input.summary,
    previewArtifactId: `design-preview-${input.id}`,
    sourceRefs: [{ projectId, path: input.sourcePath }],
    confidence: input.confidence,
    approvalStatus: input.approvalStatus ?? "needs_review",
    feedbackStatus: input.feedbackStatus ?? "none",
    reviewState: input.reviewState ?? "needs_review",
    sourceStatus: "current",
  };
}
