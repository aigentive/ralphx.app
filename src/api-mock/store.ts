/**
 * In-memory mock data store
 *
 * Provides a centralized store for mock data used by the mock API.
 * Data is seeded on initialization and can be queried/modified by mock API functions.
 * Resets to seed data on page reload (no persistence).
 */

import type { Task, InternalStatus } from "@/types/task";
import type { Project } from "@/types/project";
import type { TaskStep } from "@/types/task-step";
import type { ChatConversation } from "@/types/chat-conversation";
import {
  createMockTask,
  createMockProject,
  generateTestUuid,
} from "@/test/mock-data";

// ============================================================================
// Store State
// ============================================================================

interface MockStore {
  projects: Map<string, Project>;
  tasks: Map<string, Task>;
  taskSteps: Map<string, TaskStep[]>;
  conversations: Map<string, ChatConversation>;
  initialized: boolean;
}

const store: MockStore = {
  projects: new Map(),
  tasks: new Map(),
  taskSteps: new Map(),
  conversations: new Map(),
  initialized: false,
};

// ============================================================================
// Seed Data
// ============================================================================

function seedMockData(): void {
  if (store.initialized) return;

  // Create a demo project
  const project = createMockProject({
    id: "project-mock-1",
    name: "Demo Project",
    workingDirectory: "/demo/project",
  });
  store.projects.set(project.id, project);

  // Create tasks in various states for visual testing
  // Using actual InternalStatus values
  const statuses: { status: InternalStatus; title: string }[] = [
    { status: "backlog", title: "Backlog Task" },
    { status: "ready", title: "Ready Task" },
    { status: "blocked", title: "Blocked Task" },
    { status: "executing", title: "Executing Task" },
    { status: "pending_review", title: "Pending Review Task" },
    { status: "review_passed", title: "Review Passed Task" },
    { status: "approved", title: "Approved Task" },
  ];

  statuses.forEach(({ status, title }, idx) => {
    const task = createMockTask({
      id: `task-mock-${idx + 1}`,
      projectId: project.id,
      title,
      description: `A sample task in ${status} status for visual testing`,
      internalStatus: status,
      priority: idx,
      blockedReason: status === "blocked" ? "Waiting for dependencies" : null,
    });
    store.tasks.set(task.id, task);

    // Add some steps for tasks in active states
    if (["executing", "pending_review"].includes(status)) {
      const steps: TaskStep[] = [
        {
          id: generateTestUuid(),
          taskId: task.id,
          title: "Setup environment",
          description: "Configure dev environment",
          status: "completed",
          sortOrder: 0,
          dependsOn: null,
          createdBy: "worker",
          completionNote: "Environment ready",
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
          startedAt: new Date().toISOString(),
          completedAt: new Date().toISOString(),
        },
        {
          id: generateTestUuid(),
          taskId: task.id,
          title: "Implement feature",
          description: "Write the main feature code",
          status: status === "pending_review" ? "completed" : "in_progress",
          sortOrder: 1,
          dependsOn: null,
          createdBy: "worker",
          completionNote: null,
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
          startedAt: new Date().toISOString(),
          completedAt: status === "pending_review" ? new Date().toISOString() : null,
        },
        {
          id: generateTestUuid(),
          taskId: task.id,
          title: "Write tests",
          description: "Add unit and integration tests",
          status: "pending",
          sortOrder: 2,
          dependsOn: null,
          createdBy: "worker",
          completionNote: null,
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
          startedAt: null,
          completedAt: null,
        },
      ];
      store.taskSteps.set(task.id, steps);
    }
  });

  // Add a few extra tasks for better visual representation
  for (let i = 0; i < 3; i++) {
    const task = createMockTask({
      id: `task-mock-extra-${i + 1}`,
      projectId: project.id,
      title: `Additional Task ${i + 1}`,
      description: "Extra task for visual density",
      internalStatus: "backlog",
      priority: 10 + i,
      blockedReason: null,
    });
    store.tasks.set(task.id, task);
  }

  store.initialized = true;
}

// ============================================================================
// Store Access Functions
// ============================================================================

export function getStore(): MockStore {
  if (!store.initialized) {
    seedMockData();
  }

  // Expose store to window in web mode for Playwright testing
  if (typeof window !== 'undefined') {
    window.__mockStore = store;
  }

  return store;
}

export function resetStore(): void {
  store.projects.clear();
  store.tasks.clear();
  store.taskSteps.clear();
  store.conversations.clear();
  store.initialized = false;
  seedMockData();
}
