/**
 * DiffViewer - Split-view diff component with Changes and History tabs
 *
 * Features:
 * - Two tabs: Changes (uncommitted) and History (commits)
 * - File tree on left showing changed files with collapsible directories
 * - Unified diff view on right with syntax highlighting
 * - Collapse/expand hunks
 * - Open in IDE button using Tauri shell commands
 * - Web Worker support for off-main-thread diff computation
 *
 * Library: @git-diff-view/react for optimized diff rendering
 * Icons: Lucide React
 * Components: shadcn/ui (Tabs, Button, ScrollArea, Tooltip, Skeleton)
 */

import { useState, useCallback, useEffect } from "react";
import {
  GitBranch,
  History,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { ScrollArea } from "@/components/ui/scroll-area";

// Import types and utilities from separate file
import {
  type DiffViewTab,
  type FileChange,
  type Commit,
  type DiffData,
  type DiffViewerProps,
} from "./DiffViewer.types";

// Import sub-components
import {
  FileTree,
  CommitList,
  DiffPanel,
  CommitDiffPanel,
  FileTreeSkeleton,
} from "./DiffViewer.components";

// Re-export types for external use
export type { DiffViewTab, FileChange, Commit, DiffData, DiffViewerProps };

// ============================================================================
// Main Component
// ============================================================================

export function DiffViewer({
  changes,
  commits,
  commitFiles: commitFilesProp = [],
  onFetchDiff,
  onFetchCommitFiles,
  onOpenInIDE,
  isLoadingChanges = false,
  isLoadingHistory = false,
  isLoadingCommitFiles = false,
  defaultTab = "changes",
  onTabChange,
  onCommitSelect,
}: DiffViewerProps) {
  const [activeTab, setActiveTab] = useState<DiffViewTab>(defaultTab);
  const [selectedFilePath, setSelectedFilePath] = useState<string | null>(null);
  const [selectedCommit, setSelectedCommit] = useState<Commit | null>(null);
  const [commitSelectedFile, setCommitSelectedFile] = useState<string | null>(null);
  const [diffData, setDiffData] = useState<DiffData | null>(null);
  const [isDiffLoading, setIsDiffLoading] = useState(false);

  // Handle tab change
  const handleTabChange = useCallback((value: string) => {
    const tab = value as DiffViewTab;
    setActiveTab(tab);
    onTabChange?.(tab);
    // Reset selections when switching tabs
    setSelectedFilePath(null);
    setSelectedCommit(null);
    setCommitSelectedFile(null);
    setDiffData(null);
  }, [onTabChange]);

  // Handle file selection in Changes tab
  const handleFileSelect = useCallback(async (path: string) => {
    setSelectedFilePath(path);
    setIsDiffLoading(true);
    try {
      const data = await onFetchDiff(path);
      setDiffData(data);
    } catch {
      setDiffData(null);
    } finally {
      setIsDiffLoading(false);
    }
  }, [onFetchDiff]);

  // Handle commit selection in History tab
  const handleCommitSelect = useCallback(async (commit: Commit) => {
    setSelectedCommit(commit);
    setCommitSelectedFile(null);
    setDiffData(null);
    onCommitSelect?.(commit);
    // Fetch files changed in this commit
    if (onFetchCommitFiles) {
      await onFetchCommitFiles(commit.sha);
    }
  }, [onCommitSelect, onFetchCommitFiles]);

  // Handle file selection within a commit
  const handleCommitFileSelect = useCallback(async (path: string) => {
    if (!selectedCommit) return;
    setCommitSelectedFile(path);
    setIsDiffLoading(true);
    try {
      const data = await onFetchDiff(path, selectedCommit.sha);
      setDiffData(data);
    } catch {
      setDiffData(null);
    } finally {
      setIsDiffLoading(false);
    }
  }, [selectedCommit, onFetchDiff]);

  // Auto-select first file when changes load
  useEffect(() => {
    if (activeTab === "changes" && changes.length > 0 && !selectedFilePath) {
      const firstFile = changes[0];
      if (firstFile) {
        handleFileSelect(firstFile.path);
      }
    }
  }, [activeTab, changes, selectedFilePath, handleFileSelect]);

  return (
    <div
      className="flex flex-col h-full bg-[var(--bg-base)]"
      data-testid="diff-viewer"
    >
      <Tabs
        value={activeTab}
        onValueChange={handleTabChange}
        className="flex flex-col h-full"
      >
        {/* Tab navigation */}
        <TabsList
          className={cn(
            "h-12 px-4 rounded-none justify-start gap-0",
            "bg-[var(--bg-surface)] border-b border-[var(--border-subtle)]"
          )}
        >
          <TabsTrigger
            value="changes"
            data-testid="tab-changes"
            className={cn(
              "h-12 px-4 gap-2 rounded-none border-b-2 border-transparent",
              "text-sm font-medium text-[var(--text-secondary)]",
              "data-[state=active]:text-[var(--text-primary)]",
              "data-[state=active]:border-b-[var(--accent-primary)]",
              "data-[state=active]:shadow-none data-[state=active]:bg-transparent",
              "hover:text-[var(--text-primary)]",
              "transition-colors duration-150"
            )}
          >
            <GitBranch className="w-4 h-4" />
            Changes
            {changes.length > 0 && (
              <span
                className={cn(
                  "px-1.5 py-0.5 text-xs rounded-full ml-1",
                  activeTab === "changes"
                    ? "bg-[var(--accent-primary)] text-white"
                    : "bg-[var(--bg-elevated)] text-[var(--text-secondary)]"
                )}
              >
                {changes.length}
              </span>
            )}
          </TabsTrigger>
          <TabsTrigger
            value="history"
            data-testid="tab-history"
            className={cn(
              "h-12 px-4 gap-2 rounded-none border-b-2 border-transparent",
              "text-sm font-medium text-[var(--text-secondary)]",
              "data-[state=active]:text-[var(--text-primary)]",
              "data-[state=active]:border-b-[var(--accent-primary)]",
              "data-[state=active]:shadow-none data-[state=active]:bg-transparent",
              "hover:text-[var(--text-primary)]",
              "transition-colors duration-150"
            )}
          >
            <History className="w-4 h-4" />
            History
            {commits.length > 0 && (
              <span
                className={cn(
                  "px-1.5 py-0.5 text-xs rounded-full ml-1",
                  activeTab === "history"
                    ? "bg-[var(--accent-primary)] text-white"
                    : "bg-[var(--bg-elevated)] text-[var(--text-secondary)]"
                )}
              >
                {commits.length}
              </span>
            )}
          </TabsTrigger>
        </TabsList>

        {/* Changes tab content */}
        <TabsContent value="changes" className="flex-1 flex min-h-0 mt-0">
          {/* File tree */}
          <div
            className="shrink-0 overflow-hidden border-r border-[var(--border-subtle)]"
            style={{
              width: "25%",
              minWidth: "200px",
              maxWidth: "40%",
              backgroundColor: "var(--bg-surface)",
            }}
          >
            <ScrollArea className="h-full">
              {isLoadingChanges ? (
                <FileTreeSkeleton />
              ) : (
                <FileTree
                  files={changes}
                  selectedPath={selectedFilePath}
                  onSelect={handleFileSelect}
                />
              )}
            </ScrollArea>
          </div>

          {/* Diff panel */}
          <div className="flex-1 min-w-0">
            <DiffPanel
              diffData={diffData}
              isLoading={isDiffLoading}
              filePath={selectedFilePath}
              {...(onOpenInIDE !== undefined && { onOpenInIDE })}
            />
          </div>
        </TabsContent>

        {/* History tab content */}
        <TabsContent value="history" className="flex-1 flex min-h-0 mt-0">
          {/* Commit list */}
          <div
            className="shrink-0 overflow-hidden border-r border-[var(--border-subtle)]"
            style={{
              width: "25%",
              minWidth: "200px",
              maxWidth: "40%",
              backgroundColor: "var(--bg-surface)",
            }}
          >
            <ScrollArea className="h-full">
              {isLoadingHistory ? (
                <FileTreeSkeleton />
              ) : (
                <CommitList
                  commits={commits}
                  selectedSha={selectedCommit?.sha ?? null}
                  onSelect={handleCommitSelect}
                />
              )}
            </ScrollArea>
          </div>

          {/* Commit diff panel */}
          <div className="flex-1 min-w-0">
            <CommitDiffPanel
              commit={selectedCommit}
              files={commitFilesProp}
              selectedFilePath={commitSelectedFile}
              onSelectFile={handleCommitFileSelect}
              diffData={diffData}
              isLoading={isDiffLoading}
              isLoadingFiles={isLoadingCommitFiles}
              {...(onOpenInIDE !== undefined && { onOpenInIDE })}
            />
          </div>
        </TabsContent>
      </Tabs>
    </div>
  );
}
