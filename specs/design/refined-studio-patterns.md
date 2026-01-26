# macOS Tahoe Liquid Glass Design System

The definitive reference for RalphX's design aesthetic, inspired by macOS Tahoe's Liquid Glass language. Clean, translucent surfaces with frosted glass effects and warm ambient lighting.

**Design Philosophy**: Flat elegance through translucency. No heavy gradients on components. Depth via backdrop-blur and subtle shadows. Warmth through ambient orange glow in backgrounds. Premium feel through restraint and precision.

---

## 1. Core Aesthetic Principles

### The Liquid Glass Look

| Principle | Description | Implementation |
|-----------|-------------|----------------|
| **Flat Translucency** | Surfaces are see-through, not gradient-heavy | `rgba(255,255,255,0.03-0.06)` backgrounds |
| **Frosted Glass** | Blur effects create depth without gradients | `backdrop-filter: blur(20px)` |
| **Subtle Borders** | Whisper-thin light borders define edges | `rgba(255,255,255,0.06-0.08)` |
| **Single Shadows** | One soft shadow, not layered | `0 1px 3px rgba(0,0,0,0.12)` |
| **Ambient Glow** | Warm orange atmosphere in backgrounds | Radial gradients at corners |
| **Clean Typography** | SF Pro, tight tracking, clear hierarchy | 13px titles, 12px body |

### Anti-Patterns to Avoid

