/* eslint-disable react-refresh/only-export-components */
/**
 * TierGroup.tsx - Visual region component for dependency tiers inside plan groups
 */

import { memo, useCallback } from "react";
import type { NodeProps, Node } from "@xyflow/react";
import { cn } from "@/lib/utils";
import { TierGroupHeader } from "./TierGroupHeader";

export const TIER_HEADER_HEIGHT = 36;

export interface TierGroupData extends Record<string, unknown> {
  tierGroupId: string;
  planArtifactId: string;
  tier: number;
  taskIds: string[];
  isCollapsed: boolean;
  width: number;
  height: number;
  onToggleCollapse?: (tierGroupId: string) => void;
}

export type TierGroupNode = Node<TierGroupData, "tierGroup">;

export interface TierGroupProps extends NodeProps<TierGroupNode> {
  onToggleCollapse?: (tierGroupId: string) => void;
}

export const TierGroup = memo(function TierGroup({ data, selected }: TierGroupProps) {
  const {
    tierGroupId,
    tier,
    taskIds,
    isCollapsed,
    width,
    height,
    onToggleCollapse,
  } = data;

  const displayHeight = isCollapsed ? TIER_HEADER_HEIGHT + 8 : height;

  const handleDoubleClick = useCallback(
    (event: React.MouseEvent) => {
      event.stopPropagation();
      onToggleCollapse?.(tierGroupId);
    },
    [onToggleCollapse, tierGroupId]
  );

  return (
    <div
      className={cn(
        "rounded-md overflow-hidden",
        "bg-[hsla(220_10%_18%_/_0.45)]",
        selected && "ring-1 ring-[hsl(var(--accent-primary)/0.35)]",
        "transition-all duration-200"
      )}
      style={{
        width,
        height: displayHeight,
      }}
      onDoubleClick={handleDoubleClick}
      data-testid={`tier-group-${tierGroupId}`}
    >
      <TierGroupHeader
        tier={tier}
        taskCount={taskIds.length}
        isCollapsed={isCollapsed}
        onToggleCollapse={() => onToggleCollapse?.(tierGroupId)}
      />
      {!isCollapsed && (
        <div
          className="relative"
          style={{
            height: displayHeight - TIER_HEADER_HEIGHT,
          }}
        />
      )}
    </div>
  );
});

export function createTierGroupNode(
  tierGroupId: string,
  planArtifactId: string,
  tier: number,
  taskIds: string[],
  position: { x: number; y: number },
  width: number,
  height: number,
  isCollapsed = false,
  onToggleCollapse?: (tierGroupId: string) => void
): TierGroupNode {
  return {
    id: tierGroupId,
    type: "tierGroup",
    position,
    data: {
      tierGroupId,
      planArtifactId,
      tier,
      taskIds,
      isCollapsed,
      width,
      height,
      ...(onToggleCollapse && { onToggleCollapse }),
    },
    style: {
      width,
      height: isCollapsed ? TIER_HEADER_HEIGHT + 8 : height,
    },
    draggable: false,
    selectable: true,
    zIndex: -1,
  };
}

export const TIER_GROUP_NODE_TYPE = "tierGroup";
