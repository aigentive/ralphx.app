/**
 * useFileDrop hook - Reusable drag-and-drop for file imports
 *
 * Handles dragging files from the system (e.g., Finder) into a drop zone.
 * Uses Tauri's onDragDropEvent for file drops (HTML5 dataTransfer is empty in Tauri)
 * and HTML5 drag events for visual feedback only.
 * Validates file type and size before accepting drops.
 */

import { useState, useCallback, useRef, useEffect } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { readTextFile } from "@tauri-apps/plugin-fs";

/**
 * Configuration for the file drop hook
 */
export interface FileDropConfig {
  /** Accepted file extensions (e.g., [".md", ".txt"]) */
  acceptedExtensions: string[];
  /** Maximum file size in bytes (default: 1MB) */
  maxSizeBytes?: number;
  /** Called when a valid file is dropped */
  onFileDrop: (file: File, content: string) => void;
  /** Called when an error occurs (invalid file type, size, etc.) */
  onError?: (error: FileDropError) => void;
  /** Whether the hook is enabled (default: true) */
  enabled?: boolean;
}

/**
 * Error types for file drop validation
 */
export type FileDropErrorType =
  | "invalid_type"
  | "too_large"
  | "read_error"
  | "multiple_files";

/**
 * Error object returned when file validation fails
 */
export interface FileDropError {
  type: FileDropErrorType;
  message: string;
}

/**
 * Return value from useFileDrop
 */
export interface FileDropResult {
  /** Whether a drag operation is in progress over the drop zone */
  isDragging: boolean;
  /** Props to spread onto the drop zone element */
  dropProps: {
    onDragEnter: (e: React.DragEvent) => void;
    onDragOver: (e: React.DragEvent) => void;
    onDragLeave: (e: React.DragEvent) => void;
    onDrop: (e: React.DragEvent) => void;
  };
  /** Current error, if any */
  error: FileDropError | null;
  /** Clear the current error */
  clearError: () => void;
}

const DEFAULT_MAX_SIZE = 1024 * 1024; // 1MB

/**
 * Hook for handling file drag-and-drop
 *
 * @param config - Configuration for accepted files and handlers
 * @returns Object with isDragging state, dropProps, and error handling
 *
 * @example
 * ```tsx
 * const { isDragging, dropProps, error } = useFileDrop({
 *   acceptedExtensions: [".md"],
 *   onFileDrop: (file, content) => {
 *     // Handle file drop - file.name and content are available
 *   },
 *   onError: (err) => toast.error(err.message),
 * });
 *
 * return (
 *   <div {...dropProps}>
 *     {isDragging && <DropZoneOverlay />}
 *     <content />
 *   </div>
 * );
 * ```
 */
export function useFileDrop(config: FileDropConfig): FileDropResult {
  const { acceptedExtensions, maxSizeBytes = DEFAULT_MAX_SIZE, onFileDrop, onError, enabled = true } = config;

  const [isDragging, setIsDragging] = useState(false);
  const [error, setError] = useState<FileDropError | null>(null);

  // Track drag enter/leave count for nested elements (visual feedback only)
  const dragCounterRef = useRef(0);

  // Store callbacks in refs to avoid re-subscribing to Tauri events
  const onFileDropRef = useRef(onFileDrop);
  const onErrorRef = useRef(onError);
  const acceptedExtensionsRef = useRef(acceptedExtensions);
  const maxSizeBytesRef = useRef(maxSizeBytes);

  useEffect(() => {
    onFileDropRef.current = onFileDrop;
    onErrorRef.current = onError;
    acceptedExtensionsRef.current = acceptedExtensions;
    maxSizeBytesRef.current = maxSizeBytes;
  }, [onFileDrop, onError, acceptedExtensions, maxSizeBytes]);

  const clearError = useCallback(() => {
    setError(null);
  }, []);

  const handleError = useCallback(
    (err: FileDropError) => {
      setError(err);
      onErrorRef.current?.(err);
    },
    []
  );

  // Validate file path
  const validateFilePath = useCallback(
    (filePath: string): FileDropError | null => {
      // Check file extension
      const extension = `.${filePath.split(".").pop()?.toLowerCase()}`;
      if (!acceptedExtensionsRef.current.includes(extension)) {
        return {
          type: "invalid_type",
          message: `Only ${acceptedExtensionsRef.current.join(", ")} files are accepted`,
        };
      }
      return null;
    },
    []
  );

  // Set up Tauri's onDragDropEvent listener for actual file drops
  useEffect(() => {
    if (!enabled) return;

    let unlisten: (() => void) | undefined;

    const setupListener = async () => {
      try {
        const webview = getCurrentWebview();
        unlisten = await webview.onDragDropEvent(async (event) => {
          if (event.payload.type === "enter" || event.payload.type === "over") {
            // User is hovering files - show visual feedback
            setIsDragging(true);
            setError(null);
          } else if (event.payload.type === "drop") {
            // User dropped files
            setIsDragging(false);
            const paths = event.payload.paths;

            // Check for multiple files
            if (paths.length > 1) {
              handleError({
                type: "multiple_files",
                message: "Please drop only one file at a time",
              });
              return;
            }

            if (paths.length === 0) {
              return;
            }

            const filePath = paths[0];
            if (!filePath) return;

            // Validate file extension
            const validationError = validateFilePath(filePath);
            if (validationError) {
              handleError(validationError);
              return;
            }

            // Read file content using Tauri's FS plugin
            try {
              const content = await readTextFile(filePath);

              // Check file size (content length in bytes approximation)
              const sizeInBytes = new Blob([content]).size;
              if (sizeInBytes > maxSizeBytesRef.current) {
                const maxSizeMB = (maxSizeBytesRef.current / (1024 * 1024)).toFixed(1);
                handleError({
                  type: "too_large",
                  message: `File too large. Maximum size is ${maxSizeMB}MB`,
                });
                return;
              }

              // Extract filename from path
              const fileName = filePath.split("/").pop() || filePath.split("\\").pop() || "file.md";

              // Create a File-like object for compatibility
              const file = new File([content], fileName, { type: "text/markdown" });

              onFileDropRef.current(file, content);
            } catch (readError) {
              console.error("File read error:", readError);
              handleError({
                type: "read_error",
                message: "Failed to read file contents",
              });
            }
          } else if (event.payload.type === "leave") {
            // User moved files away from the webview
            setIsDragging(false);
          }
        });
      } catch (err) {
        console.error("Failed to set up drag-drop listener:", err);
      }
    };

    setupListener();

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, [enabled, handleError, validateFilePath]);

  // HTML5 drag events for visual feedback only (doesn't handle actual file content)
  const onDragEnter = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    dragCounterRef.current++;
    if (dragCounterRef.current === 1) {
      // Visual feedback is now handled by Tauri's onDragDropEvent
      // but we keep these for potential non-file drags
    }
  }, []);

  const onDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
  }, []);

  const onDragLeave = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    dragCounterRef.current--;
    if (dragCounterRef.current === 0) {
      // Reset counter, Tauri handles the visual state
    }
  }, []);

  // HTML5 onDrop - prevent default but Tauri handles the actual file
  const onDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    dragCounterRef.current = 0;
    // Tauri's onDragDropEvent handles the actual file processing
    // HTML5 dataTransfer.files is empty in Tauri for file drops from Finder
  }, []);

  return {
    isDragging,
    dropProps: {
      onDragEnter,
      onDragOver,
      onDragLeave,
      onDrop,
    },
    error,
    clearError,
  };
}
