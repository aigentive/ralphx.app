/**
 * DiffViewer - Split-view diff component with Changes and History tabs
 *
 * Features:
 * - Two tabs: Changes (uncommitted) and History (commits)
 * - File tree on left showing changed files
 * - Unified diff view on right with syntax highlighting
 * - Collapse/expand hunks
 * - Open in IDE button using Tauri shell commands
 * - Web Worker support for off-main-thread diff computation
 *
 * Library: @git-diff-view/react for optimized diff rendering
 */

import { useState, useMemo, useCallback, useEffect } from "react";
import { DiffView, DiffModeEnum } from "@git-diff-view/react";
import "@git-diff-view/react/styles/diff-view.css";

// ============================================================================
// Types
// ============================================================================

export type DiffViewTab = "changes" | "history";

export interface FileChange {
  /** File path relative to repository root */
  path: string;
  /** Change status: added, modified, deleted, renamed */
  status: "added" | "modified" | "deleted" | "renamed";
  /** Number of additions */
  additions: number;
  /** Number of deletions */
  deletions: number;
  /** Old path for renamed files */
  oldPath?: string;
}

export interface Commit {
  /** Commit SHA */
  sha: string;
  /** Short SHA (first 7 characters) */
  shortSha: string;
  /** Commit message subject line */
  message: string;
  /** Author name */
  author: string;
  /** Commit date */
  date: Date;
}

export interface DiffData {
  /** File path */
  filePath: string;
  /** Old content (before change) */
  oldContent: string;
  /** New content (after change) */
  newContent: string;
  /** Unified diff hunks (output from git diff) */
  hunks: string[];
  /** File language for syntax highlighting */
  language?: string;
}

export interface DiffViewerProps {
  /** Current changes (uncommitted) */
  changes: FileChange[];
  /** Commit history */
  commits: Commit[];
  /** Callback to fetch diff for a file */
  onFetchDiff: (filePath: string, commitSha?: string) => Promise<DiffData | null>;
  /** Callback to open file in IDE */
  onOpenInIDE?: (filePath: string) => void;
  /** Loading state for changes */
  isLoadingChanges?: boolean;
  /** Loading state for history */
  isLoadingHistory?: boolean;
  /** Currently active tab */
  defaultTab?: DiffViewTab;
  /** Callback when tab changes */
  onTabChange?: (tab: DiffViewTab) => void;
  /** Callback when commit is selected */
  onCommitSelect?: (commit: Commit) => void;
}

// ============================================================================
// Icons
// ============================================================================

function FileAddedIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
      <rect x="2" y="1" width="10" height="12" rx="1" stroke="currentColor" strokeWidth="1.2" />
      <path d="M5 7h4M7 5v4" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" />
    </svg>
  );
}

function FileModifiedIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
      <rect x="2" y="1" width="10" height="12" rx="1" stroke="currentColor" strokeWidth="1.2" />
      <circle cx="7" cy="7" r="2" stroke="currentColor" strokeWidth="1.2" />
    </svg>
  );
}

function FileDeletedIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
      <rect x="2" y="1" width="10" height="12" rx="1" stroke="currentColor" strokeWidth="1.2" />
      <path d="M5 7h4" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" />
    </svg>
  );
}

function FileRenamedIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
      <rect x="2" y="1" width="10" height="12" rx="1" stroke="currentColor" strokeWidth="1.2" />
      <path d="M5 7h4M8 5l2 2-2 2" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

function ChevronDownIcon() {
  return (
    <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
      <path d="M3 4.5L6 7.5L9 4.5" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

// ChevronRightIcon available for future directory expansion use
// function ChevronRightIcon() { ... }

function ExternalLinkIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
      <path d="M9 2h3v3M5 9l7-7M7 2H3a1 1 0 00-1 1v8a1 1 0 001 1h8a1 1 0 001-1V7" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

function CommitIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
      <circle cx="7" cy="7" r="3" stroke="currentColor" strokeWidth="1.2" />
      <path d="M7 1v3M7 10v3" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" />
    </svg>
  );
}

function FolderIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
      <path d="M1 4V3a1 1 0 011-1h3l1.5 2H12a1 1 0 011 1v6a1 1 0 01-1 1H2a1 1 0 01-1-1V4z" stroke="currentColor" strokeWidth="1.2" />
    </svg>
  );
}

