# macOS Tahoe Style Guide

> **Reference:** Based on macOS Tahoe Finder (2025)
> **Purpose:** Definitive style guide for RalphX UI components

---

## Core Philosophy

macOS Tahoe is defined by **extreme minimalism**:

| Principle | Description |
|-----------|-------------|
| **Flat surfaces** | No gradients, no glass effects, no blur on content |
| **Color through subtraction** | Separate areas by color difference, not borders |
| **Invisible structure** | No visible borders, dividers, or separators |
| **Monochromatic base** | Blue-gray palette with colored icons only |
| **Quiet typography** | Small, understated, never demanding attention |

### What Tahoe Is NOT (for content areas)

- ❌ Gradients (no linear-gradient, radial-gradient on surfaces)
- ❌ Box shadows (no elevation, no depth simulation)
- ❌ Backdrop blur (no frosted glass on content areas)
- ❌ Glowing accents (no box-shadow color bleeds)
- ❌ Borders on containers (no 1px solid anywhere)
- ❌ Decorative elements (no ornaments, flourishes)

### Strategic Elevation (Flat by Default, Elevated When Meaningful)

RalphX uses a **hierarchical elevation system**:

| Layer | Treatment | Examples |
|-------|-----------|----------|
| **Page background** | Flat `hsl(220 10% 8%)` | Kanban canvas, main areas |
| **Content cards** | Subtle floating panel | Task cards, info cards, grouped content |
| **Floating chrome** | Full floating panel | Chat sidebar, control bars |
| **Completion celebration** | Glow (2 spots only) | Approved dot in timeline, completion banner icon |

**Key principle:** Elevation creates visual hierarchy. Don't elevate everything—reserve it for distinct items and important groupings.

### Content Cards (Subtle Elevation)

For grouped content like task cards, info panels, and data blocks:

```css
.content-card {
  border-radius: 8px;
  background: hsla(220 10% 14% / 0.85);
  backdrop-filter: blur(12px) saturate(150%);
  -webkit-backdrop-filter: blur(12px) saturate(150%);
  border: 1px solid hsla(220 10% 100% / 0.06);
  box-shadow: 0 2px 8px hsla(220 10% 0% / 0.25);
}
```

### Floating Panels (Full Elevation)

Floating panels (sidebars, toolbars, control bars) use:
- ✅ Flat semi-transparent background
- ✅ `backdrop-filter: blur(20px) saturate(180%)`
- ✅ Luminous border edge
- ✅ Layered shadows for depth

This creates the effect of panels floating OVER content.

**Where to use:** Chat sidebars, execution control bars, floating toolbars.

### Celebration Glows (EXTREMELY RARE)

Glows are reserved for **the final celebration moment only** - use sparingly to maintain impact.

**Allowed locations (exhaustive list):**
1. The **Approved status dot** in the state timeline breadcrumb
2. The **main icon** in the Task Completed status banner

That's it. Two spots per completed task view.

```css
/* Timeline approved dot - subtle glow */
.approved-dot {
  box-shadow: 0 0 8px hsla(145 60% 50% / 0.4);
}

/* Status banner completion icon - subtle glow */
.completion-banner-icon {
  box-shadow: 0 0 12px hsla(145 60% 50% / 0.35);
}
```

**NEVER use glows on:**
- Success badges (flat color only)
- Success pills or tags
- Timeline items (except the approved dot)
- Progress indicators
- Secondary success elements
- Any non-completion states

### Vibrant Status Indicators

Status dots in the timeline breadcrumb use **saturated colors** for quick scanning:

```css
/* Status dot colors - vibrant, ALL flat except approved in timeline */
--status-ready:      hsl(220 80% 60%);   /* Blue */
--status-executing:  hsl(14 100% 60%);   /* Orange accent */
--status-pending:    hsl(45 90% 55%);    /* Yellow */
--status-reviewing:  hsl(220 80% 60%);   /* Blue */
--status-approved:   hsl(145 60% 50%);   /* Green */
--status-escalated:  hsl(45 90% 55%);    /* Yellow */
--status-failed:     hsl(0 70% 55%);     /* Red */
```

**Note:** The approved dot ONLY gets a glow when it's the final state in the timeline breadcrumb. Approved badges elsewhere are flat.

