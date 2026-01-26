# Chat Panel (Global)

The global Chat Panel is a persistent, resizable side panel for interacting with AI agents during task execution. It embodies the **Refined Studio** aesthetic—premium glass effects, compact sizing, and warm orange accents.

**Reference Inspiration**: Raycast (resizable panel, Mac-native), Vercel v0 (contextual chat), Linear (compact density)

## Panel Structure

**Positioning:**
- Right-side slide-in panel
- Minimum width: 280px
- Maximum width: 50% of viewport
- Default width: 360px
- Full viewport height minus header

**Appearance:**
- Background: `--bg-surface`
- Left border: `1px solid var(--border-subtle)`
- Shadow: `--shadow-md` for floating effect
- Z-index: 40 (above main content, below modals)

**Animation:**
- Slide in from right: `translateX(100%) -> translateX(0)`
- Duration: 250ms
- Easing: `ease-out` for open, `ease-in` for close
- Collapse transition: 200ms width animation

```css
.chat-panel {
  position: fixed;
  top: var(--header-height, 48px);
  right: 0;
  bottom: 0;
  background: var(--bg-surface);
  border-left: 1px solid var(--border-subtle);
  box-shadow: var(--shadow-md);
  z-index: 40;
}

.chat-panel.entering {
  animation: slideInRight 250ms ease-out forwards;
}

.chat-panel.exiting {
  animation: slideOutRight 200ms ease-in forwards;
}

@keyframes slideInRight {
  from { transform: translateX(100%); opacity: 0.5; }
  to { transform: translateX(0); opacity: 1; }
}

@keyframes slideOutRight {
  from { transform: translateX(0); opacity: 1; }
  to { transform: translateX(100%); opacity: 0.5; }
}
```

## Resize Handle

**Structure:**
- 6px wide invisible hit area positioned on left edge
- Visible indicator: 2px wide bar, centered
- Cursor: `ew-resize`

**Visual States:**
- Default: hidden (only cursor change hints at resizability)
- Hover: subtle vertical bar appears (`--border-default`)
- Dragging: accent-colored bar with glow

```css
.resize-handle {
  position: absolute;
  top: 0;
  left: -3px;
  bottom: 0;
  width: 6px;
  cursor: ew-resize;
  z-index: 41;
}

.resize-handle::after {
  content: '';
  position: absolute;
  top: 50%;
  left: 50%;
  width: 2px;
  height: 48px;
  transform: translate(-50%, -50%);
  background: transparent;
  border-radius: 1px;
  transition: all 150ms ease;
}

.resize-handle:hover::after {
  background: var(--border-default);
}

.resize-handle.dragging::after {
  background: var(--accent-primary);
  box-shadow: 0 0 8px rgba(255,107,53,0.4);
  height: 64px;
}
```

## Panel Header

**Structure:**
- Height: 48px
- Padding: 0 12px
- Border bottom: `1px solid var(--border-subtle)`
- Background: `--bg-surface` (matches panel)
- Display: flex, align items center, justify content between

**Left Section:**
- Context indicator icon (Lucide, 16px, `--text-secondary`)
- Context title (truncated, `text-sm`, `font-medium`)
- Status badge (optional, for active agent)

**Context Indicators by Type:**
| Context | Icon | Title Example |
|---------|------|---------------|
| Task | `CheckSquare` | "Implement auth system" |
| Project | `FolderKanban` | "RalphX" |
| General | `MessageSquare` | "Chat" |
| Agent | `Bot` | "Worker Agent" |

**Right Section:**
- Collapse button (Lucide `PanelRightClose`, 18px)
- Close button (Lucide `X`, 18px)
- Both using shadcn Button (ghost variant, size="icon-sm")
- 4px gap between buttons

```tsx
<div className="flex items-center justify-between h-12 px-3 border-b border-[var(--border-subtle)]">
  <div className="flex items-center gap-2 min-w-0 flex-1">
    <CheckSquare className="w-4 h-4 shrink-0 text-[var(--text-secondary)]" />
    <span className="text-sm font-medium truncate">{contextTitle}</span>
    {activeAgent && (
      <Badge variant="secondary" className="shrink-0">
        <Loader2 className="w-3 h-3 mr-1 animate-spin" />
        Working
      </Badge>
    )}
  </div>
  <div className="flex items-center gap-1 shrink-0">
    <Button variant="ghost" size="icon-sm" onClick={onCollapse}>
      <PanelRightClose className="w-[18px] h-[18px]" />
    </Button>
    <Button variant="ghost" size="icon-sm" onClick={onClose}>
      <X className="w-[18px] h-[18px]" />
    </Button>
  </div>
</div>
```

