# Activity Stream View

The Activity Stream provides real-time monitoring of agent execution, tool calls, and system events. It should feel like a sophisticated terminal/console experience with the warmth of RalphX's design language - a live feed where you watch AI agents think and act.

**Reference Inspiration**: Linear's activity feed (clean temporal flow), Vercel logs (collapsible detail sections), VS Code terminal (syntax-aware formatting), Arc's command bar (subtle categorization)

## Overall Layout

**Container:**
- Viewport-filling height (`calc(100vh - header)`)
- Background: `--bg-surface` with subtle radial gradient
- Gradient: `radial-gradient(ellipse at bottom left, rgba(255,107,53,0.015) 0%, var(--bg-surface) 50%)`
- No horizontal scroll; vertical scroll only

**Header:**
- Glass effect: `rgba(26,26,26,0.85)` + `backdrop-filter: blur(12px)`
- Height: 56px with vertical centering
- Title: "Activity" with Lucide `Activity` icon (20px)
- Title styling: `text-lg`, `font-semibold`, `--tracking-tight`
- Alert badge (when alerts exist): pill badge with count, `--status-error` background
- Clear button: ghost variant, right-aligned, `--text-muted`
- Border bottom: `1px solid var(--border-subtle)`

```tsx
<div className="flex items-center justify-between px-4 py-3 backdrop-blur-md bg-[rgba(26,26,26,0.85)] border-b border-[var(--border-subtle)]">
  <div className="flex items-center gap-3">
    <div className="p-1.5 rounded-lg bg-[var(--accent-muted)]">
      <Activity className="w-5 h-5 text-[var(--accent-primary)]" />
    </div>
    <h2 className="text-lg font-semibold tracking-tight text-[var(--text-primary)]">Activity</h2>
    {alertCount > 0 && (
      <span className="px-2 py-0.5 text-xs font-medium rounded-full bg-[var(--status-error)] text-white">
        {alertCount} alert{alertCount > 1 ? 's' : ''}
      </span>
    )}
  </div>
  <Button variant="ghost" size="sm" onClick={clearMessages} disabled={isEmpty}>
    <Trash2 className="w-4 h-4 mr-1.5" />
    Clear
  </Button>
</div>
```

## Search & Filter Bar

**Container:**
- Padding: 16px horizontal, 12px vertical
- Border bottom: `1px solid var(--border-subtle)`
- Background: `--bg-surface` (matches main background)
- Flex column, gap: 12px

**Search Input:**
- Using shadcn Input component
- Full width
- Height: 36px
- Placeholder: "Search activities..."
- Left icon: Lucide `Search` (16px, `--text-muted`)
- Clear button: Lucide `X` when has value, appears on right
- Background: `--bg-elevated`
- Border: `1px solid var(--border-default)`
- Focus: `--accent-primary` border + `--shadow-glow`

```tsx
<div className="relative">
  <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-[var(--text-muted)]" />
  <Input
    value={searchQuery}
    onChange={(e) => setSearchQuery(e.target.value)}
    placeholder="Search activities..."
    className="pl-10 pr-8 bg-[var(--bg-elevated)]"
  />
  {searchQuery && (
    <button
      onClick={() => setSearchQuery('')}
      className="absolute right-2 top-1/2 -translate-y-1/2 p-1 rounded hover:bg-white/5 text-[var(--text-muted)]"
    >
      <X className="w-4 h-4" />
    </button>
  )}
</div>
```

**Filter Tabs:**
- Using custom tab component (not shadcn Tabs - simpler inline pills)
- Horizontal scroll if overflow
- Background container: `--bg-base` with rounded-lg
- Tabs: All, Thinking, Tool Calls, Results, Text, Errors
- Tab styling:
  - Inactive: transparent bg, `--text-secondary`
  - Active: `--bg-elevated` bg, `--text-primary`, `1px solid var(--border-subtle)`
- Padding: 6px 12px per tab
- Text: `text-xs`, `font-medium`
- Transition: 150ms colors

```tsx
<div className="flex gap-1 p-1 rounded-lg bg-[var(--bg-base)] overflow-x-auto">
  {MESSAGE_TYPES.map(({ key, label }) => (
    <button
      key={key}
      onClick={() => setTypeFilter(key)}
      className={cn(
        "px-3 py-1.5 text-xs font-medium rounded-md transition-colors whitespace-nowrap",
        typeFilter === key
          ? "bg-[var(--bg-elevated)] text-[var(--text-primary)] border border-[var(--border-subtle)]"
          : "text-[var(--text-secondary)] hover:text-[var(--text-primary)]"
      )}
    >
      {label}
    </button>
  ))}
</div>
```

