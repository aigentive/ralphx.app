import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { PermissionDialog } from "./PermissionDialog";
import type { PermissionRequest } from "@/types/permission";

const mockSubscribe = vi.fn();

vi.mock("@/providers/EventProvider", () => ({
  useEventBus: () => ({
    subscribe: mockSubscribe,
    emit: vi.fn(),
  }),
}));

vi.mock("@/lib/tauri", () => ({
  api: {
    permission: {
      resolveRequest: vi.fn(),
    },
  },
}));

vi.mock("sonner", () => ({
  toast: { error: vi.fn() },
}));

vi.mock("@/stores/taskStore", () => ({
  useTaskStore: {
    getState: () => ({ tasks: { "task-abc": { title: "My Task" } } }),
  },
}));

import { api } from "@/lib/tauri";
import { toast } from "sonner";

const mockResolveRequest = vi.mocked(api.permission.resolveRequest);
const mockToastError = vi.mocked(toast.error);

describe("PermissionDialog", () => {
  let eventCallback: ((payload: PermissionRequest) => void) | null = null;
  let unlistenFn: (() => void) | null = null;

  beforeEach(() => {
    unlistenFn = vi.fn();
    mockSubscribe.mockImplementation((eventName: string, callback: (payload: PermissionRequest) => void) => {
      if (eventName === "permission:request" || eventName === "permission-request") {
        eventCallback = callback;
      }
      return unlistenFn;
    });
    mockResolveRequest.mockResolvedValue(undefined);
  });

  afterEach(() => {
    vi.clearAllMocks();
    eventCallback = null;
    unlistenFn = null;
  });

  it("renders nothing when no requests", () => {
    const { container } = render(<PermissionDialog />);
    expect(container).toBeEmptyDOMElement();
  });

  it("listens to permission:request events on mount", () => {
    render(<PermissionDialog />);
    expect(mockSubscribe).toHaveBeenCalledWith("permission:request", expect.any(Function));
  });

  it("shows dialog when permission request received", async () => {
    render(<PermissionDialog />);

    const request: PermissionRequest = {
      request_id: "test-123",
      tool_name: "Bash",
      tool_input: { command: "ls -la" },
    };

    // Trigger event
    eventCallback?.(request);

    await waitFor(() => {
      expect(screen.getByText("Permission Required")).toBeInTheDocument();
    });

    expect(screen.getByText("Bash")).toBeInTheDocument();
    expect(screen.getByText("ls -la")).toBeInTheDocument();
  });

  it("displays tool name and formatted input for Bash", async () => {
    render(<PermissionDialog />);

    const request: PermissionRequest = {
      request_id: "test-123",
      tool_name: "Bash",
      tool_input: { command: "echo hello" },
    };

    eventCallback?.(request);

    await waitFor(() => {
      expect(screen.getByText("echo hello")).toBeInTheDocument();
    });
  });

  it("displays formatted input for Write tool", async () => {
    render(<PermissionDialog />);

    const request: PermissionRequest = {
      request_id: "test-123",
      tool_name: "Write",
      tool_input: {
        file_path: "/tmp/test.txt",
        content: "Hello world!",
      },
    };

    eventCallback?.(request);

    await waitFor(() => {
      expect(screen.getByText(/Write to: \/tmp\/test.txt/)).toBeInTheDocument();
      expect(screen.getByText(/Hello world!/)).toBeInTheDocument();
    });
  });

  it("truncates long Write content", async () => {
    render(<PermissionDialog />);

    const longContent = "a".repeat(300);
    const request: PermissionRequest = {
      request_id: "test-123",
      tool_name: "Write",
      tool_input: {
        file_path: "/tmp/test.txt",
        content: longContent,
      },
    };

    eventCallback?.(request);

    await waitFor(() => {
      expect(screen.getByText(/\.\.\./)).toBeInTheDocument();
    });
  });

  it("displays formatted input for Edit tool", async () => {
    render(<PermissionDialog />);

    const request: PermissionRequest = {
      request_id: "test-123",
      tool_name: "Edit",
      tool_input: {
        file_path: "/tmp/test.txt",
        old_string: "old value",
        new_string: "new value",
      },
    };

    eventCallback?.(request);

    await waitFor(() => {
      expect(screen.getByText(/Edit: \/tmp\/test.txt/)).toBeInTheDocument();
      expect(screen.getByText(/old value/)).toBeInTheDocument();
      expect(screen.getByText(/new value/)).toBeInTheDocument();
    });
  });

  it("displays formatted input for Read tool", async () => {
    render(<PermissionDialog />);

    const request: PermissionRequest = {
      request_id: "test-123",
      tool_name: "Read",
      tool_input: {
        file_path: "/tmp/test.txt",
      },
    };

    eventCallback?.(request);

    await waitFor(() => {
      expect(screen.getByText("Read: /tmp/test.txt")).toBeInTheDocument();
    });
  });

  it("displays context when provided", async () => {
    render(<PermissionDialog />);

    const request: PermissionRequest = {
      request_id: "test-123",
      tool_name: "Bash",
      tool_input: { command: "ls" },
      context: "Listing directory contents",
    };

    eventCallback?.(request);

    await waitFor(() => {
      expect(screen.getByText("Listing directory contents")).toBeInTheDocument();
    });
  });

  it("shows queue count when multiple requests pending", async () => {
    render(<PermissionDialog />);

    const request1: PermissionRequest = {
      request_id: "test-1",
      tool_name: "Bash",
      tool_input: { command: "ls" },
    };

    const request2: PermissionRequest = {
      request_id: "test-2",
      tool_name: "Read",
      tool_input: { file_path: "/tmp/test.txt" },
    };

    eventCallback?.(request1);
    eventCallback?.(request2);

    await waitFor(() => {
      expect(screen.getByText("+1 more permission request(s) waiting")).toBeInTheDocument();
    });
  });

  it("calls resolve_permission_request with allow on Allow button click", async () => {
    const user = userEvent.setup();
    render(<PermissionDialog />);

    const request: PermissionRequest = {
      request_id: "test-123",
      tool_name: "Bash",
      tool_input: { command: "ls" },
    };

    eventCallback?.(request);

    await waitFor(() => {
      expect(screen.getByText("Allow")).toBeInTheDocument();
    });

    await user.click(screen.getByText("Allow"));

    expect(mockResolveRequest).toHaveBeenCalledWith({
      requestId: "test-123",
      decision: "allow",
    });
  });

  it("calls resolve_permission_request with deny on Deny button click", async () => {
    const user = userEvent.setup();
    render(<PermissionDialog />);

    const request: PermissionRequest = {
      request_id: "test-123",
      tool_name: "Bash",
      tool_input: { command: "ls" },
    };

    eventCallback?.(request);

    await waitFor(() => {
      expect(screen.getByText("Deny")).toBeInTheDocument();
    });

    await user.click(screen.getByText("Deny"));

    expect(mockResolveRequest).toHaveBeenCalledWith({
      requestId: "test-123",
      decision: "deny",
      message: "User denied permission",
    });
  });

  it("removes request from queue after decision", async () => {
    const user = userEvent.setup();
    render(<PermissionDialog />);

    const request: PermissionRequest = {
      request_id: "test-123",
      tool_name: "Bash",
      tool_input: { command: "ls" },
    };

    eventCallback?.(request);

    await waitFor(() => {
      expect(screen.getByText("Allow")).toBeInTheDocument();
    });

    await user.click(screen.getByText("Allow"));

    await waitFor(() => {
      expect(screen.queryByText("Permission Required")).not.toBeInTheDocument();
    });
  });

  it("shows next request after resolving first", async () => {
    const user = userEvent.setup();
    render(<PermissionDialog />);

    const request1: PermissionRequest = {
      request_id: "test-1",
      tool_name: "Bash",
      tool_input: { command: "ls" },
    };

    const request2: PermissionRequest = {
      request_id: "test-2",
      tool_name: "Read",
      tool_input: { file_path: "/tmp/test.txt" },
    };

    eventCallback?.(request1);
    eventCallback?.(request2);

    await waitFor(() => {
      expect(screen.getByText("ls")).toBeInTheDocument();
    });

    await user.click(screen.getByText("Allow"));

    await waitFor(() => {
      expect(screen.getByText("Read: /tmp/test.txt")).toBeInTheDocument();
    });
  });

  it("treats dialog close as deny", async () => {
    const user = userEvent.setup();
    render(<PermissionDialog />);

    const request: PermissionRequest = {
      request_id: "test-123",
      tool_name: "Bash",
      tool_input: { command: "ls" },
    };

    eventCallback?.(request);

    await waitFor(() => {
      expect(screen.getByTestId("dialog-close")).toBeInTheDocument();
    });

    await user.click(screen.getByTestId("dialog-close"));

    expect(mockResolveRequest).toHaveBeenCalledWith({
      requestId: "test-123",
      decision: "deny",
      message: "User denied permission",
    });
  });

  it("cleans up event listener on unmount", async () => {
    const { unmount } = render(<PermissionDialog />);

    // Wait for the listener to be set up
    await waitFor(() => {
      expect(mockSubscribe).toHaveBeenCalled();
    });

    unmount();

    // The unlisten function is called asynchronously in the cleanup
    await waitFor(() => {
      expect(unlistenFn).toHaveBeenCalled();
    });
  });

  // Identity UI tests

  it("shows identity row when agent_type provided", async () => {
    render(<PermissionDialog />);

    const request: PermissionRequest = {
      request_id: "test-id",
      tool_name: "Bash",
      tool_input: { command: "ls" },
      agent_type: "ralphx-worker",
    };

    eventCallback?.(request);

    await waitFor(() => {
      expect(screen.getByText("Worker")).toBeInTheDocument();
    });
  });

  it("shows context label when context_type provided", async () => {
    render(<PermissionDialog />);

    const request: PermissionRequest = {
      request_id: "test-id",
      tool_name: "Bash",
      tool_input: { command: "ls" },
      context_type: "task_execution",
    };

    eventCallback?.(request);

    await waitFor(() => {
      expect(screen.getByText("Executing")).toBeInTheDocument();
    });
  });

  it("hides identity row when no identity fields", async () => {
    render(<PermissionDialog />);

    const request: PermissionRequest = {
      request_id: "test-id",
      tool_name: "Bash",
      tool_input: { command: "ls" },
    };

    eventCallback?.(request);

    await waitFor(() => {
      expect(screen.getByText("Permission Required")).toBeInTheDocument();
    });

    // None of the identity-specific labels should appear
    expect(screen.queryByText("Worker")).not.toBeInTheDocument();
    expect(screen.queryByText("Executing")).not.toBeInTheDocument();
    expect(screen.queryByText(/^Task:/)).not.toBeInTheDocument();
  });

  it("buttons disabled while resolving", async () => {
    mockResolveRequest.mockImplementation(() => new Promise(() => {}));

    const user = userEvent.setup();
    render(<PermissionDialog />);

    const request: PermissionRequest = {
      request_id: "test-id",
      tool_name: "Bash",
      tool_input: { command: "ls" },
    };

    eventCallback?.(request);

    await waitFor(() => {
      expect(screen.getByText("Allow")).toBeInTheDocument();
    });

    // Click Allow to start resolving (will never complete due to infinite promise)
    await user.click(screen.getByText("Allow"));

    await waitFor(() => {
      expect(screen.getByText("Allow").closest("button")).toBeDisabled();
      expect(screen.getByText("Deny").closest("button")).toBeDisabled();
    });
  });

  it("shows toast on resolveRequest failure and keeps request in queue", async () => {
    mockResolveRequest.mockRejectedValue(new Error("Network error"));

    const user = userEvent.setup();
    render(<PermissionDialog />);

    const request: PermissionRequest = {
      request_id: "test-id",
      tool_name: "Bash",
      tool_input: { command: "ls" },
    };

    eventCallback?.(request);

    await waitFor(() => {
      expect(screen.getByText("Allow")).toBeInTheDocument();
    });

    await user.click(screen.getByText("Allow"));

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith("Failed to resolve permission request");
    });

    // Dialog should still be visible (request stays in queue)
    expect(screen.getByText("Permission Required")).toBeInTheDocument();
  });

  it("dialog close guard blocks close when resolving", async () => {
    // Never-resolving promise to keep resolving=true
    mockResolveRequest.mockImplementation(() => new Promise(() => {}));

    const user = userEvent.setup();
    render(<PermissionDialog />);

    const request: PermissionRequest = {
      request_id: "test-id",
      tool_name: "Bash",
      tool_input: { command: "ls" },
    };

    eventCallback?.(request);

    await waitFor(() => {
      expect(screen.getByText("Allow")).toBeInTheDocument();
    });

    // Click Allow — starts resolving but never completes
    await user.click(screen.getByText("Allow"));

    await waitFor(() => {
      expect(screen.getByText("Allow").closest("button")).toBeDisabled();
    });

    // resolveRequest was called once for "allow"
    expect(mockResolveRequest).toHaveBeenCalledTimes(1);

    // Clicking the close button while resolving should NOT trigger a second resolve call
    const closeButton = screen.queryByTestId("dialog-close");
    if (closeButton) {
      await user.click(closeButton);
      // Still only called once — the close guard blocked it
      expect(mockResolveRequest).toHaveBeenCalledTimes(1);
    }
  });
});
