/**
 * StartSessionPanel - Empty state panel for starting a new ideation session
 */

import { useState } from "react";
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
        className="flex-1 flex flex-col items-center justify-center p-6 relative overflow-hidden"
        style={{
          background: `
            radial-gradient(ellipse 80% 50% at 20% 0%, rgba(255,107,53,0.06) 0%, transparent 50%),
            radial-gradient(ellipse 60% 40% at 80% 100%, rgba(255,107,53,0.03) 0%, transparent 50%),
            var(--bg-base)
          `,
        }}
      >
        <div className="relative z-10 text-center max-w-md">
          {/* Icon */}
          <div
            className="w-16 h-16 rounded-2xl flex items-center justify-center mx-auto mb-6"
            style={{
              background: "rgba(255,107,53,0.1)",
              border: "1px solid rgba(255,107,53,0.2)",
            }}
          >
            <Lightbulb className="w-8 h-8 text-[#ff6b35]" />
          </div>

          {/* Content */}
          <h1 className="text-lg font-semibold text-[var(--text-primary)] mb-2 tracking-tight">
            Ideation Studio
          </h1>
          <p className="text-sm text-[var(--text-secondary)] mb-6 leading-relaxed max-w-xs mx-auto">
            Select a session from the sidebar or start a new brainstorming session.
          </p>

          {/* Action button */}
          <Button
            onClick={onNewSession}
            className="h-9 px-5 text-sm bg-[#ff6b35] hover:bg-[#ff7a4d] text-white font-medium border-0 transition-all duration-180"
            style={{ boxShadow: "0 1px 3px rgba(0,0,0,0.15)" }}
          >
            <Zap className="w-4 h-4 mr-1.5" />
            Start New Session
          </Button>

          {/* Seed from Draft Task link */}
          <button
            onClick={() => setShowTaskPicker(true)}
            disabled={isCreatingFromTask}
            className="flex items-center justify-center gap-1.5 mx-auto mt-4 text-[12px] text-[var(--text-secondary)] hover:text-[#ff6b35] transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {isCreatingFromTask ? (
              <>
                <Loader2 className="w-3.5 h-3.5 animate-spin" />
                <span>Creating session...</span>
              </>
            ) : (
              <>
                <FileText className="w-3.5 h-3.5" />
                <span>Seed from Draft Task</span>
              </>
            )}
          </button>

          {/* Hint */}
          <p className="text-[11px] text-[var(--text-muted)] mt-4">
            Press <kbd className="px-1.5 py-0.5 rounded bg-white/[0.05] border border-white/[0.1] text-[10px] font-mono">⌘ N</kbd> to quickly start
          </p>
        </div>
      </div>

      {/* Task Picker Dialog */}
      <TaskPickerDialog
        isOpen={showTaskPicker}
        onClose={() => setShowTaskPicker(false)}
        onSelect={handleSeedFromTask}
      />
    </>
  );
}
