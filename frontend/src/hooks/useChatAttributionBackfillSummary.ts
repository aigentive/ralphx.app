import { useEffect } from "react";
import { z } from "zod";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { getChatAttributionBackfillSummary } from "@/api/metrics";
import type { AttributionBackfillSummary } from "@/api/metrics";
import { useEventBus } from "@/providers/EventProvider";

export const chatAttributionBackfillSummaryKeys = {
  all: ["chat-attribution-backfill-summary"] as const,
};

export const CHAT_ATTRIBUTION_BACKFILL_PROGRESS_EVENT = "chat:attribution_backfill_progress";

const AttributionBackfillProgressEventSchema = z.object({
  processedInBatch: z.number(),
  eligibleConversationCount: z.number(),
  pendingCount: z.number(),
  runningCount: z.number(),
  completedCount: z.number(),
  partialCount: z.number(),
  sessionNotFoundCount: z.number(),
  parseFailedCount: z.number(),
  remainingCount: z.number(),
  terminalCount: z.number(),
  attentionCount: z.number(),
  isIdle: z.boolean(),
});

export function useChatAttributionBackfillSummary() {
  const eventBus = useEventBus();
  const queryClient = useQueryClient();
  const query = useQuery<AttributionBackfillSummary, Error>({
    queryKey: chatAttributionBackfillSummaryKeys.all,
    queryFn: () => getChatAttributionBackfillSummary(),
    staleTime: 30_000,
    gcTime: 5 * 60_000,
  });

  useEffect(() => {
    return eventBus.subscribe<unknown>(CHAT_ATTRIBUTION_BACKFILL_PROGRESS_EVENT, (payload) => {
      const parsed = AttributionBackfillProgressEventSchema.safeParse(payload);
      if (!parsed.success) {
        void queryClient.invalidateQueries({
          queryKey: chatAttributionBackfillSummaryKeys.all,
        });
        return;
      }

      queryClient.setQueryData(chatAttributionBackfillSummaryKeys.all, {
        eligibleConversationCount: parsed.data.eligibleConversationCount,
        pendingCount: parsed.data.pendingCount,
        runningCount: parsed.data.runningCount,
        completedCount: parsed.data.completedCount,
        partialCount: parsed.data.partialCount,
        sessionNotFoundCount: parsed.data.sessionNotFoundCount,
        parseFailedCount: parsed.data.parseFailedCount,
        remainingCount: parsed.data.remainingCount,
        terminalCount: parsed.data.terminalCount,
        attentionCount: parsed.data.attentionCount,
        isIdle: parsed.data.isIdle,
      });
    });
  }, [eventBus, queryClient]);

  return query;
}
