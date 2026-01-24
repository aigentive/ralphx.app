/**
 * ArtifactBrowser - Browse artifacts organized by bucket
 *
 * Features:
 * - Bucket sidebar with item counts
 * - System bucket indicators
 * - Artifact list filtered by bucket
 * - Artifact selection
 * - Loading and empty states
 */

import type { Artifact, ArtifactBucket } from "@/types/artifact";
import { ArtifactCard } from "./ArtifactCard";

// ============================================================================
// Types
// ============================================================================

interface ArtifactBrowserProps {
  buckets: ArtifactBucket[];
  artifacts: Artifact[];
  selectedBucketId: string | null;
  selectedArtifactId: string | null;
  onSelectBucket: (bucketId: string) => void;
  onSelectArtifact: (artifactId: string) => void;
  isLoading?: boolean;
}

// ============================================================================
// Component
// ============================================================================

export function ArtifactBrowser({
  buckets,
  artifacts,
  selectedBucketId,
  selectedArtifactId,
  onSelectBucket,
  onSelectArtifact,
  isLoading = false,
}: ArtifactBrowserProps) {
  // Count artifacts per bucket
  const bucketCounts = artifacts.reduce<Record<string, number>>((acc, a) => {
    if (a.bucketId) acc[a.bucketId] = (acc[a.bucketId] ?? 0) + 1;
    return acc;
  }, {});

  // Filter artifacts by selected bucket
  const filteredArtifacts = selectedBucketId
    ? artifacts.filter((a) => a.bucketId === selectedBucketId)
    : [];

  const handleBucketClick = (bucketId: string) => {
    if (!isLoading) onSelectBucket(bucketId);
  };

  return (
    <div data-testid="artifact-browser" className="flex h-full" style={{ backgroundColor: "var(--bg-base)" }}>
      {/* Bucket Sidebar */}
      <nav data-testid="bucket-sidebar" role="navigation" aria-label="Artifact buckets"
        className="w-48 flex-shrink-0 p-2 border-r overflow-y-auto"
        style={{ backgroundColor: "var(--bg-surface)", borderColor: "var(--border-subtle)" }}>
        {isLoading && <div data-testid="loading-indicator" className="text-xs text-center py-2" style={{ color: "var(--text-muted)" }}>Loading...</div>}
        {buckets.length === 0 ? (
          <div className="text-sm text-center py-4" style={{ color: "var(--text-muted)" }}>No buckets available</div>
        ) : (
          buckets.map((bucket) => {
            const isSelected = bucket.id === selectedBucketId;
            const count = bucketCounts[bucket.id] ?? 0;
            return (
              <button key={bucket.id} data-testid="bucket-item" data-selected={isSelected ? "true" : "false"} type="button"
                onClick={() => handleBucketClick(bucket.id)} aria-label={bucket.name}
                className="w-full flex items-center justify-between px-2 py-1.5 mb-1 rounded text-left text-sm transition-colors hover:bg-[--bg-hover]"
                style={{ backgroundColor: isSelected ? "var(--bg-hover)" : undefined, color: "var(--text-primary)" }}>
                <div className="flex items-center gap-1 min-w-0">
                  <span className="truncate">{bucket.name}</span>
                  {bucket.isSystem && (
                    <span data-testid="system-badge" className="text-xs px-1 rounded" style={{ backgroundColor: "var(--bg-base)", color: "var(--text-muted)" }}>S</span>
                  )}
                </div>
                <span data-testid="bucket-count" className="text-xs flex-shrink-0" style={{ color: "var(--text-muted)" }}>{count}</span>
              </button>
            );
          })
        )}
      </nav>

      {/* Artifact List */}
      <div data-testid="artifact-list" className="flex-1 p-3 overflow-y-auto">
        {!selectedBucketId ? (
          <div className="text-sm text-center py-8" style={{ color: "var(--text-muted)" }}>Select a bucket to view artifacts</div>
        ) : filteredArtifacts.length === 0 ? (
          <div className="text-sm text-center py-8" style={{ color: "var(--text-muted)" }}>No artifacts in this bucket</div>
        ) : (
          <div className="space-y-2">
            {filteredArtifacts.map((artifact) => (
              <ArtifactCard key={artifact.id} artifact={artifact} onClick={onSelectArtifact}
                isSelected={artifact.id === selectedArtifactId} disabled={isLoading} />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
