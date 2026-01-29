/**
 * useFileDrop hook tests
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useFileDrop, type FileDropConfig } from "./useFileDrop";

// Helper to create a mock File with working text() method
function createMockFile(
  name: string,
  content: string,
  type = "text/plain"
): File {
  const blob = new Blob([content], { type });
  const file = new File([blob], name, { type });
  // Add text() method that works in test environment
  // The native File.text() may not work in jsdom
  Object.defineProperty(file, "text", {
    value: () => Promise.resolve(content),
    writable: true,
    configurable: true,
  });
  return file;
}

// Helper to create a mock DragEvent
function createDragEvent(
  files: File[] = [],
  type: "dragenter" | "dragover" | "dragleave" | "drop" = "dragenter"
): React.DragEvent {
  const dataTransfer = {
    files: {
      length: files.length,
      item: (i: number) => files[i] ?? null,
      [Symbol.iterator]: function* () {
        for (let i = 0; i < files.length; i++) {
          yield files[i];
        }
      },
      ...files.reduce(
        (acc, file, i) => ({ ...acc, [i]: file }),
        {} as Record<number, File>
      ),
    } as FileList,
  };

  return {
    preventDefault: vi.fn(),
    stopPropagation: vi.fn(),
    dataTransfer,
    type,
  } as unknown as React.DragEvent;
}

describe("useFileDrop", () => {
  let onFileDrop: ReturnType<typeof vi.fn>;
  let onError: ReturnType<typeof vi.fn>;
  let defaultConfig: FileDropConfig;

  beforeEach(() => {
    onFileDrop = vi.fn();
    onError = vi.fn();
    defaultConfig = {
      acceptedExtensions: [".md"],
      onFileDrop,
      onError,
    };
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
  });

  describe("drag events", () => {
    it("should set isDragging=true on dragenter", () => {
      const { result } = renderHook(() => useFileDrop(defaultConfig));
      const event = createDragEvent([], "dragenter");

      act(() => {
        result.current.dropProps.onDragEnter(event);
      });

      expect(result.current.isDragging).toBe(true);
    });

    it("should set isDragging=false on dragleave", () => {
      const { result } = renderHook(() => useFileDrop(defaultConfig));
      const enterEvent = createDragEvent([], "dragenter");
      const leaveEvent = createDragEvent([], "dragleave");

      act(() => {
        result.current.dropProps.onDragEnter(enterEvent);
      });
      expect(result.current.isDragging).toBe(true);

      act(() => {
        result.current.dropProps.onDragLeave(leaveEvent);
      });
      expect(result.current.isDragging).toBe(false);
    });

    it("should handle nested elements (multiple enter/leave)", () => {
      const { result } = renderHook(() => useFileDrop(defaultConfig));
      const enterEvent = createDragEvent([], "dragenter");
      const leaveEvent = createDragEvent([], "dragleave");

      // Enter outer, enter inner
      act(() => {
        result.current.dropProps.onDragEnter(enterEvent);
        result.current.dropProps.onDragEnter(enterEvent);
      });
      expect(result.current.isDragging).toBe(true);

      // Leave inner (still dragging)
      act(() => {
        result.current.dropProps.onDragLeave(leaveEvent);
      });
      expect(result.current.isDragging).toBe(true);

      // Leave outer (done dragging)
      act(() => {
        result.current.dropProps.onDragLeave(leaveEvent);
      });
      expect(result.current.isDragging).toBe(false);
    });

    it("should preventDefault on dragover", () => {
      const { result } = renderHook(() => useFileDrop(defaultConfig));
      const event = createDragEvent([], "dragover");

      act(() => {
        result.current.dropProps.onDragOver(event);
      });

      expect(event.preventDefault).toHaveBeenCalled();
    });
  });

  describe("file drop", () => {
    it("should call onFileDrop with valid .md file", async () => {
      const { result } = renderHook(() => useFileDrop(defaultConfig));
      const file = createMockFile("test.md", "# Hello World");
      const event = createDragEvent([file], "drop");

      await act(async () => {
        await result.current.dropProps.onDrop(event);
      });

      expect(onFileDrop).toHaveBeenCalledWith(file, "# Hello World");
      expect(result.current.isDragging).toBe(false);
    });

    it("should reset isDragging on drop", async () => {
      const { result } = renderHook(() => useFileDrop(defaultConfig));
      const file = createMockFile("test.md", "content");
      const enterEvent = createDragEvent([file], "dragenter");
      const dropEvent = createDragEvent([file], "drop");

      act(() => {
        result.current.dropProps.onDragEnter(enterEvent);
      });
      expect(result.current.isDragging).toBe(true);

      await act(async () => {
        await result.current.dropProps.onDrop(dropEvent);
      });
      expect(result.current.isDragging).toBe(false);
    });
  });

  describe("file validation", () => {
    it("should reject files with wrong extension", async () => {
      const { result } = renderHook(() => useFileDrop(defaultConfig));
      const file = createMockFile("test.txt", "content");
      const event = createDragEvent([file], "drop");

      await act(async () => {
        await result.current.dropProps.onDrop(event);
      });

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
      const { result } = renderHook(() => useFileDrop(config));
      const file = createMockFile("test.md", "This content is way too long for 10 bytes");
      const event = createDragEvent([file], "drop");

      await act(async () => {
        await result.current.dropProps.onDrop(event);
      });

      expect(onFileDrop).not.toHaveBeenCalled();
      expect(onError).toHaveBeenCalledWith(
        expect.objectContaining({ type: "too_large" })
      );
    });

    it("should reject multiple files", async () => {
      const { result } = renderHook(() => useFileDrop(defaultConfig));
      const file1 = createMockFile("test1.md", "content1");
      const file2 = createMockFile("test2.md", "content2");
      const event = createDragEvent([file1, file2], "drop");

      await act(async () => {
        await result.current.dropProps.onDrop(event);
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
      const { result } = renderHook(() => useFileDrop(config));
      const file = createMockFile("test.txt", "text content");
      const event = createDragEvent([file], "drop");

      await act(async () => {
        await result.current.dropProps.onDrop(event);
      });

      expect(onFileDrop).toHaveBeenCalledWith(file, "text content");
    });
  });

  describe("error handling", () => {
    it("should clear error on dragenter", async () => {
      const { result } = renderHook(() => useFileDrop(defaultConfig));
      const badFile = createMockFile("test.txt", "content");
      const dropEvent = createDragEvent([badFile], "drop");

      await act(async () => {
        await result.current.dropProps.onDrop(dropEvent);
      });
      expect(result.current.error).not.toBeNull();

      const enterEvent = createDragEvent([], "dragenter");
      act(() => {
        result.current.dropProps.onDragEnter(enterEvent);
      });
      expect(result.current.error).toBeNull();
    });

    it("should clear error via clearError function", async () => {
      const { result } = renderHook(() => useFileDrop(defaultConfig));
      const badFile = createMockFile("test.txt", "content");
      const event = createDragEvent([badFile], "drop");

      await act(async () => {
        await result.current.dropProps.onDrop(event);
      });
      expect(result.current.error).not.toBeNull();

      act(() => {
        result.current.clearError();
      });
      expect(result.current.error).toBeNull();
    });

    it("should set error state when validation fails", async () => {
      const { result } = renderHook(() => useFileDrop(defaultConfig));
      const file = createMockFile("test.txt", "content");
      const event = createDragEvent([file], "drop");

      await act(async () => {
        await result.current.dropProps.onDrop(event);
      });

      expect(result.current.error).toEqual({
        type: "invalid_type",
        message: "Only .md files are accepted",
      });
    });
  });

  describe("edge cases", () => {
    it("should handle empty drop (no files)", async () => {
      const { result } = renderHook(() => useFileDrop(defaultConfig));
      const event = createDragEvent([], "drop");

      await act(async () => {
        await result.current.dropProps.onDrop(event);
      });

      expect(onFileDrop).not.toHaveBeenCalled();
      expect(onError).not.toHaveBeenCalled();
    });

    it("should be case-insensitive for extensions", async () => {
      const { result } = renderHook(() => useFileDrop(defaultConfig));
      const file = createMockFile("test.MD", "content");
      const event = createDragEvent([file], "drop");

      await act(async () => {
        await result.current.dropProps.onDrop(event);
      });

      expect(onFileDrop).toHaveBeenCalled();
    });

    it("should use default max size (1MB) when not specified", async () => {
      const configWithoutMaxSize: FileDropConfig = {
        acceptedExtensions: [".md"],
        onFileDrop,
        onError,
      };
      const { result } = renderHook(() => useFileDrop(configWithoutMaxSize));

      // Create a file just under 1MB (should pass)
      const content = "a".repeat(1024 * 1024 - 100);
      const file = createMockFile("test.md", content);
      const event = createDragEvent([file], "drop");

      await act(async () => {
        await result.current.dropProps.onDrop(event);
      });

      expect(onFileDrop).toHaveBeenCalled();
    });
  });
});
