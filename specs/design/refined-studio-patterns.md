# Refined Studio Design Patterns

The definitive reference for RalphX's "Refined Studio" aesthetic. These patterns create a luxurious, sophisticated dark interface with editorial typography and warm jewel accents.

**Design Philosophy**: Luxurious restraint. Every element breathes. Depth through subtle gradients and layered shadows. Warmth through carefully placed orange accents. Premium feel through glass effects and micro-animations.

---

## 1. Core Aesthetic Principles

### The Refined Studio Look

| Principle | Description | Implementation |
|-----------|-------------|----------------|
| **Sophisticated Depth** | Multiple subtle layers create dimensionality | Gradient backgrounds, layered shadows, glass panels |
| **Editorial Typography** | Clean, purposeful text hierarchy | SF Pro Display for titles, tight tracking, generous line-height |
| **Jewel Accents** | Warm orange used as precious highlight | 3-5% surface area, concentrated in interactive elements |
| **Atmospheric Backgrounds** | Rich, textured dark surfaces | Gradient meshes, subtle noise, blur effects |
| **Premium Motion** | Deliberate, refined animations | Subtle glow pulses, smooth fades, elegant transitions |

### Anti-Patterns to Avoid

- Generic flat dark backgrounds (`#0f0f0f` alone)
- Harsh borders without gradient softening
- Uniform shadow values (single box-shadow)
- Static hover states (just color change)
- Dense, cramped layouts
- Monotonous card grids

---

## 2. Background Patterns

### Atmospheric Gradient Background

The foundation layer that creates depth and warmth.

```css
.atmospheric-bg {
  background:
    /* Top warm glow */
    radial-gradient(ellipse 80% 50% at 20% 0%, rgba(255,107,53,0.04) 0%, transparent 50%),
    /* Bottom cool depth */
    radial-gradient(ellipse 60% 40% at 80% 100%, rgba(139,92,246,0.02) 0%, transparent 50%),
    /* Base gradient */
    linear-gradient(180deg, #141414 0%, #0a0a0a 100%);
}
```

### Panel Glass Effect

Premium glass panels with gradient borders.

```css
.glass-panel {
  background: linear-gradient(180deg, rgba(26,26,26,0.95) 0%, rgba(20,20,20,0.98) 100%);
  backdrop-filter: blur(20px);
  border: 1px solid transparent;
  border-image: linear-gradient(180deg, rgba(255,255,255,0.08) 0%, rgba(255,255,255,0.02) 100%) 1;
  box-shadow:
    0 4px 24px rgba(0,0,0,0.4),
    0 1px 2px rgba(0,0,0,0.2),
    inset 0 1px 0 rgba(255,255,255,0.03);
}
```

### Sidebar/Session Browser Background

Deep, layered background for sidebar areas.

```css
.sidebar-bg {
  background:
    linear-gradient(180deg, rgba(18,18,18,0.98) 0%, rgba(12,12,12,1) 100%);
  border-right: 1px solid rgba(255,255,255,0.04);
}
```

---

## 3. Card Patterns

### Premium Session/Item Card

Interactive cards with sophisticated hover states.

```css
.premium-card {
  background: linear-gradient(180deg, rgba(28,28,28,0.9) 0%, rgba(22,22,22,0.95) 100%);
  border: 1px solid rgba(255,255,255,0.06);
  border-radius: 12px;
  padding: 14px;
  transition: all 200ms cubic-bezier(0.4, 0, 0.2, 1);
  box-shadow: 0 2px 8px rgba(0,0,0,0.2);
}

.premium-card:hover {
  background: linear-gradient(180deg, rgba(32,32,32,0.95) 0%, rgba(26,26,26,0.98) 100%);
  border-color: rgba(255,255,255,0.1);
  transform: translateY(-1px);
  box-shadow:
    0 4px 12px rgba(0,0,0,0.3),
    0 0 0 1px rgba(255,255,255,0.05);
}
```

### Selected Card with Glow

Active/selected state with subtle pulsing glow.

