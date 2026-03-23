/**
 * useFeatureFlags - TanStack Query hook for UI feature flags
 *
 * Fetches flags once at startup (staleTime: Infinity).
 * Uses initialData (all enabled) to prevent startup flash before the
 * Tauri command responds. Falls back to all-enabled on error (retry: false).
 */

import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { featureFlagsSchema } from "@/types/feature-flags";
import type { FeatureFlags } from "@/types/feature-flags";

export const FEATURE_FLAGS_QUERY_KEY = ["featureFlags"] as const;

const ALL_ENABLED: FeatureFlags = {
  activityPage: true,
  extensibilityPage: true,
};

export function useFeatureFlags() {
  const query = useQuery<FeatureFlags>({
    queryKey: FEATURE_FLAGS_QUERY_KEY,
    queryFn: async () => {
      const raw = await invoke("get_ui_feature_flags");
      return featureFlagsSchema.parse(raw);
    },
    staleTime: Infinity,
    // placeholderData shows ALL_ENABLED immediately (prevents startup flash) while
    // the real fetch happens. Unlike initialData, it doesn't block the initial fetch.
    placeholderData: ALL_ENABLED,
    retry: false,
  });

  return {
    ...query,
    // Always return a defined FeatureFlags. Falls back to ALL_ENABLED on error
    // (placeholderData is not shown in error state; query.data would be undefined).
    data: query.data ?? ALL_ENABLED,
  };
}

/**
 * Pure helper to check if a view is enabled given the current flags.
 * Usable outside of React components.
 */
export function isViewEnabled(view: string, flags: FeatureFlags): boolean {
  switch (view) {
    case "activity":
      return flags.activityPage;
    case "extensibility":
      return flags.extensibilityPage;
    default:
      return true;
  }
}
