import { useQuery } from "@tanstack/react-query";
import { getChatAttributionBackfillSummary } from "@/api/metrics";
import type { AttributionBackfillSummary } from "@/api/metrics";

export const chatAttributionBackfillSummaryKeys = {
  all: ["chat-attribution-backfill-summary"] as const,
};

export function useChatAttributionBackfillSummary() {
  return useQuery<AttributionBackfillSummary, Error>({
    queryKey: chatAttributionBackfillSummaryKeys.all,
    queryFn: () => getChatAttributionBackfillSummary(),
    staleTime: 30_000,
    gcTime: 5 * 60_000,
  });
}