```css
.premium-card.selected {
  background: linear-gradient(135deg, rgba(255,107,53,0.08) 0%, rgba(255,107,53,0.04) 100%);
  border-color: rgba(255,107,53,0.25);
  animation: glowPulse 3s ease-in-out infinite;
}

@keyframes glowPulse {
  0%, 100% {
    box-shadow:
      0 0 12px rgba(255,107,53,0.08),
      0 0 24px rgba(255,107,53,0.04),
      inset 0 1px 0 rgba(255,255,255,0.05);
  }
  50% {
    box-shadow:
      0 0 18px rgba(255,107,53,0.15),
      0 0 36px rgba(255,107,53,0.08),
      inset 0 1px 0 rgba(255,255,255,0.08);
  }
}
```

### Priority-Based Card Styling

Cards with gradient stripes based on priority.

```typescript
const PRIORITY_GRADIENTS = {
  critical: {
    gradient: "linear-gradient(135deg, #ef4444 0%, #dc2626 100%)",
    glow: "shadow-[0_0_12px_rgba(239,68,68,0.1)]"
  },
  high: {
    gradient: "linear-gradient(135deg, #ff6b35 0%, #f97316 100%)",
    glow: "shadow-[0_0_12px_rgba(255,107,53,0.1)]"
  },
  medium: {
    gradient: "linear-gradient(180deg, #666 0%, #444 100%)",
    glow: ""
  },
  low: {
    gradient: "linear-gradient(180deg, #444 0%, #333 100%)",
    glow: ""
  }
};
```

---

## 4. Message Bubble Patterns

### User Message (Right-Aligned)

Premium gradient bubble for user messages.

```css
.message-user {
  background: linear-gradient(135deg, #ff6b35 0%, #f97316 100%);
  color: white;
  border-radius: 16px 16px 4px 16px;
  padding: 12px 16px;
  max-width: 85%;
  align-self: flex-end;
  box-shadow:
    0 2px 8px rgba(255,107,53,0.25),
    0 1px 2px rgba(0,0,0,0.1);
}
```

### AI/Assistant Message (Left-Aligned)

Glass-effect bubble for AI responses.

```css
.message-ai {
  background: linear-gradient(180deg, rgba(38,38,38,0.95) 0%, rgba(32,32,32,0.98) 100%);
  border: 1px solid rgba(255,255,255,0.06);
  border-radius: 16px 16px 16px 4px;
  padding: 12px 16px;
  max-width: 85%;
  align-self: flex-start;
  box-shadow: 0 2px 8px rgba(0,0,0,0.15);
  backdrop-filter: blur(8px);
}
```

---

## 5. Header & Panel Headers

### Glass Header Bar

Sophisticated header with blur and gradient border.

```css
.glass-header {
  height: 48px;
  background: linear-gradient(180deg, rgba(20,20,20,0.95) 0%, rgba(16,16,16,0.98) 100%);
  backdrop-filter: blur(12px);
  border-bottom: 1px solid rgba(255,255,255,0.06);
  box-shadow: 0 1px 0 rgba(0,0,0,0.2);
}
```

### Section Header with Icon

Header pattern for panels and sections.

```tsx
<div className="flex items-center gap-3 px-4 py-3 border-b border-white/[0.06]">
  <div className="p-2 rounded-lg bg-gradient-to-br from-[#ff6b35]/20 to-[#ff6b35]/10">
    <Icon className="w-4 h-4 text-[#ff6b35]" />
  </div>
  <div>
    <h2 className="text-sm font-semibold text-white tracking-tight">Section Title</h2>
    <p className="text-xs text-white/50">Subtitle or count</p>
  </div>
</div>
```

---

## 6. Button Patterns

### Primary Action Button

Warm orange with subtle glow.

```css
.btn-primary {
  background: linear-gradient(180deg, #ff6b35 0%, #f97316 100%);
  color: white;
  font-weight: 500;
  padding: 10px 20px;
  border-radius: 8px;
  border: none;
  box-shadow:
    0 2px 8px rgba(255,107,53,0.3),
    inset 0 1px 0 rgba(255,255,255,0.1);
  transition: all 200ms ease;
}

.btn-primary:hover {
  background: linear-gradient(180deg, #ff7a4d 0%, #fb923c 100%);
  box-shadow:
    0 4px 12px rgba(255,107,53,0.4),
    inset 0 1px 0 rgba(255,255,255,0.15);
  transform: translateY(-1px);
}
```