## Activity Entries

**Entry Structure:**
Each entry is a message from the agent execution stream. Different types have distinct visual treatments.

**Entry Container:**
- Border-radius: 8px (`--radius-md`)
- Margin bottom: 8px
- Left border: 3px solid (color varies by type)
- Background: varies by type (subtle tint)
- Transition: all 150ms ease

**Type-Specific Styling:**

| Type | Icon | Left Border | Background | Description |
|------|------|-------------|------------|-------------|
| Thinking | `Brain` (animated pulse) | `--text-muted` | `rgba(128,128,128,0.08)` | Agent reasoning |
| Tool Call | `Terminal` | `--accent-primary` | `rgba(255,107,53,0.08)` | Tool invocation |
| Tool Result | `CheckCircle` | `--status-success` | `rgba(34,197,94,0.08)` | Tool output |
| Text | `MessageSquare` | `--text-secondary` | `rgba(128,128,128,0.04)` | Plain text output |
| Error | `AlertCircle` | `--status-error` | `rgba(239,68,68,0.1)` | Errors/failures |

**Entry Header Row:**
- Flex row with gap: 12px
- Padding: 10px 12px
- Cursor: pointer (for expandable entries)
- Hover: slightly lighter background

**Expand/Collapse Indicator:**
- Lucide `ChevronDown` (12px)
- Rotation: 0deg expanded, -90deg collapsed
- Transition: transform 150ms ease
- Color: `--text-muted`
- Only shown for tool_call, tool_result, or entries with metadata

**Type Icon:**
- Size: 16px
- Color: matches left border color
- Thinking icon: subtle pulse animation (opacity 0.5 to 1, 1.5s)

**Tool Name Badge (for tool calls):**
- Monospace font (`--font-mono`)
- Background: `--bg-base`
- Border-radius: 4px
- Padding: 2px 6px
- Text: `text-xs`, color matches type
- Example: "Read", "Write", "Bash"

**Type Label:**
- Text: `text-xs`, `--text-muted`
- Capitalize, replace underscores: "Tool Call", "Tool Result"

**Content Preview:**
- Text: `text-sm`, `--text-primary`
- White-space: pre-wrap
- Word-break: break-words
- Truncate to 200 chars when collapsed, with "..." ellipsis
- Full content when expanded

**Timestamp:**
- Right-aligned, shrink-0
- Text: `text-xs`, `--text-muted`
- Format: "HH:MM:SS" (24-hour)

```tsx
<div
  className="rounded-lg transition-all cursor-pointer"
  style={{
    backgroundColor: getMessageBgColor(type),
    borderLeft: `3px solid ${getMessageColor(type)}`,
  }}
  onClick={hasDetails ? onToggle : undefined}
>
  <div className="flex items-start gap-3 px-3 py-2.5">
    {hasDetails && (
      <ChevronDown
        className={cn(
          "w-3 h-3 mt-1 text-[var(--text-muted)] transition-transform",
          !isExpanded && "-rotate-90"
        )}
      />
    )}
    {!hasDetails && <span className="w-3" />}

    <span className="mt-0.5" style={{ color: getMessageColor(type) }}>
      {getMessageIcon(type)}
    </span>

    <div className="flex-1 min-w-0">
      <div className="flex items-center gap-2 mb-1">
        {toolName && (
          <span className="text-xs font-mono px-1.5 py-0.5 rounded bg-[var(--bg-base)]"
                style={{ color: getMessageColor(type) }}>
            {toolName}
          </span>
        )}
        <span className="text-xs text-[var(--text-muted)] capitalize">
          {type.replace('_', ' ')}
        </span>
      </div>
      <p className="text-sm text-[var(--text-primary)] whitespace-pre-wrap break-words">
        {displayContent}
      </p>
    </div>

    <span className="text-xs text-[var(--text-muted)] shrink-0 ml-2">
      {formatTimestamp(timestamp)}
    </span>
  </div>
</div>
```

## Tool Call Details (Expanded State)

**Details Container:**
- Appears below header when expanded
- Padding: 0 12px 12px 36px (indent to align with content)
- Border-top: `1px solid var(--border-subtle)`
- Margin-left: 36px (past the chevron and icon)

