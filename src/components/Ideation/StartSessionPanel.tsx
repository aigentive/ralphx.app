/**
 * StartSessionPanel - macOS Tahoe styled welcome screen
 *
 * Design: Elegant empty state with subtle radial gradient,
 * refined typography, and smooth interactions.
 */

import { useEffect, useState } from "react";
import { Lightbulb, Zap, FileText, Loader2, Upload } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { TaskPickerDialog } from "./TaskPickerDialog";
import { TeamConfigPanel } from "./TeamConfigPanel";
import { useCreateIdeationSession } from "@/hooks/useIdeation";
import { useSessionExportImport } from "@/hooks/useSessionExportImport";
import { useIdeationStore } from "@/stores/ideationStore";
import { useProjectStore } from "@/stores/projectStore";
import type { Task } from "@/types/task";
import type { TeamMode, TeamConfig } from "@/types/ideation";

interface StartSessionPanelProps {
  onNewSession: () => void;
}

const TEAM_MODES: { value: TeamMode; label: string; recommended?: boolean }[] = [
  { value: "solo", label: "Solo" },
  { value: "research", label: "Research Team", recommended: true },
  { value: "debate", label: "Debate Team", recommended: true },
];

const DEFAULT_TEAM_CONFIG: TeamConfig = {
  maxTeammates: 5,
  modelCeiling: "sonnet",
  compositionMode: "dynamic",
};

