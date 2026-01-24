---
name: acceptance-criteria-writing
description: Guide for writing testable acceptance criteria for QA
---

# Acceptance Criteria Writing

Guidelines for generating clear, testable acceptance criteria from task specifications.

## What Makes Good Acceptance Criteria

### SMART Criteria
- **Specific**: Describes exactly what should happen
- **Measurable**: Has a clear pass/fail condition
- **Achievable**: Can be implemented as specified
- **Relevant**: Directly relates to the task
- **Testable**: Can be verified with agent-browser

### Bad vs Good Examples

Bad: "The UI looks nice"
Good: "The task card displays title, status badge, and priority indicator"

Bad: "Performance is acceptable"
Good: "Task list loads within 2 seconds for 100 tasks"

Bad: "Drag and drop works"
Good: "Dragging a task to the Planned column triggers the planned animation"

## Criteria Types

### Visual (`type: "visual"`)
UI appearance and layout requirements.
- Element visibility
- Layout structure
- Color/styling (when specified)
- Responsive behavior

Examples:
- "Task board displays 7 columns"
- "Each column has a header with task count"
- "Task cards show status icon"

### Behavior (`type: "behavior"`)
User interaction outcomes.
- Click actions
- Form submissions
- Drag-drop operations
- Keyboard navigation

Examples:
- "Clicking a task opens the detail panel"
- "Submitting form creates new task"
- "Pressing Escape closes modal"

### Data (`type: "data"`)
Data display and accuracy.
- Correct values displayed
- Data persistence
- State synchronization

Examples:
- "Task title matches input value"
- "Task count updates after creation"
- "Status reflects backend state"

### Accessibility (`type: "accessibility"`)
A11y requirements.
- ARIA labels
- Focus management
- Screen reader support

Examples:
- "All interactive elements have aria-labels"
- "Focus moves to modal when opened"
- "Tab order follows visual layout"

## Output Format

```json
{
  "acceptance_criteria": [
    {
      "id": "AC1",
      "description": "Specific testable statement",
      "testable": true,
      "type": "visual"
    }
  ]
}
```

## ID Convention
- Use sequential IDs: AC1, AC2, AC3...
- Keep IDs stable within a task
- Reference IDs in test steps

## Common Patterns

### Component Renders
- "Component X is visible on page Y"
- "Component shows N child elements"

### User Action
- "Clicking X triggers Y"
- "Submitting form with valid data shows success"
- "Submitting form with invalid data shows error"

### State Change
- "After action X, element Y shows Z"
- "Status badge updates to reflect new state"

## Anti-Patterns to Avoid

- Vague descriptions ("works correctly")
- Implementation details ("uses useState")
- Multiple behaviors in one criterion
- Untestable conditions ("feels responsive")
- Criteria outside task scope