**Full CSS pattern:**
```css
/* Outer container - provides padding for float effect */
.floating-outer {
  padding: 8px;
  background: transparent; /* or hsl(220 10% 8%) if needs base */
}

/* Inner floating container - FLAT, no gradient */
.floating-panel {
  border-radius: 10px;
  background: hsla(220 10% 10% / 0.92);  /* FLAT semi-transparent */
  backdrop-filter: blur(20px) saturate(180%);
  -webkit-backdrop-filter: blur(20px) saturate(180%);
  border: 1px solid hsla(220 20% 100% / 0.08);
  box-shadow:
    0 4px 16px hsla(220 20% 0% / 0.4),
    0 12px 32px hsla(220 20% 0% / 0.3);
}
```

---

## Color System

### Base Palette (Blue-Gray)

All grays use HSL with `220` hue and `10%` saturation for a subtle cool undertone.

```css
/* Background Layers (darkest to lightest) */
--tahoe-bg-deep:      hsl(220 10% 6%);    /* Deepest background */
--tahoe-bg-base:      hsl(220 10% 8%);    /* Main canvas */
--tahoe-bg-surface:   hsl(220 10% 12%);   /* Cards, elevated surfaces */
--tahoe-bg-raised:    hsl(220 10% 16%);   /* Hover states, raised elements */
--tahoe-bg-hover:     hsl(220 10% 20%);   /* Active hover */

/* Text Colors */
--tahoe-text-primary:   hsl(220 10% 90%);  /* Primary text, file names */
--tahoe-text-secondary: hsl(220 10% 60%);  /* Secondary info */
--tahoe-text-muted:     hsl(220 10% 45%);  /* Section headers, hints */
--tahoe-text-faint:     hsl(220 10% 35%);  /* Disabled, placeholder */

/* Selection (Blue) */
--tahoe-selection:      hsla(220 60% 50% / 0.20);  /* Selected item background */
--tahoe-selection-text: hsl(220 80% 70%);          /* Selected item text (optional) */

/* Focus Ring */
--tahoe-focus:          hsl(220 80% 60%);          /* Focus outline */
--tahoe-focus-ring:     hsla(220 80% 60% / 0.5);   /* Focus ring with transparency */
```

### Semantic Colors

Used sparingly for status indicators and icons only:

```css
/* Status - Only for badges/indicators, never backgrounds */
--tahoe-red:     hsl(0 70% 55%);
--tahoe-orange:  hsl(25 90% 55%);
--tahoe-yellow:  hsl(45 90% 55%);
--tahoe-green:   hsl(145 60% 45%);
--tahoe-blue:    hsl(220 80% 60%);
--tahoe-purple:  hsl(270 60% 60%);
--tahoe-pink:    hsl(330 70% 60%);

/* RalphX Accent (use VERY sparingly) */
--tahoe-accent:  hsl(14 100% 60%);  /* #ff6b35 - only for primary actions */
```

### Color Usage Rules

| Element | Color | Notes |
|---------|-------|-------|
| Page background | `--tahoe-bg-base` | Solid, no gradient |
| Sidebar | `--tahoe-bg-surface` | Slightly lighter than content |
| Cards/Items | `--tahoe-bg-surface` | Same as sidebar |
| Hover | `--tahoe-bg-raised` | Simple color change |
| Selection | `--tahoe-selection` | Blue tint overlay |
| Primary text | `--tahoe-text-primary` | File names, titles |
| Secondary text | `--tahoe-text-secondary` | Descriptions |
| Section headers | `--tahoe-text-muted` | Small, uppercase |

---

## Typography

### Font Stack

```css
--tahoe-font-ui: -apple-system, BlinkMacSystemFont, 'SF Pro Text', system-ui, sans-serif;
--tahoe-font-display: -apple-system, BlinkMacSystemFont, 'SF Pro Display', system-ui, sans-serif;
--tahoe-font-mono: 'SF Mono', 'JetBrains Mono', 'Menlo', monospace;
```

### Type Scale

| Token | Size | Weight | Use Case |
|-------|------|--------|----------|
| `--text-xxs` | 9px | 500 | Tiny badges |
| `--text-xs` | 10px | 500 | Metadata, counts |
| `--text-sm` | 11px | 500-600 | Section headers, labels |
| `--text-base` | 13px | 400-500 | Body text, file names |
| `--text-lg` | 15px | 500 | Titles, headers |
| `--text-xl` | 17px | 600 | Page titles |

### Typography Patterns