## Collapsed State

When collapsed, the panel reduces to a thin bar that can be expanded.

**Structure:**
- Width: 40px
- Background: `--bg-surface`
- Border left: `1px solid var(--border-subtle)`

**Content:**
- Expand button (Lucide `PanelRightOpen`, 18px)
- Unread indicator dot if new messages
- Vertically centered

**Unread Indicator:**
- 8px diameter circle
- Background: `--accent-primary`
- Position: top of button area
- Subtle pulse animation when new

```css
.unread-dot {
  width: 8px;
  height: 8px;
  background: var(--accent-primary);
  border-radius: 50%;
  animation: pulse 2s ease-in-out infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; transform: scale(1); }
  50% { opacity: 0.7; transform: scale(1.1); }
}
```

## Message Thread

**Container:**
- Using shadcn ScrollArea for smooth scrolling
- Flex column layout
- Padding: 12px
- Gap between messages: 8px
- Auto-scroll to newest (with manual scroll override)

**Scroll Behavior:**
- Auto-scroll when at bottom and new message arrives
- Lock auto-scroll when user scrolls up
- "New messages" button appears when scrolled up and new messages arrive
- Smooth scroll animation

**Message Groups:**
- Consecutive messages from same sender grouped
- Only first message in group shows avatar/indicator
- 4px gap within group, 12px between groups

## Message Design

Messages use the **Refined Studio** aesthetic with gradient backgrounds and compact sizing.

**User Messages:**
- Right-aligned
- Background: `linear-gradient(135deg, #ff6b35 0%, #f97316 100%)`
- Text: white
- Border radius: 12px 12px 4px 12px (tail bottom-right)
- Max width: 80%
- Padding: 8px 12px (px-3 py-2)
- Shadow: `0 2px 8px rgba(255,107,53,0.2)`
- Font size: 13px

**Assistant Messages:**
- Left-aligned
- Background: `linear-gradient(180deg, rgba(38,38,38,0.95) 0%, rgba(32,32,32,0.98) 100%)`
- Border: `1px solid rgba(255,255,255,0.06)`
- Border radius: 12px 12px 12px 4px (tail bottom-left)
- Max width: 80%
- Padding: 8px 12px (px-3 py-2)
- Font size: 13px
- backdrop-filter: blur(8px)

```css
.message-user {
  align-self: flex-end;
  background: linear-gradient(135deg, #ff6b35 0%, #f97316 100%);
  color: white;
  border-radius: 12px 12px 4px 12px;
  padding: 8px 12px;
  box-shadow: 0 2px 8px rgba(255,107,53,0.2);
  font-size: 13px;
}

.message-assistant {
  align-self: flex-start;
  background: linear-gradient(180deg, rgba(38,38,38,0.95) 0%, rgba(32,32,32,0.98) 100%);
  border: 1px solid rgba(255,255,255,0.06);
  border-radius: 12px 12px 12px 4px;
  padding: 8px 12px;
  backdrop-filter: blur(8px);
  font-size: 13px;
}
```

**Agent Indicator:**
- Shown for first message in assistant group
- Icon: Lucide `Bot` (14px) or agent-specific icon
- Color: `--text-muted`
- Position: left of message, vertically centered to first line

**Timestamps:**
- Size: 11px
- Color: `--text-muted`
- Position: below message, aligned to message edge
- Format: "2:34 PM" or "Just now"
- Only show on last message of group (or on hover for others)

## Markdown Rendering

Assistant messages support full markdown rendering.

**Typography:**
- Paragraphs: 14px, `--leading-normal`, 8px bottom margin
- Headers: bold, reduced size scale (h1=18px, h2=16px, h3=15px)
- Lists: 16px left padding, proper bullet/number styling
- Links: `--accent-primary`, underline on hover

**Code Blocks:**
- Background: `--bg-base`
- Border: `1px solid var(--border-subtle)`
- Border radius: 6px
- Font: `--font-mono` (JetBrains Mono)
- Font size: 13px
- Padding: 8px 12px
- Syntax highlighting: Dracula-inspired dark theme
- Copy button: top-right, visible on hover
- Language label: top-left, muted text

**Inline Code:**
- Background: `--bg-base`
- Border radius: 3px
- Padding: 2px 4px
- Font: `--font-mono`
- Font size: 13px