// ============================================================================
// Utility Functions
// ============================================================================

function getFileIcon(status: FileChange["status"]) {
  switch (status) {
    case "added":
      return <FileAddedIcon />;
    case "modified":
      return <FileModifiedIcon />;
    case "deleted":
      return <FileDeletedIcon />;
    case "renamed":
      return <FileRenamedIcon />;
  }
}

function getStatusColor(status: FileChange["status"]) {
  switch (status) {
    case "added":
      return "var(--status-success)";
    case "modified":
      return "var(--accent-primary)";
    case "deleted":
      return "var(--status-error)";
    case "renamed":
      return "var(--status-info)";
  }
}

function getFileName(path: string): string {
  const parts = path.split("/");
  return parts[parts.length - 1] ?? path;
}

function getDirectory(path: string): string {
  const parts = path.split("/");
  if (parts.length <= 1) return "";
  parts.pop();
  return parts.join("/");
}

function formatRelativeDate(date: Date): string {
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMins = Math.floor(diffMs / 60000);
  const diffHours = Math.floor(diffMs / 3600000);
  const diffDays = Math.floor(diffMs / 86400000);

  if (diffMins < 1) return "just now";
  if (diffMins < 60) return `${diffMins}m ago`;
  if (diffHours < 24) return `${diffHours}h ago`;
  if (diffDays < 7) return `${diffDays}d ago`;
  return date.toLocaleDateString();
}

function getLanguageFromPath(path: string): string {
  const ext = path.split(".").pop()?.toLowerCase() ?? "";
  const langMap: Record<string, string> = {
    ts: "typescript",
    tsx: "typescript",
    js: "javascript",
    jsx: "javascript",
    rs: "rust",
    py: "python",
    go: "go",
    java: "java",
    c: "c",
    cpp: "cpp",
    h: "c",
    hpp: "cpp",
    cs: "csharp",
    rb: "ruby",
    php: "php",
    swift: "swift",
    kt: "kotlin",
    md: "markdown",
    json: "json",
    yaml: "yaml",
    yml: "yaml",
    toml: "toml",
    html: "html",
    css: "css",
    scss: "scss",
    sql: "sql",
    sh: "bash",
    bash: "bash",
    zsh: "bash",
  };
  return langMap[ext] ?? "plaintext";
}

// Group files by directory for tree view
interface DirectoryNode {
  name: string;
  path: string;
  isDirectory: true;
  children: (DirectoryNode | FileNode)[];
  expanded: boolean;
}

interface FileNode {
  name: string;
  path: string;
  isDirectory: false;
  file: FileChange;
}

type TreeNode = DirectoryNode | FileNode;

function buildFileTree(files: FileChange[]): TreeNode[] {
  const root: DirectoryNode = {
    name: "",
    path: "",
    isDirectory: true,
    children: [],
    expanded: true,
  };

  for (const file of files) {
    const parts = file.path.split("/");
    let current = root;

    for (let i = 0; i < parts.length; i++) {
      const part = parts[i];
      if (part === undefined) continue;

      const isFile = i === parts.length - 1;
      const pathSoFar = parts.slice(0, i + 1).join("/");

      if (isFile) {
        current.children.push({
          name: part,
          path: file.path,
          isDirectory: false,
          file,
        });
      } else {
        let dirNode = current.children.find(
          (c): c is DirectoryNode => c.isDirectory && c.name === part
        );
        if (!dirNode) {
          dirNode = {
            name: part,
            path: pathSoFar,
            isDirectory: true,
            children: [],
            expanded: true,
          };
          current.children.push(dirNode);
        }
        current = dirNode;
      }
    }
  }

  // Sort: directories first, then files, both alphabetically
  function sortChildren(node: DirectoryNode): void {
    node.children.sort((a, b) => {
      if (a.isDirectory && !b.isDirectory) return -1;
      if (!a.isDirectory && b.isDirectory) return 1;
      return a.name.localeCompare(b.name);
    });
    for (const child of node.children) {
      if (child.isDirectory) {
        sortChildren(child);
      }
    }
  }
  sortChildren(root);

  return root.children;
}

// ============================================================================
// Sub-components
// ============================================================================

interface TabBarProps {
  activeTab: DiffViewTab;
  onTabChange: (tab: DiffViewTab) => void;
  changesCount: number;
  commitsCount: number;
}