**Section Headers (like "Favourites", "Locations")**
```css
.section-header {
  font-size: 11px;
  font-weight: 600;
  color: hsl(220 10% 45%);
  text-transform: uppercase;
  letter-spacing: 0.02em;
}
```

**Group Headers (like "Previous 7 Days")**
```css
.group-header {
  font-size: 11px;
  font-weight: 500;
  color: hsl(220 10% 50%);
  /* NOT uppercase */
}
```

**Item Text (file names)**
```css
.item-text {
  font-size: 13px;
  font-weight: 400;
  color: hsl(220 10% 90%);
  line-height: 1.4;
}
```

**Secondary Text**
```css
.secondary-text {
  font-size: 12px;
  font-weight: 400;
  color: hsl(220 10% 55%);
}
```

---

## Spacing System

### Base Unit

4px base unit. All spacing should be multiples of 4.

```css
--space-0:   0;
--space-1:   4px;
--space-2:   8px;
--space-3:   12px;
--space-4:   16px;
--space-5:   20px;
--space-6:   24px;
--space-8:   32px;
```

### Component Spacing

| Component | Padding | Gap |
|-----------|---------|-----|
| Sidebar item | 8px 12px | - |
| Section header | 16px 12px 8px 12px | - |
| List items | - | 2px vertical |
| Card content | 10px 12px | 8px |
| Button | 6px 12px | 6px (icon-text) |

### Density

Tahoe is **compact**. Avoid excessive whitespace.

```
Tight:    2-4px  (between list items)
Normal:   8px    (within components)
Loose:    16px   (between sections)
```

---

## Components

### Sidebar Item

```css
.sidebar-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 12px;
  border-radius: 6px;
  color: hsl(220 10% 85%);
  font-size: 13px;
  font-weight: 400;
  cursor: pointer;

  /* NO border, NO shadow */
  background: transparent;
}

.sidebar-item:hover {
  background: hsl(220 10% 16%);
}

.sidebar-item.selected {
  background: hsla(220 60% 50% / 0.20);
  color: hsl(220 10% 95%);
}

.sidebar-item .icon {
  width: 16px;
  height: 16px;
  flex-shrink: 0;
}
```

### Section Header

```css
.section-header {
  padding: 16px 12px 8px 12px;
  font-size: 11px;
  font-weight: 600;
  color: hsl(220 10% 45%);
  text-transform: uppercase;
  letter-spacing: 0.02em;

  /* NO border-bottom, NO background */
}
```

### List Item (Content Area)

```css
.list-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 8px;
  border-radius: 4px;
  color: hsl(220 10% 88%);
  font-size: 13px;

  /* NO border */
  background: transparent;
}

.list-item:hover {
  background: hsl(220 10% 14%);
}

.list-item.selected {
  background: hsla(220 60% 50% / 0.18);
}
```

### Card

```css
.card {
  padding: 10px 12px;
  border-radius: 8px;
  background: hsl(220 10% 12%);

  /* NO border, NO shadow */
}

.card:hover {
  background: hsl(220 10% 14%);
}

.card.selected {
  background: hsla(220 60% 50% / 0.15);
}
```

### Button

**Primary (use sparingly)**
```css
.btn-primary {
  padding: 6px 14px;
  border-radius: 6px;
  background: hsl(14 100% 60%);  /* RalphX accent */
  color: white;
  font-size: 13px;
  font-weight: 500;

  /* NO shadow, NO gradient */
  border: none;
}

.btn-primary:hover {
  background: hsl(14 100% 55%);
}
```

**Secondary/Ghost**
```css
.btn-secondary {
  padding: 6px 12px;
  border-radius: 6px;
  background: transparent;
  color: hsl(220 10% 70%);
  font-size: 13px;
  font-weight: 500;

  border: none;
}

.btn-secondary:hover {
  background: hsl(220 10% 16%);
  color: hsl(220 10% 85%);
}
```

### Input Field

```css
.input {
  padding: 8px 12px;
  border-radius: 6px;
  background: hsl(220 10% 10%);
  color: hsl(220 10% 90%);
  font-size: 13px;

  /* Subtle border only */
  border: 1px solid hsl(220 10% 20%);
}

.input:focus {
  outline: none;
  border-color: hsl(220 80% 60%);
  box-shadow: 0 0 0 2px hsla(220 80% 60% / 0.25);
}

.input::placeholder {
  color: hsl(220 10% 40%);
}
```

