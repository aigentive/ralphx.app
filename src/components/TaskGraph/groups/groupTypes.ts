export type GroupNodeKind = "plan" | "tier" | "custom";

export const GROUP_NODE_PREFIX = "group:";

export function getGroupNodeId(kind: GroupNodeKind, id: string): string {
  return `${GROUP_NODE_PREFIX}${kind}:${id}`;
}

export function getPlanGroupNodeId(planArtifactId: string): string {
  return getGroupNodeId("plan", planArtifactId);
}

export function getTierGroupNodeId(tierGroupId: string): string {
  return getGroupNodeId("tier", tierGroupId);
}

export function isGroupNodeId(nodeId: string): boolean {
  return nodeId.startsWith(GROUP_NODE_PREFIX);
}

export function parseGroupNodeId(nodeId: string): { kind: GroupNodeKind; id: string } | null {
  if (!isGroupNodeId(nodeId)) return null;
  const [, kind, ...rest] = nodeId.split(":");
  if (!kind || rest.length === 0) return null;
  if (kind !== "plan" && kind !== "tier" && kind !== "custom") return null;
  return { kind, id: rest.join(":") };
}
