import { useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { chatApi } from "@/api/chat";
import { TaskCardTranscriptView } from "@/components/Chat/TaskCardTranscript";
import { buildTaskCardTranscriptEntriesFromConversation } from "@/components/Chat/TaskCardTranscript.utils";

interface VerificationChildTranscriptProps {
  childSessionId: string;
  isActiveRun: boolean;
}

export function VerificationChildTranscript({
  childSessionId,
  isActiveRun,
}: VerificationChildTranscriptProps) {
  const conversationsQuery = useQuery({
    queryKey: ["verification-child-transcript", childSessionId, "conversations"],
    queryFn: () => chatApi.listConversations("ideation", childSessionId),
    enabled: childSessionId.length > 0,
    staleTime: 0,
    refetchInterval: isActiveRun ? 2000 : false,
  });

  const conversationId = useMemo(() => {
    const conversations = conversationsQuery.data ?? [];
    if (conversations.length === 0) return null;

    return [...conversations]
      .sort(
        (left, right) =>
          new Date(right.updatedAt).getTime() - new Date(left.updatedAt).getTime()
      )[0]?.id ?? null;
  }, [conversationsQuery.data]);

  const transcriptQuery = useQuery({
    queryKey: ["verification-child-transcript", childSessionId, "conversation", conversationId],
    queryFn: () => {
      if (!conversationId) {
        throw new Error("Conversation ID is required");
      }
      return chatApi.getConversation(conversationId);
    },
    enabled: conversationId != null,
    staleTime: 0,
    refetchInterval: isActiveRun ? 2000 : false,
  });

  const transcriptEntries = buildTaskCardTranscriptEntriesFromConversation(
    transcriptQuery.data?.messages ?? []
  );

  if (conversationsQuery.isLoading || transcriptQuery.isLoading) {
    return (
      <div
        data-testid="verification-child-transcript-loading"
        className="text-[11px] px-2.5 py-2 rounded-md"
        style={{
          background: "var(--overlay-faint)",
          color: "var(--text-secondary)",
        }}
      >
        Loading verification transcript...
      </div>
    );
  }

  if (conversationsQuery.isError || transcriptQuery.isError) {
    return (
      <div
        data-testid="verification-child-transcript-error"
        className="text-[11px] px-2.5 py-2 rounded-md"
        style={{
          background: "var(--status-error-muted)",
          color: "var(--status-error)",
        }}
      >
        Unable to load verification transcript.
      </div>
    );
  }

  if (transcriptEntries.length === 0) {
    return (
      <div
        data-testid="verification-child-transcript-empty"
        className="text-[11px] px-2.5 py-2 rounded-md"
        style={{
          background: "var(--overlay-faint)",
          color: "var(--text-secondary)",
        }}
      >
        {isActiveRun
          ? "Waiting for verification transcript..."
          : "No verification transcript was captured for this run."}
      </div>
    );
  }

  return (
    <div className="space-y-3" data-testid="verification-child-transcript">
      <div
        className="text-[10px] uppercase tracking-[0.08em]"
        style={{ color: "var(--text-muted)" }}
      >
        Verification Agent Activity
      </div>
      <TaskCardTranscriptView
        entries={transcriptEntries}
        dataTestId="verification-child-transcript-entries"
      />
    </div>
  );
}