function TabBar({ activeTab, onTabChange, changesCount, commitsCount }: TabBarProps) {
  return (
    <div
      className="flex border-b"
      style={{ borderColor: "var(--border-subtle)" }}
    >
      <button
        role="tab"
        aria-selected={activeTab === "changes"}
        data-testid="tab-changes"
        onClick={() => onTabChange("changes")}
        className="flex items-center gap-2 px-4 py-2.5 text-sm font-medium transition-colors"
        style={{
          color: activeTab === "changes" ? "var(--text-primary)" : "var(--text-secondary)",
          borderBottom: activeTab === "changes" ? "2px solid var(--accent-primary)" : "2px solid transparent",
          marginBottom: "-1px",
        }}
      >
        Changes
        {changesCount > 0 && (
          <span
            className="px-1.5 py-0.5 text-xs rounded-full"
            style={{
              backgroundColor: activeTab === "changes" ? "var(--accent-primary)" : "var(--bg-elevated)",
              color: activeTab === "changes" ? "white" : "var(--text-secondary)",
            }}
          >
            {changesCount}
          </span>
        )}
      </button>
      <button
        role="tab"
        aria-selected={activeTab === "history"}
        data-testid="tab-history"
        onClick={() => onTabChange("history")}
        className="flex items-center gap-2 px-4 py-2.5 text-sm font-medium transition-colors"
        style={{
          color: activeTab === "history" ? "var(--text-primary)" : "var(--text-secondary)",
          borderBottom: activeTab === "history" ? "2px solid var(--accent-primary)" : "2px solid transparent",
          marginBottom: "-1px",
        }}
      >
        History
        {commitsCount > 0 && (
          <span
            className="px-1.5 py-0.5 text-xs rounded-full"
            style={{
              backgroundColor: activeTab === "history" ? "var(--accent-primary)" : "var(--bg-elevated)",
              color: activeTab === "history" ? "white" : "var(--text-secondary)",
            }}
          >
            {commitsCount}
          </span>
        )}
      </button>
    </div>
  );
}

interface FileTreeItemProps {
  node: TreeNode;
  depth: number;
  selectedPath: string | null;
  onSelect: (path: string) => void;
  onToggleExpand?: ((path: string) => void) | undefined;
  expandedDirs: Set<string>;
}

function FileTreeItem({
  node,
  depth,
  selectedPath,
  onSelect,
  onToggleExpand,
  expandedDirs,
}: FileTreeItemProps) {
  const isSelected = selectedPath === node.path;
  const isExpanded = node.isDirectory && expandedDirs.has(node.path);

  if (node.isDirectory) {
    return (
      <>
        <button
          data-testid={`dir-${node.path}`}
          onClick={() => onToggleExpand?.(node.path)}
          className="w-full flex items-center gap-1.5 px-2 py-1 text-sm transition-colors hover:bg-white/5"
          style={{
            paddingLeft: `${depth * 12 + 8}px`,
            color: "var(--text-secondary)",
          }}
        >
          <span
            className="transition-transform"
            style={{
              transform: isExpanded ? "rotate(0deg)" : "rotate(-90deg)",
            }}
          >
            <ChevronDownIcon />
          </span>
          <span style={{ color: "var(--text-muted)" }}>
            <FolderIcon />
          </span>
          <span className="truncate">{node.name}</span>
        </button>
        {isExpanded && (
          <div>
            {node.children.map((child) => (
              <FileTreeItem
                key={child.path}
                node={child}
                depth={depth + 1}
                selectedPath={selectedPath}
                onSelect={onSelect}
                onToggleExpand={onToggleExpand}
                expandedDirs={expandedDirs}
              />
            ))}
          </div>
        )}
      </>
    );
  }

  const file = node.file;
  return (
    <button
      data-testid={`file-${file.path}`}
      onClick={() => onSelect(file.path)}
      className="w-full flex items-center gap-1.5 px-2 py-1 text-sm transition-colors"
      style={{
        paddingLeft: `${depth * 12 + 8}px`,
        backgroundColor: isSelected ? "var(--bg-elevated)" : "transparent",
        color: isSelected ? "var(--text-primary)" : "var(--text-secondary)",
      }}
    >
      <span className="w-3" />
      <span style={{ color: getStatusColor(file.status) }}>
        {getFileIcon(file.status)}
      </span>
      <span className="truncate flex-1 text-left">{node.name}</span>
      <span
        className="text-xs shrink-0"
        style={{ color: "var(--text-muted)" }}
      >
        +{file.additions} -{file.deletions}
      </span>
    </button>
  );
}

