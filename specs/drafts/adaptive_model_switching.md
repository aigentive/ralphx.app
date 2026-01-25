# Adaptive Model Switching - Draft Idea

**Status:** Draft - Future feature consideration

---

## Problem Statement

Different tasks have different complexity levels. Using a single model for all tasks is suboptimal:
- **Opus** for simple tasks = unnecessary cost and latency
- **Sonnet** for complex architectural tasks = potential quality issues

Currently, model switching requires manual intervention between ralph.sh runs.

---

## Proposed Solution

**Adaptive Model Switching** - RalphX automatically selects the appropriate model based on task complexity signals.

### Complexity Signals

| Signal | Indicator | Example |
|--------|-----------|---------|
| **Cross-cutting changes** | Task touches multiple architectural layers | ExecutionChatService + ClaudeCodeClient |
| **Major refactoring** | Task description contains "refactor", removes/replaces existing code | Orchestrator service refactor |
| **Event-driven systems** | Task involves event flows, triggers, reactive patterns | ArtifactFlow proactive sync |
| **State machine changes** | Task modifies state transitions or handlers | TransitionHandler updates |
| **New service creation** | Task creates a new service with dependencies | TaskContextService |

### Task Metadata

Extend task JSON with complexity hints:

```json
{
  "category": "backend",
  "description": "Refactor orchestrator service for --resume and MCP delegation",
  "complexity": "high",
  "complexity_reason": "Major architectural refactor, cross-cutting concerns",
  "recommended_model": "opus",
  "steps": [...]
}
```

### Automatic Detection

Alternatively, analyze task description and steps for complexity indicators:

```typescript
function assessComplexity(task: Task): 'low' | 'medium' | 'high' {
  const indicators = {
    high: [
      /refactor.*service/i,
      /cross-cutting/i,
      /state.?machine/i,
      /event.?driven/i,
      /architectural/i,
      /multiple.*layers/i,
    ],
    medium: [
      /create.*service/i,
      /integrate/i,
      /modify.*client/i,
    ],
  };

  const text = `${task.description} ${task.steps.join(' ')}`;

  if (indicators.high.some(re => re.test(text))) return 'high';
  if (indicators.medium.some(re => re.test(text))) return 'medium';
  return 'low';
}
```

---

## Integration Points

### 1. Worker Agent Spawning

Update `ClaudeCodeClient` to accept model override:

```rust
pub async fn spawn_with_model(
    &self,
    agent: &str,
    task_id: &str,
    model: Option<ModelAlias>,  // sonnet, opus, haiku
) -> AgentHandle {
    let mut cmd = Command::new(&self.cli_path);

    if let Some(model) = model {
        cmd.args(["--model", model.as_str()]);
    }

    // ... rest of spawn logic
}
```

### 2. Task Entity Extension

Add complexity fields to Task:

```rust
pub struct Task {
    // ... existing fields
    pub complexity: Option<TaskComplexity>,
    pub recommended_model: Option<ModelAlias>,
}

pub enum TaskComplexity {
    Low,
    Medium,
    High,
}

pub enum ModelAlias {
    Sonnet,
    Opus,
    Haiku,
}
```

### 3. PRD Task Schema

Extend PRD task JSON schema:

```json
{
  "complexity": "high",
  "recommended_model": "opus",
  "model_switch_after": true  // Signal to pause after this task
}
```

### 4. Execution Strategy Options

| Strategy | Behavior |
|----------|----------|
| **Manual** | Stop at marked tasks, user switches model |
| **Automatic** | RalphX switches model per-task automatically |
| **Hybrid** | Auto-switch within phase, pause between phases |

---

## UI Considerations

### Settings Panel

Add to execution settings:
- Model switching strategy: Manual / Automatic / Hybrid
- Default model for low complexity: [dropdown]
- Default model for high complexity: [dropdown]
- Confirm before switching models: [checkbox]

### Task Card Indicators

Show complexity badge on task cards:
- 🟢 Low (Haiku/Sonnet)
- 🟡 Medium (Sonnet)
- 🔴 High (Opus)

### Execution Log

Log model switches in activity stream:
```
[10:30:15] Switching to Opus for high-complexity task: "Refactor orchestrator service"
[10:45:22] Task complete. Switching back to Sonnet.
```

---

## Cost Tracking

Track per-model usage for cost visibility:

```typescript
interface ModelUsage {
  model: string;
  inputTokens: number;
  outputTokens: number;
  taskCount: number;
  estimatedCost: number;
}
```

Display in project dashboard:
- Tokens by model
- Cost by model
- Avg tokens per task by complexity

---

## Implementation Phases

### Phase A: Manual Switching (Current)
- PRD tasks include `model_switch_after` flag
- ralph.sh stops at flagged tasks
- User manually sets `ANTHROPIC_MODEL` and restarts

### Phase B: Metadata Support
- Add complexity fields to Task entity
- PRD parser extracts complexity hints
- UI shows complexity indicators

### Phase C: Automatic Switching
- Worker spawner reads task complexity
- Passes `--model` flag based on recommendation
- Logs model switches to activity stream

### Phase D: Learning
- Track task outcomes by model used
- Suggest complexity adjustments based on results
- "This task type succeeds 95% with Sonnet, consider downgrading"

---

## Open Questions

1. **Granularity**: Switch per-task or per-task-group?
2. **Fallback**: What if Opus is unavailable/rate-limited?
3. **User override**: Can user force a specific model for a task?
4. **Cost limits**: Stop execution if cost exceeds threshold?
5. **Learning data**: How to collect and use success/failure signals?

---

## Related Documents

- `specs/phases/prd_phase_15_context_aware_chat.md` - First phase with model switch points
- `specs/phases/prd_phase_15b_task_execution_chat.md` - Contains model switch points
- `specs/phases/prd_phase_16_ideation_plan_artifacts.md` - Contains model switch points
- `ralph.sh` - Current loop script with manual model switching

---

## History

- 2026-01-26: Initial draft based on Phase 15-17 complexity analysis
