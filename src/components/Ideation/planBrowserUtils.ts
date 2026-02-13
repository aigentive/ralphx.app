import type { IdeationSession } from "@/types/ideation";
import type { SessionProgress } from "@/hooks/useSessionProgress";

export type SessionGroup = "drafts" | "in-progress" | "accepted" | "done" | "archived";

export interface GroupedSessions {
  drafts: IdeationSession[];
  "in-progress": IdeationSession[];
  accepted: IdeationSession[];
  done: IdeationSession[];
  archived: IdeationSession[];
}

function sortByUpdatedAtDesc(sessions: IdeationSession[]): IdeationSession[] {
  return [...sessions].sort(
    (a, b) => new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime()
  );
}

/**
 * Classify sessions into five semantic groups based on session status
 * and task progress data.
 *
 * - active → drafts
 * - archived → archived
 * - accepted → split by progress:
 *   - all tasks terminal + total > 0 → done
 *   - has active tasks → in-progress
 *   - otherwise (all idle, 0 tasks, or no progress data) → accepted
 *
 * Each group is sorted by updatedAt descending.
 */
export function groupSessions(
  sessions: IdeationSession[],
  progressMap: Map<string, SessionProgress>
): GroupedSessions {
  const groups: GroupedSessions = {
    drafts: [],
    "in-progress": [],
    accepted: [],
    done: [],
    archived: [],
  };

  for (const session of sessions) {
    if (session.status === "active") {
      groups.drafts.push(session);
    } else if (session.status === "archived") {
      groups.archived.push(session);
    } else {
      // accepted — classify by progress
      const progress = progressMap.get(session.id);

      if (progress && progress.total > 0 && progress.done === progress.total) {
        groups.done.push(session);
      } else if (progress && progress.active > 0) {
        groups["in-progress"].push(session);
      } else {
        groups.accepted.push(session);
      }
    }
  }

  // Sort each group by updatedAt descending
  groups.drafts = sortByUpdatedAtDesc(groups.drafts);
  groups["in-progress"] = sortByUpdatedAtDesc(groups["in-progress"]);
  groups.accepted = sortByUpdatedAtDesc(groups.accepted);
  groups.done = sortByUpdatedAtDesc(groups.done);
  groups.archived = sortByUpdatedAtDesc(groups.archived);

  return groups;
}
