# Plan: Dependency Graph & Priority Assessment Integration

## Overview

Complete the dependency/priority system by:
1. Wiring up priority assessment commands to actually compute scores
2. Implementing AI-based dependency suggestions (following session-namer pattern)
3. Integrating dependency/priority indicators into the session view UI

---

## Part 1: Wire Up Priority Assessment Commands (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(ideation): wire up priority assessment commands`

**Problem:** `assess_proposal_priority` and `assess_all_priorities` commands are stubs that return stored values instead of computing.

### 1.1 Fix `assess_proposal_priority` Command
**File:** `src-tauri/src/commands/ideation_commands/ideation_commands_proposals.rs`

Current (stub):
```rust
// Just returns stored values, doesn't compute
```

Change to:
1. Get proposal by ID
2. Build dependency graph via `DependencyService::build_graph()`
3. Call `PriorityService::assess_priority(proposal, graph)`
4. Store result in database
5. Emit `proposal:priority_assessed` event
6. Return assessment

### 1.2 Fix `assess_all_priorities` Command
**File:** Same as above

1. Get all proposals for session
2. Call `PriorityService::assess_and_update_all_priorities(session_id)`
3. Emit `session:priorities_assessed` event
4. Return assessments

### 1.3 Add Event Emissions
**New events:**
- `proposal:priority_assessed` - single proposal
- `session:priorities_assessed` - batch
- `dependency:added` - when dependency created
- `dependency:removed` - when dependency removed

### 1.4 Frontend Event Handlers
**File:** `src/hooks/useIdeationEvents.ts`

Add listeners for new events to trigger TanStack Query invalidation.

---

## Part 2: AI-Based Dependency Suggestions (BLOCKING)
**Dependencies:** Part 3.1-3.3
**Atomic Commit:** `feat(ideation): add AI-based dependency suggestions`

**Approach:** Follow the session-namer pattern - spawn Claude agent, auto-apply results via MCP tool.

**Key decisions:**
- **Auto-accept all** - No review UI, dependencies applied automatically
- **Auto-trigger at 2+ proposals** - Runs on create/update/delete when count >= 2
- **Loading indicator** - UI shows when analysis is running
- **Toast notification** - Brief feedback on completion

### 2.1 Create Agent Definition
**File:** `ralphx-plugin/agents/dependency-suggester.md`

```yaml
---
name: dependency-suggester
description: Analyzes proposals and suggests dependencies based on semantic relationships
model: haiku
---

# Instructions
1. Analyze provided proposals (titles, descriptions, categories)
2. Identify logical dependencies:
   - Setup/config before features
   - Features before tests
   - Core before extensions
   - Keyword signals: "requires", "after", "before", "depends on", "prerequisite"
3. Call `apply_proposal_dependencies` tool with findings
4. Only suggest dependencies that don't already exist
```

### 2.2 Create MCP Tool Definition
**File:** `ralphx-plugin/ralphx-mcp-server/src/tools.ts`

```typescript
{
  name: "apply_proposal_dependencies",
  description: "Apply AI-suggested dependencies directly to proposals",
  inputSchema: {
    type: "object",
    properties: {
      session_id: { type: "string" },
      dependencies: {
        type: "array",
        items: {
          type: "object",
          properties: {
            proposal_id: { type: "string" },
            depends_on_id: { type: "string" },
            reason: { type: "string" }
          },
          required: ["proposal_id", "depends_on_id"]
        }
      }
    },
    required: ["session_id", "dependencies"]
  }
}
```

### 2.3 MCP Handler
**File:** `ralphx-plugin/ralphx-mcp-server/src/index.ts`

- Gate by `RALPHX_AGENT_TYPE=dependency-suggester`
- Forward to HTTP endpoint `/api/apply_dependency_suggestions`

### 2.4 Backend Command
**File:** `src-tauri/src/commands/ideation_commands/ideation_commands_dependencies.rs`

New command `apply_dependency_suggestions`:
1. **Clear all existing dependencies** for the session (fresh start each run)
2. For each suggestion, call `add_proposal_dependency` (skip if would create cycle)
3. Count successfully added
4. Emit `dependencies:suggestions_applied` event with count
5. Return applied count

**Note:** "Replace all" approach - user doesn't manage dependencies manually. Each auto-run provides a clean slate based on current proposal content. This keeps the system simple and consistent.

