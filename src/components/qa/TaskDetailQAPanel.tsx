/**
 * TaskDetailQAPanel - Tabbed panel showing QA data for a task
 *
 * Displays:
 * - Acceptance Criteria tab with checkmarks
 * - Test Results tab with pass/fail icons
 * - Screenshots tab with thumbnail gallery and lightbox
 */

import { useState, useCallback, useEffect, useRef } from "react";
import type {
  TaskQAResponse,
  QAResultsResponse,
  AcceptanceCriterionResponse,
  QATestStepResponse,
} from "@/lib/tauri";
import type { QAStepStatus } from "@/types/qa";

// ============================================================================
// Types
// ============================================================================

type TabId = "criteria" | "results" | "screenshots";

interface TaskDetailQAPanelProps {
  taskQA: TaskQAResponse | null;
  results: QAResultsResponse | null;
  isLoading?: boolean;
  onRetry?: () => void;
  onSkip?: () => void;
  isRetrying?: boolean;
  isSkipping?: boolean;
}

// ============================================================================
// Icons
// ============================================================================

function CheckIcon({ className = "" }: { className?: string }) {
  return (
    <svg
      width="16"
      height="16"
      viewBox="0 0 16 16"
      fill="none"
      className={className}
    >
      <path
        d="M13 4L6 11L3 8"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

function XIcon({ className = "" }: { className?: string }) {
  return (
    <svg
      width="16"
      height="16"
      viewBox="0 0 16 16"
      fill="none"
      className={className}
    >
      <path
        d="M12 4L4 12M4 4L12 12"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

function CircleIcon({ className = "" }: { className?: string }) {
  return (
    <svg
      width="16"
      height="16"
      viewBox="0 0 16 16"
      fill="none"
      className={className}
    >
      <circle cx="8" cy="8" r="4" stroke="currentColor" strokeWidth="2" />
    </svg>
  );
}

function MinusIcon({ className = "" }: { className?: string }) {
  return (
    <svg
      width="16"
      height="16"
      viewBox="0 0 16 16"
      fill="none"
      className={className}
    >
      <path
        d="M4 8H12"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
      />
    </svg>
  );
}

function ChevronLeftIcon() {
  return (
    <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
      <path
        d="M12 4L6 10L12 16"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

function ChevronRightIcon() {
  return (
    <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
      <path
        d="M8 4L14 10L8 16"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

function CloseIcon() {
  return (
    <svg width="24" height="24" viewBox="0 0 24 24" fill="none">
      <path
        d="M18 6L6 18M6 6L18 18"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

function ImageIcon({ className = "" }: { className?: string }) {
  return (
    <svg
      width="16"
      height="16"
      viewBox="0 0 16 16"
      fill="none"
      className={className}
    >
      <rect
        x="2"
        y="2"
        width="12"
        height="12"
        rx="2"
        stroke="currentColor"
        strokeWidth="1.5"
      />
      <circle cx="5.5" cy="5.5" r="1.5" fill="currentColor" />
      <path
        d="M14 10L11 7L5 13H12C13.1046 13 14 12.1046 14 11V10Z"
        fill="currentColor"
      />
    </svg>
  );
}

// ============================================================================
// Helper Functions
// ============================================================================

function getStatusIcon(status: QAStepStatus | "pending") {
  switch (status) {
    case "passed":
      return <CheckIcon className="text-[--status-success]" />;
    case "failed":
      return <XIcon className="text-[--status-error]" />;
    case "skipped":
      return <MinusIcon className="text-[--text-muted]" />;
    case "running":
      return <CircleIcon className="text-[--status-info] animate-pulse" />;
    case "pending":
    default:
      return <CircleIcon className="text-[--text-muted]" />;
  }
}

function getCriterionStatus(
  criterionId: string,
  testSteps: QATestStepResponse[] | undefined,
  results: QAResultsResponse | null
): "passed" | "failed" | "pending" {
  if (!results || !testSteps) return "pending";

  // Find steps that test this criterion
  const relatedSteps = testSteps.filter((s) => s.criteria_id === criterionId);
  if (relatedSteps.length === 0) return "pending";

  // Get results for these steps
  const stepResults = results.steps.filter((r) =>
    relatedSteps.some((s) => s.id === r.step_id)
  );

  if (stepResults.length === 0) return "pending";

  // If any failed, criterion failed
  if (stepResults.some((r) => r.status === "failed")) return "failed";

  // If all passed, criterion passed
  if (stepResults.every((r) => r.status === "passed")) return "passed";

  return "pending";
}

function getFilename(path: string): string {
  return path.split("/").pop() || path;
}

// ============================================================================
// Skeleton Component
// ============================================================================

function QAPanelSkeleton() {
  return (
    <div data-testid="qa-panel-skeleton" className="animate-pulse">
      <div className="flex gap-2 mb-4">
        <div className="h-8 w-32 rounded bg-[--bg-hover]" />
        <div className="h-8 w-28 rounded bg-[--bg-hover]" />
        <div className="h-8 w-28 rounded bg-[--bg-hover]" />
      </div>
      <div className="space-y-3">
        <div className="h-12 rounded bg-[--bg-hover]" />
        <div className="h-12 rounded bg-[--bg-hover]" />
        <div className="h-12 rounded bg-[--bg-hover]" />
      </div>
    </div>
  );
}

// ============================================================================
// Tab Button Component
// ============================================================================

interface TabButtonProps {
  id: TabId;
  label: string;
  count?: number | undefined;
  countTestId?: string | undefined;
  isSelected: boolean;
  onClick: (id: TabId) => void;
  onKeyDown: (e: React.KeyboardEvent, id: TabId) => void;
}

function TabButton({
  id,
  label,
  count,
  countTestId,
  isSelected,
  onClick,
  onKeyDown,
}: TabButtonProps) {
  return (
    <button
      role="tab"
      id={`tab-${id}`}
      aria-selected={isSelected}
      aria-controls={`tabpanel-${id}`}
      tabIndex={isSelected ? 0 : -1}
      onClick={() => onClick(id)}
      onKeyDown={(e) => onKeyDown(e, id)}
      className={`px-3 py-2 text-sm font-medium rounded-t transition-colors ${
        isSelected
          ? "bg-[--bg-elevated] text-[--text-primary] border-b-2 border-[--accent-primary]"
          : "text-[--text-secondary] hover:text-[--text-primary] hover:bg-[--bg-hover]"
      }`}
    >
      {label}
      {count !== undefined && (
        <span
          data-testid={countTestId}
          className="ml-1.5 px-1.5 py-0.5 text-xs rounded bg-[--bg-hover]"
        >
          {count}
        </span>
      )}
    </button>
  );
}

// ============================================================================
// Acceptance Criteria Tab
// ============================================================================

interface AcceptanceCriteriaTabProps {
  criteria: AcceptanceCriterionResponse[];
  testSteps?: QATestStepResponse[] | undefined;
  results: QAResultsResponse | null;
}

function AcceptanceCriteriaTab({
  criteria,
  testSteps,
  results,
}: AcceptanceCriteriaTabProps) {
  if (criteria.length === 0) {
    return (
      <p className="text-[--text-muted] text-sm py-4">
        No acceptance criteria defined
      </p>
    );
  }

  return (
    <div className="space-y-2">
      {criteria.map((criterion) => {
        const status = getCriterionStatus(criterion.id, testSteps, results);

        return (
          <div
            key={criterion.id}
            className="flex items-start gap-3 p-3 rounded bg-[--bg-elevated]"
          >
            <span
              data-testid={`criterion-status-${criterion.id}`}
              data-status={status}
              className="flex-shrink-0 mt-0.5"
            >
              {getStatusIcon(status)}
            </span>
            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-2 mb-1">
                <span className="px-1.5 py-0.5 text-xs font-medium rounded bg-[--bg-hover] text-[--text-secondary]">
                  {criterion.id}
                </span>
                <span className="px-1.5 py-0.5 text-xs rounded bg-[--bg-hover] text-[--text-muted]">
                  {criterion.criteria_type}
                </span>
                {criterion.testable && (
                  <span
                    data-testid={`criterion-testable-${criterion.id}`}
                    className="px-1.5 py-0.5 text-xs rounded bg-[--status-info] text-[--bg-base]"
                  >
                    Testable
                  </span>
                )}
              </div>
              <p className="text-sm text-[--text-primary]">
                {criterion.description}
              </p>
            </div>
          </div>
        );
      })}
    </div>
  );
}

// ============================================================================
// Test Results Tab
// ============================================================================

interface TestResultsTabProps {
  testSteps?: QATestStepResponse[] | undefined;
  results: QAResultsResponse | null;
  onScreenshotClick?: ((path: string) => void) | undefined;
}

function TestResultsTab({
  testSteps,
  results,
  onScreenshotClick,
}: TestResultsTabProps) {
  if (!results) {
    return (
      <p className="text-[--text-muted] text-sm py-4">
        No test results available yet
      </p>
    );
  }

  // Create a map of step ID to step for quick lookup
  const stepMap = new Map(testSteps?.map((s) => [s.id, s]) ?? []);

  return (
    <div className="space-y-4">
      {/* Overall Status */}
      <div className="flex items-center justify-between p-3 rounded bg-[--bg-elevated]">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium text-[--text-secondary]">
            Overall:
          </span>
          <span
            data-testid="overall-status"
            className={`px-2 py-0.5 text-xs font-medium rounded ${
              results.overall_status === "passed"
                ? "bg-[--status-success] text-[--bg-base]"
                : results.overall_status === "failed"
                  ? "bg-[--status-error] text-[--bg-base]"
                  : "bg-[--text-muted] text-[--bg-base]"
            }`}
          >
            {results.overall_status}
          </span>
        </div>
        <span
          data-testid="results-summary"
          className="text-sm text-[--text-muted]"
        >
          {results.passed_steps}/{results.total_steps}
        </span>
      </div>

      {/* Step Results */}
      <div className="space-y-2">
        {results.steps.map((stepResult) => {
          const step = stepMap.get(stepResult.step_id);
          const isFailed = stepResult.status === "failed";

          return (
            <div
              key={stepResult.step_id}
              className="p-3 rounded bg-[--bg-elevated]"
            >
              <div className="flex items-start gap-3">
                <span
                  data-testid={`step-status-${stepResult.step_id}`}
                  data-status={stepResult.status}
                  className="flex-shrink-0 mt-0.5"
                >
                  {getStatusIcon(stepResult.status)}
                </span>
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2 mb-1">
                    <span className="px-1.5 py-0.5 text-xs font-medium rounded bg-[--bg-hover] text-[--text-secondary]">
                      {stepResult.step_id}
                    </span>
                    {stepResult.screenshot && (
                      <button
                        data-testid={`step-screenshot-link-${stepResult.step_id}`}
                        onClick={() => onScreenshotClick?.(stepResult.screenshot!)}
                        className="flex items-center gap-1 text-xs text-[--accent-primary] hover:underline"
                      >
                        <ImageIcon className="w-3 h-3" />
                        Screenshot
                      </button>
                    )}
                  </div>
                  <p className="text-sm text-[--text-primary]">
                    {step?.description ?? stepResult.step_id}
                  </p>
                </div>
              </div>

              {/* Failure Details */}
              {isFailed && (stepResult.expected || stepResult.actual || stepResult.error) && (
                <div
                  data-testid={`failure-details-${stepResult.step_id}`}
                  className="mt-3 ml-7 p-2 rounded bg-[--bg-base] border border-[--status-error] border-opacity-30"
                >
                  {stepResult.error && (
                    <p
                      data-testid={`error-message-${stepResult.step_id}`}
                      className="text-xs text-[--status-error] mb-2"
                    >
                      {stepResult.error}
                    </p>
                  )}
                  {(stepResult.expected || stepResult.actual) && (
                    <div className="grid grid-cols-2 gap-2 text-xs">
                      {stepResult.expected && (
                        <div>
                          <span className="text-[--text-muted]">Expected:</span>
                          <p className="text-[--text-primary] mt-0.5">
                            {stepResult.expected}
                          </p>
                        </div>
                      )}
                      {stepResult.actual && (
                        <div>
                          <span className="text-[--text-muted]">Actual:</span>
                          <p className="text-[--text-primary] mt-0.5">
                            {stepResult.actual}
                          </p>
                        </div>
                      )}
                    </div>
                  )}
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}

// ============================================================================
// Screenshots Tab
// ============================================================================

interface ScreenshotsTabProps {
  screenshots: string[];
  onThumbnailClick: (index: number) => void;
}

function ScreenshotsTab({ screenshots, onThumbnailClick }: ScreenshotsTabProps) {
  if (screenshots.length === 0) {
    return (
      <p className="text-[--text-muted] text-sm py-4">
        No screenshots captured
      </p>
    );
  }

  return (
    <div className="grid grid-cols-3 gap-2">
      {screenshots.map((path, index) => (
        <button
          key={path}
          data-testid={`screenshot-thumbnail-${index}`}
          onClick={() => onThumbnailClick(index)}
          className="aspect-video rounded overflow-hidden bg-[--bg-hover] hover:ring-2 hover:ring-[--accent-primary] transition-all"
        >
          <img
            src={path}
            alt={getFilename(path)}
            className="w-full h-full object-cover"
            onError={(e) => {
              // If image fails to load, show placeholder
              e.currentTarget.style.display = "none";
            }}
          />
          <div className="flex items-center justify-center h-full text-[--text-muted]">
            <ImageIcon className="w-8 h-8" />
          </div>
        </button>
      ))}
    </div>
  );
}

// ============================================================================
// Lightbox Component
// ============================================================================

interface LightboxProps {
  screenshots: string[];
  currentIndex: number;
  onClose: () => void;
  onPrev: () => void;
  onNext: () => void;
}

function Lightbox({
  screenshots,
  currentIndex,
  onClose,
  onPrev,
  onNext,
}: LightboxProps) {
  const currentPath = screenshots[currentIndex] ?? "";
  const filename = getFilename(currentPath);

  // Handle escape key
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
      if (e.key === "ArrowLeft") onPrev();
      if (e.key === "ArrowRight") onNext();
    };
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [onClose, onPrev, onNext]);

  return (
    <div
      data-testid="screenshot-lightbox"
      className="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-90"
      onClick={onClose}
    >
      {/* Header */}
      <div className="absolute top-0 left-0 right-0 flex items-center justify-between p-4">
        <span
          data-testid="lightbox-filename"
          className="text-white text-sm font-medium"
        >
          {filename}
        </span>
        <div className="flex items-center gap-4">
          <span
            data-testid="lightbox-current-index"
            className="text-white text-sm"
          >
            {currentIndex + 1} of {screenshots.length}
          </span>
          <button
            data-testid="lightbox-close"
            onClick={(e) => {
              e.stopPropagation();
              onClose();
            }}
            className="text-white hover:text-gray-300 transition-colors"
          >
            <CloseIcon />
          </button>
        </div>
      </div>

      {/* Navigation */}
      {screenshots.length > 1 && (
        <>
          <button
            data-testid="lightbox-prev"
            onClick={(e) => {
              e.stopPropagation();
              onPrev();
            }}
            disabled={currentIndex === 0}
            className="absolute left-4 p-2 text-white hover:text-gray-300 disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
          >
            <ChevronLeftIcon />
          </button>
          <button
            data-testid="lightbox-next"
            onClick={(e) => {
              e.stopPropagation();
              onNext();
            }}
            disabled={currentIndex === screenshots.length - 1}
            className="absolute right-4 p-2 text-white hover:text-gray-300 disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
          >
            <ChevronRightIcon />
          </button>
        </>
      )}

      {/* Image */}
      <img
        src={currentPath}
        alt={filename}
        className="max-w-[90vw] max-h-[80vh] object-contain"
        onClick={(e) => e.stopPropagation()}
      />
    </div>
  );
}

// ============================================================================
// Main Component
// ============================================================================

export function TaskDetailQAPanel({
  taskQA,
  results,
  isLoading = false,
  onRetry,
  onSkip,
  isRetrying = false,
  isSkipping = false,
}: TaskDetailQAPanelProps) {
  const [activeTab, setActiveTab] = useState<TabId>("criteria");
  const [lightboxIndex, setLightboxIndex] = useState<number | null>(null);
  const tablistRef = useRef<HTMLDivElement>(null);

  // Handle keyboard navigation
  const handleTabKeyDown = useCallback(
    (e: React.KeyboardEvent, currentTabId: TabId) => {
      const tabs: TabId[] = ["criteria", "results", "screenshots"];
      const currentIndex = tabs.indexOf(currentTabId);

      let nextIndex: number | null = null;

      if (e.key === "ArrowRight") {
        nextIndex = (currentIndex + 1) % tabs.length;
      } else if (e.key === "ArrowLeft") {
        nextIndex = (currentIndex - 1 + tabs.length) % tabs.length;
      }

      if (nextIndex !== null) {
        const nextTab = tabs[nextIndex];
        if (nextTab) {
          e.preventDefault();
          setActiveTab(nextTab);

          // Focus the next tab button
          const nextTabButton = tablistRef.current?.querySelector(
            `#tab-${nextTab}`
          ) as HTMLElement | null;
          nextTabButton?.focus();
        }
      }
    },
    []
  );

  // Show loading skeleton
  if (isLoading) {
    return <QAPanelSkeleton />;
  }

  // Show empty state
  if (!taskQA) {
    return (
      <div className="text-center py-8 text-[--text-muted]">
        <p>No QA data available for this task</p>
      </div>
    );
  }

  const criteria = taskQA.acceptance_criteria ?? [];
  const testSteps = taskQA.qa_test_steps ?? taskQA.refined_test_steps;
  const screenshots = taskQA.screenshots ?? [];
  const isFailed = results?.overall_status === "failed";

  return (
    <div className="flex flex-col h-full">
      {/* Tab List */}
      <div
        ref={tablistRef}
        role="tablist"
        aria-label="QA Information"
        className="flex gap-1 border-b border-[--border-subtle] mb-4"
      >
        <TabButton
          id="criteria"
          label="Acceptance Criteria"
          count={criteria.length > 0 ? criteria.length : undefined}
          countTestId="criteria-count"
          isSelected={activeTab === "criteria"}
          onClick={setActiveTab}
          onKeyDown={handleTabKeyDown}
        />
        <TabButton
          id="results"
          label="Test Results"
          count={results?.total_steps}
          countTestId="results-count"
          isSelected={activeTab === "results"}
          onClick={setActiveTab}
          onKeyDown={handleTabKeyDown}
        />
        <TabButton
          id="screenshots"
          label="Screenshots"
          count={screenshots.length > 0 ? screenshots.length : undefined}
          countTestId="screenshots-count"
          isSelected={activeTab === "screenshots"}
          onClick={setActiveTab}
          onKeyDown={handleTabKeyDown}
        />
      </div>

      {/* Tab Panels */}
      <div
        role="tabpanel"
        id={`tabpanel-${activeTab}`}
        aria-labelledby={`tab-${activeTab}`}
        className="flex-1 overflow-auto"
      >
        {activeTab === "criteria" && (
          <AcceptanceCriteriaTab
            criteria={criteria}
            testSteps={testSteps}
            results={results}
          />
        )}
        {activeTab === "results" && (
          <TestResultsTab
            testSteps={testSteps}
            results={results}
            onScreenshotClick={(path) => {
              const index = screenshots.indexOf(path);
              if (index !== -1) setLightboxIndex(index);
            }}
          />
        )}
        {activeTab === "screenshots" && (
          <ScreenshotsTab
            screenshots={screenshots}
            onThumbnailClick={setLightboxIndex}
          />
        )}
      </div>

      {/* Action Buttons */}
      {isFailed && onRetry && onSkip && (
        <div className="flex gap-2 mt-4 pt-4 border-t border-[--border-subtle]">
          <button
            onClick={onRetry}
            disabled={isRetrying || isSkipping}
            className="px-4 py-2 text-sm font-medium rounded bg-[--accent-primary] text-[--bg-base] hover:opacity-90 disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {isRetrying ? "Retrying..." : "Retry QA"}
          </button>
          <button
            onClick={onSkip}
            disabled={isRetrying || isSkipping}
            className="px-4 py-2 text-sm font-medium rounded bg-[--bg-hover] text-[--text-primary] hover:bg-[--bg-elevated] disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {isSkipping ? "Skipping..." : "Skip QA"}
          </button>
        </div>
      )}

      {/* Lightbox */}
      {lightboxIndex !== null && (
        <Lightbox
          screenshots={screenshots}
          currentIndex={lightboxIndex}
          onClose={() => setLightboxIndex(null)}
          onPrev={() =>
            setLightboxIndex((prev) => Math.max(0, (prev ?? 0) - 1))
          }
          onNext={() =>
            setLightboxIndex((prev) =>
              Math.min(screenshots.length - 1, (prev ?? 0) + 1)
            )
          }
        />
      )}
    </div>
  );
}
