import type { InternalStatus } from "@/types/status";

export type BattleStatusGroup =
  | "queue"
  | "execution"
  | "review"
  | "merge"
  | "complete"
  | "failure";

export interface BattleGroupSpec {
  group: BattleStatusGroup;
  color: string;
  speed: number;
  hp: number;
}

const GROUP_SPECS: Record<Exclude<BattleStatusGroup, "complete">, BattleGroupSpec> = {
  queue: { group: "queue", color: "#ff7b72", speed: 18, hp: 1 },
  execution: { group: "execution", color: "#ffa657", speed: 26, hp: 1 },
  review: { group: "review", color: "#d2a8ff", speed: 34, hp: 1 },
  merge: { group: "merge", color: "#3fb950", speed: 22, hp: 2 },
  failure: { group: "failure", color: "#f85149", speed: 44, hp: 1 },
};

export function mapStatusToBattleGroup(status: InternalStatus): BattleStatusGroup {
  if (["backlog", "ready", "blocked"].includes(status)) return "queue";

  if (["executing", "re_executing", "qa_refining", "qa_testing", "qa_passed", "qa_failed", "paused"].includes(status)) {
    return "execution";
  }

  if (["pending_review", "reviewing", "review_passed", "escalated", "revision_needed"].includes(status)) {
    return "review";
  }

  if (["pending_merge", "merging", "merge_incomplete", "merge_conflict"].includes(status)) {
    return "merge";
  }

  if (["approved", "merged"].includes(status)) return "complete";

  return "failure";
}

export function getBattleSpecForStatus(status: InternalStatus): BattleGroupSpec | null {
  const group = mapStatusToBattleGroup(status);
  if (group === "complete") return null;
  return GROUP_SPECS[group];
}

export function isActivelyWorkedStatus(status: InternalStatus): boolean {
  return [
    "executing",
    "re_executing",
    "qa_refining",
    "qa_testing",
    "pending_review",
    "reviewing",
    "merging",
  ].includes(status);
}

export function getThreatWeight(status: InternalStatus): number {
  const group = mapStatusToBattleGroup(status);
  switch (group) {
    case "failure":
      return 5;
    case "merge":
      return 4;
    case "review":
      return 3;
    case "execution":
      return 2;
    case "queue":
      return 1;
    case "complete":
      return 0;
  }
}