### Badge / Tag

```css
.badge {
  display: inline-flex;
  align-items: center;
  padding: 2px 6px;
  border-radius: 4px;
  background: hsl(220 10% 18%);
  color: hsl(220 10% 60%);
  font-size: 10px;
  font-weight: 500;
}

/* Status variants - background is very subtle */
.badge.success {
  background: hsla(145 60% 45% / 0.15);
  color: hsl(145 60% 55%);
}

.badge.warning {
  background: hsla(45 90% 55% / 0.15);
  color: hsl(45 90% 55%);
}

.badge.error {
  background: hsla(0 70% 55% / 0.15);
  color: hsl(0 70% 60%);
}
```

### Scrollbar

```css
::-webkit-scrollbar {
  width: 8px;
  height: 8px;
}

::-webkit-scrollbar-track {
  background: transparent;
}

::-webkit-scrollbar-thumb {
  background: hsl(220 10% 25%);
  border-radius: 4px;
}

::-webkit-scrollbar-thumb:hover {
  background: hsl(220 10% 35%);
}
```

### Chat Message Bubbles

Messages use flat colors with uniform rounded corners. No gradients, no shadows.

```css
/* User message - accent color */
.message-user {
  padding: 8px 12px;
  border-radius: 12px;           /* Uniform, not asymmetric */
  background: hsl(14 100% 60%);  /* RalphX accent - FLAT */
  color: white;
  font-size: 13px;
  line-height: 1.6;              /* Relaxed for readability */

  /* NO gradient, NO shadow, NO border */
}

/* Assistant message - dark surface */
.message-assistant {
  padding: 8px 12px;
  border-radius: 12px;
  background: hsl(220 10% 14%);  /* FLAT surface color */
  color: hsl(220 10% 90%);
  font-size: 13px;
  line-height: 1.6;

  /* NO gradient, NO shadow, NO border */
}
```

**Typography consistency:** Both message types use identical `line-height: 1.6` (`leading-relaxed` in Tailwind).

### Inline Code (in messages)

```css
.inline-code {
  padding: 2px 6px;
  border-radius: 4px;
  background: hsl(220 10% 18%);
  color: hsl(220 10% 85%);
  font-family: var(--tahoe-font-mono);
  font-size: 12px;
}
```

### Code Blocks (in messages)

```css
.code-block {
  border-radius: 8px;
  background: hsl(220 10% 10%);  /* FLAT, no border */
  overflow-x: auto;
}

.code-block code {
  display: block;
  padding: 12px;
  font-family: var(--tahoe-font-mono);
  font-size: 12px;
  color: hsl(220 10% 80%);
  white-space: pre-wrap;
  word-break: break-all;
}

.code-block .language-label {
  position: absolute;
  top: 6px;
  left: 12px;
  font-size: 10px;
  text-transform: uppercase;
  letter-spacing: 0.02em;
  color: hsl(220 10% 45%);
}
```

### Markdown Tables (horizontal scroll)

Tables scroll horizontally to prevent text wrapping/breaking.

```css
.markdown-table-container {
  overflow-x: auto;
  margin: 12px 0;
  border-radius: 8px;
  background: hsl(220 10% 12%);
}

.markdown-table {
  font-size: 12px;
  border-collapse: collapse;
  min-width: max-content;  /* Prevents column shrinking */
}

.markdown-table thead {
  background: hsl(220 10% 16%);
}

.markdown-table th {
  padding: 8px 12px;
  text-align: left;
  font-size: 11px;
  font-weight: 500;
  text-transform: uppercase;
  letter-spacing: 0.02em;
  color: hsl(220 10% 55%);
  white-space: nowrap;  /* Prevent header wrapping */
}

.markdown-table td {
  padding: 8px 12px;
  color: hsl(220 10% 80%);
  white-space: nowrap;  /* Prevent cell wrapping */
}

.markdown-table tr {
  border-bottom: 1px solid hsla(220 10% 100% / 0.04);
}
```

### Dropdown Menu

