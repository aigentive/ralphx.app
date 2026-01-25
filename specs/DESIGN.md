# RalphX Design System

The definitive design guide for RalphX. All UI work must follow these specifications.

---

## 1. Design Philosophy

### Premium 10x Designer Aesthetic

RalphX is a native Mac app that should feel like it was crafted by a world-class designer. Every pixel matters. Every interaction is intentional.

**What makes it feel premium:**
- **Intentionality** - Every element has a purpose; no visual noise
- **Restraint** - Strategic use of color, especially accent (5% rule)
- **Polish** - Micro-interactions, layered shadows, refined typography
- **Consistency** - Same patterns applied everywhere
- **Depth** - Not flat; surfaces have subtle elevation and dimensionality

### Reference Apps
- **Linear** - Board layout, card interactions, keyboard-first UX
- **Raycast** - Mac-native feel, glass effects, spacing discipline
- **Arc** - Spatial organization, bold typography, warm palette
- **Vercel Dashboard** - Typography scale, dark theme execution, status indicators

---

## 2. Anti-AI-Slop Guardrails

**NEVER use these patterns:**
- ❌ Purple or blue gradients (the #1 AI design cliché)
- ❌ Inter font (use SF Pro instead)
- ❌ Generic icon grids (4x4 squares of identical icons)
- ❌ High saturation colors on dark backgrounds
- ❌ Flat, lifeless surfaces without depth
- ❌ Rainbow gradients or excessive color variety
- ❌ Overly rounded corners (pill shapes everywhere)
- ❌ Generic stock-photo-style illustrations

**ALWAYS use these patterns:**
- ✅ Warm orange accent (#ff6b35) - distinctive and intentional
- ✅ Layered shadows for realistic depth
- ✅ Micro-interactions (hover lift, press scale)
- ✅ Typography refinement (letter-spacing, line-height)
- ✅ Strategic accent usage (5% of surface area max)
- ✅ Glass effects only for modals/overlays
- ✅ Subtle gradients (nearly imperceptible)

---

## 3. Color System

### Backgrounds
Dark grays, NOT pure black. Create depth through subtle variation.

| Token | Value | Usage |
|-------|-------|-------|
| `--bg-base` | `#0f0f0f` | Page background, main canvas |
| `--bg-surface` | `#1a1a1a` | Cards, panels, elevated containers |
| `--bg-elevated` | `#242424` | Modals, popovers, highest elevation |
| `--bg-hover` | `#2d2d2d` | Hover states on interactive elements |

### Text
Off-white, NOT pure white. Reduce eye strain with softer contrast.

| Token | Value | Usage |
|-------|-------|-------|
| `--text-primary` | `#f0f0f0` | Headings, primary content, labels |
| `--text-secondary` | `#a0a0a0` | Descriptions, secondary info, timestamps |
| `--text-muted` | `#666666` | Placeholders, disabled, tertiary info |

### Accent
Warm orange - the signature RalphX color. Use sparingly for maximum impact.

| Token | Value | Usage |
|-------|-------|-------|
| `--accent-primary` | `#ff6b35` | Primary actions, active states, focus rings |
| `--accent-secondary` | `#ffa94d` | Secondary highlights, hover accents |
| `--accent-hover` | `#ff8050` | Hover state for accent buttons |
| `--accent-muted` | `rgba(255, 107, 53, 0.15)` | Subtle tinted backgrounds |

### Status Colors
Semantic colors for feedback and state communication.

| Token | Value | Usage |
|-------|-------|-------|
| `--status-success` | `#10b981` | Success states, completed, passed |
| `--status-warning` | `#f59e0b` | Warnings, caution, pending review |
| `--status-error` | `#ef4444` | Errors, failures, destructive actions |
| `--status-info` | `#3b82f6` | Information (use sparingly - blue) |

### Borders
Subtle dividers and containers. Never harsh lines.

| Token | Value | Usage |
|-------|-------|-------|
| `--border-subtle` | `rgba(255, 255, 255, 0.06)` | Dividers, separators, subtle cards |
| `--border-default` | `rgba(255, 255, 255, 0.1)` | Card borders, input borders |
| `--border-focus` | `rgba(255, 107, 53, 0.5)` | Focus states, selected items |

### Usage Guidelines
1. **5% Rule**: Accent color should cover ~5% of any given screen
2. **One Primary Button**: Only one accent button per section/card
3. **Status Over Accent**: Use status colors for feedback, not accent
4. **Borders Are Optional**: Often shadow alone is enough

---

## 4. Typography

### Font Families
System fonts for performance and Mac-native feel.

| Token | Value | Usage |
|-------|-------|-------|
| `--font-display` | SF Pro Display, -apple-system, sans-serif | Headings, titles |
| `--font-body` | SF Pro Text, -apple-system, sans-serif | Body text, UI labels |
| `--font-mono` | JetBrains Mono, Menlo, monospace | Code, diffs, IDs |

### Type Scale

| Class | Size | Usage |
|-------|------|-------|
| `text-xs` | 12px (0.75rem) | Timestamps, badges, fine print |
| `text-sm` | 14px (0.875rem) | Secondary text, descriptions |
| `text-base` | 16px (1rem) | Body text, primary content |
| `text-lg` | 18px (1.125rem) | Section titles, card headers |
| `text-xl` | 20px (1.25rem) | Page section headings |
| `text-2xl` | 24px (1.5rem) | Page titles |
| `text-3xl` | 30px (1.875rem) | Hero text (rare) |

### Letter Spacing

| Token | Value | Usage |
|-------|-------|-------|
| `--tracking-tight` | -0.02em | Headings, titles (tighter = premium) |
| `--tracking-normal` | 0 | Body text |
| `--tracking-wide` | 0.05em | Uppercase labels, badges |

### Line Heights

| Token | Value | Usage |
|-------|-------|-------|
| `--leading-tight` | 1.2 | Headings, single-line labels |
| `--leading-normal` | 1.5 | Body text, descriptions |
| `--leading-relaxed` | 1.65 | Long-form content, readability |

### Font Weights

| Weight | Value | Usage |
|--------|-------|-------|
| Normal | 400 | Body text, descriptions |
| Medium | 500 | Labels, subtle emphasis |
| Semibold | 600 | Headings, titles, buttons |

---

## 5. Spacing System

### Base Unit
4px base unit, 8pt grid for component alignment.

### Spacing Tokens

| Token | Value | Usage |
|-------|-------|-------|
| `--space-1` | 4px | Micro gaps (icon-to-text, badge padding) |
| `--space-2` | 8px | Tight spacing (list items, inline elements) |
| `--space-3` | 12px | Component padding (buttons, inputs) |
| `--space-4` | 16px | Card padding, section gaps |
| `--space-5` | 20px | Medium sections |
| `--space-6` | 24px | Large gaps, column gutters |
| `--space-8` | 32px | Section separations |
| `--space-10` | 40px | Page margins (sides) |
| `--space-12` | 48px | Major section breaks |
| `--space-16` | 64px | Hero spacing, page headers |

### Usage Categories
- **Micro** (4-8px): Inside badges, between icons and text
- **Component** (12-16px): Padding inside buttons, inputs, cards
- **Section** (24-32px): Between related groups
- **Page** (48-64px): Page margins, major breaks

---

## 6. Shadow System (Layered)

Shadows provide depth and hierarchy. Use multiple layers for realistic elevation.

### Shadow Tokens

| Token | CSS | Usage |
|-------|-----|-------|
| `--shadow-xs` | `0 1px 2px rgba(0,0,0,0.2), 0 1px 3px rgba(0,0,0,0.1)` | Subtle card lift |
| `--shadow-sm` | `0 1px 2px rgba(0,0,0,0.3), 0 2px 4px rgba(0,0,0,0.2)` | Hover states, dropdowns |
| `--shadow-md` | `0 4px 6px rgba(0,0,0,0.3), 0 8px 16px rgba(0,0,0,0.2)` | Modals, floating panels |
| `--shadow-lg` | `0 10px 15px rgba(0,0,0,0.3), 0 20px 40px rgba(0,0,0,0.25)` | Large overlays |
| `--shadow-glow` | `0 0 0 2px var(--bg-base), 0 0 0 4px var(--accent-primary)` | Focus rings |

### Shadow Usage
- **Cards at rest**: `--shadow-xs` or no shadow (border-only)
- **Cards on hover**: `--shadow-sm` + translateY(-2px)
- **Dropdowns/Popovers**: `--shadow-sm` to `--shadow-md`
- **Modals**: `--shadow-md` to `--shadow-lg`
- **Focus states**: `--shadow-glow`

---

## 7. Border & Radius System

### Radius Tokens

| Token | Value | Usage |
|-------|-------|-------|
| `--radius-sm` | 4px | Small elements (badges, chips) |
| `--radius-md` | 8px | Buttons, inputs, cards |
| `--radius-lg` | 12px | Larger cards, modals |
| `--radius-xl` | 16px | Hero sections, large containers |
| `--radius-full` | 9999px | Pills, avatars, circular buttons |

### Gradient Border Technique
For premium cards with subtle depth:

```css
.premium-card {
  border: 1px solid transparent;
  background:
    linear-gradient(var(--bg-elevated), var(--bg-elevated)) padding-box,
    linear-gradient(180deg, rgba(255,255,255,0.08) 0%, rgba(255,255,255,0.02) 100%) border-box;
  border-radius: var(--radius-md);
}
```

### Border Patterns
- **Subtle dividers**: `1px solid var(--border-subtle)`
- **Card outlines**: `1px solid var(--border-default)`
- **Focus/Selected**: `2px solid var(--accent-primary)` or use `--shadow-glow`
- **No border + shadow**: Often cleaner than bordered cards

---

## 8. Component Patterns

### Buttons

**Variants:**
| Variant | Background | Text | Usage |
|---------|------------|------|-------|
| Primary | `--accent-primary` | white | Main actions (1 per section) |
| Secondary | `--bg-surface` | `--text-primary` | Secondary actions |
| Ghost | transparent | `--text-secondary` | Tertiary, navigation |
| Destructive | `--status-error` | white | Delete, dangerous actions |

**States:**
- Hover: lighten bg, subtle shadow
- Active: `scale(0.98)`, darken bg
- Focus: `--shadow-glow`
- Disabled: 50% opacity, no pointer events

**Sizing:**
- Small: 28px height, text-sm, px-3
- Default: 36px height, text-sm, px-4
- Large: 44px height, text-base, px-6

### Cards

**Variants:**
| Variant | Background | Border | Shadow |
|---------|------------|--------|--------|
| Flat | `--bg-surface` | none | none |
| Outlined | `--bg-surface` | `--border-default` | none |
| Raised | `--bg-surface` | none | `--shadow-xs` |
| Glass | rgba(26,26,26,0.85) + blur(20px) | gradient | `--shadow-md` |

**Card Padding:** 16px (--space-4) default, 12px for compact cards

### Inputs

**States:**
| State | Border | Shadow | Background |
|-------|--------|--------|------------|
| Default | `--border-default` | none | `--bg-surface` |
| Hover | `--border-default` | none | `--bg-hover` |
| Focus | `--accent-primary` | `--shadow-glow` | `--bg-surface` |
| Error | `--status-error` | error glow | `--bg-surface` |
| Disabled | `--border-subtle` | none | `--bg-base` + 50% opacity |

**Input Sizing:** 36px height default, 12px horizontal padding

### Badges

**Variants:**
| Variant | Background | Text | Usage |
|---------|------------|------|-------|
| Default | `--bg-surface` | `--text-secondary` | Neutral labels |
| Primary | `--accent-muted` | `--accent-primary` | Highlighted |
| Success | success-muted | `--status-success` | Passed, complete |
| Warning | warning-muted | `--status-warning` | Pending, caution |
| Error | error-muted | `--status-error` | Failed, blocked |

**Badge Sizing:** text-xs, px-2 py-0.5, rounded-md

### Modals

**Structure:**
```
Backdrop: rgba(0,0,0,0.6) + backdrop-blur(8px)
Content: --bg-elevated + --shadow-lg + --radius-lg
Animation: scale(0.95)→scale(1) + opacity(0)→opacity(1), 200ms
Close: Escape key + backdrop click (unless destructive)
```

**Sizes:**
- Small (max-w-sm): Simple confirmations
- Medium (max-w-md): Forms, details
- Large (max-w-lg): Complex content
- Full (max-w-2xl): Task detail, wizards

**Spacing:** 24px padding (--space-6)

---

## 9. Motion & Micro-interactions

### Timing Functions

| Name | Value | Usage |
|------|-------|-------|
| `ease-smooth` | `cubic-bezier(0.4, 0, 0.2, 1)` | Default for most transitions |
| `ease-out` | `cubic-bezier(0, 0, 0.2, 1)` | Enter animations |
| `ease-in` | `cubic-bezier(0.4, 0, 1, 1)` | Exit animations |
| `ease-spring` | `cubic-bezier(0.34, 1.56, 0.64, 1)` | Playful interactions |

### Durations

| Token | Value | Usage |
|-------|-------|-------|
| `--transition-fast` | 150ms | Hover states, micro-feedback |
| `--transition-normal` | 200ms | Most transitions |
| `--transition-slow` | 300ms | Complex animations, modals |

### Standard Interactions

**Hover Lift (Cards, Buttons):**
```css
.hoverable {
  transition: transform 150ms ease, box-shadow 150ms ease;
}
.hoverable:hover {
  transform: translateY(-2px);
  box-shadow: var(--shadow-sm);
}
```

**Active Press:**
```css
.pressable:active {
  transform: scale(0.98);
}
```

**Focus Ring:**
```css
.focusable:focus-visible {
  outline: none;
  box-shadow: 0 0 0 2px var(--bg-base), 0 0 0 4px var(--accent-primary);
}
```

**Drag State:**
```css
.dragging {
  transform: scale(1.02) rotate(2deg);
  box-shadow: var(--shadow-md);
  opacity: 0.9;
}
```

---

## 10. Icon Usage (Lucide)

### Size Guidelines

| Context | Size | Stroke | Usage |
|---------|------|--------|-------|
| Inline text | 16px | 1.5px | Badge icons, inline labels |
| Buttons | 18-20px | 2px | Button icons |
| Navigation | 20-24px | 2px | Nav items, prominent actions |
| Empty states | 48-64px | 1.5px | Illustration-style usage |

### Color
Icons inherit text color by default. Use `currentColor`.

### Common Icons by Category

**Status:**
- CheckCircle (success), XCircle (error), AlertCircle (warning)
- Clock (pending), Loader2 (loading - animated)

**Navigation:**
- LayoutGrid (kanban), Lightbulb (ideation), Settings
- Activity (activity stream), Puzzle (extensibility)

**Actions:**
- Plus (add), Trash2 (delete), Edit (edit), MoreHorizontal (menu)
- ChevronRight, ChevronDown (expandable)

**Task/Git:**
- GitBranch, GitCommit, GitMerge, GitPullRequest
- FolderOpen, File, FileCode

### Stroke Width Consistency
Always use stroke-width 1.5-2. Never use filled icons in Lucide.

---

## 11. Page-Specific Patterns

Each page has its own detailed design specification. See the individual files for complete requirements.

| Page | File | Status |
|------|------|--------|
| Kanban Board | [kanban-board.md](design/pages/kanban-board.md) | Complete |
| Ideation View | [ideation-view.md](design/pages/ideation-view.md) | Complete |
| Settings View | [settings-view.md](design/pages/settings-view.md) | Complete |
| Activity Stream | [activity-stream.md](design/pages/activity-stream.md) | Task 6 |
| Extensibility View | [extensibility-view.md](design/pages/extensibility-view.md) | Task 7 |
| Task Detail | [task-detail.md](design/pages/task-detail.md) | Complete |
| Reviews Panel | [reviews-panel.md](design/pages/reviews-panel.md) | Complete |
| Chat Panel (Global) | [chat-panel.md](design/pages/chat-panel.md) | Complete |
| QA Components | [qa-components.md](design/pages/qa-components.md) | Complete |
| Project Sidebar | [project-sidebar.md](design/pages/project-sidebar.md) | Complete |
| Project Dialogs | [project-dialogs.md](design/pages/project-dialogs.md) | Complete |
| Diff Viewer | [diff-viewer.md](design/pages/diff-viewer.md) | Task 14 |
| Execution Control Bar | [execution-control-bar.md](design/pages/execution-control-bar.md) | Task 15 |
| Header and Navigation | [header-navigation.md](design/pages/header-navigation.md) | Task 16 |
| Modal Standards | [modal-standards.md](design/pages/modal-standards.md) | Task 17 |

---

## 12. shadcn/ui Integration

RalphX uses shadcn/ui as the component foundation. All shadcn components are customized to use RalphX design tokens.

### Installed Components
- Button, Card, Dialog, Dropdown Menu
- Input, Label, Tabs, Tooltip, Popover
- Select, Checkbox, Switch, Badge
- Scroll Area, Separator, Skeleton

### CSS Variable Mapping
shadcn variables are mapped to RalphX tokens in `globals.css`:
- `--primary` → `--accent-primary`
- `--background` → `--bg-base`
- `--card` → `--bg-elevated`
- `--foreground` → `--text-primary`
- `--ring` → `--accent-primary`

### Component Location
All shadcn components live in `src/components/ui/`.

### Usage Pattern
```tsx
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader } from '@/components/ui/card';
import { Dialog, DialogContent, DialogTrigger } from '@/components/ui/dialog';
```

---

## 13. Accessibility

### Color Contrast
- Text on backgrounds: minimum 4.5:1 ratio
- Large text (18px+): minimum 3:1 ratio
- All colors tested against WCAG AA

### Focus States
- All interactive elements have visible focus
- Focus ring: 2px offset, accent color
- Never remove outline without replacement

### Keyboard Navigation
- Tab order follows visual order
- Escape closes modals/popovers
- Arrow keys for lists and menus
- Enter/Space activates buttons

### Screen Readers
- Semantic HTML elements
- ARIA labels where needed
- Icon-only buttons have aria-label

---

## References

- **Design Overhaul Plan**: `specs/DESIGN_OVERHAUL_PLAN.md`
- **Global CSS**: `src/styles/globals.css`
- **shadcn Components**: `src/components/ui/`
- **Lucide Icons**: https://lucide.dev/icons/
