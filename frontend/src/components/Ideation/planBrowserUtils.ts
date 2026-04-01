import type { SessionGroupKey } from "@/types/ideation";

export type SessionGroup = "drafts" | "in-progress" | "accepted" | "done" | "archived";

/**
 * Maps UI group keys (hyphenated) to backend API group keys (underscored).
 * PlanBrowser uses hyphenated keys in GROUP_CONFIG; the backend uses underscores.
 */
export const GROUP_KEY_TO_API: Record<SessionGroup, SessionGroupKey> = {
  "drafts": "drafts",
  "in-progress": "in_progress",
  "accepted": "accepted",
  "done": "done",
  "archived": "archived",
};
