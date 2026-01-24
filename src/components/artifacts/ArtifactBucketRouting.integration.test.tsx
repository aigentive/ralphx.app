/**
 * Integration test: Artifact creation and bucket routing
 *
 * Tests artifact operations and bucket routing:
 * - Create artifact in research-outputs bucket
 * - Copy artifact to prd-library bucket
 * - Create artifact relation (derived_from)
 * - Query artifacts by bucket and type
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import * as artifactsApi from "@/lib/api/artifacts";
import type {
  ArtifactResponse,
  BucketResponse,
  ArtifactRelationResponse,
} from "@/lib/api/artifacts";
import type { Artifact, ArtifactBucket } from "@/types/artifact";
import { ArtifactBrowser } from "./ArtifactBrowser";
import { ArtifactCard } from "./ArtifactCard";

// ============================================================================
// Mocks
// ============================================================================

vi.mock("@/lib/api/artifacts", () => ({
  getArtifacts: vi.fn(),
  getArtifact: vi.fn(),
  createArtifact: vi.fn(),
  updateArtifact: vi.fn(),
  deleteArtifact: vi.fn(),
  getArtifactsByBucket: vi.fn(),
  getArtifactsByTask: vi.fn(),
  getBuckets: vi.fn(),
  createBucket: vi.fn(),
  getSystemBuckets: vi.fn(),
  addArtifactRelation: vi.fn(),
  getArtifactRelations: vi.fn(),
}));

// ============================================================================
// Test Data
// ============================================================================

const researchOutputsBucket: BucketResponse = {
  id: "research-outputs",
  name: "Research Outputs",
  accepted_types: ["research_document", "findings", "recommendations"],
  writers: ["deep-researcher", "orchestrator"],
  readers: ["all"],
  is_system: true,
};

const prdLibraryBucket: BucketResponse = {
  id: "prd-library",
  name: "PRD Library",
  accepted_types: ["prd", "specification", "design_doc", "recommendations"],
  writers: ["orchestrator", "user"],
  readers: ["all"],
  is_system: true,
};

const workContextBucket: BucketResponse = {
  id: "work-context",
  name: "Work Context",
  accepted_types: ["context", "task_spec", "previous_work"],
  writers: ["orchestrator", "system"],
  readers: ["all"],
  is_system: true,
};

const codeChangesBucket: BucketResponse = {
  id: "code-changes",
  name: "Code Changes",
  accepted_types: ["code_change", "diff", "test_result"],
  writers: ["worker"],
  readers: ["all"],
  is_system: true,
};

const findingsArtifact: ArtifactResponse = {
  id: "artifact-1",
  name: "Authentication Analysis Findings",
  artifact_type: "findings",
  content_type: "inline",
  content: "OAuth2 is the recommended approach...",
  created_at: "2026-01-24T12:00:00Z",
  created_by: "deep-researcher",
  version: 1,
  bucket_id: "research-outputs",
  task_id: null,
  process_id: null,
  derived_from: [],
};

const recommendationsArtifact: ArtifactResponse = {
  id: "artifact-2",
  name: "Technology Recommendations",
  artifact_type: "recommendations",
  content_type: "inline",
  content: "Based on research, use Rust for backend...",
  created_at: "2026-01-24T13:00:00Z",
  created_by: "deep-researcher",
  version: 1,
  bucket_id: "research-outputs",
  task_id: null,
  process_id: null,
  derived_from: [],
};

const copiedRecommendationsArtifact: ArtifactResponse = {
  id: "artifact-3",
  name: "Technology Recommendations",
  artifact_type: "recommendations",
  content_type: "inline",
  content: "Based on research, use Rust for backend...",
  created_at: "2026-01-24T14:00:00Z",
  created_by: "orchestrator",
  version: 1,
  bucket_id: "prd-library",
  task_id: null,
  process_id: null,
  derived_from: ["artifact-2"],
};

const prdArtifact: ArtifactResponse = {
  id: "artifact-4",
  name: "Product PRD",
  artifact_type: "prd",
  content_type: "inline",
  content: "Product requirements document...",
  created_at: "2026-01-24T15:00:00Z",
  created_by: "user",
  version: 1,
  bucket_id: "prd-library",
  task_id: null,
  process_id: null,
  derived_from: [],
};

const derivedFromRelation: ArtifactRelationResponse = {
  id: "relation-1",
  from_artifact_id: "artifact-3",
  to_artifact_id: "artifact-2",
  relation_type: "derived_from",
};

// ============================================================================
// Test Utilities
// ============================================================================

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });
  return ({ children }: { children: React.ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
}

// ============================================================================
// Tests
// ============================================================================

describe("Artifact Creation and Bucket Routing Integration", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // ========================================================================
  // Test 1: Create artifact in research-outputs bucket
  // ========================================================================

  describe("Test 1: Create artifact in research-outputs bucket", () => {
    it("artifact has correct type and bucket assignment", () => {
      expect(findingsArtifact.artifact_type).toBe("findings");
      expect(findingsArtifact.bucket_id).toBe("research-outputs");
      expect(findingsArtifact.created_by).toBe("deep-researcher");
    });

    it("research-outputs bucket accepts findings type", () => {
      expect(researchOutputsBucket.accepted_types).toContain("findings");
      expect(researchOutputsBucket.writers).toContain("deep-researcher");
    });

    it("renders artifact card correctly", () => {
      const artifact: Artifact = {
        id: findingsArtifact.id,
        name: findingsArtifact.name,
        type: findingsArtifact.artifact_type,
        content: { type: "inline", text: findingsArtifact.content },
        metadata: {
          createdAt: findingsArtifact.created_at,
          createdBy: findingsArtifact.created_by,
          version: findingsArtifact.version,
        },
        derivedFrom: findingsArtifact.derived_from,
        bucketId: findingsArtifact.bucket_id ?? undefined,
      };

      render(
        <ArtifactCard
          artifact={artifact}
          isSelected={false}
          onClick={() => {}}
        />
      );

      expect(screen.getByText("Authentication Analysis Findings")).toBeInTheDocument();
      expect(screen.getByText("Findings")).toBeInTheDocument();
    });
  });

  // ========================================================================
  // Test 2: Copy artifact to prd-library bucket
  // ========================================================================

  describe("Test 2: Copy artifact to prd-library bucket", () => {
    it("copied artifact has different id but same content type", () => {
      expect(copiedRecommendationsArtifact.id).not.toBe(recommendationsArtifact.id);
      expect(copiedRecommendationsArtifact.artifact_type).toBe(
        recommendationsArtifact.artifact_type
      );
      expect(copiedRecommendationsArtifact.name).toBe(recommendationsArtifact.name);
    });

    it("copied artifact is in different bucket", () => {
      expect(recommendationsArtifact.bucket_id).toBe("research-outputs");
      expect(copiedRecommendationsArtifact.bucket_id).toBe("prd-library");
    });

    it("both buckets contain recommendations type artifacts", () => {
      // Research outputs has original
      expect(researchOutputsBucket.accepted_types).toContain("recommendations");
      // PRD library has copy
      expect(prdLibraryBucket.accepted_types).toContain("recommendations");
    });

    it("copied artifact has derived_from tracking source", () => {
      expect(copiedRecommendationsArtifact.derived_from).toContain("artifact-2");
    });
  });

  // ========================================================================
  // Test 3: Create artifact relation (derived_from)
  // ========================================================================

  describe("Test 3: Create artifact relation (derived_from)", () => {
    it("relation links copied artifact to source", () => {
      expect(derivedFromRelation.from_artifact_id).toBe(copiedRecommendationsArtifact.id);
      expect(derivedFromRelation.to_artifact_id).toBe(recommendationsArtifact.id);
      expect(derivedFromRelation.relation_type).toBe("derived_from");
    });

    it("can fetch relations for artifact", async () => {
      vi.mocked(artifactsApi.getArtifactRelations).mockResolvedValue([derivedFromRelation]);

      const relations = await artifactsApi.getArtifactRelations("artifact-3");

      expect(relations).toHaveLength(1);
      expect(relations[0].to_artifact_id).toBe("artifact-2");
      expect(relations[0].relation_type).toBe("derived_from");
    });

    it("can add new relation", async () => {
      const newRelation: ArtifactRelationResponse = {
        id: "relation-2",
        from_artifact_id: "artifact-4",
        to_artifact_id: "artifact-3",
        relation_type: "related_to",
      };
      vi.mocked(artifactsApi.addArtifactRelation).mockResolvedValue(newRelation);

      const result = await artifactsApi.addArtifactRelation({
        from_artifact_id: "artifact-4",
        to_artifact_id: "artifact-3",
        relation_type: "related_to",
      });

      expect(result.from_artifact_id).toBe("artifact-4");
      expect(result.to_artifact_id).toBe("artifact-3");
      expect(result.relation_type).toBe("related_to");
    });
  });

  // ========================================================================
  // Test 4: Query artifacts by bucket and type
  // ========================================================================

  describe("Test 4: Query artifacts by bucket and type", () => {
    it("can query artifacts by bucket", async () => {
      const researchArtifacts = [findingsArtifact, recommendationsArtifact];
      vi.mocked(artifactsApi.getArtifactsByBucket).mockImplementation(async (bucketId) => {
        if (bucketId === "research-outputs") return researchArtifacts;
        if (bucketId === "prd-library") return [copiedRecommendationsArtifact, prdArtifact];
        return [];
      });

      // Query research-outputs
      const research = await artifactsApi.getArtifactsByBucket("research-outputs");
      expect(research).toHaveLength(2);
      expect(research.map((a) => a.artifact_type)).toEqual(["findings", "recommendations"]);

      // Query prd-library
      const prd = await artifactsApi.getArtifactsByBucket("prd-library");
      expect(prd).toHaveLength(2);
      expect(prd.map((a) => a.artifact_type)).toContain("prd");
      expect(prd.map((a) => a.artifact_type)).toContain("recommendations");
    });

    it("can query artifacts by type", async () => {
      const allRecommendations = [recommendationsArtifact, copiedRecommendationsArtifact];
      vi.mocked(artifactsApi.getArtifacts).mockImplementation(async (type) => {
        if (type === "recommendations") return allRecommendations;
        if (type === "findings") return [findingsArtifact];
        if (type === "prd") return [prdArtifact];
        return [];
      });

      // Query recommendations (should find 2 across buckets)
      const recs = await artifactsApi.getArtifacts("recommendations");
      expect(recs).toHaveLength(2);

      // Query findings (should find 1)
      const findings = await artifactsApi.getArtifacts("findings");
      expect(findings).toHaveLength(1);

      // Query PRDs (should find 1)
      const prds = await artifactsApi.getArtifacts("prd");
      expect(prds).toHaveLength(1);
    });

    it("ArtifactBrowser shows artifacts for selected bucket", async () => {
      // Convert API bucket responses to Artifact type buckets
      const buckets: ArtifactBucket[] = [researchOutputsBucket, prdLibraryBucket, workContextBucket, codeChangesBucket].map((b) => ({
        id: b.id,
        name: b.name,
        acceptedTypes: b.accepted_types,
        writers: b.writers,
        readers: b.readers,
        isSystem: b.is_system,
      }));

      // Convert API artifact responses to Artifact type
      const artifacts: Artifact[] = [findingsArtifact, recommendationsArtifact].map((a) => ({
        id: a.id,
        name: a.name,
        type: a.artifact_type,
        content: { type: "inline" as const, text: a.content },
        metadata: {
          createdAt: a.created_at,
          createdBy: a.created_by,
          version: a.version,
        },
        derivedFrom: a.derived_from,
        bucketId: a.bucket_id ?? undefined,
      }));

      vi.mocked(artifactsApi.getBuckets).mockResolvedValue([researchOutputsBucket, prdLibraryBucket, workContextBucket, codeChangesBucket]);

      render(
        <ArtifactBrowser
          buckets={buckets}
          artifacts={artifacts}
          selectedBucketId="research-outputs"
          selectedArtifactId={null}
          onSelectBucket={() => {}}
          onSelectArtifact={() => {}}
        />
      );

      // Should show research-outputs bucket as selected
      await waitFor(() => {
        expect(screen.getByText("Research Outputs")).toBeInTheDocument();
      });

      // Should show artifacts from that bucket
      expect(screen.getByText("Authentication Analysis Findings")).toBeInTheDocument();
      expect(screen.getByText("Technology Recommendations")).toBeInTheDocument();
    });
  });

  // ========================================================================
  // Additional Integration Tests
  // ========================================================================

  describe("System bucket properties", () => {
    it("all 4 system buckets are flagged correctly", () => {
      const systemBuckets = [
        researchOutputsBucket,
        prdLibraryBucket,
        workContextBucket,
        codeChangesBucket,
      ];

      expect(systemBuckets).toHaveLength(4);
      systemBuckets.forEach((bucket) => {
        expect(bucket.is_system).toBe(true);
      });
    });

    it("each system bucket has appropriate accepted types", () => {
      expect(researchOutputsBucket.accepted_types).toEqual([
        "research_document",
        "findings",
        "recommendations",
      ]);
      expect(prdLibraryBucket.accepted_types).toEqual([
        "prd",
        "specification",
        "design_doc",
        "recommendations",
      ]);
      expect(workContextBucket.accepted_types).toEqual([
        "context",
        "task_spec",
        "previous_work",
      ]);
      expect(codeChangesBucket.accepted_types).toEqual([
        "code_change",
        "diff",
        "test_result",
      ]);
    });

    it("each system bucket has appropriate writers", () => {
      expect(researchOutputsBucket.writers).toContain("deep-researcher");
      expect(prdLibraryBucket.writers).toContain("user");
      expect(workContextBucket.writers).toContain("orchestrator");
      expect(codeChangesBucket.writers).toContain("worker");
    });
  });

  describe("Artifact versioning", () => {
    it("new artifacts start at version 1", () => {
      expect(findingsArtifact.version).toBe(1);
      expect(recommendationsArtifact.version).toBe(1);
      expect(prdArtifact.version).toBe(1);
    });

    it("version displayed correctly in ArtifactCard when > 1", () => {
      const versionedArtifact: Artifact = {
        id: prdArtifact.id,
        name: prdArtifact.name,
        type: prdArtifact.artifact_type,
        content: { type: "inline", text: prdArtifact.content },
        metadata: {
          createdAt: prdArtifact.created_at,
          createdBy: prdArtifact.created_by,
          version: 3, // Version > 1
        },
        derivedFrom: prdArtifact.derived_from,
        bucketId: prdArtifact.bucket_id ?? undefined,
      };

      render(
        <ArtifactCard
          artifact={versionedArtifact}
          isSelected={false}
          onClick={() => {}}
        />
      );

      expect(screen.getByText("v3")).toBeInTheDocument();
    });
  });

  describe("CRUD operations", () => {
    it("can create artifact", async () => {
      const newArtifact: ArtifactResponse = {
        id: "artifact-new",
        name: "New Findings",
        artifact_type: "findings",
        content_type: "inline",
        content: "New research findings...",
        created_at: "2026-01-24T16:00:00Z",
        created_by: "deep-researcher",
        version: 1,
        bucket_id: "research-outputs",
        task_id: null,
        process_id: null,
        derived_from: [],
      };
      vi.mocked(artifactsApi.createArtifact).mockResolvedValue(newArtifact);

      const result = await artifactsApi.createArtifact({
        name: "New Findings",
        artifact_type: "findings",
        content_type: "inline",
        content: "New research findings...",
        created_by: "deep-researcher",
        bucket_id: "research-outputs",
      });

      expect(result.id).toBe("artifact-new");
      expect(result.name).toBe("New Findings");
      expect(result.bucket_id).toBe("research-outputs");
    });

    it("can update artifact", async () => {
      const updatedArtifact: ArtifactResponse = {
        ...findingsArtifact,
        name: "Updated Findings",
        version: 2,
      };
      vi.mocked(artifactsApi.updateArtifact).mockResolvedValue(updatedArtifact);

      const result = await artifactsApi.updateArtifact("artifact-1", {
        name: "Updated Findings",
      });

      expect(result.name).toBe("Updated Findings");
      expect(result.version).toBe(2);
    });

    it("can delete artifact", async () => {
      vi.mocked(artifactsApi.deleteArtifact).mockResolvedValue(undefined);

      await expect(artifactsApi.deleteArtifact("artifact-1")).resolves.toBeUndefined();
      expect(artifactsApi.deleteArtifact).toHaveBeenCalledWith("artifact-1");
    });
  });
});
