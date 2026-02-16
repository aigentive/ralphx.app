/**
 * TeamActivityPanel — Live teammate status panel (collapsible)
 *
 * Collapsed (default): compact summary bar with teammate count, status, cost.
 * Expanded: full view with teammate cards, messages, cost breakdown.
 */

import React, { useState, useMemo, useCallback } from "react";
import { Square, ChevronDown } from "lucide-react";
import { AnimatePresence, motion } from "framer-motion";
import { Button } from "@/components/ui/button";
import { useTeamStore, selectActiveTeam, selectTeammates, selectTeamMessages } from "@/stores/teamStore";
import { TeammateCard } from "./TeammateCard";
import { TeamMessageBubble } from "./TeamMessageBubble";
import { TeamCostDisplay } from "./TeamCostDisplay";
import type { TeammateState } from "@/stores/teamStore";

interface TeamActivityPanelProps {
  contextKey: string;
  onMessageTeammate?: ((name: string) => void) | undefined;
  onStopTeammate?: ((name: string) => void) | undefined;
  onStopAll?: (() => void) | undefined;
  isHistorical?: boolean | undefined;
}

const MAX_RECENT_MESSAGES = 5;

function formatSummaryCost(usd: number): string {
  return usd < 0.01 ? "<$0.01" : `$${usd.toFixed(2)}`;
}

export const TeamActivityPanel = React.memo(function TeamActivityPanel({
  contextKey,
  onMessageTeammate,
  onStopTeammate,
  onStopAll,
  isHistorical = false,
}: TeamActivityPanelProps) {
  const [isCollapsed, setIsCollapsed] = useState(true);

  const teamSelector = useMemo(() => selectActiveTeam(contextKey), [contextKey]);
  const team = useTeamStore(teamSelector);
  const teammatesSelector = useMemo(() => selectTeammates(contextKey), [contextKey]);
  const teammates = useTeamStore(teammatesSelector);
  const messagesSelector = useMemo(() => selectTeamMessages(contextKey), [contextKey]);
  const messages = useTeamStore(messagesSelector);

  const activeCount = useMemo(
    () => teammates.filter((m: TeammateState) => m.status !== "shutdown").length,
    [teammates]
  );

  const runningCount = useMemo(
    () => teammates.filter((m: TeammateState) => m.status === "running" || m.status === "spawning").length,
    [teammates]
  );

  const recentMessages = useMemo(
    () => messages.slice(-MAX_RECENT_MESSAGES),
    [messages]
  );

  // Build a color lookup for message display
  const colorMap = useMemo(() => {
    const map = new Map<string, string>();
    teammates.forEach((m: TeammateState) => map.set(m.name, m.color));
    return map;
  }, [teammates]);

  const handleMessage = useCallback((name: string) => {
    onMessageTeammate?.(name);
  }, [onMessageTeammate]);

  const handleStop = useCallback((name: string) => {
    onStopTeammate?.(name);
  }, [onStopTeammate]);

  const toggleCollapsed = useCallback(() => {
    setIsCollapsed((prev) => !prev);
  }, []);

  if (!team) return null;

  return (
    <div
      className="flex flex-col overflow-hidden"
      style={{
        backgroundColor: "hsl(220 10% 9%)",
        borderTop: "1px solid hsl(220 10% 14%)",
      }}
    >
      {/* Header — always visible, clickable to toggle */}
      <button
        type="button"
        onClick={toggleCollapsed}
        className="flex items-center justify-between px-3 py-2 shrink-0 w-full text-left cursor-pointer hover:brightness-110 transition-[filter] duration-150"
        style={{ background: "transparent" }}
      >
        <div className="flex items-center gap-2">
          <span className="text-[11px] font-medium uppercase tracking-wide" style={{ color: "hsl(220 10% 45%)" }}>
            Team Activity
          </span>
          <span
            className="text-[10px] px-1.5 rounded"
            style={{
              backgroundColor: "hsl(220 10% 14%)",
              color: "hsl(220 10% 55%)",
            }}
          >
            {activeCount}/{teammates.length}
          </span>
          {isHistorical && (
            <span
              className="text-[10px] px-1.5 py-0.5 rounded"
              style={{
                backgroundColor: "hsl(220 10% 16%)",
                color: "hsl(220 10% 50%)",
              }}
            >
              Session ended
            </span>
          )}
          {/* Collapsed summary stats */}
          {isCollapsed && (
            <>
              <span className="text-[10px]" style={{ color: "hsl(220 10% 40%)" }}>
                •
              </span>
              <span className="text-[10px]" style={{ color: "hsl(220 10% 50%)" }}>
                {runningCount} Running
              </span>
              <span className="text-[10px]" style={{ color: "hsl(220 10% 40%)" }}>
                •
              </span>
              <span className="text-[10px]" style={{ color: "hsl(220 10% 50%)" }}>
                {formatSummaryCost(team.totalEstimatedCostUsd)}
              </span>
            </>
          )}
        </div>
        <motion.span
          animate={{ rotate: isCollapsed ? 0 : 180 }}
          transition={{ duration: 0.2 }}
          className="flex items-center"
        >
          <ChevronDown className="w-3.5 h-3.5" style={{ color: "hsl(220 10% 40%)" }} />
        </motion.span>
      </button>

      {/* Expandable content */}
      <AnimatePresence initial={false}>
        {!isCollapsed && (
          <motion.div
            key="team-expanded"
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ duration: 0.2, ease: "easeInOut" }}
            className="overflow-hidden"
          >
            {/* Teammate cards */}
            <div className="overflow-y-auto px-3 pb-2 space-y-2 max-h-[240px]">
              {teammates.map((mate: TeammateState) => (
                <TeammateCard
                  key={mate.name}
                  teammate={mate}
                  onMessage={!isHistorical && onMessageTeammate ? handleMessage : undefined}
                  onStop={!isHistorical && onStopTeammate ? handleStop : undefined}
                />
              ))}
            </div>

            {/* Recent messages */}
            {recentMessages.length > 0 && (
              <div className="px-3 pb-2">
                <div className="text-[10px] uppercase tracking-wide mb-1" style={{ color: "hsl(220 10% 40%)" }}>
                  Recent Messages ({messages.length})
                </div>
                <div className="space-y-1">
                  {recentMessages.map((msg) => (
                    <TeamMessageBubble
                      key={msg.id}
                      from={msg.from}
                      to={msg.to}
                      content={msg.content}
                      fromColor={colorMap.get(msg.from)}
                      timestamp={msg.timestamp}
                    />
                  ))}
                </div>
              </div>
            )}

            {/* Cost display */}
            <TeamCostDisplay
              totalTokens={team.totalTokens}
              totalEstimatedCostUsd={team.totalEstimatedCostUsd}
              teammates={teammates}
            />
          </motion.div>
        )}
      </AnimatePresence>

      {/* Actions — always visible */}
      {!isHistorical && onStopAll && activeCount > 0 && (
        <div className="flex items-center gap-2 px-3 py-2 shrink-0" style={{ borderTop: "1px solid hsl(220 10% 14%)" }}>
          <Button
            variant="ghost"
            size="sm"
            onClick={onStopAll}
            className="text-[11px] h-7 gap-1.5"
          >
            <Square className="w-3 h-3" />
            Stop All
          </Button>
        </div>
      )}
    </div>
  );
});
