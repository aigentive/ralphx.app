# Ideation View

The Ideation view is a three-panel interface for brainstorming and generating task proposals. It embodies the **Refined Studio** aesthetic—luxurious dark surfaces, sophisticated depth, and warm orange jewel accents.

**Reference Inspiration**: Linear (clean panels), Notion (editorial typography), Raycast (Mac-native glass effects), ChatGPT (premium message bubbles)

---

## Overall Layout

**Structure:**
- Three-panel horizontal layout
- Left: Session Browser (280px, always visible)
- Center: Conversation Panel (~50% of remaining space)
- Right: Proposals Panel (~50% of remaining space)
- Resizable divider between conversation and proposals
- Viewport-filling height (`calc(100vh - header)`)

**Background:**
Atmospheric gradient with warm top-left glow:

```css
.ideation-view {
  background:
    radial-gradient(ellipse 80% 50% at 20% 0%, rgba(255,107,53,0.04) 0%, transparent 50%),
    radial-gradient(ellipse 60% 40% at 80% 100%, rgba(139,92,246,0.02) 0%, transparent 50%),
    linear-gradient(180deg, #141414 0%, #0a0a0a 100%);
}
```

---

## Session Browser (Left Sidebar)

The session browser is always visible, showing existing ideation sessions. Uses **compact sizing** for maximum information density.

**Container:**
```css
.session-browser {
  width: 260px;
  min-width: 260px;
  background: #0a0a0a;
  border-right: 1px solid rgba(255,255,255,0.06);
  display: flex;
  flex-direction: column;
}
```

**Header:**
```tsx
<div className="px-3 py-3 border-b border-white/[0.06]">
  <div className="flex items-center justify-between mb-3">
    <div className="flex items-center gap-2">
      <div className="w-7 h-7 rounded-lg bg-gradient-to-br from-[#ff6b35]/20 to-[#ff6b35]/5 flex items-center justify-center border border-[#ff6b35]/20">
        <Layers className="w-3.5 h-3.5 text-[#ff6b35]" />
      </div>
      <div>
        <h2 className="text-sm font-semibold text-white tracking-tight">Sessions</h2>
        <p className="text-[10px] text-white/50">{count} total</p>
      </div>
    </div>
  </div>
  <Button size="sm" className="w-full h-8 text-xs">New Session</Button>
</div>
```

### Session Card

Compact cards with hover and selected states:

```css
.session-card {
  background: transparent;
  border: 1px solid transparent;
  border-radius: 8px;
  padding: 10px; /* p-2.5 */
  transition: all 200ms cubic-bezier(0.4, 0, 0.2, 1);
  cursor: pointer;
}

.session-card:hover {
  background: linear-gradient(180deg, rgba(32,32,32,0.95) 0%, rgba(26,26,26,0.98) 100%);
  border-color: rgba(255,255,255,0.1);
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(0,0,0,0.3);
}

.session-card.selected {
  background: linear-gradient(135deg, rgba(255,107,53,0.08) 0%, rgba(255,107,53,0.04) 100%);
  border-color: rgba(255,107,53,0.25);
  animation: glowPulse 3s ease-in-out infinite;
}
```

**Card Content:**
- Title: `text-sm font-medium text-white/90` (truncate with ellipsis)
- Preview: `text-xs text-white/40` (2-line clamp)
- Metadata row: timestamp + message count
- Status badge (active sessions only)

**Empty State:**
```tsx
<div className="flex flex-col items-center justify-center h-48 text-center">
  <div className="relative mb-4">
    <div className="absolute inset-0 blur-xl bg-[#ff6b35]/20 rounded-full" />
    <Archive className="relative w-8 h-8 text-white/30" />
  </div>
  <p className="text-sm text-white/40 mb-1">No sessions yet</p>
  <p className="text-xs text-white/25">Start a new ideation session</p>
</div>
```

---

## Start Session Panel (No Session Selected)

Shown when no session is selected. Premium centered CTA.

