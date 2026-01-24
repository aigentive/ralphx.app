# Ideation View

The Ideation view is a two-panel interface for brainstorming and generating task proposals. It should feel like a premium AI-powered workspace with clear visual separation between conversation and proposals.

**Reference Inspiration**: Linear (clean panels, refined typography), Raycast (glass effects, Mac-native feel), ChatGPT (message bubbles, conversation flow)

## Overall Layout

**Structure:**
- Two-panel horizontal split with resizable divider
- Chat panel on left (~50%), Proposals panel on right (~50%)
- Viewport-filling height (`calc(100vh - header)`)
- Minimum panel width: 320px
- Drag handle for resizing (4px wide, cursor: `ew-resize`)

**Background:**
- Subtle radial gradient similar to Kanban
- Gradient: `radial-gradient(ellipse at top left, rgba(255,107,53,0.02) 0%, var(--bg-base) 40%)`
- Creates visual warmth and depth

**Header:**
- Session title with glass effect background
- Action buttons (New Session, Archive) using shadcn Button (ghost variant)
- Lucide icons: `Plus` (new), `Archive` (archive)
- Height: 52px with proper vertical centering

```css
.ideation-header {
  backdrop-filter: blur(8px);
  background: rgba(26, 26, 26, 0.85);
  border-bottom: 1px solid var(--border-subtle);
}
```

## Conversation Panel (Chat)

**Panel Container:**
- Background: `--bg-surface`
- Right border: `1px solid var(--border-subtle)`
- Subtle inner shadow for depth: `inset 0 0 80px rgba(0,0,0,0.1)`

**Panel Header:**
- Title: "Conversation" with Lucide `MessageSquare` icon
- Height: 40px
- Glass effect with subtle border bottom
- Icon + title left-aligned

```tsx
<div className="flex items-center gap-2 px-4 py-2 backdrop-blur-sm bg-[rgba(26,26,26,0.7)] border-b border-[var(--border-subtle)]">
  <MessageSquare className="w-4 h-4 text-[var(--text-secondary)]" />
  <h2 className="text-sm font-semibold text-[var(--text-primary)]">Conversation</h2>
</div>
```

**Message Area:**
- Scrollable with auto-scroll to newest
- Scroll behavior: `smooth`
- Padding: 16px (`--space-4`)
- Message spacing: 12px between messages

**Message Bubbles:**