```css
.code-block {
  background: var(--bg-base);
  border: 1px solid var(--border-subtle);
  border-radius: 6px;
  padding: 8px 12px;
  font-family: var(--font-mono);
  font-size: 13px;
  overflow-x: auto;
  position: relative;
}

.code-block .copy-button {
  position: absolute;
  top: 4px;
  right: 4px;
  opacity: 0;
  transition: opacity 150ms ease;
}

.code-block:hover .copy-button {
  opacity: 1;
}
```

## Typing Indicator

Same design as Ideation view for consistency.

**Container:**
- Left-aligned, same styling as assistant message
- Padding: 10px 14px
- Background: `--bg-elevated`

**Dots:**
- Three dots, 6px diameter each
- Color: `--text-muted`
- 4px gap between dots
- Staggered bounce animation

```css
.typing-indicator {
  display: flex;
  gap: 4px;
  align-items: center;
}

.typing-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--text-muted);
  animation: typingBounce 1.4s ease-in-out infinite;
}

.typing-dot:nth-child(1) { animation-delay: 0s; }
.typing-dot:nth-child(2) { animation-delay: 0.15s; }
.typing-dot:nth-child(3) { animation-delay: 0.3s; }

@keyframes typingBounce {
  0%, 60%, 100% { transform: translateY(0); }
  30% { transform: translateY(-4px); }
}
```

## Chat Input

**Container:**
- Sticky at bottom of panel
- Padding: 10px 12px
- Border top: `1px solid var(--border-subtle)`
- Background: `--bg-surface`

**Input Field:**
- Using shadcn Textarea (not Input, for multi-line)
- Single row default, auto-expands up to 4 rows
- Min height: 40px
- Max height: 100px
- Border radius: 8px
- Placeholder: "Send a message..." with muted color
- Focus: `--shadow-glow` ring

**Send Button:**
- Positioned inside input container, right side
- Using shadcn Button (ghost variant when empty, primary when has content)
- Lucide `ArrowUp` icon (18px) - matches modern chat UIs
- Size: 32px x 32px
- Border radius: 6px
- Position: absolute right, vertically centered
- Disabled state: opacity 40%, no hover

**Keyboard Hints:**
- Below input (optional, can be hidden)
- Size: 11px, color: `--text-muted`
- Text: "Enter to send"
- Show Shift+Enter hint on multi-line input

```tsx
<div className="relative p-3 border-t border-[var(--border-subtle)]">
  <Textarea
    value={message}
    onChange={(e) => setMessage(e.target.value)}
    onKeyDown={handleKeyDown}
    placeholder="Send a message..."
    className="pr-12 min-h-[40px] max-h-[100px] resize-none"
    rows={1}
  />
  <Button
    variant={message.trim() ? "default" : "ghost"}
    size="icon-sm"
    disabled={!message.trim() || isSending}
    onClick={handleSend}
    className="absolute right-4 top-1/2 -translate-y-1/2"
  >
    {isSending ? (
      <Loader2 className="w-[18px] h-[18px] animate-spin" />
    ) : (
      <ArrowUp className="w-[18px] h-[18px]" />
    )}
  </Button>
</div>
```

**Loading State:**
- Send button shows Loader2 with spin animation
- Input disabled during send
- Subtle dimming of input container

## Empty State

**Structure:**
- Centered in message area
- Lucide `MessageSquare` icon (40px, `--text-muted`)
- Title: "Start a conversation" (14px, `font-medium`, `--text-secondary`)
- Subtitle: "Ask questions or get help with your tasks" (13px, `--text-muted`)
- Padding: 32px

**Container:**
- Optional dashed border for emphasis
- Background: subtle `--bg-base` tint
- Border radius: 8px

```tsx
<div className="flex flex-col items-center justify-center h-full p-8 text-center">
  <MessageSquare className="w-10 h-10 mb-3 text-[var(--text-muted)]" />
  <p className="text-sm font-medium text-[var(--text-secondary)]">
    Start a conversation
  </p>
  <p className="text-[13px] text-[var(--text-muted)] mt-1">
    Ask questions or get help with your tasks
  </p>
</div>
```

## Context Switching

When context changes (e.g., selecting a different task), the panel updates.

**Transition:**
- Header updates immediately
- Message area fades out/in (150ms crossfade)
- Scroll position resets to bottom
- Loading state shown while fetching history

**Context Badge (in header):**
- Shows current context type
- Icon + short label
- Clickable to return to context (e.g., task detail)

