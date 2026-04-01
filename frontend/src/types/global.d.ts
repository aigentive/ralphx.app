/**
 * Global type declarations for window extensions
 * Used for Playwright testing in web mode
 */

import type { QueryClient } from "@tanstack/react-query";
import type { EventBus } from "@/lib/event-bus";
import type { MockStore } from "@/api-mock/store";

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
    // Playwright testing utilities (web mode only)
    __mockStore?: MockStore;
    __queryClient?: QueryClient;
    __eventBus?: EventBus;
    __uiStore?: unknown;
    __openReviewDetailModal?: (taskId: string) => void;
  }
}

export {};