*User Messages:*
- Right-aligned
- Background: `--accent-primary` (#ff6b35)
- Text: white
- Border radius: 12px (bottom-right: 4px for tail effect)
- Max-width: 85%
- Padding: 12px 16px
- Shadow: `--shadow-xs` for lift

*AI/Orchestrator Messages:*
- Left-aligned
- Background: `--bg-elevated`
- Border: `1px solid var(--border-subtle)`
- Border radius: 12px (bottom-left: 4px for tail effect)
- Max-width: 85%
- Padding: 12px 16px

```css
.message-user {
  border-radius: 12px 12px 4px 12px;
  background: var(--accent-primary);
  color: white;
  box-shadow: var(--shadow-xs);
}

.message-ai {
  border-radius: 12px 12px 12px 4px;
  background: var(--bg-elevated);
  border: 1px solid var(--border-subtle);
}
```

**Timestamps:**
- Size: 11px (`text-xs` - 1px smaller)
- Color: `--text-muted`
- Position: Below bubble, aligned to bubble edge
- Format: "2:34 PM" (short)

**Typing Indicator:**
- Three animated dots
- Color: `--text-muted`
- Animation: bounce with stagger (0.1s delay between each)
- Container: same styling as AI message bubble

```css
.typing-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--text-muted);
  animation: bounce 1.4s infinite ease-in-out both;
}
.typing-dot:nth-child(1) { animation-delay: 0s; }
.typing-dot:nth-child(2) { animation-delay: 0.1s; }
.typing-dot:nth-child(3) { animation-delay: 0.2s; }

@keyframes bounce {
  0%, 80%, 100% { transform: translateY(0); }
  40% { transform: translateY(-6px); }
}
```

**Empty State:**
- Centered vertically
- Lucide `MessageSquareText` icon (48px, `--text-muted`)
- Text: "Start the conversation" (font-medium)
- Subtext: "Describe your ideas and I'll help create task proposals" (text-sm, muted)
- Dashed border container (optional)

## Chat Input

**Container:**
- Sticky at bottom
- Padding: 12px 16px
- Border top: `1px solid var(--border-subtle)`
- Background: `--bg-surface`

**Input Field:**
- Using shadcn Input with custom styling
- Multi-line textarea with auto-resize
- Min height: 44px, Max height: 120px
- Border radius: 8px
- Focus state: `--shadow-glow` ring

**Send Button:**
- Using shadcn Button (primary variant)
- Lucide `Send` icon (18px)
- Positioned right of input
- Size: 44px x 44px (touch-friendly)
- Disabled state: opacity 50%, no hover effects
- Loading state: Lucide `Loader2` with spin animation

**Attach Button (future):**
- Ghost button with Lucide `Paperclip` icon
- Positioned left of input
- Disabled state for now (not implemented)

```tsx
<div className="flex items-end gap-2 p-3 border-t border-[var(--border-subtle)] bg-[var(--bg-surface)]">
  <Button variant="ghost" size="icon" disabled className="shrink-0">
    <Paperclip className="w-5 h-5" />
  </Button>
  <Textarea
    placeholder="Send a message..."
    className="flex-1 resize-none min-h-[44px] max-h-[120px]"
  />
  <Button size="icon" disabled={!canSend} className="shrink-0">
    {isSending ? <Loader2 className="w-5 h-5 animate-spin" /> : <Send className="w-5 h-5" />}
  </Button>
</div>
```

**Helper Text:**
- Below input: "Enter to send, Shift+Enter for new line"
- Size: 11px, color: `--text-muted`

## Proposals Panel

**Panel Container:**
- Background: `--bg-surface` (slightly different shade for distinction)
- No left border (divider provides separation)

**Panel Header:**
- Title: "Task Proposals" with Lucide `ListTodo` icon
- Count badge: shadcn Badge (secondary variant) showing proposal count
- Height: 40px

```tsx
<div className="flex items-center justify-between px-4 py-2 border-b border-[var(--border-subtle)]">
  <div className="flex items-center gap-2">
    <ListTodo className="w-4 h-4 text-[var(--text-secondary)]" />
    <h2 className="text-sm font-semibold">Task Proposals</h2>
  </div>
  <Badge variant="secondary">{count}</Badge>
</div>
```

**Toolbar:**
- Horizontal layout with selection count on left
- Action buttons on right: Select All, Deselect All, Sort by Priority, Clear All
- Using shadcn Button (ghost variant, icon-only with tooltips)
- Lucide icons:
  - `CheckSquare` (select all)
  - `Square` (deselect all)
  - `ArrowUpDown` (sort by priority)
  - `Trash2` (clear all)
- Separator between select buttons and other actions

**Proposal List:**
- Scrollable area
- Spacing: 8px between cards
- Padding: 16px

## ProposalCard

**Card Structure:**
- Using shadcn Card with custom styling
- Padding: 12px (`--space-3`)
- Border radius: 8px (`--radius-md`)
- Background: `--bg-elevated`
- Border: `1px solid var(--border-subtle)`
- Shadow: `--shadow-xs` for subtle lift

**Selection Checkbox:**
- Using shadcn Checkbox (not native checkbox)
- Positioned top-left
- Size: 18px
- Custom accent color when checked: `--accent-primary`

```tsx
<Checkbox
  checked={proposal.selected}
  onCheckedChange={() => onSelect(proposal.id)}
  className="data-[state=checked]:bg-[var(--accent-primary)] data-[state=checked]:border-[var(--accent-primary)]"
/>
```

**Content Layout:**
- Title: `text-sm`, `font-medium`, `--text-primary`
- Description: `text-xs`, `--text-secondary`, 2-line clamp
- Badge row: flex wrap, gap-1.5

**Priority Badge:**
- Using shadcn Badge with semantic variants
- Colors by priority:
  - Critical: destructive variant (red background)
  - High: warning variant (orange background - custom)
  - Medium: default variant with accent tint
  - Low: secondary variant (gray)

```tsx
const priorityVariant = {
  critical: "destructive",
  high: "warning", // custom variant with orange
  medium: "default",
  low: "secondary"
};
```

**Category Badge:**
- shadcn Badge (secondary variant)
- Smaller padding for compactness

**Dependency Indicators:**
- Small Lucide icons: `ArrowUp` (depends on), `ArrowDown` (blocks)
- Color: `--text-muted`
- Text: "Depends on 2" / "Blocks 3"

**Hover State:**
- Card lifts: `transform: translateY(-2px)`
- Shadow increases: `--shadow-sm`
- Border lightens slightly
- Transition: 150ms ease

**Selected State:**
- Border: `2px solid var(--accent-primary)`
- Background: `var(--accent-muted)` (subtle orange tint)
- Box shadow: `0 0 0 3px rgba(255,107,53,0.15)` for glow

```css
.proposal-card.selected {
  border: 2px solid var(--accent-primary);
  background: var(--accent-muted);
  box-shadow: 0 0 0 3px rgba(255,107,53,0.15);
}
```

**Action Buttons (Edit/Remove):**
- Visible on hover only (opacity transition)
- Using shadcn Button (ghost variant, size="icon-sm")
- Lucide icons: `Pencil` (edit), `X` (remove)
- Position: top-right corner

**Drag Handle:**
- Lucide `GripVertical` icon
- Visible on hover
- Position: left side of card
- Color: `--text-muted`, hover: `--text-secondary`
- Cursor: `grab` (dragging: `grabbing`)

**Drag State:**
- Same as TaskCard: `scale(1.02) rotate(2deg)`
- Shadow: `--shadow-md`
- Opacity: 0.9
- Z-index: 50

## Empty State (Proposals)

- Centered vertically
- Lucide `Lightbulb` icon (48px, `--text-muted`)
- Main text: "No proposals yet" (font-medium)
- Subtext: "Chat with the orchestrator to generate task proposals" (text-sm)
- Dashed border container
- Padding: 48px

```tsx
<div className="flex flex-col items-center justify-center h-full p-12 text-center">
  <div className="p-4 rounded-lg border-2 border-dashed border-[var(--border-subtle)]">
    <Lightbulb className="w-12 h-12 mx-auto mb-4 text-[var(--text-muted)]" />
    <p className="font-medium text-[var(--text-secondary)]">No proposals yet</p>
    <p className="text-sm text-[var(--text-muted)] mt-1">
      Chat with the orchestrator to generate task proposals
    </p>
  </div>
</div>
```

## Apply Section

**Container:**
- Fixed at bottom of proposals panel
- Border top: `1px solid var(--border-subtle)`
- Background: `--bg-surface`
- Padding: 12px 16px
- Height: 56px

**Layout:**
- Selection count on left: "3 selected"
- Apply dropdown on right

**Apply Button:**
- Using shadcn DropdownMenu with Button trigger
- Button: primary variant when enabled, secondary when disabled
- Chevron down icon (Lucide `ChevronDown`)
- Text: "Apply to..."

**Dropdown Menu:**
- shadcn DropdownMenuContent
- Options: Draft, Backlog, Todo
- Each with description text
- Lucide icons for each column type

```tsx
<DropdownMenu>
  <DropdownMenuTrigger asChild>
    <Button disabled={!canApply}>
      Apply to
      <ChevronDown className="w-4 h-4 ml-1" />
    </Button>
  </DropdownMenuTrigger>
  <DropdownMenuContent align="end">
    <DropdownMenuItem onClick={() => handleApply("draft")}>
      <FileEdit className="w-4 h-4 mr-2" />
      Draft
    </DropdownMenuItem>
    <DropdownMenuItem onClick={() => handleApply("backlog")}>
      <Inbox className="w-4 h-4 mr-2" />
      Backlog
    </DropdownMenuItem>
    <DropdownMenuItem onClick={() => handleApply("todo")}>
      <ListTodo className="w-4 h-4 mr-2" />
      Todo
    </DropdownMenuItem>
  </DropdownMenuContent>
</DropdownMenu>
```

## Resize Handle

**Structure:**
- 4px wide invisible hit area
- Visible line: 1px, centered
- Color: `--border-subtle`, hover: `--accent-primary`
- Cursor: `ew-resize`
- Height: 100% of panel

**Visual States:**
- Default: subtle line
- Hover: accent color, slight glow
- Dragging: accent color, stronger glow

```css
.resize-handle {
  width: 4px;
  cursor: ew-resize;
  position: relative;
  background: transparent;
}

.resize-handle::after {
  content: '';
  position: absolute;
  top: 0;
  bottom: 0;
  left: 50%;
  width: 1px;
  background: var(--border-subtle);
  transition: all 150ms ease;
}

.resize-handle:hover::after {
  background: var(--accent-primary);
  box-shadow: 0 0 8px rgba(255,107,53,0.3);
}
```

## Component Hierarchy

```
IdeationView
├── IdeationHeader (glass effect)
│   ├── SessionTitle
│   └── ActionButtons (New, Archive)
├── ResizeHandle
├── ConversationPanel
│   ├── PanelHeader (icon + title)
│   ├── MessageList (scrollable)
│   │   ├── ChatMessage (user - right aligned)
│   │   ├── ChatMessage (ai - left aligned)
│   │   └── TypingIndicator (if loading)
│   └── ChatInput
│       ├── AttachButton (disabled)
│       ├── Textarea (auto-resize)
│       └── SendButton
└── ProposalsPanel
    ├── PanelHeader (icon + title + count)
    ├── Toolbar (select/sort/clear actions)
    ├── ProposalList (scrollable, sortable)
    │   └── ProposalCard (×N)
    │       ├── Checkbox
    │       ├── Title + Description
    │       ├── Badges (Priority, Category)
    │       ├── DependencyInfo
    │       └── ActionButtons (Edit, Remove)
    ├── EmptyState (if no proposals)
    └── ApplySection
        ├── SelectionCount
        └── ApplyDropdown
```

## Acceptance Criteria

- Two-panel layout fills available viewport height
- Panels are resizable with drag handle
- Minimum panel width is 320px
- User messages align right with warm orange background
- AI messages align left with elevated background
- Message bubbles have asymmetric border radius (tail effect)
- Timestamps appear below each message bubble
- Typing indicator shows animated dots during AI response
- Auto-scroll to newest message works smoothly
- Chat input supports multi-line with auto-resize
- Enter sends message, Shift+Enter adds newline
- Send button shows loading spinner while sending
- Proposal cards use shadcn Card component
- Selection checkbox uses shadcn Checkbox
- Selected proposals have orange border and tinted background
- Drag-and-drop reordering works with visual feedback
- Priority badges use correct semantic colors
- Action buttons appear on card hover
- Apply dropdown shows column options
- Empty states show appropriate Lucide icons and text

## Design Quality Checklist

- NO purple or blue gradients anywhere
- Background uses subtle warm radial gradient (not flat)
- Message bubbles have asymmetric corners for tail effect
- Shadows are layered for realistic depth
- Orange accent used sparingly - only for user messages, selection, and primary buttons
- Typography uses SF Pro with proper tracking
- All spacing follows 4px/8px grid
- Glass effect on headers uses backdrop-blur
- Micro-interactions feel snappy (150ms transitions)
- Proposal cards lift on hover (translateY -2px)
- Resize handle glows orange on hover
- Empty states use Lucide icons (Lightbulb, MessageSquareText)
- Focus rings use --shadow-glow pattern
- Typing indicator animation is smooth and playful
- Panel headers have consistent height and alignment
