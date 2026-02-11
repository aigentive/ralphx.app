# Task: Add keyboard navigation polish to PlanQuickSwitcherPalette

## Summary

This task adds keyboard navigation polish to the `PlanQuickSwitcherPalette` component, improving accessibility and user experience when navigating through plan candidates with the keyboard.

## Changes Made

### 1. Application Integration (Revision 2)

Integrated the PlanQuickSwitcherPalette component into the main application:

#### Keyboard Shortcut Handler (`src/hooks/useAppKeyboardShortcuts.ts`)
- Added `openPlanQuickSwitcher` prop to hook interface
- Implemented Cmd+Shift+P handler to open the quick switcher
- Added safety checks to prevent activation when typing in inputs

#### App Component (`src/App.tsx`)
- Added state management for quick switcher open/close (`isPlanQuickSwitcherOpen`)
- Created `handleOpenPlanQuickSwitcher` callback
- Integrated component into JSX (renders only when projects exist)
- Wired up keyboard shortcut to component state

#### Stub Store (`src/stores/planStore.ts`)
- Created minimal stub implementation to prevent runtime errors
- Exports types matching the implementation plan (PlanCandidate, TaskStats, etc.)
- Provides empty state and stub methods with console warnings
- Allows component to load gracefully until backend is implemented

#### Store Tests (`src/stores/planStore.test.ts`)
- Tests for stub store initialization
- Tests for all stub methods (loadActivePlan, setActivePlan, etc.)
- Verifies console warnings are shown

**Result:** Component can now be opened via Cmd+Shift+P. Shows "No accepted plans found" until backend APIs are implemented.

### 2. Component Enhancements (`src/components/plan/PlanQuickSwitcherPalette.tsx`)

#### Added Features:
- **Ref to highlighted list item** (line 44): Added `highlightedItemRef` to track the currently highlighted item
- **Auto-scroll behavior** (lines 87-95): Highlighted items automatically scroll into view using `scrollIntoView({ block: 'nearest', behavior: 'smooth' })`
- **Visual focus indicator** (line 218): Added orange ring (`ring-2 ring-[#ff6b35] ring-opacity-50`) to highlighted items for better accessibility
- **Home/End key support** (lines 143-149):
  - `Home` key jumps to first item
  - `End` key jumps to last item
- **Edge case handling** (lines 129-132): Prevents navigation errors when list is empty

#### Keyboard Shortcuts:
- `↓` / `↑` - Navigate down/up (with wrapping from last to first)
- `Home` - Jump to first item
- `End` - Jump to last item
- `Enter` - Select highlighted item
- `Escape` - Close palette

### 2. Test Suite (`src/components/plan/PlanQuickSwitcherPalette.test.tsx`)

Created comprehensive test suite covering:

#### Scroll Behavior Tests:
- Scrolls highlighted item into view when navigating down
- Scrolls highlighted item into view when navigating up
- Handles rapid keyboard navigation smoothly (10 consecutive presses)

#### Home/End Key Tests:
- Jumps to first item on `Home` press
- Jumps to last item on `End` press
- Scrolls to correct position with long lists (150+ items)

#### Edge Case Tests:
- Handles empty list gracefully (no errors on navigation keys)
- Wraps from last to first on `ArrowDown`
- Wraps from first to last on `ArrowUp`
- Resets highlighted index when search query changes

#### Focus Ring Tests:
- Shows orange focus ring on highlighted item
- Moves focus ring when navigating between items

#### Performance Tests:
- Renders 150 items in < 1000ms
- Navigation operations complete in < 100ms
- All 150 items are rendered and accessible

## Manual Testing Guide

When the full plan feature is merged, perform the following manual tests:

### Basic Navigation:
1. Press `Cmd+Shift+P` to open the quick switcher
2. Press `↓` multiple times - verify smooth scrolling and focus ring movement
3. Press `↑` multiple times - verify smooth scrolling in reverse
4. Press `Home` - should jump to first item with smooth scroll
5. Press `End` - should jump to last item with smooth scroll
6. Press `Escape` - should close the palette

### Edge Cases:
7. With an empty list (no plans), press navigation keys - should not error
8. At the last item, press `↓` - should wrap to first item
9. At the first item, press `↑` - should wrap to last item
10. Type in search box while navigating - should reset to first matching item

### Visual Verification:
11. Verify orange ring appears around highlighted item
12. Verify highlighted item is always visible in scroll area
13. Verify smooth animation when scrolling between distant items

### Performance Testing:
14. Create 100+ plans in the system
15. Open quick switcher - should render quickly
16. Navigate rapidly with keyboard - should be responsive
17. Use `Home` and `End` keys - should jump instantly with smooth scroll

## Integration Notes

This component is part of the larger "Global Active Plan + Non-Modal Plan Quick Switcher" feature (plan artifact ID: `cb81bf42-32e3-42ba-b68d-2a8e7b3a74fb`).

### Dependencies:
- `@/stores/planStore` - Zustand store for plan state
- `@/api/plan` - API layer for plan operations
- Backend Tauri commands for plan management

### Files Modified:
- `src/components/plan/PlanQuickSwitcherPalette.tsx` - Component implementation
- `src/components/plan/PlanQuickSwitcherPalette.test.tsx` - Test suite (new)
- `src/App.tsx` - Application integration
- `src/hooks/useAppKeyboardShortcuts.ts` - Keyboard shortcut handler
- `src/stores/planStore.ts` - Stub store implementation (new)
- `src/stores/planStore.test.ts` - Store tests (new)

## Design Decisions

1. **Smooth scrolling behavior**: Used `behavior: 'smooth'` for better UX, but can be changed to `'auto'` if performance issues arise
2. **Orange focus ring**: Uses design system accent color `#ff6b35` at 50% opacity for subtlety
3. **Wrapping navigation**: Following command palette UX patterns (VSCode, Slack)
4. **Home/End keys**: Standard pattern for jumping to list boundaries
5. **Empty list handling**: Graceful degradation - navigation keys do nothing rather than error

## Performance Considerations

- Component efficiently renders 150+ items
- `scrollIntoView` with `block: 'nearest'` minimizes unnecessary scrolling
- Ref updates only trigger for the highlighted item (not all items)
- Smooth behavior may be GPU-accelerated by browser

## Accessibility

- Focus ring provides clear visual indication of current selection
- All keyboard navigation is preventDefault'ed to avoid conflicts
- Works without mouse (keyboard-only navigation)
- Follows ARIA best practices for command palettes
