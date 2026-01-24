# Settings View

The Settings View provides comprehensive configuration for project behavior. It should feel like a premium native Mac System Preferences panel with clear organization, refined form controls, and subtle depth cues.

**Reference Inspiration**: macOS System Preferences (sectioned organization), Linear (card-based settings), Raycast (refined toggles and inputs)

## Overall Layout

**Container:**
- Viewport-filling height (`calc(100vh - header)`)
- Scrollable content area with proper overscroll behavior
- Background: `--bg-surface` with subtle warm radial gradient
- Gradient: `radial-gradient(ellipse at top right, rgba(255,107,53,0.02) 0%, var(--bg-surface) 40%)`

**Header:**
- Glass effect background: `rgba(26,26,26,0.85)` + `backdrop-filter: blur(12px)`
- Height: 64px with vertical centering
- Title: "Settings" with Lucide `Settings` icon (20px)
- Title styling: `text-lg`, `font-semibold`, `--tracking-tight`
- Subtitle: "Configure project behavior" (`text-sm`, `--text-muted`)
- Saving indicator on right side (pulsing when active)
- Border bottom: `1px solid var(--border-subtle)`

```tsx
<div className="flex items-center justify-between px-6 py-4 backdrop-blur-md bg-[rgba(26,26,26,0.85)] border-b border-[var(--border-subtle)]">
  <div className="flex items-center gap-3">
    <div className="p-2 rounded-lg bg-[var(--accent-muted)]">
      <Settings className="w-5 h-5 text-[var(--accent-primary)]" />
    </div>
    <div>
      <h2 className="text-lg font-semibold tracking-tight text-[var(--text-primary)]">Settings</h2>
      <p className="text-sm text-[var(--text-muted)]">Configure project behavior</p>
    </div>
  </div>
  {isSaving && <SavingIndicator />}
</div>
```

**Content Area:**
- Padding: 24px (`--space-6`)
- Max width: 720px, centered
- Gap between sections: 24px (`--space-6`)
- Scrollable with shadcn ScrollArea

## Section Cards

**Card Structure:**
- Using shadcn Card component
- Background: `--bg-elevated`
- Border: `1px solid var(--border-default)` with gradient technique
- Border radius: 12px (`--radius-lg`)
- Shadow: `--shadow-xs` for subtle lift
- Padding: 20px (`--space-5`)

**Card Header:**
- Flex row with icon + title + description
- Icon container: 36px × 36px, rounded-lg, `--accent-muted` background
- Icon: Lucide icon (18px), `--accent-primary` color
- Title: `text-sm`, `font-semibold`, `--text-primary`, `--tracking-tight`
- Description: `text-xs`, `--text-muted`, margin-top 2px
- Separator below header: shadcn Separator, margin 16px vertical

```tsx
<Card className="bg-[var(--bg-elevated)] border-[var(--border-default)] shadow-xs">
  <div className="flex items-start gap-3 p-5 pb-0">
    <div className="p-2 rounded-lg bg-[var(--accent-muted)] shrink-0">
      <Zap className="w-[18px] h-[18px] text-[var(--accent-primary)]" />
    </div>
    <div>
      <h3 className="text-sm font-semibold tracking-tight text-[var(--text-primary)]">Execution</h3>
      <p className="text-xs text-[var(--text-muted)] mt-0.5">Control task execution behavior</p>
    </div>
  </div>
  <Separator className="my-4" />
  <div className="px-5 pb-5 space-y-1">
    {/* Setting rows */}
  </div>
</Card>
```

**Gradient Border Technique:**
```css
.settings-card {
  border: 1px solid transparent;
  background:
    linear-gradient(var(--bg-elevated), var(--bg-elevated)) padding-box,
    linear-gradient(180deg, rgba(255,255,255,0.08) 0%, rgba(255,255,255,0.02) 100%) border-box;
}
```

## Section Icons

Each section has a distinctive Lucide icon:
- **Execution**: `Zap` (energy/speed)
- **Model**: `Brain` (AI/intelligence)
- **Review**: `FileSearch` (code review)
- **Supervisor**: `Shield` (protection/monitoring)