interface FileTreeProps {
  files: FileChange[];
  selectedPath: string | null;
  onSelect: (path: string) => void;
}

function FileTree({ files, selectedPath, onSelect }: FileTreeProps) {
  const [expandedDirs, setExpandedDirs] = useState<Set<string>>(() => {
    // Initially expand all directories
    const dirs = new Set<string>();
    for (const file of files) {
      const parts = file.path.split("/");
      let path = "";
      for (let i = 0; i < parts.length - 1; i++) {
        const part = parts[i];
        if (part === undefined) continue;
        path = path ? `${path}/${part}` : part;
        dirs.add(path);
      }
    }
    return dirs;
  });

  const tree = useMemo(() => buildFileTree(files), [files]);

  const handleToggleExpand = useCallback((path: string) => {
    setExpandedDirs((prev) => {
      const next = new Set(prev);
      if (next.has(path)) {
        next.delete(path);
      } else {
        next.add(path);
      }
      return next;
    });
  }, []);

  if (files.length === 0) {
    return (
      <div
        className="flex flex-col items-center justify-center h-full p-4 text-center"
        data-testid="file-tree-empty"
      >
        <p style={{ color: "var(--text-secondary)" }}>No changes</p>
        <p className="text-xs mt-1" style={{ color: "var(--text-muted)" }}>
          Working tree is clean
        </p>
      </div>
    );
  }

  return (
    <div className="py-2" data-testid="file-tree">
      {tree.map((node) => (
        <FileTreeItem
          key={node.path}
          node={node}
          depth={0}
          selectedPath={selectedPath}
          onSelect={onSelect}
          onToggleExpand={handleToggleExpand}
          expandedDirs={expandedDirs}
        />
      ))}
    </div>
  );
}

interface CommitListProps {
  commits: Commit[];
  selectedSha: string | null;
  onSelect: (commit: Commit) => void;
}

function CommitList({ commits, selectedSha, onSelect }: CommitListProps) {
  if (commits.length === 0) {
    return (
      <div
        className="flex flex-col items-center justify-center h-full p-4 text-center"
        data-testid="commit-list-empty"
      >
        <p style={{ color: "var(--text-secondary)" }}>No commits</p>
        <p className="text-xs mt-1" style={{ color: "var(--text-muted)" }}>
          Commit history will appear here
        </p>
      </div>
    );
  }

  return (
    <div className="py-2" data-testid="commit-list">
      {commits.map((commit) => {
        const isSelected = selectedSha === commit.sha;
        return (
          <button
            key={commit.sha}
            data-testid={`commit-${commit.shortSha}`}
            onClick={() => onSelect(commit)}
            className="w-full flex items-start gap-3 px-3 py-2.5 text-left transition-colors"
            style={{
              backgroundColor: isSelected ? "var(--bg-elevated)" : "transparent",
            }}
          >
            <span
              className="mt-0.5 shrink-0"
              style={{ color: isSelected ? "var(--accent-primary)" : "var(--text-muted)" }}
            >
              <CommitIcon />
            </span>
            <div className="flex-1 min-w-0">
              <p
                className="text-sm font-medium truncate"
                style={{ color: "var(--text-primary)" }}
              >
                {commit.message}
              </p>
              <p
                className="text-xs mt-0.5 flex items-center gap-2"
                style={{ color: "var(--text-muted)" }}
              >
                <span className="font-mono">{commit.shortSha}</span>
                <span>by {commit.author}</span>
                <span>{formatRelativeDate(commit.date)}</span>
              </p>
            </div>
          </button>
        );
      })}
    </div>
  );
}

interface DiffPanelProps {
  diffData: DiffData | null;
  isLoading: boolean;
  filePath: string | null;
  onOpenInIDE?: ((path: string) => void) | undefined;
}

