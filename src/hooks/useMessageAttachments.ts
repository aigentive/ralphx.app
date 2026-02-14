/**
 * useMessageAttachments — Fetch attachments for messages
 *
 * Fetches attachments for all messages in a conversation and returns a map
 * of message ID to attachments array.
 */

import { useQuery } from "@tanstack/react-query";
import { chatApi, type ChatAttachmentResponse } from "@/api/chat";
import type { ChatMessageData } from "@/components/Chat/ChatMessageList";
import type { MessageAttachment } from "@/components/Chat/MessageAttachments";

/**
 * Transform ChatAttachmentResponse from backend to MessageAttachment for UI
 */
function transformAttachment(attachment: ChatAttachmentResponse): MessageAttachment {
  const base = {
    id: attachment.id,
    fileName: attachment.fileName,
    fileSize: attachment.fileSize,
    filePath: attachment.filePath,
  };

  // Only include optional properties when they have values
  return {
    ...base,
    ...(attachment.mimeType !== null && { mimeType: attachment.mimeType }),
  };
}

/**
 * Fetch attachments for all messages in a list
 *
 * @param messages - Array of chat messages
 * @param conversationId - Current conversation ID (used as cache key)
 * @returns Map of message ID to attachments array
 */
export function useMessageAttachments(
  messages: ChatMessageData[],
  conversationId: string | null
) {
  return useQuery({
    queryKey: ["message-attachments", conversationId],
    queryFn: async () => {
      const attachmentsMap = new Map<string, MessageAttachment[]>();

      // Fetch attachments for all user messages
      await Promise.all(
        messages
          .filter(msg => msg.role === "user")
          .map(async (msg) => {
            try {
              const attachments = await chatApi.listMessageAttachments(msg.id);
              if (attachments.length > 0) {
                attachmentsMap.set(msg.id, attachments.map(transformAttachment));
              }
            } catch {
              // Silently ignore — attachment fetching is optional
            }
          })
      );

      return attachmentsMap;
    },
    enabled: !!conversationId && messages.length > 0,
    staleTime: 30000, // Cache for 30 seconds
  });
}
