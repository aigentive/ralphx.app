/**
 * TanStack Query client configuration
 *
 * Configures the QueryClient with sensible defaults for the Tauri app.
 */

import { QueryClient } from "@tanstack/react-query";

/**
 * Default stale time for queries (5 minutes)
 * Data is considered fresh for this duration.
 */
const DEFAULT_STALE_TIME = 5 * 60 * 1000;

/**
 * Default retry count for failed queries
 * Backend errors typically don't benefit from retries.
 */
const DEFAULT_RETRY = 1;

/**
 * Create and configure the QueryClient
 */
export function createQueryClient(): QueryClient {
  return new QueryClient({
    defaultOptions: {
      queries: {
        // How long data is considered fresh (5 minutes)
        staleTime: DEFAULT_STALE_TIME,

        // Only retry once for transient errors
        retry: DEFAULT_RETRY,

        // Don't refetch on window focus for desktop app
        refetchOnWindowFocus: false,

        // Refetch when reconnecting (for remote backends)
        refetchOnReconnect: true,

        // Keep failed data visible while refetching
        placeholderData: (previousData: unknown) => previousData,
      },
      mutations: {
        // Don't retry mutations by default
        retry: false,
      },
    },
  });
}

/**
 * Singleton QueryClient instance for the app
 * Created lazily to support testing with fresh instances.
 */
let queryClient: QueryClient | null = null;

export function getQueryClient(): QueryClient {
  if (!queryClient) {
    queryClient = createQueryClient();

    // Expose queryClient to window in web mode for Playwright testing
    if (typeof window !== 'undefined' && !window.__TAURI_INTERNALS__) {
      window.__queryClient = queryClient;
    }
  }
  return queryClient;
}

/**
 * Reset the query client (for testing)
 */
export function resetQueryClient(): void {
  queryClient = null;
}
