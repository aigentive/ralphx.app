/**
 * Mock Artifact API
 *
 * Mirrors the interface of src/api/artifact.ts with mock implementations.
 */

import type { Artifact } from "@/types/artifact";

// ============================================================================
// Mock Artifact API
// ============================================================================

export const mockArtifactApi = {
  get: async (_artifactId: string): Promise<Artifact | null> => {
    return null;
  },

  getAtVersion: async (_artifactId: string, _version: number): Promise<Artifact | null> => {
    return null;
  },

  getByTask: async (_taskId: string): Promise<Artifact[]> => {
    return [];
  },

  getByBucket: async (_bucketId: string): Promise<Artifact[]> => {
    return [];
  },
} as const;
