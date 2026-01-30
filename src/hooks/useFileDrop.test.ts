/**
 * useFileDrop hook tests
 *
 * Tests the Tauri-based file drop hook.
 * The hook uses Tauri's onDragDropEvent for actual file handling
 * and HTML5 drag events for compatibility (though they can't access file content in Tauri).
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, act, waitFor } from "@testing-library/react";
import { useFileDrop, type FileDropConfig } from "./useFileDrop";

// Mock Tauri APIs
const mockOnDragDropEvent = vi.fn();
const mockReadTextFile = vi.fn();
let dragDropHandler: ((event: { payload: { type: string; paths?: string[]; position?: { x: number; y: number } } }) => void) | null = null;

vi.mock("@tauri-apps/api/webview", () => ({
  getCurrentWebview: () => ({
    onDragDropEvent: (handler: typeof dragDropHandler) => {
      dragDropHandler = handler;
      mockOnDragDropEvent(handler);
      return Promise.resolve(() => {
        dragDropHandler = null;
      });
    },
  }),
}));

vi.mock("@tauri-apps/plugin-fs", () => ({
  readTextFile: (path: string) => mockReadTextFile(path),
}));

// Helper to create a mock DragEvent (for HTML5 compatibility layer)
function createDragEvent(
  type: "dragenter" | "dragover" | "dragleave" | "drop" = "dragenter"
): React.DragEvent {
  return {
    preventDefault: vi.fn(),
    stopPropagation: vi.fn(),
    dataTransfer: {
      files: { length: 0 } as FileList,
    },
    type,
  } as unknown as React.DragEvent;
}

// Helper to simulate Tauri drag-drop events
function simulateTauriDragEvent(
  type: "enter" | "over" | "drop" | "leave",
  paths: string[] = [],
  position = { x: 100, y: 100 }
) {
  if (dragDropHandler) {
    const payload: { type: string; paths?: string[]; position?: { x: number; y: number } } = { type };
    if (type === "enter" || type === "drop") {
      payload.paths = paths;
      payload.position = position;
    } else if (type === "over") {
      payload.position = position;
    }
    dragDropHandler({ payload });
  }
}

describe("useFileDrop", () => {
  let onFileDrop: ReturnType<typeof vi.fn>;
  let onError: ReturnType<typeof vi.fn>;
  let defaultConfig: FileDropConfig;

  beforeEach(() => {
    vi.clearAllMocks();
    dragDropHandler = null;
    onFileDrop = vi.fn();
    onError = vi.fn();
    defaultConfig = {
      acceptedExtensions: [".md"],
      onFileDrop,
      onError,
    };
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  describe("initial state", () => {
    it("should start with isDragging=false", () => {
      const { result } = renderHook(() => useFileDrop(defaultConfig));
      expect(result.current.isDragging).toBe(false);
    });

    it("should start with no error", () => {
      const { result } = renderHook(() => useFileDrop(defaultConfig));
      expect(result.current.error).toBeNull();
    });

    it("should return dropProps with all event handlers", () => {
      const { result } = renderHook(() => useFileDrop(defaultConfig));
      expect(result.current.dropProps).toHaveProperty("onDragEnter");
      expect(result.current.dropProps).toHaveProperty("onDragOver");
      expect(result.current.dropProps).toHaveProperty("onDragLeave");
      expect(result.current.dropProps).toHaveProperty("onDrop");
    });

    it("should set up Tauri drag-drop listener on mount", async () => {
      renderHook(() => useFileDrop(defaultConfig));
      await waitFor(() => {
        expect(mockOnDragDropEvent).toHaveBeenCalled();
      });
    });

    it("should not set up listener when disabled", async () => {
      renderHook(() => useFileDrop({ ...defaultConfig, enabled: false }));
      // Give time for effect to run
      await new Promise(resolve => setTimeout(resolve, 10));
      expect(mockOnDragDropEvent).not.toHaveBeenCalled();
    });
  });

  describe("Tauri drag events", () => {
    it("should set isDragging=true on Tauri enter event", async () => {
      const { result } = renderHook(() => useFileDrop(defaultConfig));
      await waitFor(() => expect(dragDropHandler).not.toBeNull());

      act(() => {
        simulateTauriDragEvent("enter", ["/path/to/file.md"]);
      });

      expect(result.current.isDragging).toBe(true);
    });

    it("should set isDragging=true on Tauri over event", async () => {
      const { result } = renderHook(() => useFileDrop(defaultConfig));
      await waitFor(() => expect(dragDropHandler).not.toBeNull());

      act(() => {
        simulateTauriDragEvent("over");
      });

      expect(result.current.isDragging).toBe(true);
    });

    it("should set isDragging=false on Tauri leave event", async () => {
      const { result } = renderHook(() => useFileDrop(defaultConfig));
      await waitFor(() => expect(dragDropHandler).not.toBeNull());

      act(() => {
        simulateTauriDragEvent("enter", ["/path/to/file.md"]);
      });
      expect(result.current.isDragging).toBe(true);

      act(() => {
        simulateTauriDragEvent("leave");
      });
      expect(result.current.isDragging).toBe(false);
    });
  });

  describe("file drop via Tauri", () => {
    it("should call onFileDrop with valid .md file", async () => {
      mockReadTextFile.mockResolvedValue("# Hello World");

      const { result } = renderHook(() => useFileDrop(defaultConfig));
      await waitFor(() => expect(dragDropHandler).not.toBeNull());

      await act(async () => {
        simulateTauriDragEvent("drop", ["/path/to/test.md"]);
        // Wait for async file read
        await new Promise(resolve => setTimeout(resolve, 10));
      });

      expect(mockReadTextFile).toHaveBeenCalledWith("/path/to/test.md");
      expect(onFileDrop).toHaveBeenCalledWith(
        expect.objectContaining({ name: "test.md" }),
        "# Hello World"
      );
      expect(result.current.isDragging).toBe(false);
    });

    it("should reset isDragging on drop", async () => {
      mockReadTextFile.mockResolvedValue("content");

      const { result } = renderHook(() => useFileDrop(defaultConfig));
      await waitFor(() => expect(dragDropHandler).not.toBeNull());

      act(() => {
        simulateTauriDragEvent("enter", ["/path/to/file.md"]);
      });
      expect(result.current.isDragging).toBe(true);

      await act(async () => {
        simulateTauriDragEvent("drop", ["/path/to/test.md"]);
        await new Promise(resolve => setTimeout(resolve, 10));
      });
      expect(result.current.isDragging).toBe(false);
    });
  });

  describe("file validation", () => {
    it("should reject files with wrong extension", async () => {
      const { result } = renderHook(() => useFileDrop(defaultConfig));
      await waitFor(() => expect(dragDropHandler).not.toBeNull());

      await act(async () => {
        simulateTauriDragEvent("drop", ["/path/to/test.txt"]);
        await new Promise(resolve => setTimeout(resolve, 10));
      });

      expect(mockReadTextFile).not.toHaveBeenCalled();
      expect(onFileDrop).not.toHaveBeenCalled();
      expect(onError).toHaveBeenCalledWith(
        expect.objectContaining({ type: "invalid_type" })
      );
      expect(result.current.error?.type).toBe("invalid_type");
    });

    it("should reject files that are too large", async () => {
      const config: FileDropConfig = {
        ...defaultConfig,
        maxSizeBytes: 10, // 10 bytes max
      };
      mockReadTextFile.mockResolvedValue("This content is way too long for 10 bytes");

      const { result: _result } = renderHook(() => useFileDrop(config));
      await waitFor(() => expect(dragDropHandler).not.toBeNull());

      await act(async () => {
        simulateTauriDragEvent("drop", ["/path/to/test.md"]);
        await new Promise(resolve => setTimeout(resolve, 10));
      });

      expect(onFileDrop).not.toHaveBeenCalled();
      expect(onError).toHaveBeenCalledWith(
        expect.objectContaining({ type: "too_large" })
      );
    });

    it("should reject multiple files", async () => {
      const { result: _result } = renderHook(() => useFileDrop(defaultConfig));
      await waitFor(() => expect(dragDropHandler).not.toBeNull());

      await act(async () => {
        simulateTauriDragEvent("drop", ["/path/to/test1.md", "/path/to/test2.md"]);
        await new Promise(resolve => setTimeout(resolve, 10));
      });

      expect(onFileDrop).not.toHaveBeenCalled();
      expect(onError).toHaveBeenCalledWith(
        expect.objectContaining({ type: "multiple_files" })
      );
    });

    it("should accept multiple extensions when configured", async () => {
      const config: FileDropConfig = {
        ...defaultConfig,
        acceptedExtensions: [".md", ".txt"],
      };
      mockReadTextFile.mockResolvedValue("text content");

      renderHook(() => useFileDrop(config));
      await waitFor(() => expect(dragDropHandler).not.toBeNull());

      await act(async () => {
        simulateTauriDragEvent("drop", ["/path/to/test.txt"]);
        await new Promise(resolve => setTimeout(resolve, 10));
      });

      expect(onFileDrop).toHaveBeenCalledWith(
        expect.objectContaining({ name: "test.txt" }),
        "text content"
      );
    });
  });

  describe("error handling", () => {
    it("should clear error on Tauri enter event", async () => {
      const { result } = renderHook(() => useFileDrop(defaultConfig));
      await waitFor(() => expect(dragDropHandler).not.toBeNull());

      // Trigger an error first
      await act(async () => {
        simulateTauriDragEvent("drop", ["/path/to/test.txt"]);
        await new Promise(resolve => setTimeout(resolve, 10));
      });
      expect(result.current.error).not.toBeNull();

      // Enter should clear the error
      act(() => {
        simulateTauriDragEvent("enter", ["/path/to/test.md"]);
      });
      expect(result.current.error).toBeNull();
    });

    it("should clear error via clearError function", async () => {
      const { result } = renderHook(() => useFileDrop(defaultConfig));
      await waitFor(() => expect(dragDropHandler).not.toBeNull());

      // Trigger an error
      await act(async () => {
        simulateTauriDragEvent("drop", ["/path/to/test.txt"]);
        await new Promise(resolve => setTimeout(resolve, 10));
      });
      expect(result.current.error).not.toBeNull();

      act(() => {
        result.current.clearError();
      });
      expect(result.current.error).toBeNull();
    });

    it("should set error on file read failure", async () => {
      mockReadTextFile.mockRejectedValue(new Error("Permission denied"));

      const { result } = renderHook(() => useFileDrop(defaultConfig));
      await waitFor(() => expect(dragDropHandler).not.toBeNull());

      await act(async () => {
        simulateTauriDragEvent("drop", ["/path/to/test.md"]);
        await new Promise(resolve => setTimeout(resolve, 10));
      });

      expect(result.current.error).toEqual({
        type: "read_error",
        message: "Failed to read file contents",
      });
    });
  });

  describe("HTML5 drag events (compatibility)", () => {
    it("should preventDefault on dragover", () => {
      const { result } = renderHook(() => useFileDrop(defaultConfig));
      const event = createDragEvent("dragover");

      act(() => {
        result.current.dropProps.onDragOver(event);
      });

      expect(event.preventDefault).toHaveBeenCalled();
    });

    it("should preventDefault on drop", () => {
      const { result } = renderHook(() => useFileDrop(defaultConfig));
      const event = createDragEvent("drop");

      act(() => {
        result.current.dropProps.onDrop(event);
      });

      expect(event.preventDefault).toHaveBeenCalled();
    });
  });

  describe("edge cases", () => {
    it("should handle empty drop (no files)", async () => {
      renderHook(() => useFileDrop(defaultConfig));
      await waitFor(() => expect(dragDropHandler).not.toBeNull());

      await act(async () => {
        simulateTauriDragEvent("drop", []);
        await new Promise(resolve => setTimeout(resolve, 10));
      });

      expect(onFileDrop).not.toHaveBeenCalled();
      expect(onError).not.toHaveBeenCalled();
    });

    it("should be case-insensitive for extensions", async () => {
      mockReadTextFile.mockResolvedValue("content");

      renderHook(() => useFileDrop(defaultConfig));
      await waitFor(() => expect(dragDropHandler).not.toBeNull());

      await act(async () => {
        simulateTauriDragEvent("drop", ["/path/to/test.MD"]);
        await new Promise(resolve => setTimeout(resolve, 10));
      });

      expect(onFileDrop).toHaveBeenCalled();
    });

    it("should extract filename from path correctly", async () => {
      mockReadTextFile.mockResolvedValue("content");

      renderHook(() => useFileDrop(defaultConfig));
      await waitFor(() => expect(dragDropHandler).not.toBeNull());

      await act(async () => {
        simulateTauriDragEvent("drop", ["/Users/test/Documents/my-plan.md"]);
        await new Promise(resolve => setTimeout(resolve, 10));
      });

      expect(onFileDrop).toHaveBeenCalledWith(
        expect.objectContaining({ name: "my-plan.md" }),
        "content"
      );
    });
  });
});
