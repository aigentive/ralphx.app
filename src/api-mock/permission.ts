/**
 * Mock Permission API
 *
 * Provides mock implementation for permission resolution operations.
 * Used for browser testing and visual regression testing.
 */

import type { ResolvePermissionInput } from "@/api/permission";

/**
 * Mock Permission API matching the real API interface
 */
export const mockPermissionApi = {
  /**
   * Mock permission resolution - no-op for visual testing
   * In web mode, permission dialogs are simulated via events
   */
  resolveRequest: async (_input: ResolvePermissionInput): Promise<void> => {
    // No-op - visual testing doesn't process permission responses
    console.log("[mock] resolveRequest called");
  },
} as const;