Icon styling:
- Size: 18px
- Color: `--accent-primary`
- Container: `--accent-muted` background with rounded-lg

## Setting Rows

**Row Layout:**
- Flex row with justify-between
- Vertical padding: 12px (creates 24px gap between rows due to overlap)
- Border bottom: `1px solid var(--border-subtle)` (except last row)
- Hover state: subtle background tint (`--bg-hover` at 30% opacity)

**Label Column (Left):**
- Label: `text-sm`, `font-medium`, `--text-primary`
- Description: `text-xs`, `--text-muted`, margin-top 2px
- Max width: 70% of row

**Control Column (Right):**
- Flex items center, right-aligned
- Gap: 8px for multi-element controls

```tsx
<div className="flex items-start justify-between py-3 border-b border-[var(--border-subtle)] last:border-0 hover:bg-[rgba(45,45,45,0.3)] -mx-2 px-2 rounded-md transition-colors">
  <div className="flex-1 min-w-0 pr-4">
    <Label htmlFor="setting-id" className="text-sm font-medium text-[var(--text-primary)]">
      Setting Label
    </Label>
    <p className="text-xs text-[var(--text-muted)] mt-0.5">
      Helpful description of what this setting does
    </p>
  </div>
  <div className="shrink-0">
    {/* Control component */}
  </div>
</div>
```

## Form Controls

**Toggle Switch (shadcn Switch):**
- Using shadcn Switch component
- Size: 44px × 24px (touch-friendly)
- Track off: `--bg-hover`
- Track on: `--accent-primary`
- Thumb: white with subtle shadow
- Animation: 200ms ease-smooth
- Focus: `--shadow-glow`
- Disabled: 50% opacity

```tsx
<Switch
  id="auto-commit"
  checked={value}
  onCheckedChange={onChange}
  disabled={disabled}
  className="data-[state=checked]:bg-[var(--accent-primary)]"
/>
```

**Number Input (shadcn Input):**
- Using shadcn Input with type="number"
- Width: 80px (compact)
- Text align: right
- Background: `--bg-surface`
- Border: `1px solid var(--border-default)`
- Focus: `--accent-primary` border + `--shadow-glow`
- Unit label: `text-xs`, `--text-muted`, right of input
- Spin buttons: hidden (use CSS)

```tsx
<div className="flex items-center gap-2">
  <Input
    type="number"
    id="max-concurrent"
    value={value}
    min={min}
    max={max}
    step={step}
    onChange={onChange}
    disabled={disabled}
    className="w-20 text-right [appearance:textfield] [&::-webkit-outer-spin-button]:appearance-none [&::-webkit-inner-spin-button]:appearance-none"
  />
  {unit && <span className="text-xs text-[var(--text-muted)]">{unit}</span>}
</div>
```

**Select Dropdown (shadcn Select):**
- Using shadcn Select component
- Width: 200px
- Background: `--bg-surface`
- Border: `1px solid var(--border-default)`
- Chevron: Lucide `ChevronDown` (16px)
- Focus: `--accent-primary` border + `--shadow-glow`
- Dropdown content: `--bg-elevated`, `--shadow-md`
- Selected item: `--accent-muted` background

```tsx
<Select value={value} onValueChange={onChange} disabled={disabled}>
  <SelectTrigger className="w-[200px]">
    <SelectValue placeholder="Select model" />
  </SelectTrigger>
  <SelectContent>
    {options.map((opt) => (
      <SelectItem key={opt.value} value={opt.value}>
        <div className="flex flex-col">
          <span>{opt.label}</span>
          <span className="text-xs text-[var(--text-muted)]">{opt.description}</span>
        </div>
      </SelectItem>
    ))}
  </SelectContent>
</Select>
```

## Section-Specific Details

**Execution Section:**
- Icon: `Zap` (18px, accent)
- Settings:
  1. Max Concurrent Tasks (number, 1-10)
  2. Auto Commit (toggle)
  3. Pause on Failure (toggle)
  4. Review Before Destructive (toggle)

