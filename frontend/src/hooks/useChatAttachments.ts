// Hook for managing chat file attachments (pre-send state)
// Handles upload, validation, progress tracking, and cleanup

import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

// ============================================================================
// Types
// ============================================================================

/**
 * Chat attachment metadata
 */
export interface ChatAttachment {
  id: string;
  conversationId: string;
  messageId?: string;
  fileName: string;
  filePath: string;
  mimeType?: string;
  fileSize: number;
  createdAt: string;
}

/**
 * Upload progress state for an individual file
 */
interface UploadProgress {
  fileName: string;
  status: "uploading" | "complete" | "error";
  error?: string;
}

// ============================================================================
// Constants
// ============================================================================

const MAX_FILE_SIZE = 10 * 1024 * 1024; // 10MB
const MAX_FILES = 5;

// ============================================================================
// Hook
// ============================================================================

export interface UseChatAttachmentsResult {
  attachments: ChatAttachment[];
  uploadFiles: (files: File[]) => Promise<ChatAttachment[]>;
  removeAttachment: (id: string) => Promise<void>;
  clearAttachments: () => void;
  uploading: boolean;
  uploadProgress: UploadProgress[];
}

/**
 * Hook for managing pending chat attachments (pre-send)
 *
 * @param conversationId The conversation ID to associate attachments with
 * @returns Attachment state and operations
 */
export function useChatAttachments(conversationId: string): UseChatAttachmentsResult {
  const [attachments, setAttachments] = useState<ChatAttachment[]>([]);
  const [uploading, setUploading] = useState(false);
  const [uploadProgress, setUploadProgress] = useState<UploadProgress[]>([]);

  /**
   * Upload multiple files with validation
   */
  const uploadFiles = useCallback(
    async (files: File[]): Promise<ChatAttachment[]> => {
      // Validation: max file count
      if (attachments.length + files.length > MAX_FILES) {
        throw new Error(`Cannot upload more than ${MAX_FILES} files total`);
      }

      // Validation: individual file sizes
      const oversizedFiles = files.filter((f) => f.size > MAX_FILE_SIZE);
      if (oversizedFiles.length > 0) {
        throw new Error(
          `Files exceed 10MB limit: ${oversizedFiles.map((f) => f.name).join(", ")}`
        );
      }

      setUploading(true);
      setUploadProgress(
        files.map((f) => ({ fileName: f.name, status: "uploading" }))
      );

      const uploadedAttachments: ChatAttachment[] = [];

      try {
        for (const file of files) {
          try {
            // Read file data as array buffer
            const arrayBuffer = await file.arrayBuffer();
            const fileData = Array.from(new Uint8Array(arrayBuffer));

            // Call Tauri command
            const response = await invoke<ChatAttachment>(
              "upload_chat_attachment",
              {
                input: {
                  conversationId,
                  fileName: file.name,
                  fileData,
                  mimeType: file.type || undefined,
                },
              }
            );

            uploadedAttachments.push(response);

            // Update progress
            setUploadProgress((prev) =>
              prev.map((p) =>
                p.fileName === file.name ? { ...p, status: "complete" } : p
              )
            );
          } catch (error) {
            // Update progress with error
            setUploadProgress((prev) =>
              prev.map((p) =>
                p.fileName === file.name
                  ? {
                      ...p,
                      status: "error",
                      error: error instanceof Error ? error.message : "Upload failed",
                    }
                  : p
              )
            );
            throw error;
          }
        }

        // Add to attachments state
        setAttachments((prev) => [...prev, ...uploadedAttachments]);

        return uploadedAttachments;
      } finally {
        setUploading(false);
        // Clear progress after a delay
        setTimeout(() => setUploadProgress([]), 2000);
      }
    },
    [conversationId, attachments.length]
  );

  /**
   * Remove an attachment
   */
  const removeAttachment = useCallback(async (id: string): Promise<void> => {
    await invoke("delete_chat_attachment", { attachmentId: id });
    setAttachments((prev) => prev.filter((a) => a.id !== id));
  }, []);

  /**
   * Clear all attachments
   */
  const clearAttachments = useCallback(() => {
    setAttachments([]);
    setUploadProgress([]);
  }, []);

  return {
    attachments,
    uploadFiles,
    removeAttachment,
    clearAttachments,
    uploading,
    uploadProgress,
  };
}