**Deprecate `add_proposal_dependency` (soft):**
- **Remove from TOOL_ALLOWLIST** for `orchestrator-ideation` (agent can't use it)
- **Keep tool definition** in tools.ts (don't delete, may re-enable later for manual flagging)
- **Update agent prompt** to remove references to manual dependency management
- This way the code remains for potential future "flag for review" feature

### 2.5 Spawn Command
**File:** `src-tauri/src/commands/ideation_commands/ideation_commands_session.rs`

New command `spawn_dependency_suggester(session_id)`:
1. Get all proposals for session (must be >= 2)
2. Get existing dependencies (so AI knows what exists)
3. Build prompt with proposal summaries + existing deps
4. Emit `dependencies:analysis_started` event
5. Spawn agent with `RALPHX_AGENT_TYPE=dependency-suggester`
6. Fire-and-forget (60s timeout)

### 2.6 Auto-Trigger Logic
**File:** `src-tauri/src/commands/ideation_commands/ideation_commands_proposals.rs`

After `create_task_proposal`, `update_task_proposal`, `remove_task_proposal`:
1. Count proposals in session
2. If count >= 2, spawn dependency suggester in background
3. Debounce to avoid rapid re-triggers (e.g., 2s delay)

### 2.7 Frontend Integration
**File:** `src/api/ideation.ts`

Add: `spawnDependencySuggester(sessionId)` for manual trigger

**File:** `src/hooks/useIdeationEvents.ts`

Add event listeners:
- `dependencies:analysis_started` → set loading state
- `dependencies:suggestions_applied` → clear loading, show toast, invalidate graph query

---

## Part 3: UI Integration

### 3.1-3.3 Dependency Badges & Critical Path (BLOCKING)
**Dependencies:** Part 1
**Atomic Commit:** `feat(ideation): add dependency badges and critical path indicators`

### 3.1 Dependency Badges on ProposalCard
**File:** `src/components/Ideation/ProposalCard.tsx`

Add props:
```typescript
interface ProposalCardProps {
  // existing...
  dependsOnCount?: number;
  blocksCount?: number;
  isOnCriticalPath?: boolean;
}
```

Display compact badges in the tags row:
```tsx
{(dependsOnCount > 0 || blocksCount > 0) && (
  <div className="flex gap-1.5 text-[9px]">
    {dependsOnCount > 0 && (
      <span className="text-[var(--text-muted)]" title={`Depends on ${dependsOnCount} proposal(s)`}>
        ←{dependsOnCount}
      </span>
    )}
    {blocksCount > 0 && (
      <span className="text-[#ff6b35]" title={`Blocks ${blocksCount} proposal(s)`}>
        →{blocksCount}
      </span>
    )}
  </div>
)}
```

### 3.2 Wire Dependency Counts to ProposalList
**File:** `src/components/Ideation/ProposalList.tsx`

1. Fetch dependency graph via `useDependencyGraph(sessionId)`
2. Build counts map from graph nodes (Map<proposalId, {in, out}>)
3. Pass counts to each ProposalCard

### 3.3 Critical Path Indicator
On cards that are on the critical path:
- Pass `isOnCriticalPath` prop based on `graph.criticalPath.includes(id)`
- Add subtle orange bottom border: `border-b-2 border-[#ff6b35]/40`
- Tooltip on priority badge: "On critical path"

### 3.4-3.5 Loading States & Manual Trigger (BLOCKING)
**Dependencies:** Part 2
**Atomic Commit:** `feat(ideation): add dependency analysis loading states`

### 3.4 Loading State During Analysis
**File:** `src/components/Ideation/ProposalList.tsx`

Add local state for analysis status:
```typescript
const [isAnalyzing, setIsAnalyzing] = useState(false);

// Listen for events
useEffect(() => {
  const unsub1 = listen('dependencies:analysis_started', () => setIsAnalyzing(true));
  const unsub2 = listen('dependencies:suggestions_applied', (e) => {
    setIsAnalyzing(false);
    toast.success(`${e.payload.count} dependencies added`);
  });
  return () => { unsub1(); unsub2(); };
}, []);
```

Show subtle indicator when `isAnalyzing`:
- Small spinner icon in header: "Analyzing dependencies..."
- Semi-transparent overlay on proposal cards (optional)

### 3.5 Manual Re-Trigger Button (Optional)
Small icon button in proposal list header:
- Network/link icon
- Tooltip: "Re-analyze dependencies"
- Calls `spawnDependencySuggester(sessionId)`
- Disabled while `isAnalyzing`

---

## Data Model

**No new tables needed** - Auto-accept means we use existing `proposal_dependencies` table directly.

Optional: Add `reason TEXT` column to `proposal_dependencies` to store AI explanation (nice for debugging, not required).

---

## File Changes Summary

### Backend (Rust)
| File | Changes |
|------|---------|
| `ideation_commands_proposals.rs` | Wire up assess commands to PriorityService, add auto-trigger logic |
| `ideation_commands_dependencies.rs` | Add `apply_dependency_suggestions` command |
| `ideation_commands_session.rs` | Add `spawn_dependency_suggester` command |
| `app_state.rs` | Add `analyzing_dependencies: HashSet<SessionId>` for state tracking |

### Plugin
| File | Changes |
|------|---------|
| `agents/dependency-suggester.md` | New agent definition |
| `ralphx-mcp-server/src/tools.ts` | Add `apply_proposal_dependencies` tool, remove `add_proposal_dependency` from orchestrator-ideation allowlist (keep definition) |
| `ralphx-mcp-server/src/index.ts` | Add handler + agent type gating |

### Frontend (React)
| File | Changes |
|------|---------|
| `src/api/ideation.ts` | Add `spawnDependencySuggester` API call |
| `src/hooks/useIdeationEvents.ts` | Add `analysis_started`, `suggestions_applied` listeners |
| `ProposalCard.tsx` | Add dependency count badges, critical path indicator |
| `ProposalList.tsx` | Wire counts from graph, add loading state, optional re-trigger button |

### Phase 37 Extension (Part 4)
| File | Changes |
|------|---------|
| `ralphx-mcp-server/src/tools.ts` | Add `analyze_session_dependencies` tool |
| `ralphx-mcp-server/src/index.ts` | Add GET dispatch for analysis |
| `src-tauri/http_server/handlers/ideation.rs` | Add handler using DependencyService |
| `src-tauri/http_server/mod.rs` | Add route for analysis endpoint |

### Agent Prompt Enhancement (Part 5)
| File | Changes |
|------|---------|
| `agents/orchestrator-ideation.md` | Add Proactive Behaviors section, new tool docs, examples |

---

## Verification Plan

1. **Priority Assessment**
   - Create proposals → call `assess_all_priorities` → verify scores computed (not just stored values)
   - Add dependency manually → re-assess → verify DependencyFactor increases
   - Check events emitted and UI updates

2. **AI Suggestions (Auto-Apply)**
   - Create 2 proposals → verify analysis auto-triggers
   - Check loading indicator shows
   - Verify dependencies auto-applied, toast appears
   - Add 3rd proposal → verify re-analysis triggers
   - Update existing proposal → verify re-analysis triggers

3. **UI Integration**
   - Verify count badges (←N →M) appear on cards with dependencies
   - Verify critical path cards have orange indicator
   - Test manual re-trigger button

4. **Chat Agent Integration (after Phase 37)**
   - Verify `analyze_session_dependencies` tool appears for orchestrator-ideation
   - Create proposals with dependencies → call analysis tool
   - Verify response includes critical path, cycles, node degrees, analysis_in_progress
   - Test chat agent can provide intelligent recommendations

5. **Proactive Agent Behavior**
   - Update plan → verify agent checks existing proposals
   - Create 3+ proposals → verify agent proactively analyzes dependencies
   - Create proposal → verify agent suggests next step (not just stops)
   - Test plan-proposal sync detection

---

---

## Part 4: Chat Agent Integration (Phase 37 Extension) (BLOCKING)
**Dependencies:** Part 3.4-3.5
**Atomic Commit:** `feat(mcp): add analyze_session_dependencies tool for chat agent`

**Context:** Phase 37 adds `list_session_proposals` and `get_proposal` tools. The `orchestrator-ideation` agent already has `add_proposal_dependency` in its allowlist. But the agent can't see graph analysis.

**Goal:** Let the chat agent provide intelligent dependency insights.

### 4.1 Add `analyze_session_dependencies` Tool
**File:** `ralphx-plugin/ralphx-mcp-server/src/tools.ts`

Add to TOOL_ALLOWLIST for `orchestrator-ideation`:
```typescript
{
  name: "analyze_session_dependencies",
  description: "Get full dependency graph analysis including critical path, cycle detection, and blocking relationships. Use to provide intelligent recommendations about proposal execution order.",
  inputSchema: {
    type: "object",
    properties: {
      session_id: { type: "string" }
    },
    required: ["session_id"]
  }
}
```

### 4.2 Analysis State Tracking
**Purpose:** Let the chat agent know if background analysis is in progress so it can retry or inform user.

**Backend State:**
- Add `analyzing_dependencies: HashSet<IdeationSessionId>` to `AppState`
- When `spawn_dependency_suggester` is called → add session_id to set
- When `apply_dependency_suggestions` completes → remove session_id from set
- When agent crashes/times out → cleanup via timeout mechanism

### 4.3 HTTP Handler for Dependency Analysis
**File:** `src-tauri/src/http_server/handlers/ideation.rs`

Add handler that calls `DependencyService::build_graph()` and returns:
```json
{
  "nodes": [
    { "id": "uuid-1", "title": "Setup DB", "in_degree": 0, "out_degree": 2, "is_root": true, "is_blocker": true }
  ],
  "edges": [
    { "from": "uuid-2", "to": "uuid-1" }
  ],
  "critical_path": ["uuid-1", "uuid-2", "uuid-3"],
  "critical_path_length": 3,
  "has_cycles": false,
  "cycles": null,
  "analysis_in_progress": true,
  "message": "Background analysis in progress. Results may update shortly.",
  "summary": {
    "total_proposals": 5,
    "root_count": 1,
    "leaf_count": 2,
    "max_depth": 3
  }
}
```

When `analysis_in_progress: true`, agent should:
1. Show partial results with caveat
2. Retry after 2-3 seconds for complete picture
3. Or inform user: "Dependencies are being analyzed..."

### 4.4 Chat Agent Usage Examples

With this tool, the agent can:

**Recommend execution order:**
> "Based on the dependency analysis, I recommend starting with 'Database Setup' - it's on the critical path and blocks 3 other proposals. After that, 'API Schema' and 'Auth Service' can be worked in parallel."

**Warn about cycles:**
> "I notice you asked to make 'Tests' depend on 'Deployment', but 'Deployment' already depends on 'Tests' through 'Integration'. This would create a cycle."

**Explain priority scores:**
> "The 'Core Service' proposal has high priority because it blocks 4 other proposals and sits on the critical path (path length: 5)."

### 4.5 MCP Dispatch
**File:** `ralphx-plugin/ralphx-mcp-server/src/index.ts`

Add GET dispatch:
```typescript
} else if (name === "analyze_session_dependencies") {
  const { session_id } = args as { session_id: string };
  result = await callTauriGet(`analyze_dependencies/${session_id}`);
}
```

---

## Part 5: Enhance Orchestrator-Ideation Agent Prompt
**Dependencies:** Part 4
**Atomic Commit:** `docs(plugin): enhance orchestrator-ideation agent with proactive behaviors`

**File:** `ralphx-plugin/agents/orchestrator-ideation.md`

**Goal:** Make the agent more proactive - don't wait for user to ask, anticipate needs.

### 5.1 Add Proactive Behaviors Section

Add new section after "Guidelines":

```markdown
## Proactive Behaviors

**Be anticipatory, not just responsive.** After completing an action, consider what comes next.

### After Plan Updates
When user updates a plan (or you update it):
1. Call `list_session_proposals` to check existing proposals
2. Compare proposal content against new plan version
3. If proposals seem misaligned:
   - Say: "I notice the plan has changed. Let me check if any proposals need updating..."
   - Suggest specific updates or removals
   - Offer to create new proposals for new plan sections

### After Creating Multiple Proposals
When session has 3+ proposals:
1. Call `analyze_session_dependencies` to see the dependency graph
2. Proactively share insights:
   - "Based on the dependencies, I recommend starting with [X] - it's on the critical path and unblocks [Y] and [Z]"
   - "I notice [A] and [B] could be worked in parallel since they have no shared dependencies"
3. If critical path is long, warn about bottlenecks

### After Each Major Action
Don't just stop - suggest the next step:
- After creating plan: "Ready to break this into tasks?"
- After creating proposals: "Want me to analyze the optimal execution order?"
- After linking proposals: "The proposals are linked. Shall I recalculate priorities based on the dependency graph?"

### Continuous Awareness
Periodically (every few exchanges in a long session):
- Check for stale data using `list_session_proposals`
- Mention if proposals have changed: "I see you've edited [X] in the UI..."
- Offer to re-analyze priorities if dependencies changed
```

### 5.2 Add New Tool Documentation

Add to "MCP Tools Available" section:

```markdown
### Query Tools (Phase 37)

#### list_session_proposals
List all proposals in the session with summary info.
```json
{
  "session_id": "session_abc123"
}
```
Returns: Array of proposals with id, title, category, priority, depends_on.

**Use proactively** to check what proposals exist before creating new ones.

#### get_proposal
Get full details of a specific proposal.
```json
{
  "proposal_id": "proposal_xyz"
}
```
Returns: Full proposal including steps, acceptance_criteria, description.

**Use proactively** when user mentions a proposal by name - verify it exists.

### Analysis Tools

#### analyze_session_dependencies
Get full dependency graph analysis with critical path and cycle detection.
```json
{
  "session_id": "session_abc123"
}
```
Returns: Nodes with degrees, edges, critical_path, has_cycles, analysis_in_progress.

**Use proactively** after adding dependencies to provide insights.

**If `analysis_in_progress: true`**: Wait 2-3 seconds and retry, or inform user analysis is still running.
```

### 5.3 Add Proactive Examples

Add new example section:

```markdown
### Example 5: Proactive Plan-Proposal Sync

**User**: Update the plan - change from WebSockets to SSE

**Assistant**:
*Calls update_plan_artifact with SSE architecture*

Done, I've updated the plan to use SSE instead of WebSockets.

*Calls list_session_proposals to check existing proposals*

I notice we have 4 proposals linked to the previous plan. Let me check if they need updating...

Looking at the proposals:
1. ~~"WebSocket server setup"~~ → Should become "SSE endpoint setup"
2. "OT engine implementation" → Still valid
3. "Connection status UI" → Needs update for SSE events

Want me to update these proposals to match the new architecture?

### Example 6: Proactive Dependency Analysis

**User**: (After creating 5 proposals) Looks good!

**Assistant**: Great! Let me analyze the dependency graph to give you the optimal execution order.

*Calls analyze_session_dependencies*

Here's what I found:
- **Critical path**: Auth Setup → API Endpoints → Dashboard (path length: 3)
- **Recommended first**: "Auth Setup" - it blocks 3 other tasks
- **Parallel opportunities**: "Docs" and "Tests" can be done alongside "API Endpoints"

If you complete Auth Setup first, you'll unblock the most work downstream. Want me to adjust priorities accordingly?

### Example 7: Proactive Continuation

**User**: Create a proposal for the login form

**Assistant**:
*Calls create_task_proposal for login form*

Done! I've created the "Login Form Component" proposal.

*Calls list_session_proposals to see what else exists*

I see you already have "Auth API Endpoints" and "Token Storage" proposals. The login form will depend on both of these.

*Calls add_proposal_dependency twice*

I've added the dependencies:
- Login Form → depends on → Auth API Endpoints
- Login Form → depends on → Token Storage

*Calls analyze_session_dependencies*

Updated execution order: Build the API first, then token storage, then the login form. Want me to recalculate all priorities?
```

### 5.4 Update Do Not Section

Update:
```markdown
## Do Not
- **Wait passively** - If you see an opportunity to help, offer it
- **Stop after one action** - Always suggest the next logical step
- **Ignore changed context** - If proposals or plan changed, acknowledge it
... (existing items)
```

---

## Implementation Order

1. **Part 1** - Wire up priority assessment (backend only, fastest win)
2. **Part 3.1-3.3** - UI badges and critical path (frontend, visible improvement)
3. **Part 2** - AI suggestions (agent + MCP + auto-trigger, most complex)
4. **Part 3.4-3.5** - Loading states and manual trigger
5. **Part 4** - Chat agent dependency analysis tool (Phase 37 extension)
6. **Part 5** - Enhance agent prompt for proactive behavior

---

## Open Questions (Resolved)

| Question | Decision |
|----------|----------|
| Auto-accept vs review | **Auto-accept all** - frictionless UX |
| Trigger timing | **Auto at 2+ proposals** - on create/update/delete |
| Review UI | **None needed** - badges show results, toast confirms |

---

## Commit Lock Workflow (Parallel Agent Coordination)

Reference: `.claude/rules/commit-lock.md`

### Before Committing
```bash
# 1. Establish project root (works from any subdirectory)
PROJECT_ROOT="$(git rev-parse --show-toplevel)"

# 2. Check/acquire lock
if [ -f "$PROJECT_ROOT/.commit-lock" ]; then
  # Read lock content, wait 3s, retry up to 30s
  # If stale (same content >30s), delete and proceed
fi

# 3. Create lock
echo "<stream-name> $(date -u +%Y-%m-%dT%H:%M:%S)" > "$PROJECT_ROOT/.commit-lock"

# 4. Stage and commit
git -C "$PROJECT_ROOT" add <files>
git -C "$PROJECT_ROOT" commit -m "message"
```

### After Committing
```bash
# ALWAYS release lock (success or failure)
rm -f "$PROJECT_ROOT/.commit-lock"
```

### Lock Rules
1. Acquire lock BEFORE `git add`
2. Release lock AFTER commit (success OR failure)
3. Stale = same content + >30 sec old
4. Never force-delete active lock from another agent