**Model Section:**
- Icon: `Brain` (18px, accent)
- Settings:
  1. Default Model (select: Haiku, Sonnet, Opus)
  2. Allow Opus Upgrade (toggle)
- Model descriptions shown in select dropdown

**Review Section:**
- Icon: `FileSearch` (18px, accent)
- Master toggle: Enable AI Review
- Sub-settings (disabled when master is off):
  1. Auto Create Fix Tasks (toggle)
  2. Require Fix Approval (toggle)
  3. Require Human Review (toggle)
  4. Max Fix Attempts (number, 1-10)
- Visual indication of disabled state: 50% opacity, `not-allowed` cursor

**Supervisor Section:**
- Icon: `Shield` (18px, accent)
- Master toggle: Enable Supervisor
- Sub-settings (disabled when master is off):
  1. Loop Threshold (number, 2-10)
  2. Stuck Timeout (number, 60-1800, unit: "seconds")

## Conditional Disabling Pattern

When a master toggle is off, related sub-settings are disabled:
- Opacity: 50%
- Cursor: not-allowed
- Pointer events: none on control only (label still readable)
- Subtle visual grouping (indent or border-left accent)

```tsx
<div className={cn(
  "flex items-start justify-between py-3",
  isDisabled && "opacity-50"
)}>
  <div className="flex-1 min-w-0 pr-4 pl-4 border-l-2 border-[var(--border-subtle)]">
    {/* Indented to show dependency */}
    <Label>Sub-setting</Label>
    <p className="text-xs text-[var(--text-muted)]">Description</p>
  </div>
  <Switch disabled={isDisabled} />
</div>
```

## Saving Indicator

**Structure:**
- Small badge/chip, right side of header
- Pulsing animation when saving
- Text: "Saving..." with Lucide `Loader2` spinning icon

**Styling:**
- Background: `--bg-elevated`
- Text: `--text-muted`
- Border radius: full (pill)
- Padding: 4px 12px
- Icon: 14px, spin animation (1s linear infinite)

```tsx
<div className="flex items-center gap-2 px-3 py-1 rounded-full bg-[var(--bg-elevated)] text-[var(--text-muted)] text-sm">
  <Loader2 className="w-3.5 h-3.5 animate-spin" />
  <span>Saving...</span>
</div>
```

## Error State

**Error Banner:**
- Below header, full width minus padding
- Background: `rgba(239, 68, 68, 0.1)` (error red at 10%)
- Border: `1px solid rgba(239, 68, 68, 0.3)`
- Border radius: 8px
- Padding: 12px 16px
- Icon: Lucide `AlertCircle` (16px, `--status-error`)
- Text: `text-sm`, `--status-error`
- Dismiss button: Lucide `X` (ghost, right side)

```tsx
<div className="mx-6 mt-4 p-3 rounded-lg bg-[rgba(239,68,68,0.1)] border border-[rgba(239,68,68,0.3)] flex items-center gap-3">
  <AlertCircle className="w-4 h-4 text-[var(--status-error)] shrink-0" />
  <p className="text-sm text-[var(--status-error)] flex-1">{error}</p>
  <Button variant="ghost" size="icon" onClick={dismissError}>
    <X className="w-4 h-4" />
  </Button>
</div>
```

## Loading Skeleton

**Structure:**
- Uses shadcn Skeleton component
- Mimics actual layout structure
- 4 card placeholders with 3-4 rows each
- Animated pulse effect

```tsx
<div className="p-6 space-y-6 max-w-[720px] mx-auto">
  {[1, 2, 3, 4].map((i) => (
    <Card key={i} className="p-5">
      <div className="flex items-center gap-3 mb-4">
        <Skeleton className="w-9 h-9 rounded-lg" />
        <div className="space-y-2">
          <Skeleton className="h-4 w-24" />
          <Skeleton className="h-3 w-40" />
        </div>
      </div>
      <Separator className="my-4" />
      <div className="space-y-4">
        {[1, 2, 3].map((j) => (
          <div key={j} className="flex justify-between items-center">
            <div className="space-y-1">
              <Skeleton className="h-4 w-32" />
              <Skeleton className="h-3 w-48" />
            </div>
            <Skeleton className="h-6 w-11 rounded-full" />
          </div>
        ))}
      </div>
    </Card>
  ))}
</div>
```

