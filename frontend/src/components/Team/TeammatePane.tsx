/**
 * TeammatePane — Self-contained teammate view
 *
 * Full pane for a single teammate with:
 * - PaneHeader at top
 * - PaneStream in the middle (scrollable)
 * - PaneInput at bottom
 *
 * Focused state shows orange border, unfocused shows dim border.
 * Click to focus (updates splitPaneStore.focusedPane).
 */

import React, { useMemo, useCallback } from "react";
import { useTeamStore, selectTeammateByName } from "@/stores/teamStore";
import { useSplitPaneStore, selectFocusedPane } from "@/stores/splitPaneStore";
import { PaneHeader } from "./PaneHeader";
import { PaneStream } from "./PaneStream";
import { PaneInput } from "./PaneInput";

interface TeammatePaneProps {
  contextKey: string;
  teammateName: string;
  onStop?: ((name: string) => void) | undefined;
  onSendMessage?: ((name: string, content: string) => void) | undefined;
}

export const TeammatePane = React.memo(function TeammatePane({
  contextKey,
  teammateName,
  onStop,
  onSendMessage,
}: TeammatePaneProps) {
  const mateSelector = useMemo(
    () => selectTeammateByName(contextKey, teammateName),
    [contextKey, teammateName],
  );
  const mate = useTeamStore(mateSelector);
  const focusedPane = useSplitPaneStore(selectFocusedPane);
  const setFocusedPane = useSplitPaneStore((s) => s.setFocusedPane);

  const isFocused = focusedPane === teammateName;

  const handleClick = useCallback(() => {
    setFocusedPane(teammateName);
  }, [setFocusedPane, teammateName]);

  const handleStop = useCallback(() => {
    onStop?.(teammateName);
  }, [onStop, teammateName]);

  const handleSend = useCallback(
    (content: string) => {
      onSendMessage?.(teammateName, content);
    },
    [onSendMessage, teammateName],
  );

  if (!mate) return null;

  return (
    <div
      className="flex flex-col h-full cursor-pointer"
      onClick={handleClick}
      role="button"
      tabIndex={0}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") handleClick();
      }}
      style={{
        backgroundColor: isFocused ? "var(--bg-surface)" : "var(--bg-base)",
        border: isFocused
          ? "1px solid var(--accent-primary)"
          : "1px solid var(--border-subtle)",
      }}
    >
      <PaneHeader
        name={mate.name}
        color={mate.color}
        model={mate.model}
        status={mate.status}
        roleDescription={mate.roleDescription}
        onStop={onStop ? handleStop : undefined}
      />

      <PaneStream
        contextKey={contextKey}
        teammateName={teammateName}
      />

      <PaneInput
        teammateName={teammateName}
        status={mate.status}
        onSend={handleSend}
      />
    </div>
  );
});