```tsx
<div className="flex-1 flex items-center justify-center p-8">
  <div className="relative max-w-md text-center">
    {/* Atmospheric glow */}
    <div className="absolute inset-0 blur-3xl bg-gradient-to-br from-[#ff6b35]/10 to-purple-500/5 rounded-full scale-150" />

    {/* Content */}
    <div className="relative">
      <div className="inline-flex p-4 mb-6 rounded-2xl bg-gradient-to-br from-white/[0.08] to-white/[0.02] border border-white/[0.06]">
        <Sparkles className="w-8 h-8 text-[#ff6b35]/80" />
      </div>

      <h2 className="text-2xl font-semibold text-white mb-3 tracking-tight">
        Start Ideating
      </h2>

      <p className="text-white/50 mb-8 leading-relaxed max-w-sm mx-auto">
        Transform your ideas into actionable task proposals with AI-powered brainstorming
      </p>

      <Button className="bg-gradient-to-r from-[#ff6b35] to-[#f97316] hover:from-[#ff7a4d] hover:to-[#fb923c]">
        <Plus className="w-4 h-4 mr-2" />
        New Session
      </Button>
    </div>
  </div>
</div>
```

---

## Conversation Panel

**Panel Container:**
```css
.conversation-panel {
  flex: 1;
  min-width: 320px;
  display: flex;
  flex-direction: column;
  background: transparent;
  border-right: 1px solid rgba(255,255,255,0.04);
}
```

**Panel Header:**
```tsx
<div className="flex items-center gap-3 px-4 py-3 border-b border-white/[0.06] bg-gradient-to-r from-[#141414] to-transparent">
  <div className="p-2 rounded-lg bg-gradient-to-br from-white/[0.08] to-white/[0.02]">
    <MessageSquare className="w-4 h-4 text-white/70" />
  </div>
  <div>
    <h2 className="text-sm font-semibold text-white tracking-tight">Conversation</h2>
    <p className="text-xs text-white/40">{messageCount} messages</p>
  </div>
</div>
```

### Message Bubbles

**User Message (Right-Aligned):**
```css
.message-user {
  align-self: flex-end;
  max-width: 85%;
  background: linear-gradient(135deg, #ff6b35 0%, #f97316 100%);
  color: white;
  padding: 12px 16px;
  border-radius: 16px 16px 4px 16px;
  box-shadow: 0 2px 8px rgba(255,107,53,0.25), 0 1px 2px rgba(0,0,0,0.1);
}
```

**AI Message (Left-Aligned):**
```css
.message-ai {
  align-self: flex-start;
  max-width: 85%;
  background: linear-gradient(180deg, rgba(38,38,38,0.95) 0%, rgba(32,32,32,0.98) 100%);
  border: 1px solid rgba(255,255,255,0.06);
  padding: 12px 16px;
  border-radius: 16px 16px 16px 4px;
  box-shadow: 0 2px 8px rgba(0,0,0,0.15);
  backdrop-filter: blur(8px);
}
```

**Timestamps:**
- Position: Below message, aligned to bubble edge
- Style: `text-[10px] text-white/30 mt-1`

**Typing Indicator:**
```css
.typing-indicator {
  display: flex;
  gap: 4px;
  padding: 12px 16px;
  background: linear-gradient(180deg, rgba(38,38,38,0.95) 0%, rgba(32,32,32,0.98) 100%);
  border-radius: 16px 16px 16px 4px;
}

.typing-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: rgba(255,255,255,0.4);
  animation: typingBounce 1.4s ease-in-out infinite;
}
```

### Conversation Empty State

```tsx
<div className="flex-1 flex flex-col items-center justify-center p-8 text-center">
  <div className="relative mb-6">
    <div className="absolute inset-0 blur-2xl bg-[#ff6b35]/20 rounded-full scale-150" />
    <div className="relative p-4 rounded-2xl bg-gradient-to-br from-white/[0.08] to-white/[0.02] border border-white/[0.06]">
      <MessageSquareText className="w-8 h-8 text-[#ff6b35]/80" />
    </div>
  </div>
  <h3 className="text-lg font-semibold text-white/90 mb-2 tracking-tight">
    Start the conversation
  </h3>
  <p className="text-sm text-white/40 max-w-[280px] leading-relaxed">
    Describe your ideas and I'll help create structured task proposals
  </p>
</div>
```