### Ghost/Secondary Button

Subtle with hover reveal.

```css
.btn-ghost {
  background: transparent;
  color: rgba(255,255,255,0.6);
  padding: 8px 16px;
  border-radius: 8px;
  border: 1px solid transparent;
  transition: all 200ms ease;
}

.btn-ghost:hover {
  background: rgba(255,255,255,0.05);
  color: rgba(255,255,255,0.9);
  border-color: rgba(255,255,255,0.1);
}
```

---

## 7. Empty State Patterns

### Decorative Empty State

Premium empty state with icon glow and atmospheric text.

```tsx
<div className="flex flex-col items-center justify-center h-full text-center py-16">
  {/* Glowing icon container */}
  <div className="relative mb-6">
    <div className="absolute inset-0 blur-2xl bg-[#ff6b35]/20 rounded-full scale-150" />
    <div className="relative p-4 rounded-2xl bg-gradient-to-br from-white/[0.08] to-white/[0.02] border border-white/[0.06]">
      <Lightbulb className="w-8 h-8 text-[#ff6b35]/80" />
    </div>
  </div>

  {/* Title with gradient */}
  <h3 className="text-lg font-semibold text-white/90 mb-2 tracking-tight">
    No items yet
  </h3>

  {/* Atmospheric description */}
  <p className="text-sm text-white/40 max-w-[280px] leading-relaxed">
    Description text that explains what should appear here
  </p>
</div>
```

---

## 8. Animation Patterns

### Fade Slide In

Entry animation for cards and elements.

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

.animate-in {
  animation: fadeSlideIn 0.3s ease-out forwards;
}

/* Staggered children */
.stagger-children > *:nth-child(1) { animation-delay: 0.05s; }
.stagger-children > *:nth-child(2) { animation-delay: 0.1s; }
.stagger-children > *:nth-child(3) { animation-delay: 0.15s; }
/* ... etc */
```

### Shimmer Loading

Premium loading state with moving highlight.

```css
@keyframes shimmer {
  0% { background-position: -200% 0; }
  100% { background-position: 200% 0; }
}

.shimmer-loading {
  background: linear-gradient(
    90deg,
    rgba(255,255,255,0) 0%,
    rgba(255,255,255,0.05) 50%,
    rgba(255,255,255,0) 100%
  );
  background-size: 200% 100%;
  animation: shimmer 2s infinite;
}
```

### Typing Indicator

Bouncing dots for chat loading states.

```css
@keyframes typingBounce {
  0%, 60%, 100% { transform: translateY(0); }
  30% { transform: translateY(-4px); }
}

.typing-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: rgba(255,255,255,0.4);
  animation: typingBounce 1.4s ease-in-out infinite;
}

.typing-dot:nth-child(1) { animation-delay: 0s; }
.typing-dot:nth-child(2) { animation-delay: 0.15s; }
.typing-dot:nth-child(3) { animation-delay: 0.3s; }
```

---

## 9. Scrollbar Styling

Premium dark scrollbar that doesn't distract.

```css
.custom-scrollbar::-webkit-scrollbar {
  width: 6px;
}

.custom-scrollbar::-webkit-scrollbar-track {
  background: transparent;
}

.custom-scrollbar::-webkit-scrollbar-thumb {
  background: rgba(255,255,255,0.1);
  border-radius: 3px;
}

.custom-scrollbar::-webkit-scrollbar-thumb:hover {
  background: rgba(255,255,255,0.15);
}
```

---

## 10. Input Patterns

### Premium Text Input

```css
.premium-input {
  background: rgba(0,0,0,0.3);
  border: 1px solid rgba(255,255,255,0.08);
  border-radius: 10px;
  padding: 12px 16px;
  color: white;
  font-size: 14px;
  transition: all 200ms ease;
}

.premium-input::placeholder {
  color: rgba(255,255,255,0.3);
}

