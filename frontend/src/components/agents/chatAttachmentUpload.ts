import { invoke } from "@tauri-apps/api/core";

export async function uploadDraftAttachment(
  conversationId: string,
  file: File,
): Promise<{ id: string }> {
  const arrayBuffer = await file.arrayBuffer();
  const fileData = Array.from(new Uint8Array(arrayBuffer));

  return invoke<{ id: string }>("upload_chat_attachment", {
    input: {
      conversationId,
      fileName: file.name,
      fileData,
      mimeType: file.type || undefined,
    },
  });
}
