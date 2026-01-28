/**
 * TaskDetailQAPanel - Tabbed panel showing QA data for a task
 *
 * Premium design with shadcn Tabs and Lucide icons:
 * - Acceptance Criteria tab with checkmarks
 * - Test Results tab with pass/fail icons
 * - Screenshots tab with thumbnail gallery and lightbox
 */

import type {
  TaskQAResponse,
  QAResultsResponse,
  AcceptanceCriterionResponse,
  QATestStepResponse,
} from "@/lib/tauri";
import type { QAStepStatus, QAStepResult } from "@/types/qa";
import {
  CheckCircle,
  XCircle,
  Circle,
  MinusCircle,
  Image,
  Loader2,
  RotateCcw,
  SkipForward,
} from "lucide-react";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { cn } from "@/lib/utils";
import {
  ScreenshotGallery,
  type Screenshot,
} from "./ScreenshotGallery";
import { pathsToScreenshots } from "./ScreenshotGallery/utils";

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
// Helper Functions
// ============================================================================

function getStatusIcon(status: QAStepStatus | "pending") {
  switch (status) {
    case "passed":
      return <CheckCircle className="w-4 h-4 text-[var(--status-success)]" />;
    case "failed":
      return <XCircle className="w-4 h-4 text-[var(--status-error)]" />;
    case "skipped":
      return <MinusCircle className="w-4 h-4 text-[var(--text-muted)]" />;
    case "running":
      return <Circle className="w-4 h-4 text-[var(--status-info)] animate-pulse" />;
    case "pending":
    default:
      return <Circle className="w-4 h-4 text-[var(--text-muted)]" />;
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

// ============================================================================
// Skeleton Component
// ============================================================================

function QAPanelSkeleton() {
  return (
    <div data-testid="qa-panel-skeleton" className="space-y-4">
      <div className="flex gap-2 border-b border-[var(--border-subtle)] pb-2">
        <Skeleton className="h-9 w-36" />
        <Skeleton className="h-9 w-28" />
        <Skeleton className="h-9 w-28" />
      </div>
      <div className="space-y-3">
        <Skeleton className="h-16 w-full" />
        <Skeleton className="h-16 w-full" />
        <Skeleton className="h-16 w-full" />
      </div>
    </div>
  );
}

// ============================================================================
// Tab Trigger with Count
// ============================================================================

interface TabTriggerWithCountProps {
  value: TabId;
  label: string;
  count?: number | undefined;
  countTestId?: string | undefined;
}

function TabTriggerWithCount({
  value,
  label,
  count,
  countTestId,
}: TabTriggerWithCountProps) {
  return (
    <TabsTrigger
      value={value}
      className={cn(
        "px-3 py-2 text-sm font-medium transition-all",
        "text-[var(--text-secondary)] hover:text-[var(--text-primary)]",
        "data-[state=active]:text-[var(--text-primary)]",
        "data-[state=active]:border-b-2 data-[state=active]:border-[var(--accent-primary)]",
        "data-[state=active]:shadow-none data-[state=active]:bg-transparent",
        "focus-visible:ring-2 focus-visible:ring-[var(--accent-primary)] focus-visible:ring-offset-2"
      )}
    >
      {label}
      {count !== undefined && (
        <span
          data-testid={countTestId}
          className="ml-1.5 text-xs opacity-80"
        >
          ({count})
        </span>
      )}
    </TabsTrigger>
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
      <div className="py-8 text-center text-[var(--text-muted)] text-sm">
        No acceptance criteria defined
      </div>
    );
  }

  return (
    <div className="space-y-2">
      {criteria.map((criterion) => {
        const status = getCriterionStatus(criterion.id, testSteps, results);

        return (
          <div
            key={criterion.id}
            className="flex items-start gap-3 p-3 rounded-lg bg-[var(--bg-elevated)]"
          >
            <span
              data-testid={`criterion-status-${criterion.id}`}
              data-status={status}
              className="flex-shrink-0 mt-0.5"
            >
              {getStatusIcon(status)}
            </span>
            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-1.5 mb-1.5">
                <Badge
                  variant="outline"
                  className="px-1.5 py-0.5 text-xs font-medium bg-[var(--bg-hover)] text-[var(--text-secondary)] border-0"
                >
                  {criterion.id}
                </Badge>
                <Badge
                  variant="outline"
                  className="px-1.5 py-0.5 text-xs bg-[var(--bg-hover)] text-[var(--text-muted)] border-0"
                >
                  {criterion.criteria_type}
                </Badge>
                {criterion.testable && (
                  <Badge
                    data-testid={`criterion-testable-${criterion.id}`}
                    variant="outline"
                    className="px-1.5 py-0.5 text-xs bg-blue-500/15 text-[var(--status-info)] border-0"
                  >
                    testable
                  </Badge>
                )}
              </div>
              <p className="text-sm text-[var(--text-primary)] leading-normal">
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
}

function TestResultsTab({
  testSteps,
  results,
}: TestResultsTabProps) {
  if (!results) {
    return (
      <div className="py-8 text-center text-[var(--text-muted)] text-sm">
        No test results available yet
      </div>
    );
  }

  // Create a map of step ID to step for quick lookup
  const stepMap = new Map(testSteps?.map((s) => [s.id, s]) ?? []);

  const overallStatusClass = results.overall_status === "passed"
    ? "bg-emerald-500/15 text-[var(--status-success)]"
    : results.overall_status === "failed"
      ? "bg-red-500/15 text-[var(--status-error)]"
      : "bg-[var(--bg-hover)] text-[var(--text-muted)]";

  return (
    <div className="space-y-4">
      {/* Overall Status Banner */}
      <div className="flex items-center justify-between p-4 rounded-lg bg-[var(--bg-elevated)]">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium text-[var(--text-secondary)]">
            Overall:
          </span>
          <Badge
            data-testid="overall-status"
            variant="outline"
            className={cn("capitalize border-0", overallStatusClass)}
          >
            {results.overall_status}
          </Badge>
        </div>
        <span
          data-testid="results-summary"
          className="text-sm text-[var(--text-muted)]"
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
              className="p-3 rounded-lg bg-[var(--bg-elevated)]"
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
                    <Badge
                      variant="outline"
                      className="px-1.5 py-0.5 text-xs font-medium bg-[var(--bg-hover)] text-[var(--text-secondary)] border-0"
                    >
                      {stepResult.step_id}
                    </Badge>
                    {stepResult.screenshot && (
                      <span
                        data-testid={`step-screenshot-indicator-${stepResult.step_id}`}
                        className="flex items-center gap-1 text-xs text-[var(--text-muted)]"
                      >
                        <Image className="w-3 h-3" />
                        Has screenshot
                      </span>
                    )}
                  </div>
                  <p className="text-sm text-[var(--text-primary)]">
                    {step?.description ?? stepResult.step_id}
                  </p>
                </div>
              </div>

              {/* Failure Details Box */}
              {isFailed && (stepResult.expected || stepResult.actual || stepResult.error) && (
                <div
                  data-testid={`failure-details-${stepResult.step_id}`}
                  className="mt-3 ml-7 p-2 rounded-lg bg-[var(--bg-base)] border border-red-500/30"
                >
                  {stepResult.error && (
                    <p
                      data-testid={`error-message-${stepResult.step_id}`}
                      className="text-xs text-[var(--status-error)] mb-2"
                    >
                      {stepResult.error}
                    </p>
                  )}
                  {(stepResult.expected || stepResult.actual) && (
                    <div className="grid grid-cols-2 gap-2 text-xs">
                      {stepResult.expected && (
                        <div>
                          <span className="text-[var(--text-muted)]">Expected:</span>
                          <p className="text-[var(--text-primary)] mt-0.5">
                            {stepResult.expected}
                          </p>
                        </div>
                      )}
                      {stepResult.actual && (
                        <div>
                          <span className="text-[var(--text-muted)]">Actual:</span>
                          <p className="text-[var(--text-primary)] mt-0.5">
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
  screenshots: Screenshot[];
}

function ScreenshotsTab({ screenshots }: ScreenshotsTabProps) {
  return (
    <ScreenshotGallery
      screenshots={screenshots}
      columns={3}
      emptyMessage="No screenshots captured"
    />
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
  // Show loading skeleton
  if (isLoading) {
    return <QAPanelSkeleton />;
  }

  // Show empty state
  if (!taskQA) {
    return (
      <div className="text-center py-8 text-[var(--text-muted)]">
        <p>No QA data available for this task</p>
      </div>
    );
  }

  const criteria = taskQA.acceptance_criteria ?? [];
  const testSteps = taskQA.qa_test_steps ?? taskQA.refined_test_steps;
  const screenshotPaths = taskQA.screenshots ?? [];
  const isFailed = results?.overall_status === "failed";

  // Create a map of step results for matching with screenshots
  const stepResultsMap = new Map<string, QAStepResult>();
  if (results?.steps) {
    for (const step of results.steps) {
      stepResultsMap.set(step.step_id, step);
    }
  }

  // Convert screenshot paths to Screenshot objects with step result context
  const screenshots: Screenshot[] = pathsToScreenshots(
    screenshotPaths,
    stepResultsMap
  );

  return (
    <div className="flex flex-col h-full min-h-[300px]">
      <Tabs defaultValue="criteria" className="flex flex-col h-full">
        {/* Tab List with underline indicator style */}
        <TabsList className="h-auto bg-transparent p-0 gap-1 border-b border-[var(--border-subtle)] mb-4 rounded-none justify-start">
          <TabTriggerWithCount
            value="criteria"
            label="Acceptance Criteria"
            count={criteria.length > 0 ? criteria.length : undefined}
            countTestId="criteria-count"
          />
          <TabTriggerWithCount
            value="results"
            label="Test Results"
            count={results?.total_steps}
            countTestId="results-count"
          />
          <TabTriggerWithCount
            value="screenshots"
            label="Screenshots"
            count={screenshotPaths.length > 0 ? screenshotPaths.length : undefined}
            countTestId="screenshots-count"
          />
        </TabsList>

        {/* Tab Panels */}
        <TabsContent value="criteria" className="flex-1 overflow-auto mt-0">
          <AcceptanceCriteriaTab
            criteria={criteria}
            testSteps={testSteps}
            results={results}
          />
        </TabsContent>
        <TabsContent value="results" className="flex-1 overflow-auto mt-0">
          <TestResultsTab
            testSteps={testSteps}
            results={results}
          />
        </TabsContent>
        <TabsContent value="screenshots" className="flex-1 overflow-auto mt-0">
          <ScreenshotsTab screenshots={screenshots} />
        </TabsContent>
      </Tabs>

      {/* Action Buttons */}
      {isFailed && onRetry && onSkip && (
        <div className="flex gap-2 mt-4 pt-4 border-t border-[var(--border-subtle)]">
          <Button
            onClick={onRetry}
            disabled={isRetrying || isSkipping}
            size="sm"
            className="gap-1.5"
          >
            {isRetrying ? (
              <Loader2 className="w-4 h-4 animate-spin" />
            ) : (
              <RotateCcw className="w-4 h-4" />
            )}
            {isRetrying ? "Retrying..." : "Retry QA"}
          </Button>
          <Button
            onClick={onSkip}
            disabled={isRetrying || isSkipping}
            variant="secondary"
            size="sm"
            className="gap-1.5"
          >
            {isSkipping ? (
              <Loader2 className="w-4 h-4 animate-spin" />
            ) : (
              <SkipForward className="w-4 h-4" />
            )}
            {isSkipping ? "Skipping..." : "Skip QA"}
          </Button>
        </div>
      )}
    </div>
  );
}