- Linear gradients on cards/buttons (use flat translucent instead)
- Multiple layered shadows (single subtle shadow only)
- Heavy glow effects (reserved for ambient background only)
- Purple/blue accents (warm orange #ff6b35 only)
- Inter font (SF Pro only)
- Dense, complex hover animations (subtle -translate-y-px)

---

## 2. Background Patterns

### Ambient Glow Background (Primary)

The signature look: flat base with soft orange radial glows at corners.

```css
.ambient-bg {
  background:
    /* Top-left warm glow */
    radial-gradient(ellipse 80% 50% at 20% 0%, rgba(255,107,53,0.06) 0%, transparent 50%),
    /* Bottom-right warm glow */
    radial-gradient(ellipse 60% 40% at 80% 100%, rgba(255,107,53,0.03) 0%, transparent 50%),
    /* Flat base */
    var(--bg-base);
}
```

### Frosted Glass Header

Header bars with translucent frosted effect.

```css
.glass-header {
  background: rgba(18,18,18,0.85);
  backdrop-filter: blur(24px);
  -webkit-backdrop-filter: blur(24px);
  border-bottom: 1px solid rgba(255,255,255,0.06);
  box-shadow: 0 1px 0 rgba(255,255,255,0.03);
}
```

### Panel Glass

Subtle frosted glass for panels and containers.

```css
.glass-panel {
  background: rgba(255,255,255,0.03);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  border: 1px solid rgba(255,255,255,0.06);
}
```

---

## 3. Card Patterns

### Liquid Glass Card (Standard)

Clean, flat card with subtle translucency.

```css
.glass-card {
  background: rgba(255,255,255,0.04);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  border: 1px solid rgba(255,255,255,0.08);
  border-radius: 8px;
  box-shadow: 0 1px 3px rgba(0,0,0,0.12);
  transition: all 180ms ease-out;
}

.glass-card:hover {
  transform: translateY(-1px);
  background: rgba(255,255,255,0.06);
  box-shadow: 0 4px 12px rgba(0,0,0,0.2);
}
```

### Selected Card (Flat Orange)

Selected state uses flat orange tint, no gradient.

```css
.glass-card.selected {
  background: rgba(255,107,53,0.08);
  border-color: rgba(255,107,53,0.25);
  box-shadow:
    0 0 0 1px rgba(255,107,53,0.15),
    0 2px 8px rgba(0,0,0,0.15);
}
```

### Dragging Card

Elevated state during drag.

```css
.glass-card.dragging {
  transform: scale(1.02);
  box-shadow: 0 12px 32px rgba(0,0,0,0.25);
  background: rgba(255,255,255,0.06);
  z-index: 50;
}
```

### Priority Stripe

Left border color based on priority (solid colors, no gradients).

```typescript
const PRIORITY_COLORS = {
  1: "#ef4444", // Critical - red-500
  2: "#f97316", // High - orange-500
  3: "#ff6b35", // Medium - accent
  4: "#525252", // Low - neutral-600
  5: "transparent", // None
};
```

---

## 4. Column Headers

### Liquid Glass Column Header

Frosted header for kanban columns.

```css
.column-header {
  background: rgba(255,255,255,0.03);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  border: 1px solid rgba(255,255,255,0.06);
  border-radius: 8px;
  padding: 8px 12px;
}
```

### Accent Dot

Small orange indicator dot for visual rhythm.

```tsx
<span className="w-1.5 h-1.5 rounded-full flex-shrink-0 bg-[#ff6b35]" />
```

---

## 5. Drop Zones

### Valid Drop Zone

Subtle dashed border when dragging over.

```css
.drop-zone.valid {
  border: 1px dashed rgba(255,107,53,0.4);
  background: rgba(255,107,53,0.03);
}
```

### Invalid Drop Zone

Red indicator for invalid drops.

```css
.drop-zone.invalid {
  border: 1px dashed rgba(239,68,68,0.4);
  background: rgba(239,68,68,0.03);
}
```

---

## 6. Button Patterns

### Primary Button (Flat Orange)

Solid orange, no gradient.

```css
.btn-primary {
  background: #ff6b35;
  color: white;
  font-weight: 500;
  padding: 8px 16px;
  border-radius: 8px;
  border: none;
  box-shadow: 0 1px 3px rgba(0,0,0,0.15);
  transition: all 180ms ease;
}

.btn-primary:hover {
  background: #ff7a4d;
  box-shadow: 0 2px 8px rgba(255,107,53,0.3);
}
```

### Ghost Button

Flat translucent with hover reveal.

```css
.btn-ghost {
  background: transparent;
  color: rgba(255,255,255,0.6);
  padding: 8px 16px;
  border-radius: 8px;
  border: none;
  transition: all 180ms ease;
}

.btn-ghost:hover {
  background: rgba(255,255,255,0.05);
  color: rgba(255,255,255,0.9);
}
```

### Active Nav Button

Flat orange tint for active state.

```css
.nav-btn.active {
  background: rgba(255,107,53,0.1);
  color: #ff6b35;
}
```

---

## 7. Input Patterns

### Glass Input

Translucent input field.

```css
.glass-input {
  background: rgba(0,0,0,0.3);
  border: 1px solid rgba(255,255,255,0.08);
  border-radius: 8px;
  padding: 10px 14px;
  color: white;
  font-size: 13px;
  transition: all 180ms ease;
}

.glass-input::placeholder {
  color: rgba(255,255,255,0.3);
}

.glass-input:focus {
  outline: none;
  border-color: rgba(255,107,53,0.5);
  box-shadow: 0 0 0 3px rgba(255,107,53,0.1);
}
```

---

## 8. Message Bubbles

### User Message

Solid orange bubble (flat, no gradient).

```css
.message-user {
  background: #ff6b35;
  color: white;
  border-radius: 16px 16px 4px 16px;
  padding: 10px 14px;
  max-width: 85%;
  align-self: flex-end;
  box-shadow: 0 1px 3px rgba(0,0,0,0.15);
}
```

### AI Message

Frosted glass bubble.

```css
.message-ai {
  background: rgba(255,255,255,0.04);
  backdrop-filter: blur(12px);
  border: 1px solid rgba(255,255,255,0.06);
  border-radius: 16px 16px 16px 4px;
  padding: 10px 14px;
  max-width: 85%;
  align-self: flex-start;
  box-shadow: 0 1px 3px rgba(0,0,0,0.1);
}
```

---

## 9. Empty States

### Minimal Empty State

Clean, subtle empty state without heavy glows.

```tsx
<div className="flex flex-col items-center justify-center py-12 text-center">
  <div
    className="w-10 h-10 rounded-xl flex items-center justify-center mb-4"
    style={{
      background: "rgba(255,255,255,0.03)",
      border: "1px solid rgba(255,255,255,0.06)",
    }}
  >
    <Icon className="w-5 h-5 text-white/25" />
  </div>
  <h3 className="text-sm font-medium text-white/60 mb-1">No items</h3>
  <p className="text-xs text-white/35 max-w-[200px]">
    Description text here
  </p>
</div>
```

---

## 10. Badge Patterns

### Count Badge

Small pill for counts.

```css
.count-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 18px;
  height: 18px;
  padding: 0 5px;
  background: rgba(255,255,255,0.03);
  border: 1px solid rgba(255,255,255,0.06);
  border-radius: 9px;
  font-size: 10px;
  font-weight: 500;
  color: rgba(255,255,255,0.4);
}
```

### Category Badge

Tag-style badge for categories.

```css
.category-badge {
  display: inline-flex;
  padding: 2px 6px;
  background: rgba(255,255,255,0.05);
  border: 1px solid rgba(255,255,255,0.1);
  border-radius: 4px;
  font-size: 10px;
  font-weight: 500;
  color: rgba(255,255,255,0.6);
}
```

---

## 11. Animation Patterns

### Card Pop-In

Subtle entry animation for cards.

```css
@keyframes card-pop-in {
  from {
    opacity: 0;
    transform: scale(0.95);
  }
  to {
    opacity: 1;
    transform: scale(1);
  }
}

.card-enter {
  animation: card-pop-in 150ms ease-out;
}
```

### Hover Lift

Standard hover elevation.

```css
.hover-lift:hover {
  transform: translateY(-1px);
}
```

### Loading Spinner

Uses Lucide Loader2 with spin animation.

```tsx
<Loader2 className="w-5 h-5 animate-spin" style={{ color: "var(--accent-primary)" }} />
```

---

## 12. Typography Scale

| Element | Size | Weight | Color | Tracking |
|---------|------|--------|-------|----------|
| Page title | text-xl (20px) | font-semibold | white/90 | tracking-tight |
| Section title | text-[13px] | font-medium | white/80 | tracking-tight |
| Card title | text-[13px] | font-medium | white/90 | tracking-tight |
| Body text | text-xs (12px) | font-normal | white/50 | normal |
| Badge text | text-[10px] | font-medium | white/40-60 | normal |
| Timestamp | text-[10px] | font-normal | white/35 | normal |

---

## 13. Compact Application Sizing

RalphX is a native application. All UI should be compact.

| Element | Size |
|---------|------|
| Main header | h-14 (56px) |
| Panel headers | 36-40px |
| Column width | 260-300px |
| Card padding | p-2.5 |
| Badge padding | px-1.5 py-0.5 |
| Icon (inline) | w-3.5 to w-4 |
| Icon (decorative) | w-5 |

---

## 14. Color Tokens

### Backgrounds
- `--bg-base`: hsl(0 0% 6%) - Main background
- `--bg-surface`: Translucent white (0.03-0.04)
- `--bg-elevated`: Translucent white (0.05-0.06)

### Borders
- Subtle: rgba(255,255,255,0.04)
- Default: rgba(255,255,255,0.06)
- Hover: rgba(255,255,255,0.08)
- Focus: rgba(255,107,53,0.5)

### Accent
- Primary: #ff6b35
- Primary hover: #ff7a4d
- Primary translucent: rgba(255,107,53,0.08-0.1)

### Text
- Primary: white/90
- Secondary: white/60
- Muted: white/40
- Disabled: white/25

---

## 15. Tailwind Patterns

### Backgrounds
```
bg-white/[0.03]
bg-white/[0.04]
bg-[rgba(255,107,53,0.08)]
```

### Glass Effects
```
backdrop-blur-xl
style={{ backdropFilter: "blur(20px)" }}
```

### Borders
```
border border-white/[0.06]
border-white/[0.08]
border-[rgba(255,107,53,0.25)]
```

### Shadows
```
shadow-[0_1px_3px_rgba(0,0,0,0.12)]
shadow-[0_4px_12px_rgba(0,0,0,0.2)]
```

### Hover States
```
hover:bg-white/[0.05]
hover:-translate-y-px
hover:shadow-[0_4px_12px_rgba(0,0,0,0.2)]
```

---

## 16. Z-Index Scale

| Layer | Z-Index | Usage |
|-------|---------|-------|
| Base content | 0 | Cards, lists |
| Sticky headers | 10 | Panel headers |
| Floating UI | 20 | Dropdowns, tooltips |
| Header | 50 | Fixed app header |
| Modals | 40 | Dialogs |
| Notifications | 50 | Toasts |
| Dragging | 50 | Drag overlays |

---

## References

- **Implementation Example**: `src/components/tasks/TaskBoard/TaskBoard.tsx`
- **Global CSS Tokens**: `src/styles/globals.css`
- **Main Design Guide**: `specs/DESIGN.md`
