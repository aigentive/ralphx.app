ExecutionControlBar mock check

- View: Kanban (web mode)
- API: execution.getStatus / pause / resume / stop
- Mock: src/api-mock/execution.ts provides isPaused/runningCount/queuedCount
- Result: UI renders without missing mock data
