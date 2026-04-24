import type { Project } from "@/types/project";
import type { DesignSystemResponse, DesignSystemSourceResponse } from "@/api/design";

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
  feedbackStatus: "none" | "open" | "in_progress" | "resolved";
  reviewState: DesignReviewState;
  sourceStatus: "current" | "stale";
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
  project: Project,
  response: DesignSystemResponse,
  options: { sources?: DesignSystemSourceResponse[]; conversationId?: string | null } = {},
): DesignSystem {
  const styleguide = mockStyleguideModel(project.id);

  return {
    ...styleguide,
    id: response.id,
    primaryProjectId: response.primaryProjectId,
    name: response.name,
    status: response.status,
    version: response.currentSchemaVersionId ? response.currentSchemaVersionId.slice(0, 8) : "draft",
    sourceCount: options.sources?.length ?? 0,
    updatedAt: response.updatedAt,
    conversationId: options.conversationId ?? null,
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
    feedbackStatus?: "none" | "open" | "in_progress" | "resolved";
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