### Chat Input

```tsx
<div className="p-4 border-t border-white/[0.06] bg-gradient-to-t from-[#0a0a0a] to-transparent">
  <div className="relative">
    <textarea
      placeholder="Describe your idea..."
      className="w-full bg-[rgba(0,0,0,0.3)] border border-white/[0.08] rounded-xl px-4 py-3 pr-12
                 text-sm text-white placeholder:text-white/30
                 focus:outline-none focus:border-[#ff6b35]/50 focus:ring-2 focus:ring-[#ff6b35]/10
                 resize-none min-h-[48px] max-h-[120px]"
    />
    <Button
      size="icon"
      className="absolute right-2 bottom-2 bg-gradient-to-r from-[#ff6b35] to-[#f97316]"
    >
      <ArrowRight className="w-4 h-4" />
    </Button>
  </div>
  <p className="text-[10px] text-white/25 mt-2 text-center">
    Enter to send, Shift+Enter for new line
  </p>
</div>
```

---

## Proposals Panel

**Panel Container:**
```css
.proposals-panel {
  flex: 1;
  min-width: 320px;
  display: flex;
  flex-direction: column;
  background: transparent;
}
```

**Panel Header:**
```tsx
<div className="flex items-center justify-between px-4 py-3 border-b border-white/[0.06]">
  <div className="flex items-center gap-3">
    <div className="p-2 rounded-lg bg-gradient-to-br from-white/[0.08] to-white/[0.02]">
      <Layers className="w-4 h-4 text-white/70" />
    </div>
    <div>
      <h2 className="text-sm font-semibold text-white tracking-tight">Proposals</h2>
      <p className="text-xs text-white/40">{count} generated</p>
    </div>
  </div>
  {/* Toolbar buttons */}
</div>
```

### Proposal Card

Premium cards with priority-based gradient stripes:

```tsx
const PRIORITY_CONFIG = {
  critical: {
    gradient: "linear-gradient(135deg, #ef4444 0%, #dc2626 100%)",
    glow: "shadow-[0_0_12px_rgba(239,68,68,0.1)]",
    label: "Critical"
  },
  high: {
    gradient: "linear-gradient(135deg, #ff6b35 0%, #f97316 100%)",
    glow: "shadow-[0_0_12px_rgba(255,107,53,0.1)]",
    label: "High"
  },
  medium: {
    gradient: "linear-gradient(180deg, #666 0%, #444 100%)",
    glow: "",
    label: "Medium"
  },
  low: {
    gradient: "linear-gradient(180deg, #444 0%, #333 100%)",
    glow: "",
    label: "Low"
  }
};
```

```css
.proposal-card {
  position: relative;
  background: linear-gradient(180deg, rgba(28,28,28,0.9) 0%, rgba(22,22,22,0.95) 100%);
  border: 1px solid rgba(255,255,255,0.06);
  border-radius: 12px;
  overflow: hidden;
  transition: all 200ms ease;
}

/* Priority stripe */
.proposal-card::before {
  content: '';
  position: absolute;
  left: 0;
  top: 0;
  bottom: 0;
  width: 3px;
  background: var(--priority-gradient);
}

.proposal-card:hover {
  border-color: rgba(255,255,255,0.1);
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(0,0,0,0.3);
}

.proposal-card.selected {
  background: linear-gradient(135deg, rgba(255,107,53,0.08) 0%, rgba(255,107,53,0.04) 100%);
  border-color: rgba(255,107,53,0.3);
}
```

**Card Content:**
- Checkbox: shadcn Checkbox with accent color
- Title: `text-sm font-medium text-white/90`
- Description: `text-xs text-white/50` (2-line clamp)
- Priority badge: colored with gradient background
- Category badge: neutral styling

### Proposals Empty State