## Micro-interactions

**Card Hover (subtle):**
- Border lightens slightly on hover
- Transition: 150ms ease
- Not too pronounced (settings aren't buttons)

**Toggle Switch:**
- Thumb slides with spring easing
- Track color transition: 200ms
- Focus: glow ring

**Number Input:**
- Focus: glow ring + border color change
- Invalid value: shake animation (subtle)

**Select:**
- Dropdown opens with scale + opacity animation
- Items highlight on hover
- Selected item has accent background

**Row Hover:**
- Subtle background highlight
- Transition: 150ms

## Component Hierarchy

```
SettingsView
├── Header (glass effect)
│   ├── IconContainer + SettingsIcon
│   ├── Title + Subtitle
│   └── SavingIndicator (conditional)
├── ErrorBanner (conditional)
└── ScrollArea
    └── ContentContainer (max-w-[720px])
        ├── ExecutionSection (Card)
        │   ├── SectionHeader (icon + title)
        │   ├── Separator
        │   └── SettingRows
        │       ├── MaxConcurrentRow (Input)
        │       ├── AutoCommitRow (Switch)
        │       ├── PauseOnFailureRow (Switch)
        │       └── ReviewDestructiveRow (Switch)
        ├── ModelSection (Card)
        │   ├── SectionHeader
        │   ├── Separator
        │   └── SettingRows
        │       ├── ModelSelectRow (Select)
        │       └── OpusUpgradeRow (Switch)
        ├── ReviewSection (Card)
        │   ├── SectionHeader
        │   ├── Separator
        │   └── SettingRows
        │       ├── EnableReviewRow (Switch) ← master
        │       ├── AutoFixRow (Switch) ← sub
        │       ├── RequireApprovalRow (Switch) ← sub
        │       ├── RequireHumanRow (Switch) ← sub
        │       └── MaxAttemptsRow (Input) ← sub
        └── SupervisorSection (Card)
            ├── SectionHeader
            ├── Separator
            └── SettingRows
                ├── EnableSupervisorRow (Switch) ← master
                ├── LoopThresholdRow (Input) ← sub
                └── StuckTimeoutRow (Input) ← sub
```

## Acceptance Criteria

- Settings view fills available viewport height
- Header shows Settings title with Lucide Settings icon
- Four section cards display: Execution, Model, Review, Supervisor
- Each section card has distinctive Lucide icon in accent-colored container
- Toggle switches use shadcn Switch component with accent color when on
- Number inputs use shadcn Input with proper min/max constraints
- Model dropdown uses shadcn Select with model descriptions
- Disabling master toggle (AI Review, Supervisor) disables related sub-settings
- Disabled sub-settings show 50% opacity and not-allowed cursor
- Saving indicator appears in header when isSaving is true
- Error banner displays with error icon when error prop is set
- Loading skeleton mimics actual layout structure
- All interactive elements have visible focus states
- Settings changes trigger onSettingsChange callback
- Scroll area allows vertical scrolling when content overflows

## Design Quality Checklist

- NO purple or blue gradients anywhere
- Background uses subtle warm radial gradient (not flat)
- Section cards have layered shadows for depth
- Orange accent used sparingly - only for icons, toggle on-state, and focus rings
- Typography uses SF Pro with proper tracking (-0.02em for titles)
- All spacing follows 4px/8px grid
- Glass effect on header uses backdrop-blur
- Section icons are contained in accent-muted rounded containers
- Form controls are properly sized for touch (44px min touch target)
- Disabled states are clearly indicated but still readable
- Setting rows have subtle hover highlight
- Focus rings use --shadow-glow pattern
- Separator lines are subtle (--border-subtle)
- Max content width prevents overly wide lines (720px)
- Lucide icons used throughout (Settings, Zap, Brain, FileSearch, Shield, ChevronDown, Loader2, AlertCircle, X)