**Metadata Display:**
- Label: "Details" (`text-xs`, `font-medium`, `--text-muted`)
- Content: Pre-formatted JSON
- Background: `--bg-base`
- Border-radius: 6px
- Padding: 8px 12px
- Font: `--font-mono`, `text-xs`
- Color: `--text-secondary`
- Overflow-x: auto (horizontal scroll for long lines)
- Max-height: 300px with vertical scroll

**Copy Button:**
- Position: top-right of details container
- Lucide `Copy` icon (14px)
- Tooltip: "Copy to clipboard"
- Success state: `Check` icon briefly (2s)
- Background: `--bg-elevated`
- Border-radius: 4px
- Padding: 4px
- Hover: `--bg-hover`

```tsx
{hasDetails && isExpanded && metadata && (
  <div className="ml-9 mr-3 pb-3 border-t border-[var(--border-subtle)]">
    <div className="pt-3 relative">
      <div className="flex items-center justify-between mb-2">
        <span className="text-xs font-medium text-[var(--text-muted)]">Details</span>
        <Button
          variant="ghost"
          size="icon"
          className="h-6 w-6"
          onClick={handleCopy}
        >
          {copied ? <Check className="w-3.5 h-3.5" /> : <Copy className="w-3.5 h-3.5" />}
        </Button>
      </div>
      <pre className="text-xs font-mono p-3 rounded-md bg-[var(--bg-base)] text-[var(--text-secondary)] overflow-x-auto max-h-[300px] overflow-y-auto">
        {JSON.stringify(metadata, null, 2)}
      </pre>
    </div>
  </div>
)}
```

## JSON Syntax Highlighting

**For expanded tool call/result details:**
- Strings: `#a5d6a7` (soft green)
- Numbers: `#ffcc80` (warm amber)
- Booleans: `#81d4fa` (soft blue)
- Null: `#ce93d8` (soft purple)
- Keys: `#f0f0f0` (text-primary)
- Brackets/Braces: `--text-muted`
- Commas/Colons: `--text-muted`

Use a simple regex-based highlighter or lightweight library. Keep it subtle - the goal is readability, not rainbow code.

## Empty State

**Container:**
- Centered in available space
- Flex column, items-center
- Padding: 32px

**Icon:**
- Lucide `Activity` with dashed circle effect (custom SVG)
- Size: 48px
- Color: `--text-muted`
- Opacity: 0.5
- Margin-bottom: 16px

**Primary Text:**
- "No matching activities" (if filtered)
- "No activity yet" (if empty)
- Text: `text-base`, `--text-secondary`

**Secondary Text:**
- "Try adjusting your search or filters" (if filtered)
- "Agent activity will appear here when tasks are running" (if empty)
- Text: `text-sm`, `--text-muted`
- Margin-top: 4px

```tsx
<div className="flex flex-col items-center justify-center h-full p-8 text-center">
  <div className="mb-4 opacity-50">
    <Activity className="w-12 h-12 text-[var(--text-muted)]" strokeDasharray="4 4" />
  </div>
  <p className="text-[var(--text-secondary)]">
    {hasFilter ? 'No matching activities' : 'No activity yet'}
  </p>
  <p className="text-sm text-[var(--text-muted)] mt-1">
    {hasFilter
      ? 'Try adjusting your search or filters'
      : 'Agent activity will appear here when tasks are running'}
  </p>
</div>
```

## Auto-Scroll Behavior

**Auto-scroll:**
- Enabled by default
- When new message arrives, scroll to bottom smoothly
- Behavior: `smooth`

**Manual Override:**
- When user scrolls up (more than 50px from bottom), disable auto-scroll
- Sticky footer banner appears when auto-scroll is disabled

**Scroll-to-Bottom Banner:**
- Position: fixed at bottom of messages area
- Padding: 8px 16px
- Background: `--bg-elevated`
- Border-top: `1px solid var(--border-subtle)`
- Full width button inside
- Text: "Scroll to latest" (`text-sm`, `--accent-primary`)
- Click: re-enable auto-scroll, scroll to bottom
- Animation: slide up on appear

```tsx
{!autoScroll && filteredMessages.length > 0 && (
  <div className="border-t border-[var(--border-subtle)] px-4 py-2">
    <Button
      variant="ghost"
      className="w-full text-sm text-[var(--accent-primary)] hover:bg-[var(--bg-hover)]"
      onClick={() => {
        setAutoScroll(true);
        messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
      }}
    >
      <ChevronDown className="w-4 h-4 mr-1.5" />
      Scroll to latest
    </Button>
  </div>
)}
```

## Thinking Animation