```tsx
<div className="flex-1 flex flex-col items-center justify-center p-8 text-center">
  <div className="relative mb-6">
    <div className="absolute inset-0 blur-2xl bg-[#ff6b35]/15 rounded-full scale-150" />
    <div className="relative p-4 rounded-2xl bg-gradient-to-br from-white/[0.08] to-white/[0.02] border border-white/[0.06]">
      <Lightbulb className="w-8 h-8 text-[#ff6b35]/70" />
    </div>
  </div>
  <h3 className="text-lg font-semibold text-white/90 mb-2 tracking-tight">
    No proposals yet
  </h3>
  <p className="text-sm text-white/40 max-w-[260px] leading-relaxed">
    Start chatting to generate task proposals from your ideas
  </p>
</div>
```

### Apply Section

Fixed at bottom of proposals panel:

```tsx
<div className="p-4 border-t border-white/[0.06] bg-gradient-to-t from-[#0a0a0a]/80 to-transparent backdrop-blur-sm">
  <div className="flex items-center justify-between">
    <span className="text-sm text-white/50">
      {selectedCount} selected
    </span>
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          disabled={!canApply}
          className="bg-gradient-to-r from-[#ff6b35] to-[#f97316]"
        >
          Apply to
          <ChevronDown className="w-4 h-4 ml-1" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        <DropdownMenuItem>Draft</DropdownMenuItem>
        <DropdownMenuItem>Backlog</DropdownMenuItem>
        <DropdownMenuItem>Todo</DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  </div>
</div>
```

---

## Animations

**Entry Animation:**
```css
@keyframes fadeSlideIn {
  from {
    opacity: 0;
    transform: translateY(8px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.session-card-enter {
  animation: fadeSlideIn 0.3s ease-out forwards;
}
```

**Glow Pulse (Selected State):**
```css
@keyframes glowPulse {
  0%, 100% {
    box-shadow: 0 0 12px rgba(255,107,53,0.08), 0 0 24px rgba(255,107,53,0.04);
  }
  50% {
    box-shadow: 0 0 18px rgba(255,107,53,0.15), 0 0 36px rgba(255,107,53,0.08);
  }
}

.active-session-glow {
  animation: glowPulse 3s ease-in-out infinite;
}
```

---

## Component Hierarchy

```
IdeationView
├── SessionBrowser (always visible)
│   ├── Header (icon + title + count + new button)
│   ├── SessionList (scrollable)
│   │   └── SessionCard (×N, with glow on selected)
│   └── EmptyState (if no sessions)
├── StartSessionPanel (when no session selected)
│   ├── GlowBackground
│   ├── Icon
│   ├── Title + Description
│   └── NewSessionButton
└── ActiveSessionView (when session selected)
    ├── ConversationPanel
    │   ├── PanelHeader
    │   ├── MessageList (scrollable)
    │   │   ├── MessageItem (user - gradient orange)
    │   │   ├── MessageItem (ai - glass effect)
    │   │   └── TypingIndicator
    │   ├── EmptyState
    │   └── ChatInput
    ├── ResizeHandle
    └── ProposalsPanel
        ├── PanelHeader + Toolbar
        ├── ProposalList (scrollable)
        │   └── ProposalCard (×N, with priority stripe)
        ├── EmptyState
        └── ApplySection
```

---

## Acceptance Criteria

- Session browser always visible on left
- Session cards have premium hover and selected states
- Selected session has subtle glow pulse animation
- Start session panel shows when no session selected
- Conversation and proposals panels have resizable divider
- User messages have warm orange gradient background
- AI messages have glass effect with blur
- Message bubbles have asymmetric border radius
- Typing indicator shows animated bouncing dots
- Proposal cards have priority-colored left stripe
- Selected proposals have orange tint and border
- Empty states have decorative icon with glow
- All interactive elements have 200ms transitions
- Focus states use accent-colored glow ring

---

## Design Quality Checklist

- NO purple or blue gradients anywhere
- Background uses atmospheric gradient (warm top, cool bottom)
- Cards use gradient backgrounds, not flat colors
- Shadows are layered for depth
- Orange accent used only for: user messages, selection, primary buttons, icon accents
- Typography uses tight tracking (-0.02em) for headings
- All spacing follows 4px grid
- Glass effects use backdrop-blur where appropriate
- Hover states include transform and shadow changes
- Selected states have subtle glow animation
- Empty states are decorative with icon glow
- Scrollbars are custom-styled (thin, subtle)
