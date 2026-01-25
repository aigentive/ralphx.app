# Diff Viewer

The Diff Viewer is a split-pane component for viewing code changes with syntax highlighting. It provides two tabs: "Changes" for uncommitted modifications and "History" for viewing past commits. The left panel shows a file tree, while the right panel displays the unified diff for the selected file.

**Design Inspiration:**
- GitHub's pull request file changes view (file tree, unified diff, syntax highlighting)
- VS Code's Source Control diff viewer (split view, line numbers, change indicators)
- Linear's code review interface (clean layout, focused diff display)
- Tower Git client (Mac-native feel, clear visual hierarchy)

**Aesthetic Direction:** Developer-focused precision with warmth. The diff viewer should feel like a professional code review tool - clean, readable, and efficient. Syntax highlighting uses a carefully crafted dark palette that reduces eye strain during long review sessions. The file tree provides quick navigation while the diff panel commands attention with clear change indicators.

---

## Layout Structure

### Split Pane Container

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  [ Changes ]  [ History ]                                                    │
├─────────────────────┬───────────────────────────────────────────────────────┤
│  File Tree          │  Diff Panel                                            │
│  ─────────────────  │  ───────────────────────────────────────────────────  │
│  📁 src/            │  src/components/Button.tsx                             │
│    📄 App.tsx       │  ──────────────────────────────────────────────────── │
│    📁 components/   │   10   import { cn } from '@/lib/utils';               │
│      📄 Button.tsx  │   11 - const Button = ({ children }) => {              │
│      📄 Card.tsx    │   11 + const Button = ({ children, variant }) => {     │
│                     │   12     return (                                       │
│                     │   13 -     <button>{children}</button>                  │
│                     │   13 +     <button className={cn(variant)}>            │
│                     │   14 +       {children}                                 │
│                     │   15 +     </button>                                    │
│                     │   16     );                                             │
│                     │   17   };                                               │
└─────────────────────┴───────────────────────────────────────────────────────┘
```

### Container Properties

| Property | Value | Notes |
|----------|-------|-------|
| Height | 100% of parent | Fills available space |
| Background | `--bg-base` | Darkest level for code viewing |
| Display | Flex column | Tabs above, split pane below |
| Border radius | `--radius-lg` (12px) | When used in modal/panel |

---

## Tab Navigation

### Tab Container

Using **shadcn Tabs** with underline indicator style.

| Property | Value |
|----------|-------|
| Container | `px-4 py-0`, height 48px |
| Background | `--bg-surface` |
| Border bottom | 1px `--border-subtle` |
| Gap between tabs | 0 (tabs touch) |

### Tab Button States

| State | Background | Text Color | Border |
|-------|------------|------------|--------|
| Default | transparent | `--text-secondary` | none |
| Hover | transparent | `--text-primary` | none |
| Active | transparent | `--text-primary` | 2px bottom `--accent-primary` |
| Focus | transparent | `--text-primary` | `--shadow-glow` |

### Tab Styling

| Property | Value |
|----------|-------|
| Font size | `text-sm` (14px) |
| Font weight | `font-medium` (500) |
| Padding | 12px 16px |
| Height | 48px (fills container) |
| Transition | 150ms ease (color, border) |

### Tab Indicator

| Property | Value |
|----------|-------|
| Position | Bottom of active tab |
| Height | 2px |
| Color | `--accent-primary` |
| Width | Tab content width |
| Animation | Slide to active tab (200ms ease-out) |

```tsx
<Tabs defaultValue="changes" className="flex flex-col h-full">
  <TabsList className="h-12 px-4 bg-[var(--bg-surface)] border-b border-[var(--border-subtle)] rounded-none justify-start">
    <TabsTrigger
      value="changes"
      className="data-[state=active]:border-b-2 data-[state=active]:border-[var(--accent-primary)] rounded-none"
    >
      <GitBranch className="w-4 h-4 mr-2" />
      Changes
    </TabsTrigger>
    <TabsTrigger
      value="history"
      className="data-[state=active]:border-b-2 data-[state=active]:border-[var(--accent-primary)] rounded-none"
    >
      <History className="w-4 h-4 mr-2" />
      History
    </TabsTrigger>
  </TabsList>
  {/* Tab content */}
</Tabs>
```

### Tab Icons

| Tab | Icon | Size |
|-----|------|------|
| Changes | Lucide `GitBranch` | 16px |
| History | Lucide `History` | 16px |

---

## Split Pane

### Resizable Split

| Property | Value |
|----------|-------|
| Default split | 25% file tree / 75% diff panel |
| Min file tree width | 200px |
| Max file tree width | 40% |
| Divider width | 4px |
| Divider color | `--border-subtle` |
| Divider hover | `--accent-primary` (cursor: col-resize) |

### Divider Styling

```css
.split-divider {
  width: 4px;
  background: var(--border-subtle);
  cursor: col-resize;
  transition: background 150ms ease;
}

.split-divider:hover,
.split-divider:active {
  background: var(--accent-primary);
}
```

---

## File Tree (Left Panel)

### Panel Container

| Property | Value |
|----------|-------|
| Background | `--bg-surface` |
| Border right | 1px `--border-subtle` |
| Padding | 8px 0 |
| Overflow | auto (vertical scroll) |
| Min width | 200px |

### Scrollbar Styling

| Property | Value |
|----------|-------|
| Width | 6px |
| Track color | transparent |
| Thumb color | `--bg-hover` |
| Thumb hover | `--text-muted` |
| Border radius | `--radius-full` |

### File Tree Structure

```
📁 src/
  📄 App.tsx         M
  📁 components/
    📄 Button.tsx    M
    📄 Card.tsx      A
    📄 Modal.tsx     D
  📁 utils/
    📄 helpers.ts    R
```

### Tree Item States

| State | Background | Text Color | Icon Color |
|-------|------------|------------|------------|
| Default | transparent | `--text-secondary` | `--text-muted` |
| Hover | `--bg-hover` | `--text-primary` | `--text-secondary` |
| Selected | `--bg-elevated` | `--text-primary` | `--accent-primary` |
| Focus | `--bg-hover` | `--text-primary` | `--shadow-glow` |

### Tree Item Styling

| Property | Value |
|----------|-------|
| Height | 28px |
| Padding | 4px 8px 4px (12px + indent) |
| Font | `text-sm` (14px) |
| Indent | 16px per level |
| Gap | 4px between icon and name |

### File Status Icons (Lucide)

| Status | Icon | Color | Meaning |
|--------|------|-------|---------|
| Modified | `Edit` (or `Pencil`) | `--status-warning` (#f59e0b) | File changed |
| Added | `Plus` | `--status-success` (#10b981) | New file |
| Deleted | `Minus` | `--status-error` (#ef4444) | File removed |
| Renamed | `ArrowRight` | `--status-info` (#3b82f6) | File renamed/moved |

### File/Folder Icons

| Type | Icon | Color |
|------|------|-------|
| Folder (collapsed) | Lucide `ChevronRight` + `Folder` | `--text-muted` |
| Folder (expanded) | Lucide `ChevronDown` + `FolderOpen` | `--text-muted` |
| File (generic) | Lucide `File` | `--text-muted` |
| File (TypeScript) | Lucide `FileCode` | `--status-info` (subtle) |
| File (JSON/Config) | Lucide `FileJson` | `--text-muted` |

### Status Badge (Right Side)

| Property | Value |
|----------|-------|
| Position | Right-aligned, vertically centered |
| Font | `text-xs`, `font-mono` |
| Content | Single letter (M, A, D, R) |
| Width | 16px |
| Text align | Center |
| Color | Matches status color |

```tsx
<div className="flex items-center justify-between px-2 py-1 hover:bg-[var(--bg-hover)] cursor-pointer">
  <div className="flex items-center gap-1">
    <ChevronRight className="w-4 h-4 text-[var(--text-muted)]" />
    <FileCode className="w-4 h-4 text-[var(--text-muted)]" />
    <span className="text-sm text-[var(--text-secondary)]">Button.tsx</span>
  </div>
  <span className="text-xs font-mono text-[var(--status-warning)]">M</span>
</div>
```

### Collapsible Directories

| Property | Value |
|----------|-------|
| Chevron icon | 16px, transitions 150ms |
| Expanded | `rotate-90deg` |
| Collapsed | `rotate-0deg` |
| Animation | 150ms ease-out |
| Children | Indented 16px |

```css
.tree-chevron {
  transition: transform 150ms ease-out;
}

.tree-chevron.expanded {
  transform: rotate(90deg);
}
```

---

## Diff Panel (Right Panel)

### Panel Container

| Property | Value |
|----------|-------|
| Background | `--bg-base` |
| Flex | 1 (fills remaining space) |
| Display | Flex column |
| Overflow | hidden (internal scroll) |

### File Header

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  src/components/Button.tsx                              [Open in IDE] ⎘    │
└─────────────────────────────────────────────────────────────────────────────┘
```

| Property | Value |
|----------|-------|
| Height | 40px |
| Background | `--bg-surface` |
| Border bottom | 1px `--border-subtle` |
| Padding | 0 16px |
| Display | Flex, align-items center, justify-between |

### File Path

| Property | Value |
|----------|-------|
| Font | `text-sm`, `font-mono` |
| Color | `--text-primary` |
| Truncation | Start (show filename, truncate path) |

### Open in IDE Button

| Property | Value |
|----------|-------|
| Icon | Lucide `ExternalLink` (16px) |
| Variant | Ghost |
| Size | 28px × 28px |
| Color | `--text-muted` |
| Hover | `--text-primary`, `--bg-hover` |
| Tooltip | "Open in IDE" |

```tsx
<Button
  variant="ghost"
  size="icon"
  className="h-7 w-7 text-[var(--text-muted)] hover:text-[var(--text-primary)]"
  onClick={openInIDE}
>
  <ExternalLink className="w-4 h-4" />
</Button>
```

### Diff Content Area

| Property | Value |
|----------|-------|
| Background | `--bg-base` |
| Overflow | auto (both axes) |
| Font family | `--font-mono` (JetBrains Mono) |
| Font size | 13px |
| Line height | 20px (1.54) |
| Tab size | 2 spaces |

### Scrollbar Styling

| Property | Value |
|----------|-------|
| Width | 8px (wider for code viewing) |
| Track color | `--bg-surface` |
| Thumb color | `--bg-hover` |
| Thumb hover | `--text-muted` |
| Border radius | `--radius-sm` |

---

## Unified Diff Format

### Line Structure

```
┌────────┬────────┬─────────────────────────────────────────────────────────┐
│ Old #  │ New #  │ Code Content                                             │
├────────┼────────┼─────────────────────────────────────────────────────────┤
│   10   │   10   │ import { cn } from '@/lib/utils';                        │
│   11   │        │-const Button = ({ children }) => {                       │
│        │   11   │+const Button = ({ children, variant }) => {              │
│   12   │   12   │   return (                                               │
│   13   │        │-    <button>{children}</button>                          │
│        │   13   │+    <button className={cn(variant)}>                     │
│        │   14   │+      {children}                                         │
│        │   15   │+    </button>                                            │
│   14   │   16   │   );                                                     │
└────────┴────────┴─────────────────────────────────────────────────────────┘
```

### Line Number Columns

| Property | Value |
|----------|-------|
| Width | 48px each (old + new) |
| Background | `--bg-surface` |
| Border right | 1px `--border-subtle` |
| Text align | Right |
| Padding | 0 8px |
| Font | `text-xs`, `font-mono` |
| Color | `--text-muted` |

### Line Backgrounds

| Type | Background | Opacity |
|------|------------|---------|
| Context (unchanged) | `--bg-base` | 100% |
| Added | `rgba(16, 185, 129, 0.15)` | 15% of `--status-success` |
| Removed | `rgba(239, 68, 68, 0.15)` | 15% of `--status-error` |
| Changed (word-level) | `rgba(245, 158, 11, 0.15)` | 15% of `--status-warning` |

### Line Number Colors by Type

| Type | Color |
|------|-------|
| Context | `--text-muted` |
| Added | `--status-success` |
| Removed | `--status-error` |

### Change Indicator Gutter

| Type | Symbol | Color | Width |
|------|--------|-------|-------|
| Context | (none) | - | 16px |
| Added | `+` | `--status-success` | 16px |
| Removed | `-` | `--status-error` | 16px |

```tsx
<div className="flex">
  {/* Old line number */}
  <div className="w-12 px-2 text-right text-xs font-mono text-[var(--text-muted)] bg-[var(--bg-surface)] border-r border-[var(--border-subtle)]">
    {oldLineNumber}
  </div>
  {/* New line number */}
  <div className="w-12 px-2 text-right text-xs font-mono text-[var(--text-muted)] bg-[var(--bg-surface)] border-r border-[var(--border-subtle)]">
    {newLineNumber}
  </div>
  {/* Change indicator */}
  <div className="w-4 text-center font-mono text-sm" style={{ color: indicatorColor }}>
    {indicator}
  </div>
  {/* Code content */}
  <div className="flex-1 px-4 font-mono text-sm" style={{ background: lineBackground }}>
    <SyntaxHighlightedCode code={code} />
  </div>
</div>
```

---

## Syntax Highlighting (Dracula-Inspired)

### Color Palette

Dark theme optimized for code readability and reduced eye strain.

| Token Type | Color | Example |
|------------|-------|---------|
| Background | `--bg-base` (#0f0f0f) | Code background |
| Foreground | #f8f8f2 | Default text |
| Comment | #6272a4 | `// comment` |
| Keyword | #ff79c6 | `const`, `return`, `import` |
| String | #f1fa8c | `"string value"` |
| Number | #bd93f9 | `42`, `3.14` |
| Function | #50fa7b | `functionName()` |
| Variable | #f8f8f2 | variable names |
| Operator | #ff79c6 | `=`, `+`, `=>` |
| Type | #8be9fd | Type annotations |
| Property | #66d9ef | Object properties |
| Tag (JSX) | #ff79c6 | `<Component>` |
| Attribute | #50fa7b | `className=` |
| Punctuation | #f8f8f2 | `{}`, `[]`, `()` |

### Word-Level Diff Highlighting

When showing inline changes within a line:

| Change Type | Background |
|-------------|------------|
| Added word | `rgba(16, 185, 129, 0.3)` |
| Removed word | `rgba(239, 68, 68, 0.3)` |

```tsx
// Example: highlighting "variant" as added
<span className="bg-[rgba(16,185,129,0.3)] rounded-sm px-0.5">
  variant
</span>
```

---

## History Tab

### Commit List (Left Side of Split)

When History tab is active, the left panel shows commits instead of files.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  a1b2c3d  feat: add variant prop to Button    •  John D  •  2 hours ago    │
│  e4f5g6h  fix: correct padding values         •  Jane S  •  5 hours ago    │
│  i7j8k9l  refactor: extract utility functions •  John D  •  1 day ago      │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Commit Item

| Property | Value |
|----------|-------|
| Height | 48px |
| Padding | 8px 12px |
| Background | transparent |
| Hover | `--bg-hover` |
| Selected | `--bg-elevated`, 2px left border `--accent-primary` |
| Border bottom | 1px `--border-subtle` |

### Commit SHA

| Property | Value |
|----------|-------|
| Font | `text-xs`, `font-mono` |
| Color | `--accent-primary` |
| Length | 7 characters (short SHA) |
| Margin right | 8px |

### Commit Message

| Property | Value |
|----------|-------|
| Font | `text-sm` |
| Color | `--text-primary` |
| Truncation | Single line, ellipsis |
| Max width | ~60% of item width |

### Commit Metadata

| Property | Value |
|----------|-------|
| Font | `text-xs` |
| Color | `--text-muted` |
| Content | Author name + relative date |
| Separator | " • " between items |
| Position | Right side or below message |

```tsx
<div
  className={cn(
    "flex items-center px-3 py-2 cursor-pointer border-b border-[var(--border-subtle)]",
    "hover:bg-[var(--bg-hover)]",
    selected && "bg-[var(--bg-elevated)] border-l-2 border-l-[var(--accent-primary)]"
  )}
  onClick={() => selectCommit(commit.sha)}
>
  <span className="text-xs font-mono text-[var(--accent-primary)] mr-2">
    {commit.shortSha}
  </span>
  <span className="text-sm text-[var(--text-primary)] truncate flex-1">
    {commit.message}
  </span>
  <span className="text-xs text-[var(--text-muted)] ml-2 whitespace-nowrap">
    {commit.author} • {commit.relativeTime}
  </span>
</div>
```

### Commit Selection

| Property | Value |
|----------|-------|
| Click action | Shows commit diff in right panel |
| Selected state | Background + left accent border |
| Keyboard | Arrow keys to navigate, Enter to select |

---

## Empty States

### No Changes (Changes Tab)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                                                                             │
│                        ┌───────────────┐                                    │
│                        │    (icon)     │                                    │
│                        └───────────────┘                                    │
│                      No uncommitted changes                                 │
│                   Your working directory is clean                          │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

| Element | Styling |
|---------|---------|
| Container | Centered, padding 64px |
| Icon | Lucide `CheckCircle2` (48px), `--text-muted`, 50% opacity |
| Title | "No uncommitted changes", `text-sm`, `font-medium`, `--text-secondary` |
| Subtitle | "Your working directory is clean", `text-xs`, `--text-muted` |

### No History (History Tab)

| Element | Styling |
|---------|---------|
| Icon | Lucide `GitCommit` (48px), dashed |
| Title | "No commit history" |
| Subtitle | "Make your first commit to see history here" |

### No File Selected

When file tree exists but no file is selected.

| Element | Styling |
|---------|---------|
| Icon | Lucide `FileSearch` (48px) |
| Title | "Select a file to view changes" |
| Subtitle | "Click on a file in the tree to see its diff" |

---

## Loading States

### Initial Load

```tsx
<div className="flex items-center justify-center h-full">
  <Loader2 className="w-6 h-6 animate-spin text-[var(--accent-primary)]" />
</div>
```

### File Tree Loading

| Element | Styling |
|---------|---------|
| Skeleton items | 5-7 skeleton lines at varying widths |
| Animation | Pulse/shimmer |
| Height | 28px per item |

### Diff Loading

| Element | Styling |
|---------|---------|
| Skeleton lines | 10-15 lines with line number placeholders |
| Animation | Pulse/shimmer |
| Code area | Wider skeletons, varying widths |

---

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `Tab` | Switch between file tree and diff panel |
| `↑` / `↓` | Navigate files or commits |
| `Enter` | Select highlighted item |
| `←` | Collapse folder / go to parent |
| `→` | Expand folder / enter directory |
| `Cmd + 1` | Switch to Changes tab |
| `Cmd + 2` | Switch to History tab |
| `Cmd + O` | Open selected file in IDE |
| `Escape` | Clear selection |

---

## Lucide Icons Used

| Icon | Usage | Size |
|------|-------|------|
| `GitBranch` | Changes tab icon | 16px |
| `History` | History tab icon | 16px |
| `Folder` | Collapsed folder | 16px |
| `FolderOpen` | Expanded folder | 16px |
| `File` | Generic file | 16px |
| `FileCode` | Code files (.ts, .tsx, .js) | 16px |
| `FileJson` | JSON/config files | 16px |
| `ChevronRight` | Collapsed indicator | 16px |
| `ChevronDown` | Expanded indicator | 16px |
| `Plus` | Added file status | 12px |
| `Minus` | Deleted file status | 12px |
| `Edit` / `Pencil` | Modified file status | 12px |
| `ArrowRight` | Renamed file status | 12px |
| `ExternalLink` | Open in IDE button | 16px |
| `GitCommit` | Empty history state | 48px |
| `CheckCircle2` | No changes state | 48px |
| `FileSearch` | No file selected state | 48px |
| `Loader2` | Loading spinner | 24px |

---

## Component Hierarchy

```
DiffViewer
├── DiffViewerContainer
│   │
│   ├── Tabs (shadcn)
│   │   ├── TabsList
│   │   │   ├── TabsTrigger "Changes" + GitBranch icon
│   │   │   └── TabsTrigger "History" + History icon
│   │   │
│   │   ├── TabsContent "changes"
│   │   │   └── SplitPane
│   │   │       ├── FileTree (left panel)
│   │   │       │   ├── ScrollArea (shadcn)
│   │   │       │   │   ├── TreeNode[] (recursive)
│   │   │       │   │   │   ├── FolderItem (collapsible)
│   │   │       │   │   │   │   ├── ChevronIcon (expand/collapse)
│   │   │       │   │   │   │   ├── FolderIcon
│   │   │       │   │   │   │   └── FolderName
│   │   │       │   │   │   └── FileItem
│   │   │       │   │   │       ├── FileTypeIcon
│   │   │       │   │   │       ├── FileName
│   │   │       │   │   │       └── StatusBadge (M/A/D/R)
│   │   │       │   │   └── EmptyState (conditional)
│   │   │       │   └── ResizeDivider
│   │   │       │
│   │   │       └── DiffPanel (right panel)
│   │   │           ├── FileHeader
│   │   │           │   ├── FilePath (monospace)
│   │   │           │   └── OpenInIDEButton
│   │   │           ├── DiffContent
│   │   │           │   ├── ScrollArea (shadcn)
│   │   │           │   │   └── DiffLine[] (mapped)
│   │   │           │   │       ├── OldLineNumber
│   │   │           │   │       ├── NewLineNumber
│   │   │           │   │       ├── ChangeIndicator (+/-)
│   │   │           │   │       └── CodeContent (syntax highlighted)
│   │   │           │   └── EmptyState (no file selected)
│   │   │           └── LoadingState (conditional)
│   │   │
│   │   └── TabsContent "history"
│   │       └── SplitPane
│   │           ├── CommitList (left panel)
│   │           │   ├── ScrollArea (shadcn)
│   │           │   │   └── CommitItem[] (mapped)
│   │           │   │       ├── ShortSHA (accent monospace)
│   │           │   │       ├── CommitMessage (truncated)
│   │           │   │       └── Metadata (author, time)
│   │           │   └── EmptyState (conditional)
│   │           │
│   │           └── DiffPanel (right panel)
│   │               └── (same structure as Changes tab)
│   │
│   └── LoadingOverlay (conditional)
│       └── Loader2 (spinning)
```

---

## Acceptance Criteria

### Functional Requirements

1. [ ] Tabs switch between Changes and History views
2. [ ] Active tab has underline indicator with accent color
3. [ ] File tree displays uncommitted changes in Changes tab
4. [ ] File tree shows correct file status icons (M/A/D/R)
5. [ ] File status badges use semantic colors (green/amber/red/blue)
6. [ ] Folders are collapsible with chevron animation
7. [ ] Clicking a file selects it and shows diff in right panel
8. [ ] Selected file has highlighted background
9. [ ] Split pane is resizable via drag handle
10. [ ] Divider highlights on hover
11. [ ] Diff panel shows unified diff format
12. [ ] Line numbers display for both old and new versions
13. [ ] Added lines have green background tint
14. [ ] Removed lines have red background tint
15. [ ] Change indicators (+/-) display in gutter
16. [ ] Syntax highlighting applied to code content
17. [ ] File header shows current file path
18. [ ] Open in IDE button triggers external editor
19. [ ] History tab shows commit list
20. [ ] Commit items show SHA, message, author, and time
21. [ ] Clicking a commit shows its diff
22. [ ] Selected commit has highlighted background with accent border
23. [ ] Empty states display when no changes/history/selection
24. [ ] Loading states display while fetching data
25. [ ] Keyboard navigation works for file tree and commit list
26. [ ] Keyboard shortcuts work for tab switching

---

## Design Quality Checklist

### Colors & Theming

1. [ ] NO purple or blue gradients anywhere
2. [ ] Background uses `--bg-base` for code area, `--bg-surface` for panels
3. [ ] Warm orange accent (`#ff6b35`) used for tab indicator and commit SHA
4. [ ] Status colors used correctly: green (added), red (removed), amber (modified), blue (renamed)
5. [ ] Line backgrounds use 15% opacity of status colors
6. [ ] Syntax highlighting uses Dracula-inspired palette, NOT standard theme

### Typography

7. [ ] All code uses JetBrains Mono (`--font-mono`)
8. [ ] Code font size: 13px
9. [ ] Line height for code: 20px (1.54)
10. [ ] Tab labels use SF Pro with `text-sm`, `font-medium`
11. [ ] File names use `text-sm`
12. [ ] Line numbers use `text-xs`, `font-mono`
13. [ ] Commit SHA uses `font-mono`

### Spacing & Layout

14. [ ] File tree width: 25% default, 200px min
15. [ ] Tab bar height: 48px
16. [ ] File header height: 40px
17. [ ] Tree item height: 28px
18. [ ] Commit item height: 48px
19. [ ] 16px indent per folder level
20. [ ] 8pt grid alignment maintained throughout
21. [ ] Line number columns: 48px each

### Shadows & Depth

22. [ ] No shadows on main panels (border-only separation)
23. [ ] Focus states use `--shadow-glow`
24. [ ] Subtle depth through background color hierarchy

### Borders & Radius

25. [ ] Container border radius: `--radius-lg` (12px) when in modal
26. [ ] Panel separator: 1px `--border-subtle`
27. [ ] Tab underline: 2px `--accent-primary`
28. [ ] Tree item selected: subtle left border or background only

### Motion & Interactions

29. [ ] Tab indicator slides to active tab (200ms ease-out)
30. [ ] Folder chevron rotates on expand (150ms ease-out)
31. [ ] Divider color transition on hover (150ms)
32. [ ] Tree item hover transition (150ms)
33. [ ] All transitions use appropriate easing

### Icons

34. [ ] All icons from Lucide library
35. [ ] Tab icons: 16px
36. [ ] Tree icons: 16px
37. [ ] Status icons: 12px (smaller, in badge position)
38. [ ] Empty state icons: 48px
39. [ ] Loading spinner: 24px
40. [ ] Icons inherit appropriate colors

### Accessibility

41. [ ] Color contrast meets WCAG AA (4.5:1)
42. [ ] Focus states visible on all interactive elements
43. [ ] Tree items have role="treeitem"
44. [ ] Tabs have proper ARIA attributes
45. [ ] Keyboard navigation fully functional
46. [ ] Screen reader announces selected file/commit

---

## Implementation Notes

### shadcn Components to Use

- `Tabs`, `TabsList`, `TabsTrigger`, `TabsContent` (tab navigation)
- `ScrollArea` (scrollable panels)
- `Button` (Open in IDE button)
- `Tooltip` (button tooltips)
- `Skeleton` (loading states)

### CSS Custom Properties

```css
/* Diff Viewer specific */
--diff-line-height: 20px;
--diff-font-size: 13px;
--diff-line-number-width: 48px;
--diff-gutter-width: 16px;

/* Diff backgrounds */
--diff-added-bg: rgba(16, 185, 129, 0.15);
--diff-removed-bg: rgba(239, 68, 68, 0.15);
--diff-changed-bg: rgba(245, 158, 11, 0.15);

/* Syntax highlighting (Dracula-inspired) */
--syntax-comment: #6272a4;
--syntax-keyword: #ff79c6;
--syntax-string: #f1fa8c;
--syntax-number: #bd93f9;
--syntax-function: #50fa7b;
--syntax-variable: #f8f8f2;
--syntax-type: #8be9fd;
--syntax-property: #66d9ef;

/* From DESIGN.md */
--bg-base: #0f0f0f;
--bg-surface: #1a1a1a;
--bg-elevated: #242424;
--bg-hover: #2d2d2d;
--text-primary: #f0f0f0;
--text-secondary: #a0a0a0;
--text-muted: #666666;
--accent-primary: #ff6b35;
--border-subtle: rgba(255, 255, 255, 0.06);
--status-success: #10b981;
--status-warning: #f59e0b;
--status-error: #ef4444;
--status-info: #3b82f6;
--radius-lg: 12px;
--radius-sm: 4px;
--radius-full: 9999px;
--shadow-glow: 0 0 0 2px var(--bg-base), 0 0 0 4px var(--accent-primary);
```

### Syntax Highlighting Library

Consider using:
- **Prism.js** with custom Dracula-inspired theme
- **highlight.js** with custom theme
- **Shiki** for high-quality static highlighting

Or implement minimal custom highlighting for common patterns (keywords, strings, comments).

---

## References

- [DESIGN.md](../DESIGN.md) - Master design system
- [specs/DESIGN_OVERHAUL_PLAN.md](../../DESIGN_OVERHAUL_PLAN.md) - Design overhaul strategy
- [reviews-panel.md](./reviews-panel.md) - Reviews Panel with DiffViewer integration
- GitHub Pull Request diff view - File tree and diff display reference
- VS Code Source Control - Unified diff styling reference
- Tower Git client - Mac-native diff viewer reference
- Dracula theme - Syntax highlighting palette reference
- shadcn/ui Tabs - https://ui.shadcn.com/docs/components/tabs
- Lucide icons - https://lucide.dev/icons/
