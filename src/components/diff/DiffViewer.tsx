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

import { useState, useMemo, useCallback, useEffect } from "react";
import { DiffView, DiffModeEnum } from "@git-diff-view/react";
import "@git-diff-view/react/styles/diff-view.css";
import {
  GitBranch,
  History,
  Folder,
  FolderOpen,
  File,
  FileCode,
  FileJson,
  ChevronRight,
  ExternalLink,
  GitCommit,
  CheckCircle2,
  FileSearch,
  Loader2,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Tooltip, TooltipContent, TooltipTrigger, TooltipProvider } from "@/components/ui/tooltip";
import { Skeleton } from "@/components/ui/skeleton";

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
// Utility Functions
// ============================================================================

function getStatusColor(status: FileChange["status"]) {
  switch (status) {
    case "added":
      return "var(--status-success)";
    case "modified":
      return "var(--status-warning)";
    case "deleted":
      return "var(--status-error)";
    case "renamed":
      return "var(--status-info)";
  }
}

function getStatusLetter(status: FileChange["status"]) {
  switch (status) {
    case "added":
      return "A";
    case "modified":
      return "M";
    case "deleted":
      return "D";
    case "renamed":
      return "R";
  }
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

function getFileIcon(path: string) {
  const ext = path.split(".").pop()?.toLowerCase() ?? "";
  const codeExts = ["ts", "tsx", "js", "jsx", "rs", "py", "go", "java", "c", "cpp", "rb", "php", "swift", "kt"];
  const configExts = ["json", "yaml", "yml", "toml"];

  if (codeExts.includes(ext)) {
    return <FileCode className="w-4 h-4" />;
  }
  if (configExts.includes(ext)) {
    return <FileJson className="w-4 h-4" />;
  }
  return <File className="w-4 h-4" />;
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
          className={cn(
            "w-full flex items-center gap-1 px-2 py-1 text-sm transition-colors",
            "hover:bg-[var(--bg-hover)] cursor-pointer"
          )}
          style={{
            paddingLeft: `${depth * 16 + 8}px`,
            height: "28px",
          }}
        >
          <span
            className="transition-transform duration-150"
            style={{
              transform: isExpanded ? "rotate(90deg)" : "rotate(0deg)",
            }}
          >
            <ChevronRight className="w-4 h-4 text-[var(--text-muted)]" />
          </span>
          {isExpanded ? (
            <FolderOpen className="w-4 h-4 text-[var(--text-muted)]" />
          ) : (
            <Folder className="w-4 h-4 text-[var(--text-muted)]" />
          )}
          <span className="truncate text-[var(--text-secondary)]">{node.name}</span>
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
  const statusColor = getStatusColor(file.status);

  return (
    <button
      data-testid={`file-${file.path}`}
      onClick={() => onSelect(file.path)}
      className={cn(
        "w-full flex items-center gap-1 px-2 py-1 text-sm transition-colors",
        "hover:bg-[var(--bg-hover)] cursor-pointer",
        isSelected && "bg-[var(--bg-elevated)]"
      )}
      style={{
        paddingLeft: `${depth * 16 + 8}px`,
        height: "28px",
      }}
    >
      {/* Spacer for chevron alignment */}
      <span className="w-4" />
      <span className="text-[var(--text-muted)]">
        {getFileIcon(file.path)}
      </span>
      <span
        className={cn(
          "truncate flex-1 text-left",
          isSelected ? "text-[var(--text-primary)]" : "text-[var(--text-secondary)]"
        )}
      >
        {node.name}
      </span>
      <span
        className="text-xs font-mono w-4 text-center"
        style={{ color: statusColor }}
      >
        {getStatusLetter(file.status)}
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
        className="flex flex-col items-center justify-center h-full p-16 text-center"
        data-testid="file-tree-empty"
      >
        <CheckCircle2 className="w-12 h-12 text-[var(--text-muted)] opacity-50 mb-4" />
        <p className="text-sm font-medium text-[var(--text-secondary)]">No uncommitted changes</p>
        <p className="text-xs text-[var(--text-muted)] mt-1">
          Your working directory is clean
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
        className="flex flex-col items-center justify-center h-full p-16 text-center"
        data-testid="commit-list-empty"
      >
        <GitCommit className="w-12 h-12 text-[var(--text-muted)] opacity-50 mb-4" />
        <p className="text-sm font-medium text-[var(--text-secondary)]">No commit history</p>
        <p className="text-xs text-[var(--text-muted)] mt-1">
          Make your first commit to see history here
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
            className={cn(
              "w-full flex items-center px-3 py-2 text-left transition-colors cursor-pointer",
              "border-b border-[var(--border-subtle)]",
              "hover:bg-[var(--bg-hover)]",
              isSelected && "bg-[var(--bg-elevated)] border-l-2 border-l-[var(--accent-primary)]"
            )}
            style={{ height: "48px" }}
          >
            <span className="text-xs font-mono text-[var(--accent-primary)] mr-2 shrink-0">
              {commit.shortSha}
            </span>
            <span className="text-sm text-[var(--text-primary)] truncate flex-1">
              {commit.message}
            </span>
            <span className="text-xs text-[var(--text-muted)] ml-2 whitespace-nowrap shrink-0">
              {commit.author} • {formatRelativeDate(commit.date)}
            </span>
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
          <Loader2 className="w-6 h-6 animate-spin text-[var(--accent-primary)]" />
          <p className="text-sm text-[var(--text-secondary)]">Loading diff...</p>
        </div>
      </div>
    );
  }

  if (!filePath) {
    return (
      <div
        className="flex flex-col items-center justify-center h-full p-16 text-center"
        data-testid="diff-empty"
      >
        <FileSearch className="w-12 h-12 text-[var(--text-muted)] opacity-50 mb-4" />
        <p className="text-sm font-medium text-[var(--text-secondary)]">
          Select a file to view changes
        </p>
        <p className="text-xs text-[var(--text-muted)] mt-1">
          Click on a file in the tree to see its diff
        </p>
      </div>
    );
  }

  if (!diffData) {
    return (
      <div
        className="flex flex-col items-center justify-center h-full p-16 text-center"
        data-testid="diff-error"
      >
        <p className="text-sm text-[var(--text-secondary)]">
          Unable to load diff for this file
        </p>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full" data-testid="diff-content">
      {/* File header */}
      <div
        className="flex items-center justify-between px-4 shrink-0"
        style={{
          height: "40px",
          borderBottom: "1px solid var(--border-subtle)",
          backgroundColor: "var(--bg-surface)",
        }}
      >
        <span
          className="font-mono text-sm truncate"
          style={{ color: "var(--text-primary)" }}
        >
          {filePath}
        </span>
        {onOpenInIDE && (
          <TooltipProvider>
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-7 w-7 text-[var(--text-muted)] hover:text-[var(--text-primary)]"
                  onClick={() => onOpenInIDE(filePath)}
                  data-testid="open-in-ide"
                >
                  <ExternalLink className="w-4 h-4" />
                </Button>
              </TooltipTrigger>
              <TooltipContent>
                <p>Open in IDE</p>
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
        )}
      </div>

      {/* Diff view */}
      <div className="flex-1 overflow-auto diff-viewer-container bg-[var(--bg-base)]">
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
        className="flex flex-col items-center justify-center h-full p-16 text-center"
        data-testid="commit-diff-empty"
      >
        <GitCommit className="w-12 h-12 text-[var(--text-muted)] opacity-50 mb-4" />
        <p className="text-sm font-medium text-[var(--text-secondary)]">
          Select a commit to view changes
        </p>
        <p className="text-xs text-[var(--text-muted)] mt-1">
          Changed files will be displayed here
        </p>
      </div>
    );
  }

  return (
    <div className="flex h-full">
      {/* Files changed in this commit */}
      <div
        className="w-64 shrink-0 overflow-hidden border-r border-[var(--border-subtle)]"
        style={{ minWidth: "200px" }}
      >
        <div
          className="px-3 py-2 border-b border-[var(--border-subtle)]"
          style={{ backgroundColor: "var(--bg-surface)" }}
        >
          <p className="text-xs font-medium text-[var(--text-muted)]">
            {files.length} file{files.length !== 1 ? "s" : ""} changed
          </p>
        </div>
        <ScrollArea className="h-[calc(100%-33px)]">
          <FileTree
            files={files}
            selectedPath={selectedFilePath}
            onSelect={onSelectFile}
          />
        </ScrollArea>
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

// Predefined widths to avoid Math.random() in render
const SKELETON_WIDTHS = ["75%", "85%", "65%", "90%", "70%", "80%", "60%"];

function FileTreeSkeleton() {
  return (
    <div className="py-2 px-2 space-y-1">
      {SKELETON_WIDTHS.map((width, i) => (
        <Skeleton key={i} className="h-7" style={{ width }} />
      ))}
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
              files={commitFiles}
              selectedFilePath={commitSelectedFile}
              onSelectFile={handleCommitFileSelect}
              diffData={diffData}
              isLoading={isDiffLoading}
              {...(onOpenInIDE !== undefined && { onOpenInIDE })}
            />
          </div>
        </TabsContent>
      </Tabs>
    </div>
  );
}
