import { Page } from "@playwright/test";
import type { PermissionRequest } from "@/types/permission";

/**
 * Trigger a PermissionDialog in web mode by emitting a permission:request event
 *
 * In web mode, the app uses MockEventBus which is accessible via window.__eventBus.
 * This helper emits the event to trigger the modal programmatically.
 */
export async function triggerPermissionDialog(
  page: Page,
  request: PermissionRequest
): Promise<void> {
  await page.evaluate((payload) => {
    // Access the global event bus (set by EventProvider in web mode)
    const eventBus = (window as any).__eventBus;
    if (eventBus && typeof eventBus.emit === "function") {
      eventBus.emit("permission:request", payload);
    } else {
      throw new Error("EventBus not available. Make sure app is running in web mode.");
    }
  }, request);

  // Wait a small amount of time for React to process the event and update state
  await page.waitForTimeout(100);
}

/**
 * Create a sample Bash permission request for testing
 */
export function createBashPermissionRequest(): PermissionRequest {
  return {
    request_id: "perm-bash-123",
    tool_name: "Bash",
    tool_input: {
      command: 'git commit -m "feat: add new feature"',
      description: "Commit changes to repository",
    },
    context: "Agent needs to commit code changes",
  };
}

/**
 * Create a sample Write permission request for testing
 */
export function createWritePermissionRequest(): PermissionRequest {
  return {
    request_id: "perm-write-456",
    tool_name: "Write",
    tool_input: {
      file_path: "/path/to/new-file.ts",
      content: "export function example() {\n  return 'Hello, World!';\n}\n",
    },
    context: "Agent needs to create a new utility file",
  };
}

/**
 * Create a sample Edit permission request for testing
 */
export function createEditPermissionRequest(): PermissionRequest {
  return {
    request_id: "perm-edit-789",
    tool_name: "Edit",
    tool_input: {
      file_path: "/path/to/existing-file.ts",
      old_string: "const value = 'old';",
      new_string: "const value = 'new';",
    },
    context: "Agent needs to update a configuration value",
  };
}

/**
 * Create a sample Read permission request for testing
 */
export function createReadPermissionRequest(): PermissionRequest {
  return {
    request_id: "perm-read-012",
    tool_name: "Read",
    tool_input: {
      file_path: "/path/to/sensitive-file.env",
    },
    context: "Agent needs to read environment configuration",
  };
}

/**
 * Create a sample Write permission request with long content for truncation testing
 */
export function createLongContentPermissionRequest(): PermissionRequest {
  const longContent = "a".repeat(500); // 500 characters, should be truncated at 200
  return {
    request_id: "perm-write-long-345",
    tool_name: "Write",
    tool_input: {
      file_path: "/path/to/large-file.txt",
      content: longContent,
    },
    context: "Agent needs to create a file with long content",
  };
}
