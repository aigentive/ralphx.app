/**
 * RalphX - App Shell
 * Root component with QueryClientProvider and EventProvider
 */

import { useMemo, useState } from "react";
import { QueryClientProvider } from "@tanstack/react-query";
import { ReactQueryDevtools } from "@tanstack/react-query-devtools";
import { getQueryClient } from "@/lib/queryClient";
import { EventProvider } from "@/providers/EventProvider";
import { TaskBoard } from "@/components/tasks/TaskBoard";
import { ReviewsPanel } from "@/components/reviews/ReviewsPanel";
import { ExecutionControlBar } from "@/components/execution/ExecutionControlBar";
import { AskUserQuestionModal } from "@/components/modals/AskUserQuestionModal";
import { useUiStore } from "@/stores/uiStore";
import { usePendingReviews } from "@/hooks/useReviews";
import { useTasks } from "@/hooks/useTasks";
import { api } from "@/lib/tauri";
import type { AskUserQuestionResponse } from "@/types/ask-user-question";

const queryClient = getQueryClient();

// Temporary hardcoded IDs until project selection is implemented
const DEFAULT_PROJECT_ID = "demo-project";
const DEFAULT_WORKFLOW_ID = "ralphx-default";

function ReviewIcon() {
  return (
    <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
      <path
        d="M10 18a8 8 0 100-16 8 8 0 000 16z"
        stroke="currentColor"
        strokeWidth="1.5"
      />
      <path
        d="M7 10l2 2 4-4"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

function AppContent() {
  const reviewsPanelOpen = useUiStore((s) => s.reviewsPanelOpen);
  const toggleReviewsPanel = useUiStore((s) => s.toggleReviewsPanel);
  const setReviewsPanelOpen = useUiStore((s) => s.setReviewsPanelOpen);
  const executionStatus = useUiStore((s) => s.executionStatus);
  const setExecutionStatus = useUiStore((s) => s.setExecutionStatus);
  const activeQuestion = useUiStore((s) => s.activeQuestion);
  const clearActiveQuestion = useUiStore((s) => s.clearActiveQuestion);

  const [isExecutionLoading, setIsExecutionLoading] = useState(false);
  const [isQuestionLoading, setIsQuestionLoading] = useState(false);

  const { count: pendingReviewCount } = usePendingReviews(DEFAULT_PROJECT_ID);
  const { data: tasks = [] } = useTasks(DEFAULT_PROJECT_ID);

  const handlePauseToggle = async () => {
    setIsExecutionLoading(true);
    try {
      const response = executionStatus.isPaused
        ? await api.execution.resume()
        : await api.execution.pause();
      setExecutionStatus(response.status);
    } catch (error) {
      console.error("Failed to toggle pause:", error);
    } finally {
      setIsExecutionLoading(false);
    }
  };

  const handleStop = async () => {
    setIsExecutionLoading(true);
    try {
      const response = await api.execution.stop();
      setExecutionStatus(response.status);
    } catch (error) {
      console.error("Failed to stop execution:", error);
    } finally {
      setIsExecutionLoading(false);
    }
  };

  const handleQuestionSubmit = async (response: AskUserQuestionResponse) => {
    setIsQuestionLoading(true);
    try {
      console.log("Submit answer:", response);
      // TODO: Call Tauri command to submit answer and trigger BlockersResolved event
      clearActiveQuestion();
    } catch (error) {
      console.error("Failed to submit answer:", error);
    } finally {
      setIsQuestionLoading(false);
    }
  };

  const handleQuestionClose = () => {
    // Close without submitting - question remains unanswered
    clearActiveQuestion();
  };

  // Build task titles lookup
  const taskTitles = useMemo(() => {
    const titles: Record<string, string> = {};
    for (const task of tasks) {
      titles[task.id] = task.title;
    }
    return titles;
  }, [tasks]);

  return (
    <main
      className="min-h-screen flex flex-col"
      style={{ backgroundColor: "var(--bg-base)", color: "var(--text-primary)" }}
    >
      {/* Header */}
      <header
        className="flex items-center justify-between p-4 border-b"
        style={{ borderColor: "var(--border-subtle)" }}
      >
        <h1
          className="text-xl font-bold"
          style={{ color: "var(--accent-primary)" }}
        >
          RalphX
        </h1>
        <div className="flex items-center gap-3">
          <span className="text-sm" style={{ color: "var(--text-muted)" }}>
            Demo Project
          </span>
          {/* Reviews Panel Toggle */}
          <button
            onClick={toggleReviewsPanel}
            className="relative flex items-center gap-2 px-3 py-1.5 rounded-lg transition-colors"
            style={{
              backgroundColor: reviewsPanelOpen
                ? "var(--bg-elevated)"
                : "transparent",
              color: reviewsPanelOpen
                ? "var(--accent-primary)"
                : "var(--text-secondary)",
            }}
            data-testid="reviews-toggle"
          >
            <ReviewIcon />
            <span className="text-sm font-medium">Reviews</span>
            {/* Badge with pending count */}
            {pendingReviewCount > 0 && (
              <span
                className="absolute -top-1 -right-1 flex items-center justify-center w-5 h-5 text-xs font-bold rounded-full"
                style={{
                  backgroundColor: "var(--status-review)",
                  color: "white",
                }}
                data-testid="reviews-badge"
              >
                {pendingReviewCount > 9 ? "9+" : pendingReviewCount}
              </span>
            )}
          </button>
        </div>
      </header>

      {/* Main content area with TaskBoard and optional ReviewsPanel */}
      <div className="flex-1 flex overflow-hidden">
        {/* TaskBoard with ExecutionControlBar */}
        <div className="flex-1 flex flex-col overflow-hidden">
          <div className="flex-1 overflow-hidden">
            <TaskBoard
              projectId={DEFAULT_PROJECT_ID}
              workflowId={DEFAULT_WORKFLOW_ID}
            />
          </div>
          {/* ExecutionControlBar at bottom */}
          <div className="p-4 border-t" style={{ borderColor: "var(--border-subtle)" }}>
            <ExecutionControlBar
              runningCount={executionStatus.runningCount}
              maxConcurrent={executionStatus.maxConcurrent}
              queuedCount={executionStatus.queuedCount}
              isPaused={executionStatus.isPaused}
              isLoading={isExecutionLoading}
              onPauseToggle={handlePauseToggle}
              onStop={handleStop}
            />
          </div>
        </div>

        {/* ReviewsPanel slide-out */}
        {reviewsPanelOpen && (
          <div
            className="w-96 border-l flex-shrink-0"
            style={{ borderColor: "var(--border-subtle)" }}
          >
            <ReviewsPanel
              projectId={DEFAULT_PROJECT_ID}
              taskTitles={taskTitles}
              onClose={() => setReviewsPanelOpen(false)}
              onApprove={(reviewId) => {
                console.log("Approve review:", reviewId);
                // TODO: Call approveReview mutation
              }}
              onRequestChanges={(reviewId) => {
                console.log("Request changes for review:", reviewId);
                // TODO: Open request changes modal
              }}
              onViewDiff={(reviewId) => {
                console.log("View diff for review:", reviewId);
                // TODO: Open diff viewer
              }}
            />
          </div>
        )}
      </div>

      {/* AskUserQuestionModal - renders when activeQuestion is set */}
      <AskUserQuestionModal
        question={activeQuestion}
        onSubmit={handleQuestionSubmit}
        onClose={handleQuestionClose}
        isLoading={isQuestionLoading}
      />
    </main>
  );
}

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <EventProvider>
        <AppContent />
      </EventProvider>
      <ReactQueryDevtools initialIsOpen={false} />
    </QueryClientProvider>
  );
}

export default App;
