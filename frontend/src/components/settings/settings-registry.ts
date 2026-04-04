export type SettingsSectionId =
  | "execution"
  | "global-execution"
  | "model"
  | "review"
  | "supervisor"
  | "git"
  | "github"
  | "project-analysis"
  | "ideation-workflow"
  | "ideation-effort"
  | "ideation-models"
  | "api-keys";

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
  { id: "global-execution", groupId: "general", label: "Global Execution" },
  { id: "model", groupId: "general", label: "Model" },
  { id: "review", groupId: "general", label: "Review" },
  { id: "supervisor", groupId: "general", label: "Supervisor" },
  { id: "git", groupId: "workspace", label: "Git" },
  { id: "github", groupId: "workspace", label: "GitHub" },
  { id: "project-analysis", groupId: "workspace", label: "Project Analysis" },
  { id: "ideation-workflow", groupId: "ideation", label: "Workflow" },
  { id: "ideation-effort", groupId: "ideation", label: "Effort" },
  { id: "ideation-models", groupId: "ideation", label: "Models" },
  { id: "api-keys", groupId: "access", label: "API Keys" },
];
