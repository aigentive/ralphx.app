import type { Task } from "@/types/task";

/**
 * Create a sample task for testing TaskDetailModal
 */
export function createMockTask(overrides?: Partial<Task>): Task {
  const now = new Date().toISOString();
  return {
    id: "test-task-001",
    projectId: "test-project-001",
    category: "feature",
    title: "Implement user authentication",
    description: "Add JWT-based authentication to the application. This includes login, logout, and token refresh flows.",
    priority: 100,
    internalStatus: "ready",
    needsReviewPoint: true,
    createdAt: now,
    updatedAt: now,
    startedAt: null,
    completedAt: null,
    archivedAt: null,
    blockedReason: null,
    sourceProposalId: null,
    planArtifactId: null,
    ...overrides,
  };
}

/**
 * Create an archived task
 */
export function createArchivedTask(overrides?: Partial<Task>): Task {
  const now = new Date().toISOString();
  return createMockTask({
    internalStatus: "approved",
    archivedAt: now,
    completedAt: now,
    ...overrides,
  });
}

/**
 * Create a task with context (from proposal/plan)
 */
export function createTaskWithContext(overrides?: Partial<Task>): Task {
  return createMockTask({
    sourceProposalId: "proposal-123",
    planArtifactId: "plan-456",
    ...overrides,
  });
}

/**
 * Create an executing task (system-controlled)
 */
export function createExecutingTask(overrides?: Partial<Task>): Task {
  const now = new Date().toISOString();
  return createMockTask({
    internalStatus: "executing",
    startedAt: now,
    ...overrides,
  });
}