export function StartSessionPanel({ onNewSession }: StartSessionPanelProps) {
  const [showTaskPicker, setShowTaskPicker] = useState(false);
  const [isCreatingFromTask, setIsCreatingFromTask] = useState(false);
  const [teamMode, setTeamMode] = useState<TeamMode>("solo");
  const [teamConfig, setTeamConfig] = useState<TeamConfig>(DEFAULT_TEAM_CONFIG);
  const [isCreatingTeamSession, setIsCreatingTeamSession] = useState(false);

  const createSession = useCreateIdeationSession();
  const addSession = useIdeationStore((state) => state.addSession);
  const setActiveSession = useIdeationStore((state) => state.setActiveSession);
  const activeProjectId = useProjectStore((state) => state.activeProjectId);
  const { importSession, isImporting } = useSessionExportImport();

  const isTeamMode = teamMode !== "solo";

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const activeElement = document.activeElement;
      if (
        activeElement instanceof HTMLInputElement ||
        activeElement instanceof HTMLTextAreaElement
      ) {
        return;
      }

      if (e.metaKey || e.ctrlKey) {
        if (e.key === "n" || e.key === "N") {
          e.preventDefault();
          onNewSession();
        }
        if (e.key === "d" || e.key === "D") {
          e.preventDefault();
          setShowTaskPicker(true);
        }
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onNewSession]);

  const handleStartSession = async () => {
    if (!activeProjectId) {
      toast.error("No active project selected");
      return;
    }

    setIsCreatingTeamSession(true);
    try {
      const params: Parameters<typeof createSession.mutateAsync>[0] = {
        projectId: activeProjectId,
      };
      if (isTeamMode) {
        params.teamMode = teamMode;
        params.teamConfig = teamConfig;
      }
      const session = await createSession.mutateAsync(params);
      addSession(session);
      setActiveSession(session.id);
    } catch {
      toast.error("Failed to create session");
    } finally {
      setIsCreatingTeamSession(false);
    }
  };

  const handleSeedFromTask = async (task: Task) => {
    setIsCreatingFromTask(true);
    try {
      const params: Parameters<typeof createSession.mutateAsync>[0] = {
        projectId: task.projectId,
        title: `Ideation: ${task.title}`,
        seedTaskId: task.id,
      };
      if (isTeamMode) {
        params.teamMode = teamMode;
        params.teamConfig = teamConfig;
      }
      const session = await createSession.mutateAsync(params);
      addSession(session);
      setActiveSession(session.id);
    } catch {
      toast.error("Failed to start ideation session");
    } finally {
      setIsCreatingFromTask(false);
    }
  };

  const isCreating = isCreatingTeamSession || createSession.isPending;

  return (
    <>
      <div
        className="flex-1 flex flex-col items-center justify-center p-8 relative overflow-hidden"
        style={{ background: "hsl(220 10% 8%)" }}
      >
        {/* Subtle grid pattern */}
        <div
          className="absolute inset-0 opacity-[0.015]"
          style={{
            backgroundImage: `
              linear-gradient(hsla(220 10% 100% / 0.5) 1px, transparent 1px),
              linear-gradient(90deg, hsla(220 10% 100% / 0.5) 1px, transparent 1px)
            `,
            backgroundSize: "48px 48px",
          }}
        />

        <div className="relative z-10 text-center max-w-md">
          {/* Icon */}
          <div className="relative mb-8">
            <div
              className="relative w-20 h-20 rounded-[22px] flex items-center justify-center mx-auto"
              style={{
                background: "hsla(14 100% 60% / 0.12)",
                border: "1px solid hsla(14 100% 60% / 0.25)",
              }}
            >
              <Lightbulb className="w-9 h-9" style={{ color: "hsl(14 100% 60%)" }} strokeWidth={1.5} />
            </div>
          </div>

          {/* Content */}
          <h1
            className="text-xl font-semibold tracking-[-0.02em] mb-2"
            style={{ color: "hsl(220 10% 90%)" }}
          >
            Ideation Studio
          </h1>
          <p
            className="text-[14px] leading-relaxed mb-6 max-w-xs mx-auto"
            style={{ color: "hsl(220 10% 60%)" }}
          >
            Select a session from the sidebar or start a new brainstorming session.
          </p>

          {/* Team Mode Selector */}
          <div className="mb-6">
            <p
              className="text-[12px] font-medium tracking-wide uppercase mb-3"
              style={{ color: "hsl(220 10% 50%)" }}
            >
              Ideation Mode
            </p>
            <div className="flex items-center justify-center gap-2">
              {TEAM_MODES.map((mode) => {
                const isSelected = teamMode === mode.value;
                return (
                  <button
                    key={mode.value}
                    onClick={() => setTeamMode(mode.value)}
                    className="px-4 py-2.5 rounded-xl text-[13px] font-medium transition-all duration-150"
                    style={{
                      background: isSelected ? "hsla(14 100% 60% / 0.15)" : "hsla(220 10% 100% / 0.03)",
                      border: `1px solid ${isSelected ? "hsl(14 100% 60%)" : "hsla(220 10% 100% / 0.08)"}`,
                      color: isSelected ? "hsl(14 100% 60%)" : "hsl(220 10% 60%)",
                    }}
                    onMouseEnter={(e) => {
                      if (!isSelected) {
                        e.currentTarget.style.borderColor = "hsla(220 10% 100% / 0.15)";
                        e.currentTarget.style.color = "hsl(220 10% 80%)";
                      }
                    }}
                    onMouseLeave={(e) => {
                      if (!isSelected) {
                        e.currentTarget.style.borderColor = "hsla(220 10% 100% / 0.08)";
                        e.currentTarget.style.color = "hsl(220 10% 60%)";
                      }
                    }}
                  >
                    {mode.recommended && "\u2605 "}
                    {mode.label}
                  </button>
                );
              })}
            </div>
          </div>

          {/* Team Config Panel (animated) */}
          <div
            className="overflow-hidden transition-all duration-200 ease-out"
            style={{
              maxHeight: isTeamMode ? "280px" : "0px",
              opacity: isTeamMode ? 1 : 0,
            }}
          >
            <TeamConfigPanel config={teamConfig} onChange={setTeamConfig} />

            {/* Info text */}
            <p
              className="text-[12px] mt-3 flex items-center justify-center gap-1.5"
              style={{ color: "hsl(220 10% 50%)" }}
            >
              <span className="text-[14px]">&#9432;</span>
              The lead agent will decide what specialist roles to create based on your task.
            </p>
          </div>

          {/* Primary Action */}
          <Button
            onClick={handleStartSession}
            disabled={isCreating}
            className="h-11 px-6 text-[14px] font-semibold tracking-[-0.01em] border-0 transition-colors duration-150 mt-4"
            style={{
              background: isCreating ? "hsl(14 100% 60% / 0.6)" : "hsl(14 100% 60%)",
              color: "white",
            }}
            onMouseEnter={(e) => {
              if (!isCreating) e.currentTarget.style.background = "hsl(14 100% 55%)";
            }}
            onMouseLeave={(e) => {
              if (!isCreating) e.currentTarget.style.background = "hsl(14 100% 60%)";
            }}
          >
            {isCreating ? (
              <>
                <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                Creating...
              </>
            ) : (
              <>
                <Zap className="w-4 h-4 mr-2" />
                {isTeamMode ? `Start ${teamMode === "research" ? "Research" : "Debate"} Session` : "Start New Session"}
              </>
            )}
          </Button>

          {/* Secondary Action */}
          <button
            onClick={() => setShowTaskPicker(true)}
            disabled={isCreatingFromTask}
            className="flex items-center justify-center gap-2 mx-auto mt-5 text-[13px] transition-colors duration-150 disabled:opacity-50 disabled:cursor-not-allowed"
            style={{ color: "hsl(220 10% 60%)" }}
            onMouseEnter={(e) => {
              if (!isCreatingFromTask) {
                e.currentTarget.style.color = "hsl(14 100% 60%)";
              }
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.color = "hsl(220 10% 60%)";
            }}
          >
            {isCreatingFromTask ? (
              <>
                <Loader2 className="w-4 h-4 animate-spin" />
                <span>Creating session...</span>
              </>
            ) : (
              <>
                <FileText className="w-4 h-4" />
                <span>Seed from Draft Task</span>
              </>
            )}
          </button>

          {/* Import Session */}
          <button
            onClick={() => {
              if (activeProjectId) {
                void importSession(activeProjectId);
              }
            }}
            disabled={isImporting}
            className="flex items-center justify-center gap-2 mx-auto mt-3 text-[13px] transition-colors duration-150 disabled:opacity-50 disabled:cursor-not-allowed"
            style={{ color: "hsl(220 10% 60%)" }}
            onMouseEnter={(e) => {
              if (!isImporting) {
                e.currentTarget.style.color = "hsl(14 100% 60%)";
              }
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.color = "hsl(220 10% 60%)";
            }}
          >
            {isImporting ? (
              <>
                <Loader2 className="w-4 h-4 animate-spin" />
                <span>Importing...</span>
              </>
            ) : (
              <>
                <Upload className="w-4 h-4" />
                <span>Import Session</span>
              </>
            )}
          </button>

          {/* Keyboard Hints */}
          <div
            className="flex items-center justify-center gap-4 mt-8"
            style={{ color: "hsl(220 10% 50%)" }}
          >
            <div className="flex items-center gap-1.5 text-[11px]">
              <kbd
                className="px-2 py-1 rounded-md text-[10px] font-medium"
                style={{
                  background: "hsla(220 10% 100% / 0.04)",
                  border: "1px solid hsla(220 10% 100% / 0.08)",
                }}
              >
                ⌘ N
              </kbd>
              <span>New</span>
            </div>
            <div className="flex items-center gap-1.5 text-[11px]">
              <kbd
                className="px-2 py-1 rounded-md text-[10px] font-medium"
                style={{
                  background: "hsla(220 10% 100% / 0.04)",
                  border: "1px solid hsla(220 10% 100% / 0.08)",
                }}
              >
                ⌘ D
              </kbd>
              <span>Seed</span>
            </div>
          </div>
        </div>
      </div>

      <TaskPickerDialog
        isOpen={showTaskPicker}
        onClose={() => setShowTaskPicker(false)}
        onSelect={handleSeedFromTask}
      />
    </>
  );
}