For "thinking" type messages, add a subtle pulse animation to the icon:

```css
@keyframes thinking-pulse {
  0%, 100% { opacity: 0.5; }
  50% { opacity: 1; }
}

.thinking-icon {
  animation: thinking-pulse 1.5s ease-in-out infinite;
}
```

## Lucide Icons Used

| Context | Icon | Size | Notes |
|---------|------|------|-------|
| Header | `Activity` | 20px | In accent container |
| Clear button | `Trash2` | 16px | Ghost button |
| Search input | `Search` | 16px | Left position |
| Clear search | `X` | 16px | Right position, on hover |
| Thinking type | `Brain` | 16px | With pulse animation |
| Tool call type | `Terminal` | 16px | Accent color |
| Tool result type | `CheckCircle` | 16px | Success color |
| Text type | `MessageSquare` | 16px | Secondary color |
| Error type | `AlertCircle` | 16px | Error color |
| Expand/collapse | `ChevronDown` | 12px | Rotates -90deg when collapsed |
| Copy button | `Copy` | 14px | In details section |
| Copy success | `Check` | 14px | Brief replacement |
| Scroll to bottom | `ChevronDown` | 16px | In banner |

## Component Hierarchy

```
ActivityView
├── Header (glass effect)
│   ├── IconContainer + ActivityIcon
│   ├── Title "Activity"
│   ├── AlertBadge (conditional)
│   └── ClearButton
├── SearchFilterBar
│   ├── SearchInput
│   │   ├── SearchIcon
│   │   └── ClearButton (conditional)
│   └── FilterTabs
│       └── FilterTab × 6 (All, Thinking, Tool Calls, Results, Text, Errors)
├── MessagesContainer (scrollable)
│   ├── EmptyState (conditional)
│   └── ActivityMessage × n
│       ├── ExpandIndicator (chevron)
│       ├── TypeIcon
│       ├── Content
│       │   ├── ToolNameBadge (conditional)
│       │   ├── TypeLabel
│       │   └── ContentText
│       ├── Timestamp
│       └── ExpandedDetails (conditional)
│           ├── DetailsLabel
│           ├── CopyButton
│           └── JSONPre
└── ScrollToBottomBanner (conditional)
```

## Acceptance Criteria

- Activity view fills available viewport height below header
- Header shows "Activity" title with Lucide Activity icon in accent container
- Alert badge displays with count when high/critical alerts exist
- Clear button removes all messages (disabled when empty)
- Search input filters messages by content, type, or tool name
- Filter tabs switch between All, Thinking, Tool Calls, Results, Text, Errors
- Active filter tab has elevated background and border
- Each message type has distinct left border color and background tint
- Tool name displays as monospace badge for tool_call messages
- Timestamp shows in HH:MM:SS format, right-aligned
- Expandable messages show chevron indicator
- Clicking expandable message toggles details section
- Expanded details show JSON-formatted metadata
- Copy button copies metadata to clipboard with visual feedback
- Long content truncates at 200 chars with "..." when collapsed
- Auto-scroll to bottom when new messages arrive (by default)
- Manual scroll up disables auto-scroll
- "Scroll to latest" banner appears when auto-scroll is disabled
- Clicking banner re-enables auto-scroll and scrolls to bottom
- Empty state shows appropriate message based on filter status
- All interactive elements have visible focus states

## Design Quality Checklist

- NO purple or blue gradients anywhere
- Background uses subtle warm radial gradient (bottom-left origin)
- Messages have distinct left border colors per type
- Backgrounds use type-specific tints (subtle, not overwhelming)
- Orange accent used sparingly - only for tool_call type and focus rings
- Thinking messages have subtle pulse animation on icon
- Typography uses SF Pro with proper tracking
- All spacing follows 4px/8px grid
- Glass effect on header uses backdrop-blur
- Filter tabs have clean pill styling with proper active state
- Monospace font for tool names and JSON details
- Timestamps are subtle and right-aligned
- Expand/collapse has smooth rotation transition
- JSON details have subtle syntax highlighting
- Copy button shows check mark on success
- Scroll-to-bottom banner slides up smoothly
- Messages have consistent padding and border-radius
- Lucide icons used throughout (Activity, Brain, Terminal, CheckCircle, MessageSquare, AlertCircle, Search, X, Copy, Check, ChevronDown, Trash2)
- Empty state icon uses dashed effect for visual interest
- Touch targets are properly sized (36px+ for buttons)
- Focus rings use --shadow-glow pattern
