export type SettingsSectionId =
  | "execution"
  | "execution-harnesses"
  | "global-execution"
  | "review"
  | "repository"
  | "project-analysis"
  | "ideation-workflow"
  | "ideation-harnesses"
  | "api-keys"
  | "external-mcp";

export type SettingsGroupId = "general" | "workspace" | "ideation" | "access";

export interface SettingsSectionMeta {
  id: SettingsSectionId;
  label: string;
  groupId: SettingsGroupId;
}

export const SETTINGS_GROUPS: { id: SettingsGroupId; label: string }[] = [
  { id: "general", label: "General" },
  { id: "workspace", label: "Workspace" },
  { id: "ideation", label: "Ideation" },
  { id: "access", label: "Access" },
];

export const SETTINGS_SECTIONS: SettingsSectionMeta[] = [
  { id: "execution", groupId: "general", label: "Execution" },
  { id: "execution-harnesses", groupId: "general", label: "Execution Agents" },
  { id: "global-execution", groupId: "general", label: "Global Capacity" },
  { id: "review", groupId: "general", label: "Review Policy" },
  { id: "repository", groupId: "workspace", label: "Repository" },
  { id: "project-analysis", groupId: "workspace", label: "Setup & Validation" },
  { id: "ideation-workflow", groupId: "ideation", label: "Planning & Verification" },
  { id: "ideation-harnesses", groupId: "ideation", label: "Ideation Agents" },
  { id: "api-keys", groupId: "access", label: "API Keys" },
  { id: "external-mcp", groupId: "access", label: "External MCP" },
];
