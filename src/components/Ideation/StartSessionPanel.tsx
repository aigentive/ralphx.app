/**
 * StartSessionPanel - macOS Tahoe styled welcome screen
 *
 * Design: Elegant empty state with subtle radial gradient,
 * refined typography, and smooth interactions.
 */

import { useEffect, useState } from "react";
import { Lightbulb, Zap, FileText, Loader2 } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { TaskPickerDialog } from "./TaskPickerDialog";
import { useCreateIdeationSession } from "@/hooks/useIdeation";
import { useIdeationStore } from "@/stores/ideationStore";
import type { Task } from "@/types/task";

interface StartSessionPanelProps {
  onNewSession: () => void;
}

export function StartSessionPanel({ onNewSession }: StartSessionPanelProps) {
  const [showTaskPicker, setShowTaskPicker] = useState(false);
  const [isCreatingFromTask, setIsCreatingFromTask] = useState(false);

  const createSession = useCreateIdeationSession();
  const addSession = useIdeationStore((state) => state.addSession);
  const setActiveSession = useIdeationStore((state) => state.setActiveSession);

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

  const handleSeedFromTask = async (task: Task) => {
    setIsCreatingFromTask(true);
    try {
      const session = await createSession.mutateAsync({
        projectId: task.projectId,
        title: `Ideation: ${task.title}`,
        seedTaskId: task.id,
      });
      addSession(session);
      setActiveSession(session.id);
    } catch (error) {
      console.error("Failed to create ideation session:", error);
      toast.error("Failed to start ideation session");
    } finally {
      setIsCreatingFromTask(false);
    }
  };

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

        <div className="relative z-10 text-center max-w-sm">
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
            className="text-[14px] leading-relaxed mb-8 max-w-xs mx-auto"
            style={{ color: "hsl(220 10% 60%)" }}
          >
            Select a session from the sidebar or start a new brainstorming session.
          </p>

          {/* Primary Action */}
          <Button
            onClick={onNewSession}
            className="h-11 px-6 text-[14px] font-semibold tracking-[-0.01em] border-0 transition-colors duration-150"
            style={{
              background: "hsl(14 100% 60%)",
              color: "white",
            }}
            onMouseEnter={(e) => {
              e.currentTarget.style.background = "hsl(14 100% 55%)";
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.background = "hsl(14 100% 60%)";
            }}
          >
            <Zap className="w-4 h-4 mr-2" />
            Start New Session
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