```css
.dropdown-content {
  min-width: 200px;
  max-height: 400px;
  overflow-y: auto;
  border-radius: 12px;
  background: hsl(220 10% 14%);  /* FLAT surface */
  border: 1px solid hsla(220 10% 100% / 0.08);
  box-shadow: 0 8px 32px hsla(0 0% 0% / 0.4);
}

.dropdown-label {
  padding: 8px 12px;
  font-size: 11px;
  font-weight: 500;
  text-transform: uppercase;
  letter-spacing: 0.02em;
  color: hsl(220 10% 50%);
}

.dropdown-item {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 10px 12px;
  margin: 0 4px;
  border-radius: 8px;
  font-size: 13px;
  color: hsl(220 10% 75%);
  cursor: pointer;
}

.dropdown-item:hover {
  background: hsla(220 10% 100% / 0.04);
}

.dropdown-item.active {
  background: hsla(14 100% 60% / 0.15);
  color: hsl(220 10% 95%);
}

.dropdown-separator {
  height: 1px;
  margin: 4px 0;
  background: hsla(220 10% 100% / 0.06);
}
```

### Chat Input Field

```css
.chat-input-container {
  padding: 12px;
  border-top: 1px solid hsla(220 10% 100% / 0.04);
  background: hsla(220 15% 5% / 0.5);  /* Semi-transparent footer */
}

.chat-textarea {
  width: 100%;
  min-height: 40px;
  max-height: 120px;
  padding: 10px 12px;
  border-radius: 8px;
  background: hsl(220 10% 12%);  /* FLAT, no gradient */
  color: hsl(220 10% 90%);
  font-size: 13px;
  line-height: 1.5;
  resize: none;

  /* Remove all focus outlines */
  border: none;
  outline: none;
}

.chat-textarea::placeholder {
  color: hsl(220 10% 40%);
}

.chat-send-button {
  padding: 8px 16px;
  border-radius: 8px;
  background: hsl(14 100% 60%);  /* FLAT accent */
  color: white;
  font-size: 13px;
  font-weight: 500;

  border: none;
  box-shadow: none;  /* NO glow */
}

.chat-send-button:disabled {
  background: hsla(14 100% 60% / 0.3);
  cursor: not-allowed;
}

.chat-send-button:hover:not(:disabled) {
  background: hsl(14 100% 55%);
}
```

### Tool Call Indicator

```css
.tool-call {
  padding: 10px 12px;
  border-radius: 8px;
  background: hsl(220 10% 14%);  /* FLAT, no border */
}

.tool-call.error {
  background: hsla(0 70% 55% / 0.15);
}

.tool-call-header {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  font-weight: 500;
  color: hsl(220 10% 70%);
}

.tool-call-code {
  margin-top: 8px;
  padding: 8px;
  border-radius: 6px;
  background: hsl(220 10% 10%);
  font-family: var(--tahoe-font-mono);
  font-size: 11px;
  color: hsl(220 10% 80%);
  white-space: pre-wrap;
  word-break: break-all;
}
```

### Minimal Resize Handle

For split layouts, use a nearly invisible handle:

```css
.resize-handle {
  width: 1px;
  background: hsla(220 20% 100% / 0.04);
  cursor: ew-resize;
  transition: background 150ms ease;
}

.resize-handle:hover,
.resize-handle.active {
  background: hsla(220 20% 100% / 0.08);
}
```

---

## Interactions

### Hover States

Simple background color change. No transforms, no shadows.

```css
/* Standard hover */
.hoverable:hover {
  background: hsl(220 10% 16%);  /* Just slightly lighter */
}
```

### Selection States

Blue-tinted background. No borders, no glows.

```css
.selectable.selected {
  background: hsla(220 60% 50% / 0.20);
}
```

### Focus States

Visible ring for accessibility:

```css
.focusable:focus-visible {
  outline: none;
  box-shadow: 0 0 0 2px hsla(220 80% 60% / 0.5);
}
```

### Disabled States

Reduced opacity:

```css
.element:disabled {
  opacity: 0.5;
  pointer-events: none;
}
```

### Transitions

Subtle, fast transitions:

```css
.interactive {
  transition: background 150ms ease, color 150ms ease;
}
```

---

## Layout Patterns

### Sidebar + Content

```
┌─────────────────────────────────────────┐
│ Toolbar                                 │
├────────────┬────────────────────────────┤
│            │                            │
│  Sidebar   │     Content Area           │
│  (darker)  │     (same or lighter)      │
│            │                            │
│            │                            │
└────────────┴────────────────────────────┘
```