function DiffPanel({
  diffData,
  isLoading,
  filePath,
  onOpenInIDE,
}: DiffPanelProps) {
  if (isLoading) {
    return (
      <div
        className="flex items-center justify-center h-full"
        data-testid="diff-loading"
      >
        <div className="flex flex-col items-center gap-3">
          <div
            className="w-6 h-6 border-2 rounded-full animate-spin"
            style={{
              borderColor: "var(--border-subtle)",
              borderTopColor: "var(--accent-primary)",
            }}
          />
          <p style={{ color: "var(--text-secondary)" }}>Loading diff...</p>
        </div>
      </div>
    );
  }

  if (!filePath) {
    return (
      <div
        className="flex flex-col items-center justify-center h-full p-8 text-center"
        data-testid="diff-empty"
      >
        <svg
          width="48"
          height="48"
          viewBox="0 0 48 48"
          fill="none"
          className="mb-4"
          style={{ color: "var(--text-muted)" }}
        >
          <rect x="6" y="8" width="36" height="32" rx="2" stroke="currentColor" strokeWidth="2" />
          <path d="M14 20h20M14 28h14" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
        </svg>
        <p style={{ color: "var(--text-secondary)" }}>
          Select a file to view diff
        </p>
        <p className="text-sm mt-1" style={{ color: "var(--text-muted)" }}>
          Changes will be displayed here
        </p>
      </div>
    );
  }

  if (!diffData) {
    return (
      <div
        className="flex flex-col items-center justify-center h-full p-8 text-center"
        data-testid="diff-error"
      >
        <p style={{ color: "var(--text-secondary)" }}>
          Unable to load diff for this file
        </p>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full" data-testid="diff-content">
      {/* File header */}
      <div
        className="flex items-center justify-between px-4 py-2 border-b shrink-0"
        style={{ borderColor: "var(--border-subtle)", backgroundColor: "var(--bg-surface)" }}
      >
        <div className="flex items-center gap-2 min-w-0">
          <span className="font-mono text-sm truncate" style={{ color: "var(--text-primary)" }}>
            {getFileName(filePath)}
          </span>
          <span className="text-xs" style={{ color: "var(--text-muted)" }}>
            {getDirectory(filePath)}
          </span>
        </div>
        {onOpenInIDE && (
          <button
            data-testid="open-in-ide"
            onClick={() => onOpenInIDE(filePath)}
            className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-md transition-colors"
            style={{
              backgroundColor: "var(--bg-elevated)",
              color: "var(--text-secondary)",
            }}
            title="Open in IDE"
          >
            <ExternalLinkIcon />
            <span>Open in IDE</span>
          </button>
        )}
      </div>

      {/* Diff view */}
      <div className="flex-1 overflow-auto diff-viewer-container">
        <DiffView
          data={{
            oldFile: {
              fileName: diffData.filePath,
              content: diffData.oldContent,
              fileLang: diffData.language ?? getLanguageFromPath(diffData.filePath),
            },
            newFile: {
              fileName: diffData.filePath,
              content: diffData.newContent,
              fileLang: diffData.language ?? getLanguageFromPath(diffData.filePath),
            },
            hunks: diffData.hunks,
          }}
          diffViewMode={DiffModeEnum.Unified}
          diffViewFontSize={13}
          diffViewHighlight={true}
          diffViewAddWidget={false}
          diffViewWrap={false}
        />
      </div>
    </div>
  );
}

interface CommitDiffPanelProps {
  commit: Commit | null;
  files: FileChange[];
  selectedFilePath: string | null;
  onSelectFile: (path: string) => void;
  diffData: DiffData | null;
  isLoading: boolean;
  onOpenInIDE?: ((path: string) => void) | undefined;
}

function CommitDiffPanel({
  commit,
  files,
  selectedFilePath,
  onSelectFile,
  diffData,
  isLoading,
  onOpenInIDE,
}: CommitDiffPanelProps) {

  if (!commit) {
    return (
      <div
        className="flex flex-col items-center justify-center h-full p-8 text-center"
        data-testid="commit-diff-empty"
      >
        <svg
          width="48"
          height="48"
          viewBox="0 0 48 48"
          fill="none"
          className="mb-4"
          style={{ color: "var(--text-muted)" }}
        >
          <circle cx="24" cy="24" r="8" stroke="currentColor" strokeWidth="2" />
          <path d="M24 8v8M24 32v8" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
        </svg>
        <p style={{ color: "var(--text-secondary)" }}>
          Select a commit to view changes
        </p>
        <p className="text-sm mt-1" style={{ color: "var(--text-muted)" }}>
          Changed files will be displayed here
        </p>
      </div>
    );
  }

  return (
    <div className="flex h-full">
      {/* Files changed in this commit */}
      <div
        className="w-64 border-r shrink-0 overflow-y-auto"
        style={{ borderColor: "var(--border-subtle)" }}
      >
        <div className="px-3 py-2 border-b" style={{ borderColor: "var(--border-subtle)" }}>
          <p className="text-xs font-medium" style={{ color: "var(--text-muted)" }}>
            {files.length} file{files.length !== 1 ? "s" : ""} changed
          </p>
        </div>
        <FileTree
          files={files}
          selectedPath={selectedFilePath}
          onSelect={onSelectFile}
        />
      </div>

      {/* Diff */}
      <div className="flex-1 min-w-0">
        <DiffPanel
          diffData={diffData}
          isLoading={isLoading}
          filePath={selectedFilePath}
          {...(onOpenInIDE !== undefined && { onOpenInIDE })}
        />
      </div>
    </div>
  );
}

// ============================================================================
// Main Component
// ============================================================================

export function DiffViewer({
  changes,
  commits,
  onFetchDiff,
  onOpenInIDE,
  isLoadingChanges = false,
  isLoadingHistory = false,
  defaultTab = "changes",
  onTabChange,
  onCommitSelect,
}: DiffViewerProps) {
  const [activeTab, setActiveTab] = useState<DiffViewTab>(defaultTab);
  const [selectedFilePath, setSelectedFilePath] = useState<string | null>(null);
  const [selectedCommit, setSelectedCommit] = useState<Commit | null>(null);
  const [commitFiles] = useState<FileChange[]>([]);
  const [commitSelectedFile, setCommitSelectedFile] = useState<string | null>(null);
  const [diffData, setDiffData] = useState<DiffData | null>(null);
  const [isDiffLoading, setIsDiffLoading] = useState(false);

  // Handle tab change
  const handleTabChange = useCallback((tab: DiffViewTab) => {
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
    // In a real implementation, this would fetch the files changed in the commit
    // For now, we'll set an empty list and expect the parent to update
  }, [onCommitSelect]);

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
      className="flex flex-col h-full"
      style={{ backgroundColor: "var(--bg-surface)" }}
      data-testid="diff-viewer"
    >
      {/* Tab bar */}
      <TabBar
        activeTab={activeTab}
        onTabChange={handleTabChange}
        changesCount={changes.length}
        commitsCount={commits.length}
      />

      {/* Content */}
      <div className="flex-1 flex min-h-0">
        {activeTab === "changes" ? (
          <>
            {/* File tree */}
            <div
              className="w-64 border-r shrink-0 overflow-y-auto"
              style={{ borderColor: "var(--border-subtle)" }}
            >
              {isLoadingChanges ? (
                <div className="flex items-center justify-center h-full">
                  <div
                    className="w-5 h-5 border-2 rounded-full animate-spin"
                    style={{
                      borderColor: "var(--border-subtle)",
                      borderTopColor: "var(--accent-primary)",
                    }}
                  />
                </div>
              ) : (
                <FileTree
                  files={changes}
                  selectedPath={selectedFilePath}
                  onSelect={handleFileSelect}
                />
              )}
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
          </>
        ) : (
          <>
            {/* Commit list */}
            <div
              className="w-80 border-r shrink-0 overflow-y-auto"
              style={{ borderColor: "var(--border-subtle)" }}
            >
              {isLoadingHistory ? (
                <div className="flex items-center justify-center h-full">
                  <div
                    className="w-5 h-5 border-2 rounded-full animate-spin"
                    style={{
                      borderColor: "var(--border-subtle)",
                      borderTopColor: "var(--accent-primary)",
                    }}
                  />
                </div>
              ) : (
                <CommitList
                  commits={commits}
                  selectedSha={selectedCommit?.sha ?? null}
                  onSelect={handleCommitSelect}
                />
              )}
            </div>

            {/* Commit diff panel */}
            <div className="flex-1 min-w-0">
              <CommitDiffPanel
                commit={selectedCommit}
                files={commitFiles}
                selectedFilePath={commitSelectedFile}
                onSelectFile={handleCommitFileSelect}
                diffData={diffData}
                isLoading={isDiffLoading}
                {...(onOpenInIDE !== undefined && { onOpenInIDE })}
              />
            </div>
          </>
        )}
      </div>
    </div>
  );
}
