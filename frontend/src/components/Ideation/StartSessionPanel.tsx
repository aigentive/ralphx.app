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
import { withAlpha } from "@/lib/theme-colors";
import { useTeamModeAvailability } from "@/hooks/useTeamModeAvailability";
import { TaskPickerDialog } from "./TaskPickerDialog";
import { TeamConfigPanel } from "./TeamConfigPanel";
import { useCreateIdeationSession } from "@/hooks/useIdeation";
import { useSessionExportImport } from "@/hooks/useSessionExportImport";
import { useIdeationStore } from "@/stores/ideationStore";
import { useProjectStore } from "@/stores/projectStore";
import { getGitBranches, getGitCurrentBranch, getGitDefaultBranch } from "@/api/projects";
import type { Task } from "@/types/task";
import type { TeamMode, TeamConfig } from "@/types/ideation";
import type { IdeationAnalysisBaseSelection } from "@/api/ideation";

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

interface StartFromOption {
  key: string;
  label: string;
  detail: string;
  selection: IdeationAnalysisBaseSelection;
}

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
  const activeProject = useProjectStore((state) =>
    state.activeProjectId ? state.projects[state.activeProjectId] ?? null : null,
  );
  const [startFromOptions, setStartFromOptions] = useState<StartFromOption[]>([]);
  const [selectedStartFromKey, setSelectedStartFromKey] = useState<string>("");
  const [isLoadingStartFrom, setIsLoadingStartFrom] = useState(false);
  const { importSession, isImporting } = useSessionExportImport();
  const { ideationTeamModeAvailable: teamModeVisible } =
    useTeamModeAvailability(activeProjectId);
  const isTeamMode = teamModeVisible && teamMode !== "solo";
  const activeProjectWorkingDirectory = activeProject?.workingDirectory;
  const activeProjectBaseBranch = activeProject?.baseBranch;

  useEffect(() => {
    if (!teamModeVisible && teamMode !== "solo") {
      setTeamMode("solo");
    }
  }, [teamMode, teamModeVisible]);

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

  useEffect(() => {
    if (!activeProjectWorkingDirectory) {
      setStartFromOptions([]);
      setSelectedStartFromKey("");
      return;
    }

    let cancelled = false;
    setIsLoadingStartFrom(true);

    async function loadStartFromOptions() {
      const workingDirectory = activeProjectWorkingDirectory!;
      const [defaultResult, currentResult, branchesResult] = await Promise.allSettled([
        getGitDefaultBranch(workingDirectory),
        getGitCurrentBranch(workingDirectory),
        getGitBranches(workingDirectory),
      ]);

      if (cancelled) return;

      const projectDefault =
        defaultResult.status === "fulfilled"
          ? defaultResult.value
          : activeProjectBaseBranch ?? "main";
      const currentBranch =
        currentResult.status === "fulfilled" ? currentResult.value : projectDefault;
      const branches =
        branchesResult.status === "fulfilled" ? branchesResult.value : [projectDefault];
      const optionMap = new Map<string, StartFromOption>();

      optionMap.set(`project_default:${projectDefault}`, {
        key: `project_default:${projectDefault}`,
        label: `Project default (${projectDefault})`,
        detail: "Use the configured project base branch.",
        selection: {
          kind: "project_default",
          ref: projectDefault,
          displayName: `Project default (${projectDefault})`,
        },
      });

      if (currentBranch && currentBranch !== projectDefault) {
        optionMap.set(`current_branch:${currentBranch}`, {
          key: `current_branch:${currentBranch}`,
          label: `Current branch (${currentBranch})`,
          detail: "Use the branch currently checked out in the project root.",
          selection: {
            kind: "current_branch",
            ref: currentBranch,
            displayName: `Current branch (${currentBranch})`,
          },
        });
      }

      branches
        .filter((branch) => branch && branch !== projectDefault && branch !== currentBranch)
        .forEach((branch) => {
          optionMap.set(`local_branch:${branch}`, {
            key: `local_branch:${branch}`,
            label: branch,
            detail: "Use a local branch in an isolated ideation workspace.",
            selection: {
              kind: "local_branch",
              ref: branch,
              displayName: branch,
            },
          });
        });

      const options = Array.from(optionMap.values());
      setStartFromOptions(options);
      setSelectedStartFromKey(
        currentBranch && currentBranch !== projectDefault
          ? `current_branch:${currentBranch}`
          : `project_default:${projectDefault}`,
      );
      setIsLoadingStartFrom(false);
    }

    void loadStartFromOptions().catch(() => {
      if (!cancelled) {
        const fallback = activeProjectBaseBranch ?? "main";
        setStartFromOptions([
          {
            key: `project_default:${fallback}`,
            label: `Project default (${fallback})`,
            detail: "Use the configured project base branch.",
            selection: {
              kind: "project_default",
              ref: fallback,
              displayName: `Project default (${fallback})`,
            },
          },
        ]);
        setSelectedStartFromKey(`project_default:${fallback}`);
        setIsLoadingStartFrom(false);
      }
    });

    return () => {
      cancelled = true;
    };
  }, [activeProjectBaseBranch, activeProjectWorkingDirectory]);

  const selectedStartFrom = startFromOptions.find((option) => option.key === selectedStartFromKey);

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
      if (selectedStartFrom) {
        params.analysisBase = selectedStartFrom.selection;
      }
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
      if (selectedStartFrom) {
        params.analysisBase = selectedStartFrom.selection;
      }
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
        style={{ background: "var(--bg-base)" }}
      >
        {/* Subtle grid pattern */}
        <div
          className="absolute inset-0 opacity-[0.015]"
          style={{
            backgroundImage: `
              linear-gradient(var(--overlay-moderate) 1px, transparent 1px),
              linear-gradient(90deg, var(--overlay-moderate) 1px, transparent 1px)
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
                background: withAlpha("var(--accent-primary)", 12),
                border: "1px solid var(--accent-border)",
              }}
            >
              <Lightbulb className="w-9 h-9" style={{ color: "var(--accent-primary)" }} strokeWidth={1.5} />
            </div>
          </div>

          {/* Content */}
          <h1
            className="text-xl font-semibold tracking-[-0.02em] mb-2"
            style={{ color: "var(--text-primary)" }}
          >
            Ideation Studio
          </h1>
          <p
            className="text-[14px] leading-relaxed mb-6 max-w-xs mx-auto"
            style={{ color: "var(--text-secondary)" }}
          >
            Select a session from the sidebar or start a new brainstorming session.
          </p>

          <div className="mb-6 text-left">
            <label
              htmlFor="ideation-start-from"
              className="block text-[12px] font-medium tracking-wide uppercase mb-2 text-center"
              style={{ color: "var(--text-muted)" }}
            >
              Start from
            </label>
            <select
              id="ideation-start-from"
              data-testid="start-from-select"
              value={selectedStartFromKey}
              onChange={(event) => setSelectedStartFromKey(event.target.value)}
              disabled={isLoadingStartFrom || startFromOptions.length === 0}
              className="w-full h-10 rounded-xl px-3 text-[13px] outline-none"
              style={{
                background: "var(--bg-elevated)",
                border: "1px solid var(--border-default)",
                color: "var(--text-primary)",
              }}
            >
              {startFromOptions.map((option) => (
                <option key={option.key} value={option.key}>
                  {option.label}
                </option>
              ))}
            </select>
            <p
              className="mt-2 text-[12px] text-center"
              style={{ color: "var(--text-secondary)" }}
            >
              {isLoadingStartFrom
                ? "Detecting repository branches..."
                : selectedStartFrom?.detail ?? "The selected base is locked for this ideation session."}
            </p>
          </div>

          {teamModeVisible && (
            <>
              {/* Team Mode Selector */}
              <div className="mb-6">
                <p
                  className="text-[12px] font-medium tracking-wide uppercase mb-3"
                  style={{ color: "var(--text-muted)" }}
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
                          background: isSelected ? "var(--accent-muted)" : "var(--overlay-faint)",
                          border: `1px solid ${isSelected ? "var(--accent-primary)" : "var(--overlay-weak)"}`,
                          color: isSelected ? "var(--accent-primary)" : "var(--text-secondary)",
                        }}
                        onMouseEnter={(e) => {
                          if (!isSelected) {
                            e.currentTarget.style.borderColor = "var(--overlay-moderate)";
                            e.currentTarget.style.color = "var(--text-primary)";
                          }
                        }}
                        onMouseLeave={(e) => {
                          if (!isSelected) {
                            e.currentTarget.style.borderColor = "var(--overlay-weak)";
                            e.currentTarget.style.color = "var(--text-secondary)";
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
                  style={{ color: "var(--text-muted)" }}
                >
                  <span className="text-[14px]">&#9432;</span>
                  The lead agent will decide what specialist roles to create based on your task.
                </p>
              </div>
            </>
          )}

          {/* Primary Action */}
          <Button
            onClick={handleStartSession}
            disabled={isCreating}
            className="h-11 px-6 text-[14px] font-semibold tracking-[-0.01em] border-0 transition-colors duration-150 mt-4"
            style={{
              background: isCreating ? withAlpha("var(--accent-primary)", 60) : "var(--accent-primary)",
              color: "var(--text-on-accent)",
            }}
            onMouseEnter={(e) => {
              if (!isCreating) e.currentTarget.style.background = withAlpha("var(--accent-primary)", 90);
            }}
            onMouseLeave={(e) => {
              if (!isCreating) e.currentTarget.style.background = "var(--accent-primary)";
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
            style={{ color: "var(--text-secondary)" }}
            onMouseEnter={(e) => {
              if (!isCreatingFromTask) {
                e.currentTarget.style.color = "var(--accent-primary)";
              }
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.color = "var(--text-secondary)";
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
            style={{ color: "var(--text-secondary)" }}
            onMouseEnter={(e) => {
              if (!isImporting) {
                e.currentTarget.style.color = "var(--accent-primary)";
              }
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.color = "var(--text-secondary)";
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
            style={{ color: "var(--text-muted)" }}
          >
            <div className="flex items-center gap-1.5 text-[11px]">
              <kbd
                className="px-2 py-1 rounded-md text-[10px] font-medium"
                style={{
                  background: "var(--overlay-faint)",
                  border: "1px solid var(--overlay-weak)",
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
                  background: "var(--overlay-faint)",
                  border: "1px solid var(--overlay-weak)",
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
