import {
  useResponsiveSidebarLayout,
  type ResponsiveSidebarLayoutResult,
} from "./useResponsiveSidebarLayout";

const COLLAPSE_PREF_KEY = "ralphx-plan-browser-collapsed";

export type PlanBrowserLayoutResult = ResponsiveSidebarLayoutResult;

export function usePlanBrowserLayout(): PlanBrowserLayoutResult {
  return useResponsiveSidebarLayout({
    storageKey: COLLAPSE_PREF_KEY,
    largeWidth: 340,
    mediumWidth: 276,
  });
}
