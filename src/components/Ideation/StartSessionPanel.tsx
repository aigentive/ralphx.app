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
        style={{
          background: `
            radial-gradient(ellipse 100% 80% at 50% 20%, rgba(255,107,53,0.08) 0%, transparent 60%),
            radial-gradient(ellipse 80% 60% at 80% 80%, rgba(255,107,53,0.04) 0%, transparent 50%),
            radial-gradient(ellipse 60% 40% at 20% 90%, rgba(255,107,53,0.03) 0%, transparent 50%),
            linear-gradient(180deg, rgba(8,8,8,1) 0%, rgba(5,5,5,1) 100%)
          `,
        }}
      >
        {/* Subtle grid pattern */}
        <div
          className="absolute inset-0 opacity-[0.015]"
          style={{
            backgroundImage: `
              linear-gradient(rgba(255,255,255,0.5) 1px, transparent 1px),
              linear-gradient(90deg, rgba(255,255,255,0.5) 1px, transparent 1px)
            `,
            backgroundSize: "48px 48px",
          }}
        />

        <div className="relative z-10 text-center max-w-sm">
          {/* Icon */}
          <div className="relative mb-8">
            {/* Glow effect */}
            <div
              className="absolute inset-0 blur-3xl"
              style={{
                background: "radial-gradient(circle, rgba(255,107,53,0.2) 0%, transparent 70%)",
                transform: "scale(2)",
              }}
            />
            <div
              className="relative w-20 h-20 rounded-[22px] flex items-center justify-center mx-auto"
              style={{
                background: "linear-gradient(135deg, rgba(255,107,53,0.2) 0%, rgba(255,107,53,0.08) 100%)",
                border: "1px solid rgba(255,107,53,0.3)",
                boxShadow: `
                  0 4px 24px rgba(255,107,53,0.15),
                  0 1px 2px rgba(0,0,0,0.2),
                  inset 0 1px 0 rgba(255,255,255,0.05)
                `,
              }}
            >
              <Lightbulb className="w-9 h-9 text-[#ff6b35]" strokeWidth={1.5} />
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
            className="text-[14px] leading-relaxed mb-8 max-w-xs mx-auto"
            style={{ color: "var(--text-secondary)" }}
          >
            Select a session from the sidebar or start a new brainstorming session.
          </p>

          {/* Primary Action */}
          <Button
            onClick={onNewSession}
            className="h-11 px-6 text-[14px] font-semibold tracking-[-0.01em] border-0"
            style={{
              background: "linear-gradient(180deg, #ff7a4d 0%, #ff6b35 100%)",
              boxShadow: `
                0 1px 3px rgba(0,0,0,0.3),
                0 6px 20px rgba(255,107,53,0.25),
                inset 0 1px 0 rgba(255,255,255,0.15)
              `,
              color: "white",
              transition: "all 200ms cubic-bezier(0.4, 0, 0.2, 1)",
            }}
            onMouseEnter={(e) => {
              e.currentTarget.style.transform = "translateY(-2px)";
              e.currentTarget.style.boxShadow = `
                0 2px 6px rgba(0,0,0,0.35),
                0 12px 32px rgba(255,107,53,0.3),
                inset 0 1px 0 rgba(255,255,255,0.15)
              `;
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.transform = "translateY(0)";
              e.currentTarget.style.boxShadow = `
                0 1px 3px rgba(0,0,0,0.3),
                0 6px 20px rgba(255,107,53,0.25),
                inset 0 1px 0 rgba(255,255,255,0.15)
              `;
            }}
          >
            <Zap className="w-4 h-4 mr-2" />
            Start New Session
          </Button>

          {/* Secondary Action */}
          <button
            onClick={() => setShowTaskPicker(true)}
            disabled={isCreatingFromTask}
            className="flex items-center justify-center gap-2 mx-auto mt-5 text-[13px] transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed"
            style={{
              color: "var(--text-secondary)",
            }}
            onMouseEnter={(e) => {
              if (!isCreatingFromTask) {
                e.currentTarget.style.color = "#ff6b35";
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

          {/* Keyboard Hints */}
          <div
            className="flex items-center justify-center gap-4 mt-8"
            style={{ color: "var(--text-muted)" }}
          >
            <div className="flex items-center gap-1.5 text-[11px]">
              <kbd
                className="px-2 py-1 rounded-md text-[10px] font-medium"
                style={{
                  background: "rgba(255,255,255,0.04)",
                  border: "1px solid rgba(255,255,255,0.08)",
                  boxShadow: "0 1px 2px rgba(0,0,0,0.2)",
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
                  background: "rgba(255,255,255,0.04)",
                  border: "1px solid rgba(255,255,255,0.08)",
                  boxShadow: "0 1px 2px rgba(0,0,0,0.2)",
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
