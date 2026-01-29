/**
 * useFileDrop hook - Reusable drag-and-drop for file imports
 *
 * Handles dragging files from the system (e.g., Finder) into a drop zone.
 * Uses the native HTML5 drag-and-drop API (not dnd-kit).
 * Validates file type and size before accepting drops.
 */

import { useState, useCallback, useRef } from "react";

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
  const { acceptedExtensions, maxSizeBytes = DEFAULT_MAX_SIZE, onFileDrop, onError } = config;

  const [isDragging, setIsDragging] = useState(false);
  const [error, setError] = useState<FileDropError | null>(null);

  // Track drag enter/leave count for nested elements
  const dragCounterRef = useRef(0);

  const clearError = useCallback(() => {
    setError(null);
  }, []);

  const handleError = useCallback(
    (err: FileDropError) => {
      setError(err);
      onError?.(err);
    },
    [onError]
  );

  const validateFile = useCallback(
    (file: File): FileDropError | null => {
      // Check file extension
      const extension = `.${file.name.split(".").pop()?.toLowerCase()}`;
      if (!acceptedExtensions.includes(extension)) {
        return {
          type: "invalid_type",
          message: `Only ${acceptedExtensions.join(", ")} files are accepted`,
        };
      }

      // Check file size
      if (file.size > maxSizeBytes) {
        const maxSizeMB = (maxSizeBytes / (1024 * 1024)).toFixed(1);
        return {
          type: "too_large",
          message: `File too large. Maximum size is ${maxSizeMB}MB`,
        };
      }

      return null;
    },
    [acceptedExtensions, maxSizeBytes]
  );

  const onDragEnter = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    dragCounterRef.current++;
    if (dragCounterRef.current === 1) {
      setIsDragging(true);
      setError(null);
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
      setIsDragging(false);
    }
  }, []);

  const onDrop = useCallback(
    async (e: React.DragEvent) => {
      e.preventDefault();
      e.stopPropagation();
      dragCounterRef.current = 0;
      setIsDragging(false);

      const files = e.dataTransfer.files;

      // Check for multiple files
      if (files.length > 1) {
        handleError({
          type: "multiple_files",
          message: "Please drop only one file at a time",
        });
        return;
      }

      if (files.length === 0) {
        return;
      }

      const file = files[0];
      if (!file) {
        return;
      }

      // Validate file
      const validationError = validateFile(file);
      if (validationError) {
        handleError(validationError);
        return;
      }

      // Read file content
      try {
        const content = await file.text();
        onFileDrop(file, content);
      } catch {
        handleError({
          type: "read_error",
          message: "Failed to read file contents",
        });
      }
    },
    [validateFile, handleError, onFileDrop]
  );

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