```css
.layout {
  display: flex;
  height: 100vh;
  background: hsl(220 10% 8%);
}

.sidebar {
  width: 240px;
  flex-shrink: 0;
  background: hsl(220 10% 10%);
  /* NO border-right */
}

.content {
  flex: 1;
  background: hsl(220 10% 8%);
}
```

### Column Layout (Kanban)

```css
.columns {
  display: flex;
  gap: 12px;
  padding: 16px;
  overflow-x: auto;
}

.column {
  width: 280px;
  flex-shrink: 0;
}

.column-header {
  padding: 8px;
  font-size: 11px;
  font-weight: 600;
  color: hsl(220 10% 50%);
  text-transform: uppercase;
  letter-spacing: 0.02em;
}

.column-content {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
```

### Sidebar Container Pattern (macOS Tahoe)

The macOS Tahoe Finder uses a **nested container** pattern for sidebars:

```
┌──────────────────────────────────────────────────────────────┐
│ Outer Container (darker: 6%)                                 │
│ ┌────────────────────────────────────────────────────────┐   │
│ │ Inner Rounded Container (lighter: 10%, rounded-xl)     │   │
│ │ ┌────────────────────────────────────────────────────┐ │   │
│ │ │ Header (darker: 8%, top corners rounded)           │ │   │
│ │ └────────────────────────────────────────────────────┘ │   │
│ │                                                        │   │
│ │   Content Area (inherits 10% from inner container)     │   │
│ │                                                        │   │
│ │ ┌────────────────────────────────────────────────────┐ │   │
│ │ │ Footer (darker: 8%, bottom corners rounded)        │ │   │
│ │ └────────────────────────────────────────────────────┘ │   │
│ └────────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────┘
```

```css
/* Outer container - TRANSPARENT, content flows behind */
.sidebar-outer {
  height: 100%;
  padding: 10px;
  background: transparent;  /* Content shows through */
}

/* Inner rounded container - FLAT with blur */
.sidebar-inner {
  height: 100%;
  display: flex;
  flex-direction: column;
  border-radius: 10px;
  overflow: hidden;

  /* FLAT semi-transparent (no gradient) */
  background: hsla(220 10% 10% / 0.92);
  backdrop-filter: blur(20px) saturate(180%);
  -webkit-backdrop-filter: blur(20px) saturate(180%);

  /* Luminous perimeter edge */
  border: 1px solid hsla(220 20% 100% / 0.08);
  box-shadow:
    0 4px 16px hsla(220 20% 0% / 0.4),
    0 12px 32px hsla(220 20% 0% / 0.3);
}

/* Header/Footer - semi-transparent with subtle separators */
.sidebar-header,
.sidebar-footer {
  background: hsla(220 15% 5% / 0.5);
}

.sidebar-header {
  height: 44px;
  padding: 0 12px;
  display: flex;
  align-items: center;
  border-bottom: 1px solid hsla(220 20% 100% / 0.04);
}

.sidebar-footer {
  padding: 12px;
  border-top: 1px solid hsla(220 20% 100% / 0.04);
}
```

### Key Tahoe Sidebar Characteristics

1. **Content extends BEHIND** - Main content doesn't stop at sidebar edge, it continues underneath
2. **Frosted effect** - `backdrop-filter: blur()` creates the translucent effect
3. **Flat background** - Single semi-transparent color, no gradient
4. **Luminous edge** - Visible light border around perimeter
5. **No hard separation** - Sidebar floats ON TOP of content

### Split Layout Structure

```css
/* Main content - extends full width, goes BEHIND sidebar */
.main-content {
  position: absolute;
  inset: 0;
  padding-right: var(--sidebar-width);  /* Makes room but content continues */
}

/* Sidebar - floats on top */
.sidebar {
  position: absolute;
  top: 0;
  bottom: 0;
  right: 0;
  width: var(--sidebar-width);
}
```

### Split Layout Structure

```
┌────────────────────────────────────────────────────────────────┐
│ Main Content (bg-base: 8%)  │ Resize │ Sidebar Outer (6%)      │
│                             │   │    │ ┌─────────────────────┐ │
│ Kanban board, lists, etc.   │   │    │ │ Inner (10%)         │ │
│                             │   │    │ │ ┌─────────────────┐ │ │
│                             │   │    │ │ │ Header (8%)     │ │ │
│                             │   │    │ │ ├─────────────────┤ │ │
│                             │   │    │ │ │ Content         │ │ │
│                             │   │    │ │ ├─────────────────┤ │ │
│                             │   │    │ │ │ Footer (8%)     │ │ │
│                             │   │    │ │ └─────────────────┘ │ │
│                             │   │    │ └─────────────────────┘ │
└────────────────────────────────────────────────────────────────┘
```