## Lucide Icons Used

| Icon | Usage |
|------|-------|
| `MessageSquare` | General chat context, empty state |
| `CheckSquare` | Task context indicator |
| `FolderKanban` | Project context indicator |
| `Bot` | Agent context, assistant messages |
| `X` | Close button |
| `PanelRightClose` | Collapse button |
| `PanelRightOpen` | Expand button (collapsed state) |
| `ArrowUp` | Send button |
| `Loader2` | Loading/sending states |
| `Copy` | Code block copy button |
| `Check` | Copy success feedback |

## Component Hierarchy

```
ChatPanel
├── ResizeHandle
├── ChatPanelHeader
│   ├── ContextIndicator
│   │   ├── Icon (context-specific)
│   │   └── Title (truncated)
│   ├── StatusBadge (if agent active)
│   └── HeaderActions
│       ├── CollapseButton
│       └── CloseButton
├── MessageThread (ScrollArea)
│   ├── MessageGroup (×N)
│   │   ├── AgentIndicator (first message only)
│   │   └── ChatMessage (×N)
│   │       ├── MessageContent (markdown)
│   │       └── Timestamp (last in group)
│   ├── TypingIndicator (if loading)
│   └── EmptyState (if no messages)
├── NewMessagesButton (if scrolled up)
└── ChatInput
    ├── Textarea
    ├── SendButton
    └── KeyboardHint (optional)
```

## Collapsed Panel

```
ChatPanelCollapsed
├── ExpandButton
│   └── PanelRightOpen icon
└── UnreadIndicator (if new messages)
```

## Acceptance Criteria

- Panel slides in from right when opened
- Panel width is resizable via left edge drag handle
- Minimum width is 280px, maximum is 50% of viewport
- Resize handle shows visual feedback on hover/drag
- Panel can be collapsed to thin bar (40px)
- Collapsed state shows unread indicator when new messages arrive
- Header shows current context (task, project, or general)
- Header truncates long context titles properly
- Close button closes panel completely
- Collapse button minimizes panel to thin bar
- Expand button (in collapsed state) restores panel
- User messages align right with warm orange background
- Assistant messages align left with elevated background
- Messages have asymmetric border radius (tail effect)
- Consecutive messages from same sender are grouped
- Timestamps show on last message of group
- Typing indicator displays animated dots
- Auto-scroll works when at bottom
- Auto-scroll pauses when user scrolls up
- "New messages" button appears when scrolled up with new messages
- Markdown renders properly (paragraphs, lists, headers)
- Code blocks have syntax highlighting
- Code blocks have copy button on hover
- Inline code has distinct styling
- Chat input supports multi-line with auto-resize
- Enter sends message (Shift+Enter for newline)
- Send button disabled when input empty
- Send button shows loading spinner while sending
- Empty state shows helpful message and icon
- Context switching transitions smoothly
- Panel remembers width preference

## Design Quality Checklist

**Colors:**
- NO purple or blue gradients
- User messages use warm orange (`--accent-primary`)
- Assistant messages use `--bg-elevated`
- Code blocks use `--bg-base`
- Status colors follow semantic conventions

**Typography:**
- Messages use 14px (`text-sm`) body text
- Code uses `--font-mono` (JetBrains Mono)
- Headers in markdown are proportionally scaled
- Timestamps use 11px muted text

**Spacing:**
- All spacing follows 4px base grid
- 8px gap between message groups
- 4px gap within message groups
- 12px panel padding
- 10px input container padding

**Shadows:**
- Panel has `--shadow-md` for floating effect
- User messages have `--shadow-xs` for lift
- Focus ring uses `--shadow-glow`

**Borders:**
- Panel left border is subtle (1px `--border-subtle`)
- Assistant messages have subtle border
- Code blocks have subtle border
- Border radius is 10px for messages, 8px for inputs

**Motion:**
- Panel slides in/out with 200-250ms animation
- Resize handle hover transition is 150ms
- Message fade transitions are 150ms
- Typing dot animation is 1.4s with stagger
- Loading spinner uses standard spin animation

**Icons:**
- All icons from Lucide React
- Consistent 16-18px sizes for actions
- Muted color for decorative icons
- Proper visual weight (stroke-width 2)

**Accessibility:**
- Focus visible on all interactive elements
- Escape key closes panel
- Keyboard navigation works in thread
- Screen reader labels for icon buttons
- Color contrast meets WCAG AA