.premium-input:focus {
  outline: none;
  border-color: rgba(255,107,53,0.5);
  box-shadow:
    0 0 0 3px rgba(255,107,53,0.1),
    0 2px 8px rgba(0,0,0,0.2);
}
```

### Chat Input Container

```css
.chat-input-container {
  background: linear-gradient(180deg, rgba(20,20,20,0.9) 0%, rgba(16,16,16,0.95) 100%);
  border-top: 1px solid rgba(255,255,255,0.06);
  padding: 16px;
}
```

---

## 11. Badge Patterns

### Count Badge

Small pill for counts.

```css
.count-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 20px;
  height: 20px;
  padding: 0 6px;
  background: rgba(255,255,255,0.1);
  border-radius: 10px;
  font-size: 11px;
  font-weight: 500;
  color: rgba(255,255,255,0.7);
}
```

### Status Badge

For status indicators with semantic colors.

```css
.status-badge {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 4px 8px;
  border-radius: 6px;
  font-size: 11px;
  font-weight: 500;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.status-badge.active {
  background: rgba(16,185,129,0.15);
  color: #10b981;
}

.status-badge.pending {
  background: rgba(255,107,53,0.15);
  color: #ff6b35;
}

.status-badge.archived {
  background: rgba(255,255,255,0.08);
  color: rgba(255,255,255,0.5);
}
```

---

## 12. Tailwind Utility Classes

Common Tailwind patterns for the Refined Studio aesthetic:

### Backgrounds
```
bg-gradient-to-b from-[#1c1c1c] to-[#161616]
bg-gradient-to-br from-[#ff6b35]/10 to-[#ff6b35]/5
bg-[linear-gradient(180deg,rgba(26,26,26,0.95),rgba(20,20,20,0.98))]
```

### Borders
```
border border-white/[0.06]
border-white/[0.04]
border-[#ff6b35]/30
```

### Shadows
```
shadow-[0_2px_8px_rgba(0,0,0,0.2)]
shadow-[0_4px_24px_rgba(0,0,0,0.4)]
shadow-[0_0_12px_rgba(255,107,53,0.1)]
```

### Text
```
text-white/90
text-white/60
text-white/40
text-[#ff6b35]
```

### Hover States
```
hover:bg-white/[0.05]
hover:border-white/[0.1]
hover:text-white/90
hover:-translate-y-0.5
```

---

## 13. Z-Index Scale

Maintain consistent layering:

| Layer | Z-Index | Usage |
|-------|---------|-------|
| Base content | 0 | Cards, lists |
| Sticky headers | 10 | Panel headers |
| Floating UI | 20 | Dropdowns, tooltips |
| Sidebars | 30 | Session browser, chat panel |
| Modals | 40 | Dialogs |
| Notifications | 50 | Toasts, alerts |
| Dragging | 60 | Drag overlays |

---

## 14. Compact Application Sizing

RalphX is a **native application** (not a marketing site). All UI should be compact to maximize information density.

### Size Guidelines

| Element | Size | Notes |
|---------|------|-------|
| **Sidebar width** | 260px | Not 280-300px |
| **Panel headers** | h-9 (36px) | Not h-12+ |
| **Main header** | h-11 (44px) | Compact with all info |
| **Session cards** | p-2.5 | Not p-4 |
| **Message bubbles** | px-3 py-2 | Not px-4 py-3 |
| **Icons (inline)** | w-3 to w-3.5 | Small UI icons |
| **Icons (decorative)** | w-5 to w-6 | Empty states |
| **Avatars** | w-6 to w-7 | Not w-8+ |
| **Badges** | text-[9px] px-1.5 | Very compact |
| **Timestamps** | text-[10px] | Smallest text |
| **UI text** | text-xs (12px) | Most text |
| **Titles** | text-sm (14px) | Primary titles |

### Panel Widths
- Session sidebar: 260px min
- Main content panel: 320px min
- Chat panel: 320px min

### Breakpoints
- Desktop: 1280px+ (full layout)
- Tablet: 768-1279px (collapsible sidebar)
- Mobile: <768px (stacked layout)

---

## References

- **Implementation Example**: `src/components/Ideation/IdeationView.tsx`
- **Global CSS Tokens**: `src/styles/globals.css`
- **Main Design Guide**: `specs/DESIGN.md`