### Resize Handle (Split Layout)

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
  transform: translateX(-50%);
  width: 1px;
  background: hsl(220 10% 18%);
  transition: background 150ms ease;
}

/* Active state - subtle blue, NO glow */
.resize-handle.active::after {
  background: hsl(220 80% 60%);
}
```

### Key Principles for Sidebars

| Rule | Implementation |
|------|----------------|
| **Content behind** | Main content extends BEHIND sidebar, not stopping at edge |
| **Backdrop blur** | `backdrop-filter: blur(20px) saturate(180%)` |
| **Flat background** | Single semi-transparent color `hsla(220 10% 10% / 0.92)` |
| **Luminous edge** | `border: 1px solid hsla(220 20% 100% / 0.08)` |
| **Layered shadows** | Multiple shadows for depth and grounding |
| **10px radius** | Rounded corners on inner container |

---

## Icons

### Sizing

| Context | Size |
|---------|------|
| Inline with text | 14-16px |
| Sidebar items | 16px |
| Toolbar buttons | 18-20px |
| Empty states | 32-48px |

### Styling

```css
.icon {
  color: currentColor;  /* Inherit text color */
  flex-shrink: 0;
}

/* Muted icons */
.icon-muted {
  color: hsl(220 10% 45%);
}
```

### Colored Icons (Tags, Status)

Only icons should have semantic colors, not backgrounds:

```css
.icon-folder { color: hsl(220 80% 60%); }  /* Blue folder */
.icon-tag-red { color: hsl(0 70% 55%); }
.icon-tag-green { color: hsl(145 60% 50%); }
```

---

## Anti-Patterns (NEVER Do This)

### ❌ Gradients on Surfaces
```css
/* WRONG */
background: linear-gradient(180deg, rgba(255,255,255,0.05), transparent);

/* CORRECT */
background: hsl(220 10% 12%);
```

### ❌ Box Shadows for Depth
```css
/* WRONG */
box-shadow: 0 4px 12px rgba(0,0,0,0.2);

/* CORRECT - only for focus rings */
box-shadow: 0 0 0 2px hsla(220 80% 60% / 0.5);
```

### ❌ Backdrop Blur
```css
/* WRONG */
backdrop-filter: blur(20px);

/* CORRECT */
background: hsl(220 10% 12%);  /* Solid color */
```

### ❌ Decorative Borders
```css
/* WRONG */
border: 1px solid rgba(255,255,255,0.1);

/* CORRECT - separation through color */
background: hsl(220 10% 12%);  /* Container */
/* Parent has hsl(220 10% 8%) - the difference creates separation */
```

### ❌ Glowing Accents
```css
/* WRONG */
box-shadow: 0 0 20px rgba(255,107,53,0.3);

/* CORRECT */
border-left: 3px solid hsl(14 100% 60%);  /* Simple solid accent */
```

### ❌ Large Typography
```css
/* WRONG */
font-size: 18px;
font-weight: 700;

/* CORRECT - understated */
font-size: 11px;
font-weight: 600;
text-transform: uppercase;
color: hsl(220 10% 45%);
```

---

## Quick Reference

### Copy-Paste Colors

```css
/* Backgrounds */
hsl(220 10% 6%)   /* Deep background */
hsl(220 10% 8%)   /* Base background */
hsl(220 10% 12%)  /* Surface/card */
hsl(220 10% 16%)  /* Hover */
hsl(220 10% 20%)  /* Active */

/* Text */
hsl(220 10% 90%)  /* Primary */
hsl(220 10% 60%)  /* Secondary */
hsl(220 10% 45%)  /* Muted/headers */
hsl(220 10% 35%)  /* Disabled */

/* Selection */
hsla(220 60% 50% / 0.20)  /* Selected background */

/* Focus */
hsla(220 80% 60% / 0.5)   /* Focus ring */

/* Accent (RalphX) */
hsl(14 100% 60%)  /* Primary actions only */
```

### Copy-Paste Components

```jsx
// Section Header
<h3 style={{
  fontSize: '11px',
  fontWeight: 600,
  color: 'hsl(220 10% 45%)',
  textTransform: 'uppercase',
  letterSpacing: '0.02em',
}}>
  Section Name
