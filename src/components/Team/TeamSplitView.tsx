/**
 * TeamSplitView — Top-level team split view component
 *
 * Full-height container rendering TeamSplitHeader + TeamSplitGrid.
 * Reads team state from teamStore, sets contextKey on the splitPaneStore.
 */

import React, { useEffect } from "react";
import { useSplitPaneStore } from "@/stores/splitPaneStore";
import { TeamSplitHeader } from "./TeamSplitHeader";
import { TeamSplitGrid } from "./TeamSplitGrid";

interface TeamSplitViewProps {
  contextKey?: string | undefined;
}

export const TeamSplitView = React.memo(function TeamSplitView({
  contextKey: contextKeyProp,
}: TeamSplitViewProps) {
  const storeContextKey = useSplitPaneStore((s) => s.contextKey);
  const setContextKey = useSplitPaneStore((s) => s.setContextKey);

  // Use prop if provided, otherwise fall back to store
  const contextKey = contextKeyProp ?? storeContextKey ?? "";

  // Sync context key from prop to store
  useEffect(() => {
    if (contextKeyProp && contextKeyProp !== storeContextKey) {
      setContextKey(contextKeyProp);
    }
  }, [contextKeyProp, storeContextKey, setContextKey]);

  if (!contextKey) {
    return (
      <div
        className="flex items-center justify-center h-full"
        style={{ backgroundColor: "hsl(220 10% 6%)" }}
      >
        <span className="text-[13px]" style={{ color: "hsl(220 10% 40%)" }}>
          No active team
        </span>
      </div>
    );
  }

  return (
    <div
      className="flex flex-col h-full"
      style={{ backgroundColor: "hsl(220 10% 6%)" }}
    >
      <TeamSplitHeader contextKey={contextKey} />
      <TeamSplitGrid />
    </div>
  );
});