</h3>

// Sidebar Item
<div style={{
  display: 'flex',
  alignItems: 'center',
  gap: '8px',
  padding: '6px 12px',
  borderRadius: '6px',
  fontSize: '13px',
  color: 'hsl(220 10% 85%)',
  background: selected ? 'hsla(220 60% 50% / 0.20)' : 'transparent',
}}>
  <Icon size={16} />
  <span>Item Name</span>
</div>

// Card
<div style={{
  padding: '10px 12px',
  borderRadius: '8px',
  background: 'hsl(220 10% 12%)',
}}>
  Content
</div>

// Floating Panel (FLAT, not gradient)
<div style={{ padding: '8px', background: 'transparent' }}>
  <div style={{
    borderRadius: '10px',
    background: 'hsla(220 10% 10% / 0.92)',  /* FLAT */
    backdropFilter: 'blur(20px) saturate(180%)',
    WebkitBackdropFilter: 'blur(20px) saturate(180%)',
    border: '1px solid hsla(220 20% 100% / 0.08)',
    boxShadow: '0 4px 16px hsla(220 20% 0% / 0.4), 0 12px 32px hsla(220 20% 0% / 0.3)',
  }}>
    Content
  </div>
</div>

// Chat Message (User)
<div style={{
  padding: '8px 12px',
  borderRadius: '12px',
  background: 'hsl(14 100% 60%)',
  color: 'white',
  fontSize: '13px',
  lineHeight: 1.6,
}}>
  Message text
</div>

// Chat Message (Assistant)
<div style={{
  padding: '8px 12px',
  borderRadius: '12px',
  background: 'hsl(220 10% 14%)',
  color: 'hsl(220 10% 90%)',
  fontSize: '13px',
  lineHeight: 1.6,
}}>
  Message text
</div>

// Dropdown Menu
<div style={{
  minWidth: '200px',
  maxHeight: '400px',
  overflowY: 'auto',
  borderRadius: '12px',
  background: 'hsl(220 10% 14%)',
  border: '1px solid hsla(220 10% 100% / 0.08)',
  boxShadow: '0 8px 32px hsla(0 0% 0% / 0.4)',
}}>
  <div style={{
    padding: '10px 12px',
    margin: '0 4px',
    borderRadius: '8px',
    fontSize: '13px',
    color: 'hsl(220 10% 75%)',
    background: isActive ? 'hsla(14 100% 60% / 0.15)' : 'transparent',
  }}>
    Menu Item
  </div>
</div>
```

---

## Checklist

Before shipping any UI, verify:

### General
- [ ] No gradients on backgrounds (exception: floating panels)
- [ ] No box-shadows (except focus rings and floating panels)
- [ ] No backdrop-filter/blur (exception: floating panels)
- [ ] No decorative borders
- [ ] Section headers are small, uppercase, gray
- [ ] Selection uses blue tint, not accent color
- [ ] Text colors use the blue-gray palette (hsl 220 10% xx%)
- [ ] Spacing uses 4px increments
- [ ] Hover is just a background color change
- [ ] Font sizes are compact (11-13px for most UI)

### Chat Components
- [ ] Message bubbles have uniform rounded corners (12px)
- [ ] User messages use flat accent color (hsl 14 100% 60%)
- [ ] Assistant messages use flat surface color (hsl 220 10% 14%)
- [ ] Both message types have identical line-height (1.6)
- [ ] Code blocks use flat dark background (hsl 220 10% 10%)
- [ ] Inline code uses subtle background (hsl 220 10% 18%)
- [ ] Tables scroll horizontally (no text wrapping)
- [ ] Chat input has no visible border or focus outline

### Floating Panels
- [ ] Outer container has padding (8px) for float effect
- [ ] Inner container uses flat semi-transparent background (no gradient)
- [ ] Backdrop blur: blur(20px) saturate(180%)
- [ ] Luminous border: hsla(220 20% 100% / 0.08)
- [ ] Layered shadows for depth
- [ ] 10px border-radius on inner container

### Dropdowns
- [ ] Flat background (hsl 220 10% 14%)
- [ ] Subtle border (hsla 220 10% 100% / 0.08)
- [ ] 12px border-radius
- [ ] Max height with overflow-y scroll
- [ ] Active item uses accent tint background
