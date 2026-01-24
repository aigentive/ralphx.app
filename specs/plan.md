# RalphX - Implementation Plan

## Project Overview

**RalphX** is a modern Mac desktop application that wraps the Ralph Wiggum autonomous development loop concept, providing a polished UI for managing AI-driven development workflows.

### Core Concept
Instead of manually editing files (specs/prd.md, logs/activity.md) and running `ralph.sh`, users interact through a native Mac app that:
- Orchestrates Claude agents via the **Claude Agent SDK**
- Stores project state in a **local database** (not filesystem JSON)
- Provides a **Cowork-inspired UI** with real-time progress visualization
- Supports **multiple concurrent loops** across different projects
- Enables **human-in-the-loop checkpoints** and task injection
- **Extensible architecture** supporting custom workflows, methodologies (BMAD, GSD), and Claude Code plugins

---

## Tech Stack

### Desktop Framework: **Tauri 2.0**
- **Backend**: Rust (process management, database, file system operations)
- **Frontend**: React + TypeScript + Tailwind CSS
- **Why Tauri**:
  - 10MB bundle vs Electron's 100MB+
  - 30-40MB memory vs Electron's 200-300MB
  - Native macOS integration via WKWebView
  - Excellent CLI process spawning via Shell plugin
  - Built-in sandboxing with scoped file system access

### Agent Integration: **Claude Agent SDK (TypeScript)**
- **Language**: TypeScript (same ecosystem as frontend, simpler build pipeline)
- **Runs inside**: Linux ARM64 VM for full isolation
- **Benefits over CLI spawning**:
  - Direct programmatic control of agent behavior
  - Native async streaming for real-time UI updates
  - Custom tools exposed to agent (database ops via IPC)
  - Hooks for permission callbacks and UI notifications
  - Session management for context persistence

### Database: **SQLite** (via `rusqlite` in Rust backend)
- Local-first, no server required
- Single file per workspace (portable)
- Exposes CRUD operations as custom tools for the agent

### Authentication
- Uses existing Claude CLI installation (`claude` must be installed)
- Picks up Claude Max subscription credentials automatically
- No separate API key configuration needed

### Sandboxing & Virtualization (Full VM Isolation)

**How Claude Cowork does it:**
- **Hard Isolation**: Apple's `VZVirtualMachine` framework boots a Linux ARM64 VM
- **Soft Isolation**: Inside VM, uses `bubblewrap` + `seccomp` for process-level restrictions
  - Bubblewrap: Restricts filesystem view, process capabilities, namespaces
  - Seccomp: Filters syscalls at kernel level
- **File Access**: Only explicitly mounted/shared folders accessible
- **Network**: Routes external traffic through local proxy (HTTP/SOCKS) for policy control

**RalphX Approach (Same as Cowork):**
- **Virtualization.framework**: Use Apple's native VM framework to boot Linux ARM64 VM
- **Shared Folders**: Mount only the project's working directory into the VM
- **Network Proxy**: Route all network through host-side proxy for logging/control
- **Agent Execution**: Claude Agent SDK runs inside the VM
- **IPC**: Communicate between Tauri host and VM via virtio-vsock or shared memory

Tauri already provides good security defaults. Full VM isolation adds complexity and may not be necessary for a tool that runs on the developer's own machine with their own credentials.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     RalphX (Tauri Application)               │
├─────────────────────────────────────────────────────────────┤
│  Frontend (React + TypeScript)                              │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────┐   │
│  │ Project     │ │ Task Board  │ │ Agent Activity      │   │
│  │ Selector    │ │ (Kanban)    │ │ Stream              │   │
│  └─────────────┘ └─────────────┘ └─────────────────────┘   │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────┐   │
│  │ Chat        │ │ Checkpoints │ │ Settings            │   │
│  │ Interface   │ │ Panel       │ │                     │   │
│  └─────────────┘ └─────────────┘ └─────────────────────┘   │
├─────────────────────────────────────────────────────────────┤
│  Tauri IPC Bridge (invoke commands, events)                 │
├─────────────────────────────────────────────────────────────┤
│  Host Backend (Rust)                                        │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────┐   │
│  │ VM          │ │ Database    │ │ Network             │   │
│  │ Manager     │ │ (SQLite)    │ │ Proxy               │   │
│  └─────────────┘ └─────────────┘ └─────────────────────┘   │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────┐   │
│  │ Project     │ │ Loop        │ │ Shared Folder       │   │
│  │ Manager     │ │ Coordinator │ │ Mount               │   │
│  └─────────────┘ └─────────────┘ └─────────────────────┘   │
├─────────────────────────────────────────────────────────────┤
│  VM Communication Layer (virtio-vsock / shared memory)      │
├─────────────────────────────────────────────────────────────┤
│  Linux ARM64 VM (Virtualization.framework)                  │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  Sandboxed Environment (bubblewrap + seccomp)       │   │
│  │  ┌─────────────┐ ┌─────────────┐ ┌──────────────┐  │   │
│  │  │ Agent SDK   │ │ Custom      │ │ Git          │  │   │
│  │  │ (TypeScript)│ │ Tools       │ │ Operations   │  │   │
│  │  └─────────────┘ └─────────────┘ └──────────────┘  │   │
│  │  ┌─────────────┐ ┌─────────────┐                   │   │
│  │  │ Streaming   │ │ File System │ (mounted from    │   │
│  │  │ Handler     │ │ Access      │  host)           │   │
│  │  └─────────────┘ └─────────────┘                   │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘

Parallel Execution: Multiple VMs can run simultaneously for different projects
```

---

## Data Model

### Projects Table
```sql
CREATE TABLE projects (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  working_directory TEXT NOT NULL,  -- The actual project folder (user's original)
  git_mode TEXT NOT NULL DEFAULT 'local',  -- 'local' | 'worktree'
  worktree_path TEXT,              -- Path to worktree (if git_mode = 'worktree')
  worktree_branch TEXT,            -- Branch name for worktree
  base_branch TEXT,                -- Branch to create worktree from (usually main/master)
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

---

## Git Integration & Worktree Support

### Git Initialization Flow

When user selects a folder, the system checks and handles Git state:

```
User selects folder
       ↓
Check: Is it a Git repository?
       ↓
┌──────┴──────┐
↓             ↓
Yes           No
 ↓             ↓
Continue    Prompt: "Initialize Git repository?"
             ↓
      ┌──────┴──────┐
      ↓             ↓
    Yes            No
      ↓             ↓
  git init      Warn: "Git required for
  git add .      version control and
  git commit     task tracking"
  -m "Initial"        ↓
      ↓          Allow anyway?
  Continue            ↓
                 (not recommended)
```

### Git Modes

| Mode | Description | Use Case |
|------|-------------|----------|
| **Local** (default) | Work directly in user's checked-out branch | Quick tasks, user not actively coding |
| **Worktree** | Create isolated worktree in separate directory | User actively coding, wants isolation |

### Worktree Mode Benefits

1. **Isolation** - User's branch untouched, can continue their work
2. **Clean state** - Worktree starts from clean commit (no uncommitted changes)
3. **Parallel work** - User and RalphX work simultaneously without conflicts
4. **Easy cleanup** - Delete worktree when done, no trace in original repo
5. **Branch management** - RalphX commits to separate branch, user reviews/merges later

### Project Creation Flow with Git Mode Selection

```
┌─────────────────────────────────────────────────────────────┐
│  Create New Project                                         │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Project Name: [____________________]                       │
│                                                             │
│  Folder: [/Users/dev/my-app________] [Browse]               │
│                                                             │
│  ─────────────────────────────────────────────────────────  │
│                                                             │
│  Git Mode:                                                  │
│                                                             │
│  ○ Local (default)                                          │
│    Work directly in your current branch                     │
│    ⚠️  Your uncommitted changes may be affected             │
│                                                             │
│  ○ Isolated Worktree (recommended when actively coding)     │
│    Creates separate worktree for RalphX to work in          │
│    Your branch stays untouched                              │
│                                                             │
│    Branch name: [ralphx/feature-____]                       │
│    Base branch: [main_____________▼]                        │
│    Worktree location: ~/ralphx-worktrees/my-app             │
│                                                             │
│  ─────────────────────────────────────────────────────────  │
│                                                             │
│                              [Cancel]  [Create Project]     │
└─────────────────────────────────────────────────────────────┘
```

### Worktree Setup Process

```bash
# 1. Verify clean state or stash
git status --porcelain
# If dirty, warn user or auto-stash

# 2. Fetch latest from remote
git fetch origin

# 3. Create worktree with new branch from base
git worktree add \
  ~/ralphx-worktrees/my-app \
  -b ralphx/feature-auth \
  origin/main

# 4. Store paths in database
# working_directory = /Users/dev/my-app (original)
# worktree_path = ~/ralphx-worktrees/my-app
# worktree_branch = ralphx/feature-auth
# base_branch = main
```

### Execution Directory Logic

```rust
// src-tauri/src/core/git_manager.rs

impl Project {
    /// Returns the directory where RalphX should execute tasks
    pub fn execution_directory(&self) -> &Path {
        match self.git_mode {
            GitMode::Local => &self.working_directory,
            GitMode::Worktree => self.worktree_path.as_ref()
                .expect("Worktree path required for worktree mode"),
        }
    }
}
```

### Database Schema for Git

```sql
-- Git state tracking
CREATE TABLE git_state (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id),

  -- Current state
  current_branch TEXT NOT NULL,
  current_commit TEXT NOT NULL,
  is_dirty BOOLEAN DEFAULT FALSE,

  -- Worktree info (if applicable)
  worktree_created_at DATETIME,
  worktree_base_commit TEXT,

  -- Sync state
  last_fetch_at DATETIME,
  commits_ahead INTEGER DEFAULT 0,
  commits_behind INTEGER DEFAULT 0,

  updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Track commits made by RalphX
CREATE TABLE git_commits (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id),
  task_id TEXT REFERENCES tasks(id),

  commit_sha TEXT NOT NULL,
  commit_message TEXT NOT NULL,
  files_changed TEXT,  -- JSON array of file paths

  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

### Git Operations Service

```rust
// src-tauri/src/core/git_service.rs

pub struct GitService {
    repo_path: PathBuf,
}

impl GitService {
    /// Check if directory is a git repository
    pub fn is_git_repo(path: &Path) -> bool {
        path.join(".git").exists() ||
        Command::new("git")
            .args(["rev-parse", "--git-dir"])
            .current_dir(path)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Initialize a new git repository
    pub fn init(path: &Path) -> AppResult<()> {
        Command::new("git")
            .args(["init"])
            .current_dir(path)
            .output()?;

        // Create initial commit
        Command::new("git")
            .args(["add", "."])
            .current_dir(path)
            .output()?;

        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(path)
            .output()?;

        Ok(())
    }

    /// Create a worktree for isolated development
    pub fn create_worktree(
        repo_path: &Path,
        worktree_path: &Path,
        branch_name: &str,
        base_branch: &str,
    ) -> AppResult<()> {
        // Fetch latest
        Command::new("git")
            .args(["fetch", "origin"])
            .current_dir(repo_path)
            .output()?;

        // Create worktree with new branch
        let base_ref = format!("origin/{}", base_branch);
        Command::new("git")
            .args([
                "worktree", "add",
                worktree_path.to_str().unwrap(),
                "-b", branch_name,
                &base_ref,
            ])
            .current_dir(repo_path)
            .output()?;

        Ok(())
    }

    /// Remove worktree when project is deleted/completed
    pub fn remove_worktree(repo_path: &Path, worktree_path: &Path) -> AppResult<()> {
        Command::new("git")
            .args(["worktree", "remove", worktree_path.to_str().unwrap()])
            .current_dir(repo_path)
            .output()?;
        Ok(())
    }

    /// Get current branch name
    pub fn current_branch(&self) -> AppResult<String> {
        let output = Command::new("git")
            .args(["branch", "--show-current"])
            .current_dir(&self.repo_path)
            .output()?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Check if working directory is dirty
    pub fn is_dirty(&self) -> AppResult<bool> {
        let output = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(&self.repo_path)
            .output()?;
        Ok(!output.stdout.is_empty())
    }

    /// Commit changes with task reference
    pub fn commit(&self, message: &str, task_id: Option<&str>) -> AppResult<String> {
        // Stage all changes
        Command::new("git")
            .args(["add", "."])
            .current_dir(&self.repo_path)
            .output()?;

        // Commit
        let output = Command::new("git")
            .args(["commit", "-m", message])
            .current_dir(&self.repo_path)
            .output()?;

        // Get commit SHA
        let sha_output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&self.repo_path)
            .output()?;

        Ok(String::from_utf8_lossy(&sha_output.stdout).trim().to_string())
    }
}
```

### UI Components for Git Mode

#### Git Mode Selector (Project Creation)

```typescript
// src/components/projects/GitModeSelector.tsx

interface GitModeSelectorProps {
  value: GitMode;
  onChange: (mode: GitMode, config?: WorktreeConfig) => void;
  repoPath: string;
}

export function GitModeSelector({ value, onChange, repoPath }: GitModeSelectorProps) {
  const [branches, setBranches] = useState<string[]>([]);
  const [worktreeConfig, setWorktreeConfig] = useState<WorktreeConfig>({
    branchName: `ralphx/${generateSlug()}`,
    baseBranch: 'main',
    worktreePath: getDefaultWorktreePath(repoPath),
  });

  return (
    <div className="space-y-4">
      <RadioGroup value={value} onChange={(v) => onChange(v, worktreeConfig)}>
        <RadioOption value="local">
          <div>
            <span className="font-medium">Local</span>
            <span className="text-muted ml-2">(default)</span>
          </div>
          <p className="text-sm text-secondary">
            Work directly in your current branch
          </p>
          <p className="text-sm text-warning">
            ⚠️ Your uncommitted changes may be affected
          </p>
        </RadioOption>

        <RadioOption value="worktree">
          <div>
            <span className="font-medium">Isolated Worktree</span>
            <span className="text-muted ml-2">(recommended)</span>
          </div>
          <p className="text-sm text-secondary">
            Creates separate worktree — your branch stays untouched
          </p>
        </RadioOption>
      </RadioGroup>

      {value === 'worktree' && (
        <WorktreeConfigForm
          config={worktreeConfig}
          branches={branches}
          onChange={setWorktreeConfig}
        />
      )}
    </div>
  );
}
```

#### Worktree Status Indicator

```typescript
// src/components/projects/WorktreeStatus.tsx

export function WorktreeStatus({ project }: { project: Project }) {
  if (project.gitMode === 'local') {
    return (
      <div className="flex items-center gap-2 text-sm">
        <GitBranchIcon className="w-4 h-4" />
        <span>Local: {project.currentBranch}</span>
      </div>
    );
  }

  return (
    <div className="flex items-center gap-2 text-sm">
      <GitBranchIcon className="w-4 h-4 text-accent" />
      <span>Worktree: {project.worktreeBranch}</span>
      <span className="text-muted">from {project.baseBranch}</span>
      <Tooltip content="Working in isolated worktree. Your main branch is untouched.">
        <InfoIcon className="w-4 h-4 text-muted" />
      </Tooltip>
    </div>
  );
}
```

### Worktree Lifecycle

```
Project Created (worktree mode)
       ↓
Create worktree: git worktree add ...
       ↓
RalphX executes tasks in worktree
       ↓
Commits go to worktree branch
       ↓
User can review changes:
  - View diff: git diff main...ralphx/feature
  - Merge: git merge ralphx/feature
  - Cherry-pick: git cherry-pick <sha>
       ↓
Project completed or deleted
       ↓
Cleanup prompt: "Delete worktree and branch?"
       ↓
┌──────┴──────┐
↓             ↓
Keep         Delete
 ↓             ↓
User         git worktree remove ~/ralphx-worktrees/my-app
merges       git branch -d ralphx/feature
manually
```

### Worktree Path Convention

```
Default worktree location:
~/ralphx-worktrees/{project-name}/

Example:
Original repo:     /Users/dev/my-app
Worktree:          ~/ralphx-worktrees/my-app/
Worktree branch:   ralphx/feature-auth
Base branch:       main
```

### Task-Level Git Mode Override (Future)

For advanced use cases, individual tasks could override the project's git mode:

```sql
ALTER TABLE tasks ADD COLUMN git_mode_override TEXT;  -- NULL means use project default
ALTER TABLE tasks ADD COLUMN worktree_path_override TEXT;
```

This allows:
- Most tasks use project's default mode
- Specific risky tasks can be isolated to separate worktree
- "Experiment" tasks in throwaway branches

### Merge Workflow UI (Post-Completion)

When a project completes in worktree mode:

```
┌─────────────────────────────────────────────────────────────┐
│  Project Complete: my-app                                   │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  RalphX made 12 commits on branch: ralphx/feature-auth      │
│                                                             │
│  [View Diff]  [View Commits]                                │
│                                                             │
│  ─────────────────────────────────────────────────────────  │
│                                                             │
│  What would you like to do?                                 │
│                                                             │
│  ○ Merge to main (creates merge commit)                     │
│  ○ Rebase onto main (linear history)                        │
│  ○ Create Pull Request (review first)                       │
│  ○ Keep worktree (merge manually later)                     │
│  ○ Discard changes (delete worktree and branch)             │
│                                                             │
│                              [Cancel]  [Continue]           │
└─────────────────────────────────────────────────────────────┘
```

### Tasks Table
```sql
CREATE TABLE tasks (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id),
  category TEXT NOT NULL,  -- 'setup', 'feature', 'integration', 'styling', 'testing'
  title TEXT NOT NULL,
  description TEXT,
  priority INTEGER DEFAULT 0,  -- Higher = more urgent
  status TEXT NOT NULL DEFAULT 'draft',
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  started_at DATETIME,
  completed_at DATETIME
);
```

### Task Statuses (Granular State Machine)

Each operation has its own status for observability and control. We use [**statig**](https://github.com/mdeloof/statig) for type-safe state machines with superstates.

| Superstate | Status | Description | Entry Action |
|------------|--------|-------------|--------------|
| *idle* | `backlog` | Parked, not ready for work | - |
| *idle* | `ready` | Ready to be picked up | Spawn QA Prep (background), auto-transition to `executing` |
| *idle* | `blocked` | Waiting on dependencies/human | - |
| **execution** | `executing` | Worker agent running | Spawn Worker agent |
| **execution** | `execution_done` | Worker finished | Auto-transition to QA or Review |
| **qa** | `qa_refining` | QA agent refining plan based on implementation | Wait for QA Prep, spawn QA Refiner |
| **qa** | `qa_testing` | Browser tests executing | Spawn QA Tester |
| **qa** | `qa_passed` | All QA tests passed | Auto-transition to `pending_review` |
| **qa** | `qa_failed` | QA tests failed | Notify user |
| **review** | `pending_review` | Awaiting AI reviewer | Spawn Reviewer agent |
| **review** | `revision_needed` | Review found issues | Auto-transition back to `executing` |
| *terminal* | `approved` | Complete and verified | Emit completion event, unblock dependents |
| *terminal* | `failed` | Unrecoverable error | Notify user |
| *terminal* | `cancelled` | Intentionally abandoned | - |

**Key principles:**
- **One operation per status** - `qa_testing` is ONLY browser tests, not refining
- **Superstates group related states** - Common handlers (e.g., Cancel from any QA state)
- **Entry actions** - Side effects trigger when entering a status
- **Auto-transitions** - Some states immediately transition (e.g., `qa_passed` → `pending_review`)
- **State-local data** - `qa_failed` carries failure details, `failed` carries error info

**Workflow (Granular States):**
```
                                     ┌─────────────────────────────────────┐
                                     │  QA Prep (background, non-blocking) │
                                     └──────────────────┬──────────────────┘
                                                        │ runs in parallel
                                                        ▼
backlog ──► ready ──► executing ──► execution_done ─────┼────────────────────────────► pending_review
              │            │               │            │                                    │
              ▼            ▼               │            │ [qa_enabled]                       │
          blocked ◄────────┘               │            ▼                                    ▼
              │                            │     qa_refining ──► qa_testing ──┬──► qa_passed ──► pending_review
              │ blockers_resolved          │            │                     │
              └────────────────────────────┴────────────┘                     └──► qa_failed
                                                                                       │
                                                                                       ▼
                                                                              revision_needed
                                                                                       │
                                                                                       └──► executing (retry)

pending_review ─────┬──► approved (terminal)
                    │
                    └──► revision_needed ──► executing (rework)
```

**Simplified view (without QA):**
```
backlog → ready → executing → execution_done → pending_review → approved
```

**Full view (with QA enabled):**
```
backlog → ready → executing → execution_done → qa_refining → qa_testing → qa_passed → pending_review → approved
               ↑                                                    │
               └──────── revision_needed ◄─── qa_failed ◄───────────┘
```

**Key insight**: Each state represents ONE operation:
- `qa_refining` = ONLY refining the QA plan based on actual implementation
- `qa_testing` = ONLY running browser tests
- `qa_passed` / `qa_failed` = result states with appropriate data

### Task Steps Table
```sql
CREATE TABLE task_steps (
  id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(id),
  step_order INTEGER NOT NULL,
  description TEXT NOT NULL,
  completed BOOLEAN DEFAULT FALSE
);
```

### Activity Log Table
```sql
CREATE TABLE activity_logs (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id),
  task_id TEXT REFERENCES tasks(id),
  iteration INTEGER,
  timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
  event_type TEXT NOT NULL,  -- 'task_started', 'task_completed', 'tool_call', 'error', 'checkpoint'
  content TEXT,
  metadata JSON
);
```

### Reviews Table
```sql
CREATE TABLE reviews (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id),
  task_id TEXT NOT NULL REFERENCES tasks(id),
  reviewer_type TEXT NOT NULL,     -- 'ai' or 'human'
  status TEXT DEFAULT 'pending',   -- 'pending', 'approved', 'changes_requested', 'rejected'
  notes TEXT,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  completed_at DATETIME
);

CREATE TABLE review_actions (
  id TEXT PRIMARY KEY,
  review_id TEXT NOT NULL REFERENCES reviews(id),
  action_type TEXT NOT NULL,       -- 'created_fix_task', 'moved_to_backlog', 'approved'
  target_task_id TEXT,             -- ID of created fix task, if applicable
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

### Task State History (Audit Log)
```sql
CREATE TABLE task_state_history (
  id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(id),
  from_status TEXT,                -- NULL for initial creation
  to_status TEXT NOT NULL,
  changed_by TEXT NOT NULL,        -- 'user', 'system', 'ai_worker', 'ai_reviewer', 'ai_supervisor'
  reason TEXT,                     -- Why the change happened
  metadata JSON,                   -- Additional context (e.g., review notes, error details)
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Example entries:
-- | from_status | to_status        | changed_by    | reason                                    |
-- |-------------|------------------|---------------|-------------------------------------------|
-- | NULL        | draft            | user          | Task created via chat                     |
-- | draft       | planned          | user          | Dragged to Planned column                 |
-- | planned     | in_progress      | system        | Auto-picked by worker                     |
-- | in_progress | done             | ai_worker     | Task completed successfully               |
-- | done        | in_review        | system        | Auto-triggered AI review                  |
-- | in_review   | needs_human_review| ai_reviewer  | Escalated: security-sensitive change      |
-- | needs_human_review | approved  | user          | Human approved with notes                 |
-- | in_review   | needs_changes    | ai_reviewer   | Found issues: missing error handling      |
-- | in_progress | failed           | ai_supervisor | Detected infinite loop, killed task       |
```

---

## Custom Tools for Agent

The Agent SDK will have custom tools to interact with the database:

### `get_next_task`
Returns the highest priority task with status `planned`.

### `update_task_status`
Updates a task's status (e.g., `planned` → `in_progress` → `completed`).

### `log_activity`
Appends an entry to the activity log.

### `create_checkpoint`
Creates a human-in-the-loop checkpoint that pauses execution.

### `get_project_context`
Returns project metadata and recent activity for context.

### `insert_task`
Adds a new task at the correct priority position.

---

## UI Components

### 1. Project Sidebar
- List of projects with status indicators
- "New Project" button (folder picker + name)
- Project switching

### 2. Task Board (Kanban View)
- Columns: **Draft** | **Backlog** | **To-do** | **Planned** | **In Progress** | **In Review** | **Done**
  - Draft: Ideas, brainstorming output
  - Backlog: Confirmed but deferred for later
  - To-do: Ready to schedule
  - Planned: **Drag here = auto-executes** (when capacity available)
  - In Progress: Currently running (read-only)
  - In Review: AI reviewing (shows progress)
  - Done: Approved, Skipped, Failed (with visual badges)

**Drag & Drop Behavior:**

| Action | Allowed | Effect |
|--------|---------|--------|
| Drag within same column | ✓ | Reorder = change priority (higher = first) |
| Drag to Planned | ✓ | Auto-executes when capacity available |
| Drag from Planned back | ✓ | Removes from queue (if not yet started) |
| Drag to Backlog | ✓ | Defer for later |
| Drag out of In Progress | ✗ | Locked while running (use Pause/Stop) |
| Drag out of In Review | ✗ | Locked while AI reviewing |
| Drag to Done | ✗ | Can't manually complete (must go through execution) |
| Drag within Done | ✗ | Terminal states, no reorder |

**Visual Feedback:**
- Valid drop target: column highlights with accent border
- Invalid drop: column shows ✗ icon, card snaps back
- Dragging: card becomes semi-transparent, shows ghost at cursor
- Drop: smooth animation to new position

**Priority System:**
- Higher position in column = higher priority
- Drag to top = "do next"
- New tasks added to bottom by default
- Priority number auto-calculated based on position

**Column Transition Constraints & Edge Cases:**

| From → To | Constraints | Edge Cases |
|-----------|-------------|------------|
| Draft → Planned | Must have title & description | Warn if no steps defined |
| Draft → To-do | None | - |
| Draft → Backlog | None | - |
| Backlog → Planned | Must have title & description | - |
| To-do → Planned | None (already validated) | - |
| Planned → To-do | Only if not yet picked up | **Race condition** (see below) |
| Planned → Backlog | Only if not yet picked up | **Race condition** (see below) |
| In Progress → * | **Blocked** | Must use Stop/Pause action |
| In Review → * | **Blocked** | Must wait for review to complete |
| Done → Backlog | Allowed (re-open task) | Clears review history, resets status |
| Done → Planned | Allowed (re-run task) | Clears review history, re-executes |
| Needs Changes → Planned | Allowed | Links to original review preserved |
| Pending Approval → Planned | **Means approved** | Triggers execution |
| Pending Approval → Backlog | **Means dismissed** | AI may propose alternative |

**Race Condition Handling (Planned → To-do/Backlog):**
```
User drags task from Planned
       ↓
Backend checks: is task already being picked up?
       ↓
┌──────┴──────┐
↓             ↓
Not picked    Already picked
     ↓              ↓
Allow move    Show error toast:
     ↓        "Task already started,
Update DB     use Stop to cancel"
     ↓              ↓
Emit event    Snap card back
```

**Implementation:**
```rust
// src-tauri/src/commands/tasks.rs
#[tauri::command]
async fn move_task(task_id: &str, to_status: &str) -> Result<(), String> {
    let task = db.get_task(task_id)?;

    // Validate transition
    match (&task.status, to_status) {
        // Blocked transitions
        ("in_progress", _) => return Err("Cannot move running task".into()),
        ("in_review", _) => return Err("Cannot move task under review".into()),

        // Race condition check for Planned
        ("planned", "todo" | "backlog") => {
            if worker.is_task_claimed(task_id) {
                return Err("Task already started".into());
            }
        }

        // Validation for moving to Planned
        (_, "planned") => {
            if task.title.is_empty() || task.description.is_none() {
                return Err("Task needs title and description".into());
            }
        }

        _ => {}
    }

    // Perform move with state history
    db.update_task_status(task_id, to_status, "user", None)?;
    emit_task_status(&app, task_id, Some(&task.status), to_status, "user");

    Ok(())
}
```

**Special Scenarios:**

1. **Moving fix task to Backlog:**
   - Original task stays in `needs_changes`
   - AI can propose new fix (if under max_fix_attempts)
   - State history logs: "Fix task dismissed by user"

2. **Re-opening completed task (Done → Planned):**

   **What gets cleared:**
   - Review data (AI review, human review notes)
   - Attempt counters reset
   - Status history preserved (shows this is a re-run)

   **What does NOT get cleared:**
   - Git commits remain (not reverted automatically)
   - Activity log entries preserved
   - File changes stay in codebase

   **Git Handling Options (presented in confirmation dialog):**
   ```
   ┌─────────────────────────────────────────────────────────┐
   │  Re-run Task: "Add user authentication"                │
   │                                                         │
   │  This task was completed with commit: a1b2c3d          │
   │  "feat: Add user authentication"                       │
   │                                                         │
   │  How should we handle the previous work?               │
   │                                                         │
   │  ○ Keep changes, run task again (Recommended)          │
   │    AI will see current code state and make             │
   │    additional changes if needed                        │
   │                                                         │
   │  ○ Revert commit, then run task                        │
   │    ⚠️  Warning: May break code if other work           │
   │    depends on this commit                              │
   │                                                         │
   │  ○ Create new task instead                             │
   │    Original task stays completed, new task created     │
   │                                                         │
   │  [Cancel]                          [Confirm Re-run]    │
   └─────────────────────────────────────────────────────────┘
   ```

   **Revert Commit Flow (if selected):**
   ```
   User selects "Revert commit"
          ↓
   Check: Are there commits after this one?
          ↓
   ┌──────┴──────┐
   ↓             ↓
   No later     Has later commits
   commits            ↓
      ↓         Show warning:
   Safe to      "3 commits were made after this.
   revert       Reverting may cause conflicts."
      ↓               ↓
   git revert   [Abort] or [Revert Anyway]
   --no-edit          ↓
      ↓         Attempt revert
   Success?           ↓
      ↓         If conflict:
   Task →       "Revert failed due to conflicts.
   Planned      Resolve manually or keep changes."
   ```

   **Database tracking for re-runs:**
   ```sql
   ALTER TABLE tasks ADD COLUMN run_number INTEGER DEFAULT 1;
   ALTER TABLE tasks ADD COLUMN previous_commit_sha TEXT;

   -- When re-running:
   UPDATE tasks SET
     run_number = run_number + 1,
     previous_commit_sha = (SELECT commit_sha FROM task_commits WHERE task_id = ?),
     status = 'planned'
   WHERE id = ?;
   ```

   **Re-evaluation (Automatic Context Awareness):**

   When a task re-runs, the agent automatically sees:
   - Current code state (reads files, sees implementation exists)
   - Task description and acceptance criteria
   - Previous review feedback (AI and human notes)
   - Human escalation notes
   - State history (knows this is run #2, #3, etc.)

   **Agent prompt for re-runs includes:**
   ```
   ## Task: {task.title}
   {task.description}

   ## This is run #{task.run_number}

   ## Previous Review Feedback:
   {foreach review in task.reviews}
   - [{review.reviewer_type}]: {review.notes}
   {/foreach}

   ## Instructions:
   Check the current implementation against the requirements.
   If the implementation is complete and addresses all feedback,
   verify it works and mark as done.
   If changes are needed based on feedback, make them.
   ```

   **What the agent does automatically:**
   | Scenario | Agent Behavior |
   |----------|----------------|
   | Implementation exists, meets requirements | Verifies, marks done quickly |
   | Implementation exists, has issues from feedback | Fixes based on feedback |
   | Implementation missing (was reverted) | Implements from scratch |
   | New requirements added to task | Implements additions |

   **No special logic needed** - the agent naturally:
   1. Reads current code state
   2. Compares to requirements + feedback
   3. Takes appropriate action
   4. Creates new commit if changes made

   This means "re-run" is really "re-evaluate with context" - the agent
   decides if work is needed based on what it sees.

3. **Bulk operations:**
   - Select multiple tasks → "Move all to Planned"
   - Validates each, shows summary of failures
   - Maintains relative priority order

4. **Concurrent edits (optimistic locking):**
   ```sql
   UPDATE tasks
   SET status = 'planned', updated_at = NOW()
   WHERE id = ? AND updated_at = ?  -- Check timestamp matches
   ```
   - If timestamp changed, another edit happened
   - Show conflict dialog: "Task was modified. Refresh to see changes."

5. **Keyboard shortcuts:**
   - `P` = Move selected to Planned
   - `B` = Move selected to Backlog
   - `T` = Move selected to To-do
   - `Delete` = Move to Skipped (with confirmation)

- Quick actions: Skip, Edit, Delete, Move to top, Request Review
- Task cards show: title, category badge, priority, review status
- Review badge: ✓ AI Approved, ✓✓ Human Approved, ⚠ Needs Changes

### 3. Agent Activity Stream
- Real-time display of Claude's thinking and actions
- Tool calls with expandable inputs/outputs
- Similar to Cowork's right sidebar
- Scrollable history with search

### 4. Chat Interface
- **Plan Mode**: Brainstorming, research, PRD creation (before tasks exist)
  - Guided discovery questions
  - Web search for tech recommendations
  - Outputs draft tasks for review
- **Execution Mode**: Monitor and intervene (when tasks running)
  - Inject new tasks ("add a task to fix the header")
  - Ask questions about current work
  - Request reprioritization
- Mode indicator in UI (switches automatically based on state)

### 5. Reviews Panel
- List of tasks pending review (AI or human)
- Diff viewer integration (see what changed)
- Approve / Request Changes / Reject buttons
- Notes input for feedback
- Links to original task and any fix tasks

### 5b. Task Detail View (with State History)
When clicking a task card, shows:
- Task title, description, category
- Current status with badge
- **State History Timeline:**
  ```
  ┌─────────────────────────────────────────────────┐
  │  History                                        │
  │                                                 │
  │  ● Approved                          2 min ago │
  │    └─ by: user                                 │
  │    └─ "Looks good, nice work"                  │
  │                                                 │
  │  ● Escalated to human review        15 min ago │
  │    └─ by: ai_reviewer                          │
  │    └─ "Security-sensitive: adds auth bypass"   │
  │                                                 │
  │  ● In Review                        18 min ago │
  │    └─ by: system                               │
  │                                                 │
  │  ● Done                             25 min ago │
  │    └─ by: ai_worker                            │
  │    └─ "Completed in 3 tool calls"              │
  │                                                 │
  │  ● In Progress                      30 min ago │
  │    └─ by: system                               │
  │                                                 │
  │  ● Planned                           1 hr ago  │
  │    └─ by: user                                 │
  │    └─ "Dragged from To-do"                     │
  │                                                 │
  │  ● Created                           2 hrs ago │
  │    └─ by: user                                 │
  └─────────────────────────────────────────────────┘
  ```
- Associated reviews with notes
- Related tasks (fix tasks, parent task)

### 6. Execution Status Bar
- **No "Start" button** - tasks auto-execute when `planned`
- Shows: `Running: 2/3` (current / max concurrent)
- Queued tasks count
- Global Pause toggle (stops picking up new tasks)
- Per-project pause option
- Resource usage (memory per VM)

### 7. Diff Viewer
Split into two tabs:
- **Changes** (uncommitted): Real-time view of current modifications
- **History** (commits): List of commits on left, diff on right

**UI:**
- File tree on left showing changed files
- Click file → shows unified diff on right
- Collapse/expand hunks
- "Open in IDE" button to open file in VS Code/Cursor

**Library:** **git-diff-view** (`@git-diff-view/react`)
- Web Worker support for off-main-thread diff computation
- Handles real-time streaming updates without blocking UI
- Optimized rendering for large files (unlike Monaco which lags >10k lines)
- Syntax highlighting with virtual scrolling
- Used in production AI tools for agent file modifications

**Why not Monaco?**
- Monaco can freeze on large diffs (100k+ lines)
- No built-in Web Worker support for diff computation
- Claude Desktop itself delegates to VS Code's native diff, doesn't use Monaco in-app

**Real-time update pattern:**
```
Agent modifies file → File watcher detects change →
Web Worker computes diff → Throttled UI update (50ms) →
git-diff-view renders only visible viewport
```

### 8. Settings
Settings stored per-profile, with sensible defaults.

---

## Execution Model

### Auto-Execution (No Manual Start)
Tasks are **automatically picked up** when their status changes to `planned`. No "Start Loop" button needed.

**Behavior:**
- Background worker continuously watches for `planned` tasks
- When a task becomes `planned`, it's queued for execution
- Respects `max_concurrent_loops` setting (e.g., 3 parallel VMs max)
- If at capacity, tasks wait in queue until a slot opens
- User can still **pause** a specific project or globally

**Queue Priority:**
1. Tasks ordered by `priority` (higher = first)
2. Within same priority, ordered by `created_at` (FIFO)

### Orchestrator Agent (Chat Interface)
The chat interface has two modes:

**1. Plan Mode (Brainstorming & PRD Creation)**
- Activated when no tasks exist or user asks to plan
- Conversational discovery process (like `/create-prd`)
- Helps user brainstorm, refine ideas, research options
- Outputs: Creates tasks with status `draft`
- User reviews drafts, moves to `planned` when ready → auto-executes

**2. Execution Mode (Monitoring & Intervention)**
- Active when tasks are running
- Shows real-time progress
- Accepts commands: inject task, pause, skip, reprioritize
- Can answer questions about current work

### Execution Worker (Background)
```
// Runs continuously in background
while (app_running):
    // Check for available capacity
    running_count = count_tasks_with_status('in_progress')

    if (running_count < max_concurrent_loops):
        task = get_highest_priority_planned_task()

        if (task):
            spawn_vm_and_execute(task)  // Non-blocking

    sleep(1s)  // Poll interval

// Per-task execution (runs in dedicated VM)
async function execute_task(task):
    update_task_status(task.id, 'in_progress')
    emit_ui_event('task_started', task)

    // Check for checkpoint before task
    if (has_checkpoint_before(task)):
        update_task_status(task.id, 'blocked')
        await wait_for_human_approval()

    // Execute via Agent SDK
    for message in agent.query(task_prompt):
        emit_ui_event('agent_message', message)

        if message.type == 'result':
            if message.success:
                update_task_status(task.id, 'completed')
                log_activity(task.id, 'completed', message.result)
                git_commit(task.title)
            else:
                update_task_status(task.id, 'failed')
                log_activity(task.id, 'failed', message.error)

    // Check for checkpoint after task
    if (has_checkpoint_after(task)):
        emit_ui_event('checkpoint_reached', task)
```

---

## Supervisor Agent (Watchdog System)

An always-on monitoring system that watches task execution and intervenes when problems occur.

### Trigger Events (Hooks)
| Event | Trigger | What Supervisor Checks |
|-------|---------|------------------------|
| `on_task_start` | Task begins execution | Validate acceptance criteria exists |
| `on_tool_call` | Every tool invocation | Detect repetition patterns (same call 3x = loop) |
| `on_error` | Tool or agent error | Analyze error, suggest fix or pause |
| `on_progress_tick` | Every 30 seconds | Check for forward progress (files changed, commits) |
| `on_token_threshold` | Token usage > 50k | Potential runaway, check if productive |
| `on_time_threshold` | Task running > 10 min | Check if stuck or legitimately complex |

### Detection Patterns
```
Infinite Loop Detection:
- Same tool called 3+ times with identical/similar args
- Same error occurring repeatedly
- No file changes after N tool calls

Stuck Detection:
- No git diff changes for 5+ minutes
- Agent asking clarifying questions repeatedly
- High token usage with no progress

Poor Task Definition:
- Agent requests clarification multiple times
- Vague acceptance criteria (detected at task start)
```

### Supervisor Actions
| Severity | Action |
|----------|--------|
| **Low** | Log warning, continue monitoring |
| **Medium** | Inject guidance into agent context ("Try a different approach") |
| **High** | Pause task, mark as `blocked`, notify user |
| **Critical** | Kill task, mark as `failed`, show analysis to user |

### Architecture
```
┌─────────────────────────────────────────────────────┐
│  Execution Loop (per task)                          │
│  ┌───────────────────────────────────────────────┐  │
│  │  Agent SDK Hooks                              │  │
│  │  - PreToolUse  → emit event                   │  │
│  │  - PostToolUse → emit event                   │  │
│  │  - OnError     → emit event                   │  │
│  └───────────────────────────────────────────────┘  │
│                        │                            │
│                        ▼                            │
│  ┌───────────────────────────────────────────────┐  │
│  │  Event Bus (lightweight, in-process)          │  │
│  └───────────────────────────────────────────────┘  │
│                        │                            │
│                        ▼                            │
│  ┌───────────────────────────────────────────────┐  │
│  │  Supervisor (triggered, feels always-on)      │  │
│  │  - Quick checks: pattern matching, timers     │  │
│  │  - Escalation: full agent call if anomaly     │  │
│  │  - Model: haiku for speed (upgrade if needed) │  │
│  └───────────────────────────────────────────────┘  │
│                        │                            │
│                        ▼                            │
│  ┌───────────────────────────────────────────────┐  │
│  │  Actions: log / inject / pause / kill         │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

### Implementation Notes
- **Lightweight first**: Most checks are pattern matching, no LLM call
- **Escalate to agent**: Only invoke supervisor agent (Haiku) when anomaly detected
- **State tracking**: Keep rolling window of last 10 tool calls per task
- **Configurable thresholds**: User can adjust sensitivity in settings

---

## Review System (Replaces "Checkpoints")

### AI Review (Automatic)
When a task status becomes `done`, an AI Review agent automatically verifies the work:

**What AI Review Checks:**
- Code compiles/builds without errors
- Tests pass (if applicable)
- Task acceptance criteria met
- No obvious regressions introduced
- Code quality (basic linting)

**AI Review Outcomes:**
| Outcome | Action | Configurable |
|---------|--------|--------------|
| **Pass** | Status → `ai_approved` | - |
| **Fail (fixable)** | Creates fix task → `planned`, original → `needs_changes` | Auto-fix vs backlog |
| **Escalate** | Status → `needs_human_review`, notify user | - |
| **Uncertain** | Status → `blocked`, notify user | - |

**When AI Escalates:**
- Code works but design decision needed
- Multiple valid approaches, user should choose
- Security-sensitive changes
- Breaking changes to public API
- AI confidence below threshold

**Configuration:**
- `ai_review_enabled`: `true` (default) - can disable for speed
- `ai_review_auto_fix`: `true` - auto-create fix tasks, or `false` → backlog
- `require_fix_approval`: `false` - if true, fix tasks need human approval before execution
- `require_human_review`: `false` - if true, `ai_approved` still needs human approval
- `max_fix_attempts`: `3` - max AI fix attempts before giving up → backlog

**Fix Task Approval Flow (when `require_fix_approval: true`):**
```
AI Review finds issues
       ↓
Creates fix task with status: pending_approval
       ↓
Human sees in Reviews panel
       ↓
┌──────┴──────────┬───────────────┐
↓                 ↓               ↓
Approve      Reject w/        Dismiss
   ↓         feedback            ↓
planned          ↓            backlog
   ↓             ↓           (give up)
executes    AI proposes
            alternative
               ↓
         (repeat until
          max_fix_attempts
          or approved)
```

**Rejection with Feedback:**
When human rejects a proposed fix:
1. Original fix task → `rejected` status
2. AI receives human's feedback in context
3. AI proposes new fix task considering feedback
4. Attempt counter increments
5. If `attempt >= max_fix_attempts`:
   - Original task → `backlog`
   - Notification: "Max fix attempts reached, needs manual intervention"

### Human Review (Manual)
User reviews work via the Reviews panel:
- See what changed (diff viewer integration)
- Add notes/feedback (stored in DB)
- Actions: **Approve**, **Request Changes** (creates task), **Reject** (marks failed)

**Human Review Notes Schema:**
```sql
CREATE TABLE review_notes (
  id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(id),
  reviewer TEXT NOT NULL,  -- 'ai' or 'human'
  outcome TEXT NOT NULL,   -- 'approved', 'changes_requested', 'rejected'
  notes TEXT,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

### Review Triggers
| Event | Trigger |
|-------|---------|
| Task → `done` | AI Review starts automatically |
| AI Review passes | If `require_human_review`, waits for human |
| Human approves | Status → `approved` (terminal) |
| Changes requested | Creates fix task, links to original |

---

## AskUserQuestion Handling (Chat UI)

When the agent uses the `AskUserQuestion` tool, the UI must handle it specially:

**How it works:**
1. Agent calls `AskUserQuestion` tool with options
2. Execution pauses, task status → `blocked`
3. Chat UI renders interactive question component
4. User selects answer or types custom response
5. Answer sent back to agent, execution resumes

**UI Component:**
```
┌─────────────────────────────────────────────────┐
│  🤖 Agent is asking:                            │
│                                                 │
│  "Which authentication method should we use?"  │
│                                                 │
│  ┌─────────────────────────────────────────┐   │
│  │ ○ JWT tokens (Recommended)              │   │
│  │ ○ Session cookies                        │   │
│  │ ○ OAuth only                             │   │
│  │ ○ Other: [________________]              │   │
│  └─────────────────────────────────────────┘   │
│                                                 │
│  [Submit Answer]                                │
└─────────────────────────────────────────────────┘
```

**Implementation:**
- Parse tool call parameters: `question`, `options`, `header`
- Render as radio buttons (single select) or checkboxes (multi-select)
- Always include "Other" option with text input
- On submit: resume agent with selected answer

---

## Human-in-the-Loop Features

### Review Points (Formerly "Checkpoints")
1. **Before Destructive** - Auto-inserted before tasks that delete files/configs
2. **After Complex** - Optional, for tasks marked as complex
3. **Manual** - User-defined review points on specific tasks

### Task Injection
- User can add tasks mid-loop via chat or UI
- Option: Send to **Backlog** (deferred) or **Planned** (immediate queue)
- If Planned, inserted at correct priority position
- "Make next" option → highest priority

### Loop Interruption
- Pause button stops after current task completes
- Resume continues from next planned task
- "Stop" cancels current execution (with cleanup)

---

## Development Setup

### Prerequisites

**1. Rust Toolchain (if not installed)**
```bash
# Install rustup (Rust toolchain manager)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Follow prompts, then restart terminal or run:
source $HOME/.cargo/env

# Verify installation
rustc --version
cargo --version

# Add iOS/macOS targets (for Tauri)
rustup target add aarch64-apple-darwin
```

**2. System Dependencies (macOS)**
```bash
# Install Xcode Command Line Tools (if not installed)
xcode-select --install

# Install Homebrew (if not installed)
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
```

**3. Node.js 18+**
```bash
# Using Homebrew (if we don't have nvm)
brew install node@18

# Or using nvm (recommended)
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
nvm install 18
nvm use 18
```

**4. Tauri CLI**
```bash
cargo install tauri-cli
```

**5. Claude CLI**
```bash
# Install Claude Code CLI (if not installed)
npm install -g @anthropic-ai/claude-code

# Authenticate (if not authenticated)
claude login
```

### Full Prerequisites Checklist
- [ ] macOS 12+ (Monterey or later)
- [ ] Xcode Command Line Tools
- [ ] Rust toolchain via rustup
- [ ] Node.js 18+
- [ ] Cargo (comes with Rust)
- [ ] Tauri CLI (`cargo install tauri-cli`)
- [ ] Claude CLI installed and authenticated

### Project Structure
```
ralphx/
├── src-tauri/                  # Rust backend (host)
│   ├── src/
│   │   ├── main.rs
│   │   ├── commands/           # Tauri commands
│   │   ├── database/           # SQLite operations
│   │   ├── vm/                 # Virtualization.framework wrapper
│   │   │   ├── manager.rs      # VM lifecycle
│   │   │   ├── vsock.rs        # virtio-vsock IPC
│   │   │   └── mount.rs        # Shared folder mounting
│   │   ├── proxy/              # Network proxy for VM traffic
│   │   ├── loop/               # Loop coordinator
│   │   └── core/               # Extensibility core
│   │       ├── status.rs       # Internal status state machine
│   │       ├── workflow.rs     # Custom workflow logic
│   │       ├── agent_scheduler.rs  # Agent profile loader & spawner
│   │       ├── artifact_store.rs   # Artifact storage & flow
│   │       └── methodology.rs  # Methodology extension loader
│   ├── Cargo.toml
│   └── tauri.conf.json
├── vm-image/                   # Linux ARM64 VM contents
│   ├── Dockerfile              # Build the VM filesystem
│   ├── agent/                  # Agent SDK code (TypeScript)
│   │   ├── src/
│   │   │   ├── index.ts        # Entry point
│   │   │   ├── tools/          # Custom tools (IPC to host)
│   │   │   └── ipc.ts          # Host communication
│   │   └── package.json
│   └── sandbox/                # bubblewrap + seccomp configs
├── src/                        # React frontend
│   ├── components/
│   │   ├── layout/
│   │   │   ├── Sidebar.tsx
│   │   │   └── Header.tsx
│   │   ├── projects/
│   │   │   ├── ProjectSelector.tsx
│   │   │   └── ProjectSettings.tsx
│   │   ├── tasks/
│   │   │   ├── TaskBoard.tsx   # Kanban
│   │   │   ├── TaskCard.tsx
│   │   │   ├── TaskForm.tsx
│   │   │   ├── TaskDetail.tsx  # Full task view with history
│   │   │   └── StateHistory.tsx # Timeline of status changes
│   │   ├── activity/
│   │   │   ├── ActivityStream.tsx
│   │   │   └── ToolCallDisplay.tsx
│   │   ├── chat/
│   │   │   ├── ChatInterface.tsx
│   │   │   └── MessageBubble.tsx
│   │   ├── reviews/
│   │   │   ├── ReviewsPanel.tsx
│   │   │   ├── ReviewCard.tsx
│   │   │   └── ReviewForm.tsx
│   │   ├── diff/
│   │   │   ├── DiffViewer.tsx      # Main container (tabs: Changes/History)
│   │   │   ├── FileTree.tsx        # Changed files list
│   │   │   ├── CommitList.tsx      # History tab
│   │   │   └── GitDiffView.tsx     # git-diff-view wrapper with Web Worker
│   │   ├── settings/
│   │   │   ├── SettingsModal.tsx
│   │   │   └── ProfileSelector.tsx
│   │   ├── controls/
│   │   │   └── ExecutionStatus.tsx  # Status bar (replaces LoopControls)
│   │   ├── workflows/              # Extensibility UI
│   │   │   ├── WorkflowEditor.tsx
│   │   │   └── WorkflowSelector.tsx
│   │   ├── artifacts/
│   │   │   ├── ArtifactBrowser.tsx
│   │   │   ├── ArtifactCard.tsx
│   │   │   └── ArtifactFlow.tsx
│   │   ├── research/
│   │   │   ├── ResearchLauncher.tsx
│   │   │   ├── ResearchProgress.tsx
│   │   │   └── ResearchResults.tsx
│   │   └── methodologies/
│   │       ├── MethodologyBrowser.tsx
│   │       └── MethodologyConfig.tsx
│   ├── hooks/
│   │   ├── useProjects.ts
│   │   ├── useTasks.ts
│   │   └── useLoopState.ts
│   ├── stores/                 # Zustand state
│   │   ├── projectStore.ts
│   │   ├── taskStore.ts
│   │   ├── loopStore.ts
│   │   ├── workflowStore.ts    # Custom workflows
│   │   └── artifactStore.ts    # Artifact management
│   ├── lib/
│   │   └── tauri.ts            # Tauri invoke wrappers
│   ├── types/
│   │   ├── status.ts           # InternalStatus enum
│   │   ├── workflow.ts         # WorkflowSchema types
│   │   ├── agent-profile.ts    # AgentProfile types
│   │   ├── artifact.ts         # Artifact types
│   │   └── methodology.ts      # Methodology extension types
│   ├── App.tsx
│   └── main.tsx
├── ralphx-plugin/              # Claude Code plugin (bundled)
│   ├── .claude-plugin/
│   │   └── plugin.json
│   ├── agents/
│   │   ├── worker.md
│   │   ├── reviewer.md
│   │   ├── supervisor.md
│   │   ├── orchestrator.md
│   │   └── deep-researcher.md
│   ├── skills/
│   │   ├── coding-standards/SKILL.md
│   │   ├── testing-patterns/SKILL.md
│   │   ├── code-review-checklist/SKILL.md
│   │   ├── research-methodology/SKILL.md
│   │   └── git-workflow/SKILL.md
│   ├── hooks/
│   │   └── hooks.json
│   └── .mcp.json
├── package.json
├── vite.config.ts
└── tailwind.config.js
```

### Development Commands
```bash
# Install dependencies
npm install
cd src-tauri && cargo build

# Development with hot reload
npm run tauri dev

# Build for production
npm run tauri build
```

---

## Build & Distribution

### Build Process

**Development Build:**
```bash
npm run tauri dev
```

**Production Build (unsigned):**
```bash
npm run tauri build
# Output: src-tauri/target/release/bundle/macos/RalphX.app
```

**Production Build (signed + notarized):**
```bash
# Set environment variables for signing
export APPLE_SIGNING_IDENTITY="Developer ID Application: Your Name (TEAM_ID)"
export APPLE_ID="your@email.com"
export APPLE_PASSWORD="app-specific-password"
export APPLE_TEAM_ID="TEAM_ID"

npm run tauri build
# This will sign and notarize automatically
```

### Distribution Options

**Decision: GitHub Releases (v1)**

We're starting with GitHub Releases for initial distribution:
- Free, no Apple Developer account needed initially
- Works seamlessly with Tauri's built-in updater
- Fast iteration, no review process
- Users download DMG from releases page

**Future: Mac App Store**
Consider migrating to App Store later for:
- Increased trust and discoverability
- Automatic updates via App Store
- Sandboxing may require architecture changes
- **Challenge:** VM/Virtualization.framework may have App Store restrictions - research needed
- Requires Apple Developer Program ($99/year)

**Optional: Add Notarization**
To remove Gatekeeper warnings without App Store:
- Enroll in Apple Developer Program ($99/year)
- Sign and notarize via GitHub Actions
- Better UX for users (no "unidentified developer" warning)

### Notarization (Required for non-App Store)

Apple requires notarization for apps distributed outside App Store:

```bash
# Prerequisites:
# 1. Apple Developer ID ($99/year)
# 2. Developer ID Application certificate
# 3. App-specific password from appleid.apple.com

# Tauri handles this automatically with env vars set (see above)
```

### Auto-Update System

**Tauri Updater Plugin:**
```toml
# src-tauri/Cargo.toml
[dependencies]
tauri-plugin-updater = "2"
```

```json
// src-tauri/tauri.conf.json
{
  "plugins": {
    "updater": {
      "active": true,
      "dialog": true,
      "endpoints": [
        "https://github.com/YOUR_USERNAME/ralphx/releases/latest/download/latest.json"
      ],
      "pubkey": "YOUR_PUBLIC_KEY"
    }
  }
}
```

**Update Flow:**
1. App checks endpoint on launch (configurable interval)
2. If new version found, shows dialog: "Update available (v1.2.0). Install now?"
3. User confirms → downloads in background
4. Restart prompt when ready
5. App restarts with new version

**GitHub Actions Workflow:**
```yaml
# .github/workflows/release.yml
name: Release
on:
  push:
    tags:
      - 'v*'

jobs:
  release:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 18
      - uses: dtolnay/rust-toolchain@stable
      - name: Install dependencies
        run: npm install
      - name: Build and release
        uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          APPLE_SIGNING_IDENTITY: ${{ secrets.APPLE_SIGNING_IDENTITY }}
          APPLE_ID: ${{ secrets.APPLE_ID }}
          APPLE_PASSWORD: ${{ secrets.APPLE_PASSWORD }}
          APPLE_TEAM_ID: ${{ secrets.APPLE_TEAM_ID }}
          TAURI_PRIVATE_KEY: ${{ secrets.TAURI_PRIVATE_KEY }}
        with:
          tagName: v__VERSION__
          releaseName: 'RalphX v__VERSION__'
          releaseBody: 'See changelog for details.'
          releaseDraft: false
          prerelease: false
```

### Update Signing (Security)

Generate key pair for update verification:
```bash
npm run tauri signer generate -- -w ~/.tauri/ralphx.key
# Save public key in tauri.conf.json
# Save private key as GitHub secret: TAURI_PRIVATE_KEY
```

### Distribution Checklist

**For GitHub Releases:**
- [ ] Create GitHub repo
- [ ] Set up GitHub Actions workflow
- [ ] Generate Tauri signing keys
- [ ] Add secrets to repo (TAURI_PRIVATE_KEY)
- [ ] Optional: Apple Developer ID for notarization
- [ ] Tag release: `git tag v1.0.0 && git push --tags`

**For Notarized Distribution:**
- [ ] Enroll in Apple Developer Program ($99/year)
- [ ] Create Developer ID Application certificate
- [ ] Generate app-specific password
- [ ] Add Apple secrets to GitHub repo
- [ ] Test notarization locally first

---

## Real-Time Events (Backend → Frontend)

Tauri provides a built-in event system for real-time communication between Rust backend and React frontend.

### Event Architecture
```
┌─────────────────────────────────────────────────────────────┐
│  Rust Backend                                               │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │ VM/Agent    │  │ Supervisor  │  │ Database            │ │
│  │ Executor    │  │ Watchdog    │  │ Triggers            │ │
│  └──────┬──────┘  └──────┬──────┘  └──────────┬──────────┘ │
│         │                │                     │            │
│         └────────────────┴─────────────────────┘            │
│                          │                                  │
│                    emit_event()                             │
│                          │                                  │
├──────────────────────────┼──────────────────────────────────┤
│  Tauri Event Bus         │                                  │
├──────────────────────────┼──────────────────────────────────┤
│                          ↓                                  │
│  React Frontend                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  Event Listeners (useEffect + listen())             │   │
│  │  ┌─────────────┐ ┌─────────────┐ ┌──────────────┐  │   │
│  │  │ Activity    │ │ Task        │ │ Supervisor   │  │   │
│  │  │ Stream      │ │ Board       │ │ Alerts       │  │   │
│  │  └─────────────┘ └─────────────┘ └──────────────┘  │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### Event Types

```typescript
// src/types/events.ts

// Agent activity events (high frequency)
interface AgentMessageEvent {
  taskId: string;
  type: 'thinking' | 'tool_call' | 'tool_result' | 'text' | 'error';
  content: string;
  timestamp: number;
  metadata?: Record<string, unknown>;
}

// Task status changes
interface TaskStatusEvent {
  taskId: string;
  fromStatus: string | null;
  toStatus: string;
  changedBy: 'user' | 'system' | 'ai_worker' | 'ai_reviewer' | 'ai_supervisor';
  reason?: string;
}

// Supervisor alerts
interface SupervisorAlertEvent {
  taskId: string;
  severity: 'low' | 'medium' | 'high' | 'critical';
  type: 'loop_detected' | 'stuck' | 'error' | 'escalation';
  message: string;
  suggestedAction?: string;
}

// Review events
interface ReviewEvent {
  taskId: string;
  reviewId: string;
  type: 'started' | 'completed' | 'needs_human' | 'fix_proposed';
  outcome?: 'approved' | 'changes_requested' | 'escalated';
}

// File change events (for diff viewer)
interface FileChangeEvent {
  projectId: string;
  filePath: string;
  changeType: 'created' | 'modified' | 'deleted';
}

// Progress events
interface ProgressEvent {
  taskId: string;
  progress: number;  // 0-100
  stage: string;     // "Running tests", "Committing changes"
}
```

### Rust Backend: Emitting Events

```rust
// src-tauri/src/events.rs
use tauri::{AppHandle, Manager};
use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct AgentMessagePayload {
    pub task_id: String,
    pub message_type: String,
    pub content: String,
    pub timestamp: u64,
}

pub fn emit_agent_message(app: &AppHandle, payload: AgentMessagePayload) {
    app.emit("agent:message", payload).unwrap();
}

pub fn emit_task_status(app: &AppHandle, task_id: &str, from: Option<&str>, to: &str, by: &str) {
    app.emit("task:status", serde_json::json!({
        "taskId": task_id,
        "fromStatus": from,
        "toStatus": to,
        "changedBy": by,
    })).unwrap();
}

pub fn emit_supervisor_alert(app: &AppHandle, task_id: &str, severity: &str, message: &str) {
    app.emit("supervisor:alert", serde_json::json!({
        "taskId": task_id,
        "severity": severity,
        "message": message,
    })).unwrap();
}
```

### React Frontend: Listening to Events

```typescript
// src/hooks/useEvents.ts
import { useEffect } from 'react';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { useActivityStore } from '../stores/activityStore';
import { useTaskStore } from '../stores/taskStore';

export function useAgentEvents(taskId?: string) {
  const addMessage = useActivityStore((s) => s.addMessage);

  useEffect(() => {
    let unlisten: UnlistenFn;

    listen<AgentMessageEvent>('agent:message', (event) => {
      if (!taskId || event.payload.taskId === taskId) {
        addMessage(event.payload);
      }
    }).then((fn) => { unlisten = fn; });

    return () => { unlisten?.(); };
  }, [taskId, addMessage]);
}

export function useTaskStatusEvents() {
  const updateTaskStatus = useTaskStore((s) => s.updateStatus);

  useEffect(() => {
    let unlisten: UnlistenFn;

    listen<TaskStatusEvent>('task:status', (event) => {
      updateTaskStatus(event.payload.taskId, event.payload.toStatus);
    }).then((fn) => { unlisten = fn; });

    return () => { unlisten?.(); };
  }, [updateTaskStatus]);
}

export function useSupervisorAlerts() {
  const addAlert = useActivityStore((s) => s.addAlert);

  useEffect(() => {
    let unlisten: UnlistenFn;

    listen<SupervisorAlertEvent>('supervisor:alert', (event) => {
      addAlert(event.payload);
      // Also show toast notification
      toast.warning(event.payload.message);
    }).then((fn) => { unlisten = fn; });

    return () => { unlisten?.(); };
  }, [addAlert]);
}
```

### Event Batching (Performance)

For high-frequency events (agent streaming), batch updates to avoid React re-render thrashing:

```typescript
// src/hooks/useBatchedEvents.ts
import { useEffect, useRef, useCallback } from 'react';
import { listen } from '@tauri-apps/api/event';

export function useBatchedAgentMessages(taskId: string) {
  const bufferRef = useRef<AgentMessageEvent[]>([]);
  const [messages, setMessages] = useState<AgentMessageEvent[]>([]);

  // Flush buffer every 50ms
  useEffect(() => {
    const interval = setInterval(() => {
      if (bufferRef.current.length > 0) {
        setMessages((prev) => [...prev, ...bufferRef.current]);
        bufferRef.current = [];
      }
    }, 50);

    return () => clearInterval(interval);
  }, []);

  // Buffer incoming events
  useEffect(() => {
    let unlisten: UnlistenFn;

    listen<AgentMessageEvent>('agent:message', (event) => {
      if (event.payload.taskId === taskId) {
        bufferRef.current.push(event.payload);
      }
    }).then((fn) => { unlisten = fn; });

    return () => { unlisten?.(); };
  }, [taskId]);

  return messages;
}
```

### Global Event Provider

Wrap app in provider that sets up all listeners:

```typescript
// src/providers/EventProvider.tsx
import { useTaskStatusEvents, useSupervisorAlerts, useReviewEvents } from '../hooks/useEvents';

export function EventProvider({ children }: { children: React.ReactNode }) {
  // Set up global event listeners
  useTaskStatusEvents();
  useSupervisorAlerts();
  useReviewEvents();
  useFileChangeEvents();

  return <>{children}</>;
}

// src/App.tsx
function App() {
  return (
    <EventProvider>
      <Router>
        {/* ... */}
      </Router>
    </EventProvider>
  );
}
```

### Event Summary

| Event | Frequency | Source | UI Updates |
|-------|-----------|--------|------------|
| `agent:message` | High (streaming) | VM/Agent | Activity stream |
| `task:status` | Medium | System/AI | Kanban board, task cards |
| `supervisor:alert` | Low | Supervisor | Toast + alerts panel |
| `review:update` | Low | AI Reviewer | Reviews panel, badges |
| `file:change` | Medium | File watcher | Diff viewer |
| `progress:update` | Medium | Agent | Progress bars |

---

## Building RalphX (Using the Ralph Loop)

**This app will be built autonomously by the manual Ralph loop.** The tasks are ordered so you can watch progress incrementally.

### Bootstrap Process
1. Run `/create-prd` to generate the PRD with tasks from this plan
2. Run `./ralph.sh 50` to start autonomous building
3. Watch the app come together piece by piece

### Task Order Principles
- **Visual first**: Get something on screen early (skeleton UI)
- **Core loop early**: Basic task execution before advanced features
- **Incremental complexity**: Each phase builds on the previous
- **Testable milestones**: After each phase, you can run and see progress

### What You'll See at Each Phase
| Phase | What's Visible |
|-------|----------------|
| 1 | Empty Tauri window with dark background |
| 2 | Sidebar with project list (mock data) |
| 3 | Kanban board with draggable cards |
| 4 | Agent activity stream (simulated) |
| 5 | Real agent execution, tasks moving |
| 6+ | Full features, polish |

---

## Implementation Phases

### Phase 1: Foundation
- [ ] Initialize Tauri project with React + TypeScript + Tailwind
- [ ] Set up SQLite database with schema (projects, tasks, activity_logs, checkpoints)
- [ ] Create basic project/task CRUD Tauri commands
- [ ] Implement project selector UI
- [ ] Basic app shell with navigation

### Phase 2: VM Infrastructure
- [ ] Set up Virtualization.framework wrapper in Rust
- [ ] Create minimal Linux ARM64 VM image with Node.js
- [ ] Implement virtio-vsock communication between host and VM
- [ ] Shared folder mounting for project directories
- [ ] Network proxy for external traffic control
- [ ] VM lifecycle management (start, stop, health check)

### Phase 3: Agent Integration
- [ ] Set up Claude Agent SDK (TypeScript) inside VM
- [ ] Create IPC protocol for host ↔ VM communication
- [ ] Implement custom tools that call back to host (database ops)
- [ ] Streaming message handler (VM → host → UI)
- [ ] Build activity stream UI component

### Phase 4: Task Management UI
- [ ] Build Kanban task board with columns (Draft, To-do, Planned, In Progress, Done)
- [ ] Implement drag-and-drop status changes (drag to Planned triggers execution)
- [ ] Add task creation/editing forms
- [ ] Priority management (reorder within columns)
- [ ] Category badges, queue position, visual status indicators

### Phase 5: Loop Execution
- [ ] Implement loop coordinator in Rust (manages VM execution)
- [ ] Add loop controls (start/pause/stop)
- [ ] Git commit integration (runs in VM, uses mounted folder)
- [ ] Iteration tracking and display
- [ ] Completion detection

### Phase 6: Chat & Agent Questions
- [ ] Chat interface with orchestrator agent
- [ ] AskUserQuestion tool handling (interactive question UI)
- [ ] Task injection via chat (to backlog or planned)
- [ ] Plan mode for brainstorming/PRD creation

### Phase 6b: Review System
- [ ] AI Review agent (auto-triggered on task completion)
- [ ] Human review UI (approve/request changes/reject)
- [ ] Review notes storage in database
- [ ] Auto-create fix tasks from review feedback
- [ ] Review badges on task cards

### Phase 7: Supervisor Agent (Watchdog)
- [ ] Event bus for tool call / error / progress events
- [ ] Pattern detection: loop detection, stuck detection
- [ ] Lightweight checks (no LLM) for common patterns
- [ ] Escalation to Haiku agent for anomaly analysis
- [ ] Actions: inject guidance, pause, kill task
- [ ] UI: supervisor alerts panel, intervention history

### Phase 8: Parallel Execution
- [ ] Multiple VM instance management
- [ ] Per-project loop state
- [ ] Background execution with notifications
- [ ] Resource monitoring (memory, CPU per VM)

### Phase 9: Diff Viewer
- [ ] Integrate git-diff-view (@git-diff-view/react) with Web Worker
- [ ] File tree component for changed files
- [ ] "Changes" tab: real-time uncommitted changes via `git diff`
- [ ] "History" tab: commit list with selectable diffs
- [ ] File watcher → throttled diff updates (50ms)
- [ ] "Open in IDE" integration (VS Code, Cursor)

### Phase 10: Settings & Profiles
- [ ] Settings modal with all configurable options
- [ ] Profile system (default + custom profiles)
- [ ] Per-project profile override
- [ ] Settings persistence in SQLite

### Phase 11: Polish
- [ ] Keyboard shortcuts
- [ ] System notifications
- [ ] Light/dark theme (follows system)
- [ ] Onboarding flow for first-time users
- [ ] "Open in IDE" integration

### Phase 12: Extensibility - Core Status Machine
- [ ] Implement `InternalStatus` enum with 9 statuses
- [ ] Create status transition validation logic
- [ ] Build side effect registry and executor
- [ ] Add `internal_status` and `external_status` columns to tasks
- [ ] Implement status transition audit logging

### Phase 13: Extensibility - Custom Workflows
- [ ] Create `WorkflowSchema` types and validation
- [ ] Implement workflow storage in SQLite
- [ ] Build column-to-internal-status mapping logic
- [ ] Update Kanban UI to render from workflow schema
- [ ] Add workflow editor UI (create/edit custom workflows)
- [ ] Implement workflow switching per project

### Phase 14: Extensibility - Agent Profiles
- [ ] Define `AgentProfile` schema
- [ ] Create Claude Code agent definition files (`.claude/agents/*.md`)
- [ ] Create Claude Code skill files (`.claude/skills/*/SKILL.md`)
- [ ] Implement profile loader from database + files
- [ ] Connect profiles to agent scheduler
- [ ] Add profile selection UI in settings

### Phase 15: Extensibility - Artifact System
- [ ] Implement artifact storage with versioning
- [ ] Create artifact bucket system with access control
- [ ] Build artifact relations (derivedFrom, relatedTo)
- [ ] Implement artifact flow engine (trigger → steps)
- [ ] Add artifact browser UI in task detail view
- [ ] Connect artifacts to agent I/O

### Phase 16: Extensibility - Deep Research
- [ ] Create research process type and configuration
- [ ] Implement depth presets (quick-scan to exhaustive)
- [ ] Build research progress tracking with checkpoints
- [ ] Create research UI (launch, monitor, view results)
- [ ] Connect research outputs to artifact buckets
- [ ] Integrate research spawning into orchestrator

### Phase 17: Extensibility - Methodology Support
- [ ] Define methodology extension schema
- [ ] Create methodology installer/loader
- [ ] Support task dependencies and wave-based parallelization
- [ ] Implement checkpoint types (verify, decision, human-action)
- [ ] Add phase/plan tracking for structured methodologies
- [ ] Create methodology browser/marketplace UI

### Phase 18: RalphX Plugin Packaging
- [ ] Bundle agents, skills, hooks as Claude Code plugin
- [ ] Create plugin.json with proper namespacing
- [ ] Implement plugin installation flow
- [ ] Document extension points for third-party plugins
- [ ] Add plugin management UI

---

## Verification

### Local Testing
1. Run `npm run tauri dev` for hot-reload development
2. Create a test project pointing to a sample codebase
3. Add tasks manually and verify Kanban works
4. Start loop and verify agent executes tasks
5. Test checkpoint pause/resume
6. Verify git commits are created

### End-to-End Test
1. Fresh install on clean macOS system
2. Authenticate Claude CLI (`claude login`)
3. Launch app, create new project
4. Use chat to create PRD (task generation)
5. Start autonomous loop
6. Verify tasks complete and codebase changes
7. Test pause, inject task, resume
8. Verify completion detection

---

## Prompt Engineering (Claude Agent Best Practices)

### Core Principles

1. **Be Explicit**: State exactly what you want without ambiguity
2. **Add Context**: Explain the "why" behind rules (Claude generalizes from reasoning)
3. **Use XML Tags**: Claude pays special attention to XML delimiters
4. **One Task at a Time**: Focused tasks produce higher quality
5. **Include Examples**: Even 1-2 line examples help lock in style

### System Prompt Structure

Our agent prompts follow a modular architecture:

```
┌─────────────────────────────────────────────────────────────┐
│  SYSTEM PROMPT                                              │
├─────────────────────────────────────────────────────────────┤
│  1. Role & Expertise                                        │
│     "You are a senior software engineer..."                 │
├─────────────────────────────────────────────────────────────┤
│  2. Context                                                 │
│     - Project description                                   │
│     - Tech stack                                            │
│     - Current state (from database)                         │
├─────────────────────────────────────────────────────────────┤
│  3. Task-Specific Instructions                              │
│     <task_execution>...</task_execution>                    │
├─────────────────────────────────────────────────────────────┤
│  4. Tool Usage Guidelines                                   │
│     <tool_use_policy>...</tool_use_policy>                  │
├─────────────────────────────────────────────────────────────┤
│  5. Behavioral Constraints                                  │
│     <constraints>...</constraints>                          │
├─────────────────────────────────────────────────────────────┤
│  6. Output Format                                           │
│     <output_format>...</output_format>                      │
└─────────────────────────────────────────────────────────────┘
```

### Agent Prompts by Type

#### Worker Agent (Task Execution)
```xml
<role>
You are a senior software engineer implementing a specific task.
You have access to the codebase and can read/write files, run commands.
</role>

<task>
## Task: {task.title}
{task.description}

## Acceptance Criteria:
{task.steps}

## This is run #{task.run_number}
{if task.run_number > 1}
## Previous Feedback:
{foreach review in task.reviews}
- [{review.reviewer_type}]: {review.notes}
{/foreach}
{/if}
</task>

<investigate_before_answering>
Never speculate about code you have not opened. Always read files
BEFORE proposing edits. Never make claims about code before investigating.
</investigate_before_answering>

<avoid_over_engineering>
Only make changes that are directly requested or clearly necessary.
Keep solutions simple and focused. Don't add features, refactor code,
or make "improvements" beyond what was asked.
</avoid_over_engineering>

<completion>
When the task is complete:
1. Verify all acceptance criteria are met
2. Run any available tests/linting
3. Use the update_task_status tool to mark as done
4. Use the log_activity tool to document what was changed
</completion>
```

#### Reviewer Agent (AI Review)
```xml
<role>
You are a code reviewer evaluating completed work against requirements.
Your job is to verify quality, not to reimplement.
</role>

<task>
## Review Task: {task.title}

## Original Requirements:
{task.description}

## Acceptance Criteria:
{task.steps}

## Changes Made:
{git_diff}
</task>

<review_checklist>
1. Does the implementation meet all acceptance criteria?
2. Does the code compile/build without errors?
3. Are there any obvious bugs or regressions?
4. Is the code quality acceptable (no major issues)?
5. Are there any security concerns?
</review_checklist>

<decisions>
Based on your review, choose ONE outcome:
- APPROVE: All criteria met, code quality acceptable
- NEEDS_CHANGES: Issues found that can be fixed automatically
  - Describe the specific issues
  - Propose a fix task description
- ESCALATE: Needs human review (security-sensitive, design decision, unclear requirements)
  - Explain why human input is needed
</decisions>

<output_format>
Use the complete_review tool with:
- outcome: "approved" | "needs_changes" | "escalate"
- notes: Your detailed review notes
- fix_description: (if needs_changes) Description for fix task
</output_format>
```

#### Supervisor Agent (Watchdog)
```xml
<role>
You are a supervisor monitoring agent execution for problems.
You analyze patterns and intervene when necessary.
</role>

<context>
## Task Being Monitored: {task.title}
## Current Status: {task.status}
## Tool Call History (last 10):
{tool_call_history}
## Time Elapsed: {elapsed_time}
## Token Usage: {token_count}
</context>

<detection_rules>
1. LOOP: Same tool called 3+ times with similar arguments
2. STUCK: No file changes for 5+ minutes with continued activity
3. RUNAWAY: Token usage > 50k without meaningful progress
4. ERROR_LOOP: Same error occurring repeatedly
</detection_rules>

<actions>
Based on analysis, choose action:
- CONTINUE: No issues detected, keep monitoring
- WARN: Log warning, continue monitoring more closely
- INJECT: Send guidance message to worker agent
- PAUSE: Pause task, mark as blocked, notify user
- KILL: Stop task immediately, mark as failed
</actions>
```

#### Orchestrator Agent (Chat Interface)
```xml
<role>
You are an AI assistant helping users manage their development workflow.
You can help with planning, task creation, and answering questions.
</role>

<capabilities>
- Create and modify tasks
- Start/pause/stop execution
- Answer questions about project state
- Help brainstorm and plan features
- Research technical questions
</capabilities>

<modes>
You operate in two modes:

PLAN_MODE (when no tasks exist or user asks to plan):
- Ask discovery questions one at a time
- Help refine requirements
- Create draft tasks for review
- Research technical options

EXECUTION_MODE (when tasks are running):
- Monitor progress
- Answer questions about current work
- Inject new tasks if requested
- Handle user interventions
</modes>

<default_to_information>
When user intent is unclear, default to providing information and
recommendations rather than taking action. Ask clarifying questions
if needed.
</default_to_information>
```

### Anti-Patterns to Avoid

| Anti-Pattern | Problem | Fix |
|--------------|---------|-----|
| Vague instructions | Claude can't infer intent | Be explicit and specific |
| Multiple tasks at once | Scattered, unfocused output | One task per prompt |
| No examples | Claude guesses at style | Include 1-2 examples |
| Over-engineering | Creates unnecessary abstractions | Add constraint to keep it simple |
| Speculating about code | Hallucinations | Always read before editing |
| Hard-coding for tests | Solutions only work for test inputs | Request general-purpose solutions |

### Tool Description Template

Each custom tool should have a clear description:

```typescript
{
  name: "update_task_status",
  description: `
    Updates the status of a task in the database.

    WHEN TO USE:
    - After completing work on a task (status: "done")
    - When blocked and need human input (status: "blocked")

    WHEN NOT TO USE:
    - Don't use to skip tasks (use skip_task instead)
    - Don't use for tasks you haven't worked on

    PARAMETERS:
    - task_id: The ID of the task to update
    - status: New status ("done", "blocked", "failed")
    - reason: Brief explanation of why

    EXAMPLE:
    update_task_status("task_123", "done", "Implemented login form with validation")
  `,
  parameters: { ... }
}
```

### Context Window Management

```xml
<context_awareness>
Your context window will be automatically compacted as it approaches
its limit, allowing you to continue working indefinitely. Therefore:
- Do not stop tasks early due to token concerns
- Save progress to files/database before context refreshes
- Be persistent and complete tasks fully
- Use git commits as checkpoints
</context_awareness>
```

### Parallel Tool Calls

```xml
<parallel_tool_calls>
If you intend to call multiple tools and there are no dependencies
between them, make all independent calls in parallel. However, if
some tool calls depend on previous results, call them sequentially.

Example (parallel OK):
- Read file A and Read file B simultaneously

Example (sequential required):
- Read file A, then Edit file A based on contents
</parallel_tool_calls>
```

---

## Code Quality & Best Practices

### Guiding Principles

1. **TEST FIRST (TDD)** - Write failing test before any implementation
2. **No file over 300 lines** - Split into modules when approaching limit
3. **Single responsibility** - Each module/component does one thing well
4. **Explicit over implicit** - No magic, clear data flow
5. **Types as documentation** - Types should make code self-documenting
6. **Errors as values** - Handle errors explicitly, never panic in production

---

## Test-Driven Development (TDD) - MANDATORY

**This project will be built by an autonomous system. TDD is not optional - it is the primary mechanism for validating work.**

### Why TDD is Critical for Autonomous Development

1. **Tests ARE the specification** - The agent reads tests to understand what to build
2. **Immediate validation** - Agent knows if implementation is correct
3. **Regression prevention** - Each iteration can't break previous work
4. **Clear completion criteria** - Task is done when tests pass
5. **Self-documenting** - Tests show intended behavior

### The TDD Cycle (Enforced)

```
┌─────────────────────────────────────────────────────────────┐
│  EVERY TASK FOLLOWS THIS CYCLE - NO EXCEPTIONS              │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  1. RED: Write failing test(s)                              │
│     └─ Run test → MUST FAIL                                 │
│     └─ Commit: test(scope): add failing test for X          │
│                                                             │
│  2. GREEN: Write minimal implementation                     │
│     └─ Run test → MUST PASS                                 │
│     └─ Commit: feat(scope): implement X                     │
│                                                             │
│  3. REFACTOR: Clean up (if needed)                          │
│     └─ Run test → MUST STILL PASS                           │
│     └─ Commit: refactor(scope): clean up X                  │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Test Requirements by Layer

| Layer | Test Type | Required Coverage | Run Command |
|-------|-----------|-------------------|-------------|
| **Rust Core** | Unit | Every public function | `cargo test` |
| **Rust Commands** | Integration | Every Tauri command | `cargo test --test integration` |
| **TypeScript Types** | Unit | Every Zod schema | `npm run test:unit` |
| **React Hooks** | Unit | Every custom hook | `npm run test:unit` |
| **React Components** | Component | Every interactive component | `npm run test:unit` |
| **Full Stack** | E2E | Critical user flows | `npm run test:e2e` |

### Test File Structure

```
# Rust: Tests live in same file or tests/ directory
src-tauri/src/
├── core/
│   ├── status.rs           # Contains #[cfg(test)] mod tests
│   └── task_service.rs     # Contains #[cfg(test)] mod tests
└── tests/                   # Integration tests
    ├── task_commands.rs
    └── workflow_commands.rs

# TypeScript: Tests live next to source
src/
├── components/
│   └── tasks/
│       └── TaskCard/
│           ├── TaskCard.tsx
│           └── TaskCard.test.tsx    # Component tests
├── hooks/
│   ├── useTasks.ts
│   └── useTasks.test.ts             # Hook tests
├── lib/
│   ├── validation.ts
│   └── validation.test.ts           # Unit tests
└── types/
    ├── task.ts
    └── task.test.ts                 # Schema tests
```

### TDD Patterns by Type

#### Pattern 1: Rust Unit Test (Status Machine)

```rust
// src-tauri/src/core/status.rs

// STEP 1: Write the test FIRST (this will not compile yet)
#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(InternalStatus::Ready, InternalStatus::Executing, true)]
    #[case(InternalStatus::Ready, InternalStatus::Approved, false)]  // Invalid
    #[case(InternalStatus::Executing, InternalStatus::PendingReview, true)]
    #[case(InternalStatus::Executing, InternalStatus::Approved, false)]  // Must go through review
    #[case(InternalStatus::PendingReview, InternalStatus::Approved, true)]
    #[case(InternalStatus::PendingReview, InternalStatus::RevisionNeeded, true)]
    #[case(InternalStatus::Backlog, InternalStatus::Executing, false)]  // Must go through Ready
    fn test_valid_transitions(
        #[case] from: InternalStatus,
        #[case] to: InternalStatus,
        #[case] expected_valid: bool,
    ) {
        assert_eq!(
            from.can_transition_to(to),
            expected_valid,
            "Transition {:?} -> {:?} should be {}",
            from,
            to,
            if expected_valid { "valid" } else { "invalid" }
        );
    }

    #[test]
    fn test_executing_triggers_spawn_worker() {
        let effect = InternalStatus::Ready.side_effect_for(InternalStatus::Executing);
        assert_eq!(effect, Some(SideEffect::SpawnWorker));
    }

    #[test]
    fn test_pending_review_triggers_spawn_reviewer() {
        let effect = InternalStatus::Executing.side_effect_for(InternalStatus::PendingReview);
        assert_eq!(effect, Some(SideEffect::SpawnReviewer));
    }
}

// STEP 2: Run tests - they should FAIL (or not compile)
// cargo test status

// STEP 3: Implement the minimal code to make tests pass
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InternalStatus {
    Backlog,
    Ready,
    // ... etc
}

impl InternalStatus {
    pub fn can_transition_to(&self, target: InternalStatus) -> bool {
        // Implementation here
    }

    pub fn side_effect_for(&self, target: InternalStatus) -> Option<SideEffect> {
        // Implementation here
    }
}

// STEP 4: Run tests - they should PASS
// cargo test status
```

#### Pattern 2: Rust Integration Test (Tauri Command)

```rust
// src-tauri/tests/task_commands.rs

// STEP 1: Write integration test FIRST
use ralphx::commands::tasks::*;
use ralphx::database::test_helpers::TestDb;

#[tokio::test]
async fn test_create_task_success() {
    let db = TestDb::new().await;
    let state = AppState::new(db.pool());

    let result = create_task(
        state.into(),
        "project-123".to_string(),
        "Implement login".to_string(),
        Some("Add login form with validation".to_string()),
    ).await;

    assert!(result.is_ok());
    let task = result.unwrap();
    assert_eq!(task.title, "Implement login");
    assert_eq!(task.internal_status, InternalStatus::Backlog);
}

#[tokio::test]
async fn test_create_task_empty_title_fails() {
    let db = TestDb::new().await;
    let state = AppState::new(db.pool());

    let result = create_task(
        state.into(),
        "project-123".to_string(),
        "".to_string(),  // Empty title
        None,
    ).await;

    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), AppError::Validation(_)));
}

#[tokio::test]
async fn test_move_task_valid_transition() {
    let db = TestDb::new().await;
    let state = AppState::new(db.pool());

    // Create task in Ready status
    let task = db.insert_task_with_status(InternalStatus::Ready).await;

    let result = move_task(
        state.into(),
        mock_app_handle(),
        task.id.clone(),
        "executing".to_string(),
    ).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().internal_status, InternalStatus::Executing);
}

#[tokio::test]
async fn test_move_task_invalid_transition_fails() {
    let db = TestDb::new().await;
    let state = AppState::new(db.pool());

    // Create task in Backlog status
    let task = db.insert_task_with_status(InternalStatus::Backlog).await;

    // Try to skip directly to Executing (invalid)
    let result = move_task(
        state.into(),
        mock_app_handle(),
        task.id.clone(),
        "executing".to_string(),
    ).await;

    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), AppError::InvalidTransition { .. }));
}
```

#### Pattern 3: TypeScript Zod Schema Test

```typescript
// src/types/task.test.ts

import { describe, it, expect } from "vitest";
import { TaskSchema, InternalStatusSchema } from "./task";

// STEP 1: Write tests FIRST - these define the contract
describe("InternalStatusSchema", () => {
  it("accepts valid statuses", () => {
    const validStatuses = [
      "backlog", "ready", "blocked", "executing",
      "pending_review", "revision_needed", "approved", "failed", "cancelled"
    ];

    validStatuses.forEach((status) => {
      expect(InternalStatusSchema.safeParse(status).success).toBe(true);
    });
  });

  it("rejects invalid statuses", () => {
    const invalidStatuses = ["todo", "in_progress", "done", "READY", ""];

    invalidStatuses.forEach((status) => {
      expect(InternalStatusSchema.safeParse(status).success).toBe(false);
    });
  });
});

describe("TaskSchema", () => {
  const validTask = {
    id: "550e8400-e29b-41d4-a716-446655440000",
    projectId: "550e8400-e29b-41d4-a716-446655440001",
    title: "Implement feature",
    description: null,
    internalStatus: "ready",
    externalStatus: "todo",
    priority: 1,
    wave: null,
    checkpointType: null,
    createdAt: "2024-01-15T10:30:00Z",
    updatedAt: "2024-01-15T10:30:00Z",
  };

  it("accepts valid task", () => {
    const result = TaskSchema.safeParse(validTask);
    expect(result.success).toBe(true);
  });

  it("rejects task without title", () => {
    const result = TaskSchema.safeParse({ ...validTask, title: "" });
    expect(result.success).toBe(false);
  });

  it("rejects task with invalid UUID", () => {
    const result = TaskSchema.safeParse({ ...validTask, id: "not-a-uuid" });
    expect(result.success).toBe(false);
  });

  it("rejects task with invalid status", () => {
    const result = TaskSchema.safeParse({ ...validTask, internalStatus: "invalid" });
    expect(result.success).toBe(false);
  });

  it("accepts task with checkpoint type", () => {
    const result = TaskSchema.safeParse({ ...validTask, checkpointType: "human-verify" });
    expect(result.success).toBe(true);
  });
});

// STEP 2: Run tests - should fail (schema doesn't exist yet)
// npm run test:unit -- task.test.ts

// STEP 3: Implement schema in task.ts

// STEP 4: Run tests - should pass
```

#### Pattern 4: React Hook Test

```typescript
// src/hooks/useTasks.test.ts

import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { useTasks, useTaskMutation } from "./useTasks";
import { api } from "@/lib/tauri";

// Mock the Tauri API
vi.mock("@/lib/tauri", () => ({
  api: {
    tasks: {
      list: vi.fn(),
      move: vi.fn(),
    },
  },
}));

// STEP 1: Write tests FIRST
describe("useTasks", () => {
  let queryClient: QueryClient;

  beforeEach(() => {
    queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    vi.clearAllMocks();
  });

  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );

  it("fetches tasks for project", async () => {
    const mockTasks = [
      { id: "1", title: "Task 1", internalStatus: "ready" },
      { id: "2", title: "Task 2", internalStatus: "executing" },
    ];
    vi.mocked(api.tasks.list).mockResolvedValue(mockTasks);

    const { result } = renderHook(() => useTasks("project-123"), { wrapper });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(api.tasks.list).toHaveBeenCalledWith("project-123");
    expect(result.current.data).toEqual(mockTasks);
  });

  it("returns empty array when no tasks", async () => {
    vi.mocked(api.tasks.list).mockResolvedValue([]);

    const { result } = renderHook(() => useTasks("project-123"), { wrapper });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(result.current.data).toEqual([]);
  });

  it("handles error state", async () => {
    vi.mocked(api.tasks.list).mockRejectedValue(new Error("Network error"));

    const { result } = renderHook(() => useTasks("project-123"), { wrapper });

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });

    expect(result.current.error?.message).toBe("Network error");
  });
});

describe("useTaskMutation", () => {
  // ... similar pattern for mutations
});
```

#### Pattern 5: React Component Test

```typescript
// src/components/tasks/TaskCard/TaskCard.test.tsx

import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { TaskCard } from "./TaskCard";
import { Task } from "@/types/task";

// STEP 1: Write tests FIRST - they define component behavior
describe("TaskCard", () => {
  const mockTask: Task = {
    id: "task-123",
    projectId: "project-456",
    title: "Implement login form",
    description: "Add login with email and password",
    internalStatus: "ready",
    externalStatus: "todo",
    priority: 1,
    wave: null,
    checkpointType: null,
    createdAt: "2024-01-15T10:30:00Z",
    updatedAt: "2024-01-15T10:30:00Z",
  };

  it("renders task title", () => {
    render(<TaskCard task={mockTask} />);
    expect(screen.getByText("Implement login form")).toBeInTheDocument();
  });

  it("renders status badge", () => {
    render(<TaskCard task={mockTask} />);
    expect(screen.getByText("ready")).toBeInTheDocument();
  });

  it("shows description on hover/expand", async () => {
    const user = userEvent.setup();
    render(<TaskCard task={mockTask} />);

    // Description not visible initially
    expect(screen.queryByText("Add login with email and password")).not.toBeInTheDocument();

    // Click to expand
    await user.click(screen.getByRole("button", { name: /expand/i }));

    // Description now visible
    expect(screen.getByText("Add login with email and password")).toBeInTheDocument();
  });

  it("calls onSelect when clicked", async () => {
    const user = userEvent.setup();
    const onSelect = vi.fn();

    render(<TaskCard task={mockTask} onSelect={onSelect} />);

    await user.click(screen.getByRole("article"));

    expect(onSelect).toHaveBeenCalledWith("task-123");
    expect(onSelect).toHaveBeenCalledTimes(1);
  });

  it("shows checkpoint indicator when task has checkpoint", () => {
    const taskWithCheckpoint = { ...mockTask, checkpointType: "human-verify" as const };

    render(<TaskCard task={taskWithCheckpoint} />);

    expect(screen.getByLabelText("Requires human verification")).toBeInTheDocument();
  });

  it("applies dragging styles when isDragging", () => {
    render(<TaskCard task={mockTask} isDragging />);

    const card = screen.getByRole("article");
    expect(card).toHaveClass("opacity-50");
  });
});

// STEP 2: Run tests - should fail
// STEP 3: Implement TaskCard component
// STEP 4: Run tests - should pass
```

### Agent TDD Enforcement

The Worker agent prompt MUST include TDD enforcement:

```xml
<tdd_requirement>
## MANDATORY: Test-Driven Development

You MUST follow TDD for EVERY implementation task. This is not optional.

### Before Writing ANY Implementation Code:

1. **Write failing tests first**
   - Tests define the expected behavior
   - Tests MUST fail before implementation exists
   - Commit tests separately: `test(scope): add tests for X`

2. **Run tests to confirm they fail**
   - Execute: `cargo test` (Rust) or `npm run test` (TypeScript)
   - If tests pass before implementation, they're not testing anything useful
   - Screenshot or log the failing test output

3. **Implement minimal code to pass tests**
   - Only write enough code to make tests pass
   - Don't add features not covered by tests
   - Commit implementation: `feat(scope): implement X`

4. **Run tests to confirm they pass**
   - ALL tests must pass, including previous tests
   - If any test fails, fix before proceeding
   - Screenshot or log the passing test output

5. **Refactor if needed** (optional)
   - Clean up code while keeping tests green
   - Commit: `refactor(scope): clean up X`

### Task Completion Criteria:

A task is NOT complete unless:
- [ ] Tests exist for all new functionality
- [ ] Tests were written BEFORE implementation
- [ ] All tests pass
- [ ] Test coverage meets minimum threshold

### Commit Sequence (Required):

For a feature "Add status validation":
```
1. test(status): add validation tests          # Tests written, they fail
2. feat(status): implement validation          # Implementation, tests pass
3. refactor(status): extract helper (optional) # Cleanup, tests still pass
```

NEVER combine test and implementation in the same commit.
</tdd_requirement>
```

### Test Commands in CI/Pre-commit

```yaml
# .github/workflows/test.yml
name: Tests
on: [push, pull_request]

jobs:
  rust-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Run Rust tests
        run: cargo test --all-features
        working-directory: src-tauri
      - name: Check coverage
        run: cargo tarpaulin --out Xml --fail-under 80

  typescript-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 18
      - run: npm ci
      - name: Run TypeScript tests
        run: npm run test:coverage
      - name: Check coverage threshold
        run: npm run test:coverage -- --coverage.thresholdAutoUpdate --coverage.lines 80

  e2e-tests:
    runs-on: macos-latest
    needs: [rust-tests, typescript-tests]
    steps:
      - uses: actions/checkout@v4
      - name: Build app
        run: npm run tauri build
      - name: Run E2E tests
        run: npm run test:e2e
```

```json
// package.json scripts
{
  "scripts": {
    "test": "npm run test:rust && npm run test:ts",
    "test:rust": "cd src-tauri && cargo test",
    "test:ts": "vitest run",
    "test:unit": "vitest run --coverage",
    "test:watch": "vitest",
    "test:e2e": "playwright test",
    "test:coverage": "vitest run --coverage --coverage.reporter=text --coverage.reporter=lcov"
  }
}
```

### Minimum Coverage Requirements

| Area | Minimum Coverage | Rationale |
|------|------------------|-----------|
| Rust Core (`core/`) | 90% | Business logic must be thoroughly tested |
| Rust Commands | 80% | Integration points need solid coverage |
| TypeScript Types | 100% | Schemas are contracts, must be fully tested |
| React Hooks | 85% | Hooks contain logic, need high coverage |
| React Components | 70% | Visual components, test behavior not pixels |
| E2E Critical Paths | 100% | User-facing flows must all be tested |

---

## Cost-Optimized Integration Testing (MANDATORY)

**Critical**: RalphX integration tests that spawn Claude agents or trigger loops MUST use minimal prompts to avoid expensive API costs.

### The Problem

Integration tests that verify:
- Agent spawning works correctly
- Loop iteration mechanics
- State machine transitions with agent callbacks
- Agent-browser visual verification

...all trigger **real Claude API calls**. If we use realistic prompts, testing becomes prohibitively expensive.

### The Solution: Minimal Test Prompts

**Rule**: Use the simplest possible prompt that verifies the functionality works.

#### Agent Spawning Tests

```rust
// ❌ BAD - Expensive, tests functionality we don't care about
#[tokio::test]
async fn test_worker_agent_spawns() {
    let task = create_test_task("Implement authentication flow with OAuth2");
    let result = agent_spawner.spawn("worker", &task).await;
    // This will make the agent actually try to implement OAuth!
}

// ✅ GOOD - Minimal cost, tests only spawning mechanics
#[tokio::test]
async fn test_worker_agent_spawns() {
    let task = create_test_task("Respond with: HELLO_WORLD_TEST_MARKER");
    let result = agent_spawner.spawn("worker", &task).await;
    assert!(result.output.contains("HELLO_WORLD_TEST_MARKER"));
    // Verifies: agent spawned, received prompt, returned output
}
```

#### Loop Iteration Tests

```rust
// ❌ BAD - Each iteration is expensive
#[tokio::test]
async fn test_loop_runs_multiple_iterations() {
    let prd = create_prd_with_real_tasks();  // Complex tasks
    let loop_runner = LoopRunner::new(prd, max_iterations: 5);
    loop_runner.run().await;
}

// ✅ GOOD - Minimal cost per iteration
#[tokio::test]
async fn test_loop_runs_multiple_iterations() {
    let prd = create_minimal_test_prd();  // Contains: "Echo 'ITERATION_1'", etc.
    let loop_runner = LoopRunner::new(prd, max_iterations: 3);
    let result = loop_runner.run().await;

    assert_eq!(result.iterations_completed, 3);
    assert!(result.logs.iter().any(|l| l.contains("ITERATION_1")));
    assert!(result.logs.iter().any(|l| l.contains("ITERATION_2")));
}
```

#### State Machine Agent Callback Tests

```rust
// ❌ BAD - QA agent does expensive analysis
#[tokio::test]
async fn test_qa_refining_spawns_agent() {
    let task = create_task_with_real_implementation();
    state_machine.transition(&TaskEvent::QaRefinementComplete).await;
}

// ✅ GOOD - QA agent just echoes back
#[tokio::test]
async fn test_qa_refining_spawns_agent() {
    let task = Task {
        description: "TEST_TASK_MARKER".to_string(),
        qa_plan: Some("Verify TEST_TASK_MARKER appears in output"),
        ..Default::default()
    };

    // Override the QA refiner prompt for testing
    let spawner = MockAgentSpawner::with_test_prompt(
        "Echo back: QA_REFINE_COMPLETE_MARKER"
    );

    let result = state_machine
        .with_spawner(spawner)
        .transition(&TaskEvent::ExecutionDone)
        .await;

    assert!(matches!(result.state, State::QaRefining));
}
```

### Test Prompt Constants

Define reusable minimal prompts:

```rust
// src-tauri/src/testing/test_prompts.rs

pub mod test_prompts {
    /// Minimal prompt that verifies agent received input and can respond
    pub const ECHO_MARKER: &str = "Respond with exactly: TEST_ECHO_OK";

    /// Minimal prompt for testing worker agent spawning
    pub const WORKER_SPAWN_TEST: &str =
        "Respond with exactly: WORKER_SPAWNED_SUCCESSFULLY";

    /// Minimal prompt for testing QA prep agent
    pub const QA_PREP_TEST: &str =
        "Respond with exactly: QA_PREP_COMPLETE";

    /// Minimal prompt for testing reviewer agent
    pub const REVIEWER_TEST: &str =
        "Respond with exactly: REVIEW_COMPLETE_APPROVED";

    /// Minimal prompt for loop iteration testing
    pub fn iteration_test_prompt(n: u32) -> String {
        format!("Respond with exactly: ITERATION_{}_COMPLETE", n)
    }

    /// Verify expected marker in output
    pub fn assert_marker(output: &str, marker: &str) {
        assert!(
            output.contains(marker),
            "Expected output to contain '{}', got: {}",
            marker,
            &output[..output.len().min(200)]
        );
    }
}
```

### Agent-Browser Visual Tests

```typescript
// ❌ BAD - Complex interactions, many API calls
test("user can create project and run loop", async () => {
  await page.fill('[data-testid="project-name"]', 'My Complex Project');
  await page.fill('[data-testid="project-prd"]', realPrdContent);  // Expensive!
  await page.click('[data-testid="start-loop"]');
  await page.waitForSelector('[data-testid="iteration-complete"]');
});

// ✅ GOOD - Verify UI mechanics only, minimal agent interaction
test("user can create project and trigger loop start", async () => {
  // Use test mode that mocks agent responses
  await page.evaluate(() => window.__TEST_MODE__ = true);

  await page.fill('[data-testid="project-name"]', 'Test Project');
  await page.fill('[data-testid="project-prd"]', 'Echo: TEST_PRD');
  await page.click('[data-testid="start-loop"]');

  // Verify UI state changed correctly (don't wait for real agent)
  await expect(page.locator('[data-testid="loop-status"]'))
    .toHaveText('Running');
  await expect(page.locator('[data-testid="iteration-count"]'))
    .toHaveText('1');
});
```

### Test Mode Configuration

```typescript
// src/lib/config.ts

export const TEST_CONFIG = {
  // When true, agents use minimal echo prompts
  USE_MINIMAL_PROMPTS: process.env.NODE_ENV === 'test',

  // Maximum tokens for test prompts (keeps costs low)
  TEST_MAX_TOKENS: 50,

  // Test markers to verify in output
  MARKERS: {
    WORKER: 'WORKER_TEST_OK',
    QA_PREP: 'QA_PREP_TEST_OK',
    QA_REFINE: 'QA_REFINE_TEST_OK',
    QA_TEST: 'QA_TEST_TEST_OK',
    REVIEWER: 'REVIEWER_TEST_OK',
  }
};

// src-tauri/src/config.rs

pub struct TestConfig {
    pub use_minimal_prompts: bool,
    pub test_max_tokens: u32,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            use_minimal_prompts: cfg!(test),
            test_max_tokens: 50,
        }
    }
}
```

### Cost Estimation for Test Suites

| Test Type | Real Prompts (est.) | Minimal Prompts (est.) | Savings |
|-----------|---------------------|------------------------|---------|
| Agent spawn (1 test) | ~$0.05 | ~$0.001 | 98% |
| Loop 3 iterations | ~$0.30 | ~$0.005 | 98% |
| Full integration suite (50 tests) | ~$5.00 | ~$0.10 | 98% |
| CI run (all tests) | ~$10.00 | ~$0.20 | 98% |

### PRD Task Requirement

When creating tasks for RalphX development:

```markdown
## Task: Implement Worker Agent Spawning

**Steps:**
1. Create AgentSpawner trait with spawn() method
2. Implement ClaudeAgentSpawner
3. Write integration tests using minimal test prompts (see Cost-Optimized Testing section)
4. Verify spawning works with marker: WORKER_SPAWNED_SUCCESSFULLY

**Testing Requirements:**
- Unit tests: Mock the agent response
- Integration tests: Use TEST_PROMPTS.WORKER_SPAWN_TEST
- DO NOT use realistic prompts in integration tests
```

---

## Visual Verification Layer (Agent-Browser)

**After TDD passes, UI tasks require visual verification using agent-browser.**

### Complete Testing Flow

```
┌─────────────────────────────────────────────────────────────┐
│  COMPLETE TESTING WORKFLOW FOR UI TASKS                     │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  1. TDD CYCLE (Required First)                              │
│     ├─ RED: Write failing tests                             │
│     ├─ GREEN: Implement to pass tests                       │
│     └─ REFACTOR: Clean up                                   │
│                                                             │
│  2. RUN ALL TESTS                                           │
│     ├─ cargo test (Rust)                                    │
│     ├─ npm run test (TypeScript)                            │
│     └─ ALL MUST PASS before proceeding                      │
│                                                             │
│  3. VISUAL VERIFICATION (UI tasks only)                     │
│     ├─ Start dev server: npm run tauri dev                  │
│     ├─ Open in browser: agent-browser open http://localhost:1420 │
│     ├─ Take snapshot: agent-browser snapshot -i -c          │
│     ├─ Capture screenshot: agent-browser screenshot screenshots/[task].png │
│     ├─ Verify layout and behavior                           │
│     └─ Document in activity.md                              │
│                                                             │
│  4. COMMIT SEQUENCE                                         │
│     ├─ test(scope): add tests for X                         │
│     ├─ feat(scope): implement X                             │
│     └─ verify(scope): add visual verification for X         │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Setup Instructions (Modify Current Repo)

#### Step 1: Install agent-browser globally

```bash
npm install -g agent-browser
```

#### Step 2: Create the agent-browser skill

Create `.claude/skills/agent-browser/SKILL.md`:

```markdown
---
name: agent-browser
description: Browser automation for visual testing and verification
---

# Agent Browser Skill

Headless browser automation for visual verification of UI implementations.

## Quick Reference

### Navigation
- `agent-browser open <url>` — Open URL
- `agent-browser close` — Close browser
- `agent-browser reload` — Refresh page

### Page Analysis
- `agent-browser snapshot` — Full DOM snapshot with element refs (@e1, @e2...)
- `agent-browser snapshot -i` — Interactive elements only (recommended)
- `agent-browser snapshot -c` — Compact output
- `agent-browser snapshot -i -c` — Interactive + compact (best for verification)

### Screenshots
- `agent-browser screenshot <path.png>` — Capture viewport
- `agent-browser screenshot --full <path.png>` — Full page screenshot

### Interactions
- `agent-browser click @e1` — Click element by reference
- `agent-browser fill @e1 "text"` — Fill input field
- `agent-browser type @e1 "text"` — Type character by character
- `agent-browser press Enter` — Press key
- `agent-browser hover @e1` — Hover over element
- `agent-browser scroll @e1` — Scroll element into view

### Data Extraction
- `agent-browser get text @e1` — Get text content
- `agent-browser get value @e1` — Get input value
- `agent-browser get attr @e1 href` — Get attribute

### State Verification
- `agent-browser is visible @e1` — Check visibility
- `agent-browser is enabled @e1` — Check if enabled
- `agent-browser is checked @e1` — Check checkbox state

### Wait Conditions
- `agent-browser wait @e1` — Wait for element
- `agent-browser wait 2000` — Wait milliseconds
- `agent-browser wait --load` — Wait for page load

## Verification Workflow

1. Start app: `npm run tauri dev`
2. Open browser: `agent-browser open http://localhost:1420`
3. Analyze page: `agent-browser snapshot -i -c`
4. Capture proof: `agent-browser screenshot screenshots/[task-name].png`
5. Test interactions if applicable
6. Close: `agent-browser close`
```

#### Step 3: Update .claude/settings.json

Add agent-browser permissions:

```json
{
  "permissions": {
    "allow": [
      "Bash(npm run:*)",
      "Bash(cargo:*)",
      "Bash(git:*)",
      "Bash(agent-browser:*)",
      "Bash(agent-browser open:*)",
      "Bash(agent-browser snapshot:*)",
      "Bash(agent-browser screenshot:*)",
      "Bash(agent-browser click:*)",
      "Bash(agent-browser fill:*)",
      "Bash(agent-browser close:*)",
      "Bash(agent-browser get:*)",
      "Bash(agent-browser is:*)",
      "Bash(agent-browser wait:*)"
    ]
  }
}
```

#### Step 4: Create screenshots directory

```bash
mkdir -p screenshots
touch screenshots/.gitkeep
```

#### Step 5: Update PROMPT.md with verification instructions

Add to PROMPT.md after the task execution instructions:

```markdown
## Visual Verification (UI Tasks)

After ALL tests pass, verify UI changes visually:

### 1. Start the development server
\`\`\`bash
npm run tauri dev
\`\`\`

### 2. Open in headless browser
\`\`\`bash
agent-browser open http://localhost:1420
\`\`\`

### 3. Analyze the page structure
\`\`\`bash
agent-browser snapshot -i -c
\`\`\`

### 4. Capture screenshot as proof
\`\`\`bash
agent-browser screenshot screenshots/[task-name].png
\`\`\`

### 5. Verify specific behaviors (examples)
\`\`\`bash
# Check if element exists and is visible
agent-browser is visible @e1

# Test click interaction
agent-browser click @e1
agent-browser screenshot screenshots/[task-name]-after-click.png

# Verify text content
agent-browser get text @e1
\`\`\`

### 6. Close browser
\`\`\`bash
agent-browser close
\`\`\`

### 7. Document in activity.md
Include:
- Screenshot filename
- What was verified
- Any issues found and resolved
```

### Visual Verification Test Patterns

#### Pattern 1: Component Renders Correctly

```bash
# Start app
npm run tauri dev &
sleep 5

# Open and verify
agent-browser open http://localhost:1420
agent-browser wait --load
agent-browser snapshot -i -c

# Verify component exists
agent-browser is visible "[data-testid='task-board']"

# Screenshot proof
agent-browser screenshot screenshots/task-board-renders.png
agent-browser close
```

#### Pattern 2: Kanban Drag-Drop Works

```bash
agent-browser open http://localhost:1420
agent-browser wait --load

# Find task card
agent-browser snapshot -i -c
# Assume @e5 is a task card, @e8 is target column

# Drag and drop
agent-browser drag @e5 @e8

# Verify task moved
agent-browser screenshot screenshots/kanban-drag-drop.png

# Verify task is in new column
agent-browser get text @e8  # Should contain task title

agent-browser close
```

#### Pattern 3: Form Submission

```bash
agent-browser open http://localhost:1420/new-task
agent-browser wait --load

# Fill form
agent-browser fill "[name='title']" "Test Task"
agent-browser fill "[name='description']" "Test description"
agent-browser screenshot screenshots/task-form-filled.png

# Submit
agent-browser click "[type='submit']"
agent-browser wait 1000

# Verify success
agent-browser screenshot screenshots/task-form-submitted.png
agent-browser is visible "[data-testid='success-message']"

agent-browser close
```

#### Pattern 4: Status Change Side Effects

```bash
agent-browser open http://localhost:1420
agent-browser wait --load

# Find task in Ready column
agent-browser snapshot -i -c

# Move task to Executing (should trigger agent spawn)
agent-browser click "[data-testid='task-123-move-executing']"
agent-browser wait 2000

# Verify status changed
agent-browser screenshot screenshots/task-status-executing.png

# Verify agent activity appears
agent-browser is visible "[data-testid='agent-activity-stream']"
agent-browser screenshot screenshots/agent-spawned.png

agent-browser close
```

### Activity Log Format with Screenshots

```markdown
## 2024-01-15 - Iteration 5

### Task: Implement TaskBoard component

**TDD:**
- [x] Wrote tests: `src/components/tasks/TaskBoard/TaskBoard.test.tsx`
- [x] Tests failed (RED): 5 failing tests
- [x] Implemented: `src/components/tasks/TaskBoard/TaskBoard.tsx`
- [x] Tests passed (GREEN): All 5 tests passing

**Visual Verification:**
- [x] Screenshot: `screenshots/task-board-renders.png`
- [x] Verified: Board renders with 7 columns
- [x] Verified: Task cards display correctly
- [x] Verified: Drag handles visible

**Commits:**
- `test(tasks): add TaskBoard component tests`
- `feat(tasks): implement TaskBoard component`
- `verify(tasks): add visual verification for TaskBoard`

**Status:** Task marked `"passes": true`
```

### When Visual Verification is Required

| Task Type | TDD Required | Visual Verification Required |
|-----------|--------------|------------------------------|
| Rust core logic | Yes | No |
| TypeScript types/schemas | Yes | No |
| React hooks (no UI) | Yes | No |
| React components | Yes | **Yes** |
| Tauri commands | Yes | No (unless UI-facing) |
| Layout/styling changes | Yes (snapshot tests) | **Yes** |
| User interactions | Yes | **Yes** |
| Agent activity stream | Yes | **Yes** |
| Settings modal | Yes | **Yes** |

---

## Built-in QA System (Two-Phase Approach)

RalphX provides built-in QA capabilities that can be enabled per-task or globally.

### Why QA Prep Triggers at PLANNED (Not Earlier)

QA Prep generates acceptance criteria and test steps. This happens as a **side effect of reaching PLANNED** status because:

1. **Resource efficiency**: We don't want to allocate agent resources for QA prep on tasks that may never be executed (backlog/todo items may be reprioritized or removed)
2. **Just-in-time preparation**: QA criteria should reflect the task's current state when it's actually going to be worked on
3. **Clear intent signal**: Moving to PLANNED signals "execute this task" - that's when we commit resources

### Parallel Execution Model

QA Prep and task execution run **concurrently** (non-blocking):

```
PLANNED (user action)
    │
    ├──→ [Spawn QA Prep Agent] ──→ Generates acceptance criteria ──→ Stores in task_qa table
    │                                    (runs in background)
    │
    └──→ [Auto-pick up for execution] ──→ IN_PROGRESS ──→ DONE
                                                            │
                                                            ▼
                                                    QA_TESTING
                                                    • Reads QA plan from task_qa
                                                    • Refines based on git diff (actual implementation)
                                                    • Runs browser tests
```

**Benefits of parallel execution**:
- No delay waiting for QA Prep to complete before work starts
- QA Prep can analyze codebase context while worker executes
- If execution finishes before QA Prep, QA Testing waits for the plan
- Refinement step ensures tests match actual implementation, not just original intent

### TDD vs QA: Different Purposes

| Aspect | TDD (Unit/Integration) | QA (Visual/E2E) |
|--------|------------------------|-----------------|
| **When** | Before implementation | After implementation |
| **Tests** | Code behavior | User experience |
| **Speed** | Fast (ms) | Slower (seconds) |
| **Scope** | Functions, components | Full application |
| **Required** | Always for RalphX development | Optional, per-task |

**For RalphX's own development**: TDD is mandatory, QA is mandatory for UI tasks.
**For user projects using RalphX**: TDD recommended but configurable, QA optional per-task.

### QA Configuration

#### Global Settings

```typescript
interface QASettings {
  // Global QA toggle
  qa_enabled: boolean;           // Default: true

  // Automatic QA for certain task types
  auto_qa_for_ui_tasks: boolean; // Default: true
  auto_qa_for_api_tasks: boolean; // Default: false

  // QA prep (acceptance criteria generation)
  qa_prep_enabled: boolean;      // Default: true

  // Browser testing
  browser_testing_enabled: boolean; // Default: true
  browser_testing_url: string;   // Default: http://localhost:1420
}
```

#### Task-Level Configuration

```sql
-- Extended task schema for QA
ALTER TABLE tasks ADD COLUMN needs_qa BOOLEAN DEFAULT NULL;  -- NULL = use global setting
ALTER TABLE tasks ADD COLUMN qa_prep_status TEXT;  -- 'pending' | 'running' | 'completed' | 'failed' (background prep status)
ALTER TABLE tasks ADD COLUMN qa_test_status TEXT;  -- 'pending' | 'waiting_for_prep' | 'running' | 'passed' | 'failed'

-- QA artifacts
CREATE TABLE task_qa (
  id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(id),

  -- Phase 1: QA Prep (runs in parallel with execution)
  acceptance_criteria TEXT,      -- JSON array of criteria
  qa_test_steps TEXT,            -- JSON array of test steps (initial)
  prep_agent_id TEXT,            -- Agent that generated this
  prep_started_at DATETIME,      -- When QA Prep started
  prep_completed_at DATETIME,    -- When QA Prep finished (may be after task DONE)

  -- Phase 2: QA Refinement (after execution completes)
  actual_implementation TEXT,    -- Summary of what was actually done (from git diff)
  refined_test_steps TEXT,       -- Test steps updated based on actual implementation
  refinement_agent_id TEXT,
  refinement_completed_at DATETIME,

  -- Phase 3: Test Execution (browser tests)
  test_results TEXT,             -- JSON array of test results
  screenshots TEXT,              -- JSON array of screenshot paths
  test_agent_id TEXT,
  test_completed_at DATETIME,

  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

### Two-Phase QA Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  COMPLETE TASK LIFECYCLE WITH QA (Parallel Execution Model)                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────┐                                                                │
│  │ PLANNED │ ← User drags task here                                        │
│  └────┬────┘                                                                │
│       │                                                                     │
│       ├───────────────────────────────────────┐                             │
│       │                                       │                             │
│       ▼  [Side Effect 1]                      ▼  [Side Effect 2]            │
│  ┌─────────────┐                        ┌───────────────────┐               │
│  │ IN_PROGRESS │                        │ QA PREP (bg task) │               │
│  │ Worker Agent│                        │ Runs in parallel  │               │
│  └──────┬──────┘                        └─────────┬─────────┘               │
│         │                                         │                         │
│         │ • TDD: Write tests first                │ • Reads task description│
│         │ • Implement to pass tests               │ • Analyzes codebase     │
│         │ • Commit changes                        │ • Writes acceptance     │
│         │                                         │   criteria              │
│         │                                         │ • Defines QA test steps │
│         │                                         │ • Stores in task_qa     │
│         │                                         │                         │
│         ▼                                         │                         │
│  ┌──────┐                                         │                         │
│  │ DONE │ Implementation complete                 │                         │
│  └───┬──┘                                         │                         │
│      │                                            │                         │
│      │ [Waits for QA Prep if still running] ◄─────┘                         │
│      │                                                                      │
│      ▼  [Side Effect: spawn_qa_eval if needs_qa]                           │
│  ┌────────────┐                                                             │
│  │ QA_TESTING │ QA Refinement + Execution                                  │
│  └─────┬──────┘                                                             │
│        │  PHASE 2A: Refinement                                              │
│        │  • Read QA plan from task_qa table                                 │
│        │  • Review actual implementation (git diff)                         │
│        │  • Refine test steps for what was ACTUALLY done                    │
│        │  (Implementation may differ from original task description)        │
│        │                                                                    │
│        │  PHASE 2B: Execution                                               │
│        │  • Start dev server                                                │
│        │  • Run agent-browser tests with refined steps                      │
│        │  • Capture screenshots                                             │
│        │  • Record pass/fail for each step                                  │
│        ▼                                                                    │
│  ┌───────────┐                                                              │
│  │ IN_REVIEW │ AI Reviewer checks everything                               │
│  └─────┬─────┘                                                              │
│        │  • Code quality                                                    │
│        │  • Test coverage                                                   │
│        │  • QA results                                                      │
│        ▼                                                                    │
│  ┌──────────┐                                                               │
│  │ APPROVED │ or REVISION_NEEDED                                           │
│  └──────────┘                                                               │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Phase 1: QA Prep Agent (Background Task)

**Trigger**: Side effect of task reaching `PLANNED` status (if QA enabled).

**Execution model**: Runs in parallel with task execution (non-blocking).

When a task moves to PLANNED:
1. System checks if `needs_qa` is true (or global `qa_prep_enabled`)
2. If yes, spawns QA Prep agent as **background task** (does NOT block execution)
3. Task immediately transitions to `IN_PROGRESS` (worker picks it up)
4. QA Prep and execution run concurrently
5. When task reaches `DONE`, QA Testing waits for QA Prep if still running

```typescript
// QA Prep Agent Profile
const qaPrepProfile: AgentProfile = {
  id: "qa-prep",
  name: "QA Prep Agent",
  role: "qa_prep",
  claudeCode: {
    agentDefinition: ".claude/agents/qa-prep.md",
    skills: ["acceptance-criteria-writing", "qa-step-generation"],
  },
  execution: {
    model: "sonnet",
    maxIterations: 10,
    timeoutMinutes: 5,
    permissionMode: "default",
  },
  io: {
    inputArtifactTypes: ["task_spec", "context"],
    outputArtifactTypes: ["acceptance_criteria", "qa_steps"],
  },
};
```

#### QA Prep Agent Definition

`.claude/agents/qa-prep.md`:
```markdown
---
name: qa-prep
description: Prepares acceptance criteria and QA test steps before task execution
tools: Read, Grep, Glob
disallowedTools: Write, Edit, Bash
---

You are a QA preparation specialist. Your job is to analyze a task
and create clear, testable acceptance criteria and QA steps BEFORE
the developer starts working.

## Input
You receive:
- Task title and description
- Related files in the codebase
- UI mockups or designs (if available)

## Output
You must produce:

### 1. Acceptance Criteria
Specific, measurable conditions that must be true when the task is complete.

Format:
```json
{
  "acceptance_criteria": [
    {
      "id": "AC1",
      "description": "User can see the task board with 7 columns",
      "testable": true,
      "type": "visual"
    },
    {
      "id": "AC2",
      "description": "Dragging a task to 'Planned' column triggers execution",
      "testable": true,
      "type": "behavior"
    }
  ]
}
```

### 2. QA Test Steps
Specific browser-based tests to verify each acceptance criterion.

Format:
```json
{
  "qa_steps": [
    {
      "id": "QA1",
      "criteria_id": "AC1",
      "description": "Verify task board renders with correct columns",
      "commands": [
        "agent-browser open http://localhost:1420",
        "agent-browser wait --load",
        "agent-browser snapshot -i -c",
        "agent-browser is visible [data-testid='column-draft']",
        "agent-browser is visible [data-testid='column-planned']",
        "agent-browser screenshot screenshots/task-board-columns.png"
      ],
      "expected": "All 7 columns visible: Draft, Backlog, Todo, Planned, In Progress, In Review, Done"
    }
  ]
}
```

## Guidelines
- Be specific and testable
- Use data-testid attributes when possible
- Include both positive and negative test cases
- Consider edge cases
- Keep steps atomic (one verification per step)
```

### Phase 2: QA Evaluation + Execution Agent

Runs when task transitions: `DONE → QA_TESTING`

```typescript
// QA Executor Agent Profile
const qaExecutorProfile: AgentProfile = {
  id: "qa-executor",
  name: "QA Executor Agent",
  role: "qa_executor",
  claudeCode: {
    agentDefinition: ".claude/agents/qa-executor.md",
    skills: ["agent-browser", "qa-evaluation"],
  },
  execution: {
    model: "sonnet",
    maxIterations: 30,
    timeoutMinutes: 15,
    permissionMode: "acceptEdits",
  },
  io: {
    inputArtifactTypes: ["task_spec", "acceptance_criteria", "qa_steps", "code_change"],
    outputArtifactTypes: ["qa_results", "screenshots"],
  },
};
```

#### QA Executor Agent Definition

`.claude/agents/qa-executor.md`:
```markdown
---
name: qa-executor
description: Evaluates actual implementation and executes QA tests
tools: Read, Grep, Glob, Bash
skills:
  - agent-browser
---

You are a QA execution specialist. Your job is to:
1. Evaluate what was actually implemented (may differ from plan)
2. Update test steps based on actual implementation
3. Execute browser-based QA tests
4. Report results

## Phase 2A: Evaluation

First, understand what was actually implemented:

1. Read the git diff:
   ```bash
   git diff HEAD~1 --name-only
   git diff HEAD~1
   ```

2. Compare to original acceptance criteria

3. Update test steps if needed:
   - Add tests for features that were added beyond the plan
   - Remove tests for features that weren't implemented
   - Adjust selectors if UI structure differs

Output updated test steps:
```json
{
  "revised_qa_steps": [...],
  "implementation_notes": "Developer added X but didn't implement Y",
  "additional_tests_needed": [...]
}
```

## Phase 2B: Execution

Run each QA test step:

1. Start the development server:
   ```bash
   npm run tauri dev &
   sleep 10  # Wait for compilation
   ```

2. Execute each test step:
   ```bash
   agent-browser open http://localhost:1420
   agent-browser wait --load
   # ... run test commands
   ```

3. Capture results for each step:
   ```json
   {
     "step_id": "QA1",
     "status": "passed" | "failed",
     "actual": "What actually happened",
     "expected": "What was expected",
     "screenshot": "screenshots/qa1-result.png",
     "error": null | "Error message if failed"
   }
   ```

4. Close browser and stop server:
   ```bash
   agent-browser close
   # Kill dev server
   ```

## Output Format

```json
{
  "qa_results": {
    "task_id": "task-123",
    "overall_status": "passed" | "failed",
    "total_steps": 5,
    "passed_steps": 4,
    "failed_steps": 1,
    "steps": [
      {
        "step_id": "QA1",
        "status": "passed",
        "screenshot": "screenshots/qa1-result.png"
      },
      {
        "step_id": "QA2",
        "status": "failed",
        "expected": "Button should be disabled",
        "actual": "Button is enabled",
        "screenshot": "screenshots/qa2-failed.png"
      }
    ]
  }
}
```
```

### Updated Internal Status Side Effects

```typescript
// Updated side effects to include QA phases

const SIDE_EFFECTS: Record<string, SideEffect[]> = {
  // PLANNED triggers QA prep (if enabled)
  "ready->planned": [
    { type: "check_qa_needed", action: "evaluate_task_for_qa" },
  ],

  "planned->preparing": [
    { type: "spawn_agent", profile: "qa-prep" },
  ],

  "preparing->in_progress": [
    { type: "spawn_agent", profile: "worker" },
  ],

  // If QA not needed, skip directly to in_progress
  "planned->in_progress": [
    { type: "spawn_agent", profile: "worker" },
  ],

  // DONE triggers QA evaluation (if enabled)
  "in_progress->done": [
    { type: "check_qa_needed", action: "route_to_qa_or_review" },
  ],

  "done->qa_testing": [
    { type: "spawn_agent", profile: "qa-executor" },
  ],

  "qa_testing->in_review": [
    { type: "spawn_agent", profile: "reviewer" },
  ],

  // If QA not needed, skip directly to review
  "done->in_review": [
    { type: "spawn_agent", profile: "reviewer" },
  ],
};
```

### UI for Task QA Configuration

#### Task Card QA Badge

```typescript
// Show QA status on task card
function TaskQABadge({ task }: { task: Task }) {
  if (!task.needs_qa) return null;

  const statusColors = {
    pending: "bg-gray-500",
    preparing: "bg-yellow-500",
    ready: "bg-blue-500",
    testing: "bg-purple-500",
    passed: "bg-green-500",
    failed: "bg-red-500",
  };

  return (
    <span className={`badge ${statusColors[task.qa_status]}`}>
      QA: {task.qa_status}
    </span>
  );
}
```

#### Task Detail QA Panel

```
┌─────────────────────────────────────────────────────────────┐
│  Task: Implement TaskBoard component                        │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  [Details] [Activity] [QA]                                 │
│                                                             │
│  ─────────────────────────────────────────────────────────  │
│                                                             │
│  QA Status: ● Testing                                       │
│                                                             │
│  Acceptance Criteria:                                       │
│  ✓ AC1: Task board renders with 7 columns                  │
│  ✓ AC2: Tasks display in correct columns                   │
│  ○ AC3: Drag-drop moves tasks between columns              │
│  ○ AC4: Status badge shows on each task                    │
│                                                             │
│  ─────────────────────────────────────────────────────────  │
│                                                             │
│  Test Results:                                              │
│                                                             │
│  ✓ QA1: Board renders correctly                            │
│    [View Screenshot]                                        │
│                                                             │
│  ✓ QA2: Tasks in correct columns                           │
│    [View Screenshot]                                        │
│                                                             │
│  ✗ QA3: Drag-drop not working                              │
│    Expected: Task moves to new column                       │
│    Actual: Task snaps back to original position            │
│    [View Screenshot]                                        │
│                                                             │
│  ○ QA4: Pending...                                          │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Settings UI for QA

```
┌─────────────────────────────────────────────────────────────┐
│  Settings > Quality Assurance                               │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  QA System                                                  │
│  ─────────────────────────────────────────────────────────  │
│                                                             │
│  [✓] Enable QA System                                       │
│      Automatically prepare and run QA tests for tasks       │
│                                                             │
│  Automatic QA for:                                          │
│  [✓] UI/Frontend tasks                                      │
│  [ ] API/Backend tasks                                      │
│  [ ] All tasks                                              │
│                                                             │
│  ─────────────────────────────────────────────────────────  │
│                                                             │
│  QA Phases                                                  │
│  ─────────────────────────────────────────────────────────  │
│                                                             │
│  [✓] QA Prep (generate acceptance criteria before work)    │
│  [✓] QA Evaluation (review actual implementation)          │
│  [✓] Browser Testing (run agent-browser tests)             │
│                                                             │
│  ─────────────────────────────────────────────────────────  │
│                                                             │
│  Browser Testing                                            │
│  ─────────────────────────────────────────────────────────  │
│                                                             │
│  Dev Server URL: [http://localhost:1420_______]            │
│  Start Command:  [npm run tauri dev______________]         │
│  Wait Time:      [10] seconds                               │
│                                                             │
│                              [Cancel]  [Save Settings]      │
└─────────────────────────────────────────────────────────────┘
```

### Task Creation with QA Option

```
┌─────────────────────────────────────────────────────────────┐
│  Create Task                                                │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Title: [Implement TaskBoard component_________]           │
│                                                             │
│  Description:                                               │
│  [Create a Kanban board with drag-drop support...         ]│
│  [                                                        ]│
│                                                             │
│  Category: [Feature_____________▼]                         │
│                                                             │
│  ─────────────────────────────────────────────────────────  │
│                                                             │
│  Quality Assurance:                                         │
│                                                             │
│  [✓] Enable QA for this task                               │
│      • Acceptance criteria will be generated before work   │
│      • Browser tests will run after implementation         │
│                                                             │
│  ─────────────────────────────────────────────────────────  │
│                                                             │
│                              [Cancel]  [Create Task]        │
└─────────────────────────────────────────────────────────────┘
```

### Bulk QA Toggle in Planning

During planning/PRD creation, user can set QA defaults:

```
┌─────────────────────────────────────────────────────────────┐
│  Planning: Create Tasks from PRD                            │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Generated 12 tasks from PRD                                │
│                                                             │
│  QA Defaults for these tasks:                               │
│                                                             │
│  ○ No QA (fastest, trust implementation)                   │
│  ○ QA for UI tasks only (recommended)                      │
│  ○ QA for all tasks (most thorough)                        │
│                                                             │
│  Tasks with QA enabled: 8 / 12                              │
│                                                             │
│  [Edit individual task QA settings...]                      │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Complete Status Flow with QA

```
┌─────────┐
│  DRAFT  │ ← Ideas, brainstorming
└────┬────┘
     │ User confirms
     ▼
┌─────────┐
│ BACKLOG │ ← Confirmed but deferred
└────┬────┘
     │ User prioritizes
     ▼
┌─────────┐
│  TODO   │ ← Ready to schedule
└────┬────┘
     │ User drags to Planned
     ▼
┌─────────┐     needs_qa=true      ┌───────────┐
│ PLANNED │ ──────────────────────▶│ PREPARING │
└────┬────┘                        └─────┬─────┘
     │ needs_qa=false                    │ QA Prep complete
     │                                   ▼
     │                             ┌─────────────┐
     └────────────────────────────▶│ IN_PROGRESS │
                                   └──────┬──────┘
                                          │ Worker complete
                                          ▼
                                   ┌──────┐
                                   │ DONE │
                                   └───┬──┘
                                       │
          needs_qa=false               │ needs_qa=true
               │                       │
               │              ┌────────┴────────┐
               │              ▼                 │
               │        ┌────────────┐          │
               │        │ QA_TESTING │          │
               │        └─────┬──────┘          │
               │              │ QA complete     │
               │              ▼                 │
               └─────────▶┌───────────┐◀────────┘
                          │ IN_REVIEW │
                          └─────┬─────┘
                                │
               ┌────────────────┼────────────────┐
               ▼                ▼                ▼
        ┌──────────┐    ┌─────────────┐   ┌──────────────────┐
        │ APPROVED │    │NEEDS_CHANGES│   │NEEDS_HUMAN_REVIEW│
        └──────────┘    └─────────────┘   └──────────────────┘
```

### Agent Prompt Addition for Visual Verification

Add to Worker agent prompt:

```xml
<visual_verification>
## Visual Verification (UI Tasks Only)

After ALL tests pass, if this task involves UI changes:

1. Start dev server: `npm run tauri dev`
2. Wait for compilation
3. Open: `agent-browser open http://localhost:1420`
4. Snapshot: `agent-browser snapshot -i -c`
5. Screenshot: `agent-browser screenshot screenshots/[task-name].png`
6. Verify expected elements are visible
7. Test key interactions
8. Close: `agent-browser close`
9. Document screenshot in activity.md

Task is NOT complete until:
- [ ] All unit/integration tests pass
- [ ] Visual verification screenshot captured
- [ ] Screenshot shows correct rendering
- [ ] Activity.md updated with screenshot reference
</visual_verification>
```

---

---

### Rust Backend Best Practices

#### Module Organization

```
src-tauri/src/
├── main.rs                 # Entry point only (~50 lines)
├── lib.rs                  # Re-exports, feature flags
├── error.rs                # Unified error types
├── commands/               # Tauri commands (thin layer)
│   ├── mod.rs
│   ├── projects.rs         # Project CRUD commands
│   ├── tasks.rs            # Task CRUD commands
│   └── loop_control.rs     # Start/stop/pause commands
├── domain/                 # Core domain (pure Rust, no external deps)
│   ├── mod.rs
│   ├── entities/           # Domain entities (structs, enums)
│   │   ├── mod.rs
│   │   ├── project.rs
│   │   ├── task.rs
│   │   ├── artifact.rs
│   │   └── workflow.rs
│   ├── repositories/       # Repository TRAITS (interfaces)
│   │   ├── mod.rs
│   │   ├── project_repository.rs   # trait ProjectRepository
│   │   ├── task_repository.rs      # trait TaskRepository
│   │   ├── artifact_repository.rs  # trait ArtifactRepository
│   │   └── workflow_repository.rs  # trait WorkflowRepository
│   ├── services/           # Domain services (business logic)
│   │   ├── mod.rs
│   │   ├── task_service.rs
│   │   ├── workflow_service.rs
│   │   └── agent_scheduler.rs
│   └── state_machine/      # statig state machine
│       ├── mod.rs
│       ├── task_state_machine.rs
│       └── events.rs
├── infrastructure/         # External implementations
│   ├── mod.rs
│   ├── sqlite/             # SQLite implementations
│   │   ├── mod.rs
│   │   ├── connection.rs   # Pool management
│   │   ├── migrations.rs   # Schema migrations
│   │   ├── sqlite_project_repo.rs
│   │   ├── sqlite_task_repo.rs
│   │   ├── sqlite_artifact_repo.rs
│   │   └── sqlite_workflow_repo.rs
│   ├── memory/             # In-memory implementations (for testing)
│   │   ├── mod.rs
│   │   ├── memory_project_repo.rs
│   │   ├── memory_task_repo.rs
│   │   └── memory_artifact_repo.rs
│   └── vm/                 # VM management
│       ├── mod.rs
│       ├── manager.rs
│       ├── vsock.rs
│       └── mount.rs
├── application/            # Application layer (orchestration)
│   ├── mod.rs
│   ├── app_state.rs        # Dependency injection container
│   └── use_cases/          # Use case handlers
│       ├── mod.rs
│       ├── create_project.rs
│       ├── move_task.rs
│       └── run_loop.rs
└── events/                 # Event emission
    ├── mod.rs
    └── emitters.rs
```

### Repository Pattern Architecture

**Why Repository Pattern?**
- **Testability**: Swap SQLite for in-memory during tests
- **Flexibility**: Migrate to PostgreSQL/cloud storage later
- **Clean Architecture**: Domain logic doesn't know about storage
- **Dependency Inversion**: High-level modules don't depend on low-level

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            APPLICATION LAYER                                │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────────────┐ │
│  │ Tauri Commands  │  │   Use Cases     │  │    App State (DI)           │ │
│  └────────┬────────┘  └────────┬────────┘  └──────────────┬──────────────┘ │
└───────────┼────────────────────┼───────────────────────────┼────────────────┘
            │                    │                           │
            ▼                    ▼                           ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                              DOMAIN LAYER                                   │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────────────┐ │
│  │    Entities     │  │    Services     │  │  Repository Traits          │ │
│  │  (Task, etc.)   │  │ (TaskService)   │  │  (trait TaskRepository)     │ │
│  └─────────────────┘  └─────────────────┘  └──────────────┬──────────────┘ │
└────────────────────────────────────────────────────────────┼────────────────┘
                                                             │ implements
                                                             ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                          INFRASTRUCTURE LAYER                               │
│  ┌───────────────────────────┐  ┌───────────────────────────────────────┐  │
│  │    SQLite Implementation  │  │   In-Memory Implementation (tests)   │  │
│  │  ┌─────────────────────┐  │  │  ┌─────────────────────────────────┐  │  │
│  │  │ SqliteTaskRepo      │  │  │  │ MemoryTaskRepo                  │  │  │
│  │  │ impl TaskRepository │  │  │  │ impl TaskRepository             │  │  │
│  │  └─────────────────────┘  │  │  └─────────────────────────────────┘  │  │
│  └───────────────────────────┘  └───────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Repository Trait Definitions

```rust
// src-tauri/src/domain/repositories/mod.rs

pub mod project_repository;
pub mod task_repository;
pub mod artifact_repository;
pub mod workflow_repository;

pub use project_repository::ProjectRepository;
pub use task_repository::TaskRepository;
pub use artifact_repository::ArtifactRepository;
pub use workflow_repository::WorkflowRepository;
```

```rust
// src-tauri/src/domain/repositories/task_repository.rs

use async_trait::async_trait;
use crate::domain::entities::{Task, TaskId, ProjectId};
use crate::domain::state_machine::{State, TaskEvent};
use crate::error::AppResult;

/// Repository trait for Task persistence.
/// Implementations can use SQLite, PostgreSQL, in-memory, etc.
#[async_trait]
pub trait TaskRepository: Send + Sync {
    // ═══════════════════════════════════════════════════════════════════════
    // CRUD Operations
    // ═══════════════════════════════════════════════════════════════════════

    /// Create a new task
    async fn create(&self, task: Task) -> AppResult<Task>;

    /// Get task by ID
    async fn get_by_id(&self, id: &TaskId) -> AppResult<Option<Task>>;

    /// Get all tasks for a project
    async fn get_by_project(&self, project_id: &ProjectId) -> AppResult<Vec<Task>>;

    /// Update a task
    async fn update(&self, task: &Task) -> AppResult<()>;

    /// Delete a task
    async fn delete(&self, id: &TaskId) -> AppResult<()>;

    // ═══════════════════════════════════════════════════════════════════════
    // State Machine Integration
    // ═══════════════════════════════════════════════════════════════════════

    /// Load task with its current state (for statig rehydration)
    async fn get_with_state(&self, id: &TaskId) -> AppResult<Option<(Task, State)>>;

    /// Persist a state transition atomically
    async fn persist_state_transition(
        &self,
        id: &TaskId,
        from: &State,
        to: &State,
        event: &TaskEvent,
    ) -> AppResult<()>;

    /// Get state history for audit
    async fn get_state_history(&self, id: &TaskId) -> AppResult<Vec<StateTransition>>;

    // ═══════════════════════════════════════════════════════════════════════
    // Query Operations
    // ═══════════════════════════════════════════════════════════════════════

    /// Get tasks by status
    async fn get_by_status(&self, project_id: &ProjectId, status: &State) -> AppResult<Vec<Task>>;

    /// Get next task ready for execution (READY status, no blockers)
    async fn get_next_executable(&self, project_id: &ProjectId) -> AppResult<Option<Task>>;

    /// Get tasks blocking a given task
    async fn get_blockers(&self, id: &TaskId) -> AppResult<Vec<Task>>;

    /// Get tasks blocked by a given task
    async fn get_dependents(&self, id: &TaskId) -> AppResult<Vec<Task>>;
}

/// State transition record for audit log
#[derive(Debug, Clone)]
pub struct StateTransition {
    pub from: State,
    pub to: State,
    pub event: String,
    pub trigger: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}
```

```rust
// src-tauri/src/domain/repositories/project_repository.rs

use async_trait::async_trait;
use crate::domain::entities::{Project, ProjectId};
use crate::error::AppResult;

#[async_trait]
pub trait ProjectRepository: Send + Sync {
    async fn create(&self, project: Project) -> AppResult<Project>;
    async fn get_by_id(&self, id: &ProjectId) -> AppResult<Option<Project>>;
    async fn get_all(&self) -> AppResult<Vec<Project>>;
    async fn update(&self, project: &Project) -> AppResult<()>;
    async fn delete(&self, id: &ProjectId) -> AppResult<()>;
    async fn get_by_working_directory(&self, path: &str) -> AppResult<Option<Project>>;
}
```

### SQLite Implementation

```rust
// src-tauri/src/infrastructure/sqlite/sqlite_task_repo.rs

use async_trait::async_trait;
use rusqlite::{Connection, params};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::domain::entities::{Task, TaskId, ProjectId};
use crate::domain::repositories::{TaskRepository, StateTransition};
use crate::domain::state_machine::{State, TaskEvent};
use crate::error::{AppError, AppResult};

pub struct SqliteTaskRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteTaskRepository {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }
}

#[async_trait]
impl TaskRepository for SqliteTaskRepository {
    async fn create(&self, task: Task) -> AppResult<Task> {
        let conn = self.conn.lock().await;
        conn.execute(
            r#"INSERT INTO tasks (id, project_id, title, description, internal_status, created_at)
               VALUES (?, ?, ?, ?, ?, CURRENT_TIMESTAMP)"#,
            params![
                task.id.0,
                task.project_id.0,
                task.title,
                task.description,
                task.internal_status.to_string(),
            ],
        )?;
        Ok(task)
    }

    async fn get_by_id(&self, id: &TaskId) -> AppResult<Option<Task>> {
        let conn = self.conn.lock().await;
        let result = conn.query_row(
            "SELECT * FROM tasks WHERE id = ?",
            params![id.0],
            |row| Task::from_row(row),
        );
        match result {
            Ok(task) => Ok(Some(task)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e)),
        }
    }

    async fn get_by_project(&self, project_id: &ProjectId) -> AppResult<Vec<Task>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT * FROM tasks WHERE project_id = ? ORDER BY priority DESC, created_at ASC"
        )?;
        let tasks = stmt
            .query_map(params![project_id.0], |row| Task::from_row(row))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(tasks)
    }

    async fn update(&self, task: &Task) -> AppResult<()> {
        let conn = self.conn.lock().await;
        conn.execute(
            r#"UPDATE tasks SET
               title = ?, description = ?, internal_status = ?, updated_at = CURRENT_TIMESTAMP
               WHERE id = ?"#,
            params![
                task.title,
                task.description,
                task.internal_status.to_string(),
                task.id.0,
            ],
        )?;
        Ok(())
    }

    async fn delete(&self, id: &TaskId) -> AppResult<()> {
        let conn = self.conn.lock().await;
        conn.execute("DELETE FROM tasks WHERE id = ?", params![id.0])?;
        Ok(())
    }

    async fn persist_state_transition(
        &self,
        id: &TaskId,
        from: &State,
        to: &State,
        event: &TaskEvent,
    ) -> AppResult<()> {
        let conn = self.conn.lock().await;
        let tx = conn.unchecked_transaction()?;

        // Update task status
        tx.execute(
            "UPDATE tasks SET internal_status = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
            params![to.to_string(), id.0],
        )?;

        // Record in audit log
        tx.execute(
            r#"INSERT INTO task_state_history (id, task_id, from_status, to_status, trigger, created_at)
               VALUES (?, ?, ?, ?, ?, CURRENT_TIMESTAMP)"#,
            params![
                uuid::Uuid::new_v4().to_string(),
                id.0,
                from.to_string(),
                to.to_string(),
                format!("{:?}", event),
            ],
        )?;

        tx.commit()?;
        Ok(())
    }

    async fn get_next_executable(&self, project_id: &ProjectId) -> AppResult<Option<Task>> {
        let conn = self.conn.lock().await;
        let result = conn.query_row(
            r#"SELECT * FROM tasks
               WHERE project_id = ?
               AND internal_status = 'ready'
               AND id NOT IN (
                   SELECT task_id FROM task_blockers WHERE resolved = FALSE
               )
               ORDER BY priority DESC, created_at ASC
               LIMIT 1"#,
            params![project_id.0],
            |row| Task::from_row(row),
        );
        match result {
            Ok(task) => Ok(Some(task)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e)),
        }
    }

    // ... other methods
}
```

### In-Memory Implementation (for Testing)

```rust
// src-tauri/src/infrastructure/memory/memory_task_repo.rs

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::domain::entities::{Task, TaskId, ProjectId};
use crate::domain::repositories::{TaskRepository, StateTransition};
use crate::domain::state_machine::{State, TaskEvent};
use crate::error::AppResult;

/// In-memory implementation for testing (no real database)
pub struct MemoryTaskRepository {
    tasks: Arc<RwLock<HashMap<TaskId, Task>>>,
    history: Arc<RwLock<Vec<StateTransition>>>,
}

impl MemoryTaskRepository {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Create with pre-populated data (for tests)
    pub fn with_tasks(tasks: Vec<Task>) -> Self {
        let map: HashMap<TaskId, Task> = tasks.into_iter().map(|t| (t.id.clone(), t)).collect();
        Self {
            tasks: Arc::new(RwLock::new(map)),
            history: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

#[async_trait]
impl TaskRepository for MemoryTaskRepository {
    async fn create(&self, task: Task) -> AppResult<Task> {
        let mut tasks = self.tasks.write().await;
        tasks.insert(task.id.clone(), task.clone());
        Ok(task)
    }

    async fn get_by_id(&self, id: &TaskId) -> AppResult<Option<Task>> {
        let tasks = self.tasks.read().await;
        Ok(tasks.get(id).cloned())
    }

    async fn get_by_project(&self, project_id: &ProjectId) -> AppResult<Vec<Task>> {
        let tasks = self.tasks.read().await;
        Ok(tasks
            .values()
            .filter(|t| t.project_id == *project_id)
            .cloned()
            .collect())
    }

    async fn update(&self, task: &Task) -> AppResult<()> {
        let mut tasks = self.tasks.write().await;
        tasks.insert(task.id.clone(), task.clone());
        Ok(())
    }

    async fn delete(&self, id: &TaskId) -> AppResult<()> {
        let mut tasks = self.tasks.write().await;
        tasks.remove(id);
        Ok(())
    }

    async fn persist_state_transition(
        &self,
        id: &TaskId,
        from: &State,
        to: &State,
        event: &TaskEvent,
    ) -> AppResult<()> {
        // Update task
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.get_mut(id) {
            task.internal_status = to.clone();
        }

        // Record history
        let mut history = self.history.write().await;
        history.push(StateTransition {
            from: from.clone(),
            to: to.clone(),
            event: format!("{:?}", event),
            trigger: "test".to_string(),
            timestamp: chrono::Utc::now(),
        });

        Ok(())
    }

    async fn get_next_executable(&self, project_id: &ProjectId) -> AppResult<Option<Task>> {
        let tasks = self.tasks.read().await;
        Ok(tasks
            .values()
            .filter(|t| t.project_id == *project_id && matches!(t.internal_status, State::Ready))
            .next()
            .cloned())
    }

    // ... other methods
}
```

### Dependency Injection (App State)

```rust
// src-tauri/src/application/app_state.rs

use std::sync::Arc;
use crate::domain::repositories::{
    ProjectRepository, TaskRepository, ArtifactRepository, WorkflowRepository
};
use crate::infrastructure::sqlite::{
    SqliteProjectRepository, SqliteTaskRepository, SqliteArtifactRepository, SqliteWorkflowRepository
};
use crate::infrastructure::memory::{
    MemoryProjectRepository, MemoryTaskRepository, MemoryArtifactRepository
};

/// Application state container (dependency injection)
pub struct AppState {
    pub project_repo: Arc<dyn ProjectRepository>,
    pub task_repo: Arc<dyn TaskRepository>,
    pub artifact_repo: Arc<dyn ArtifactRepository>,
    pub workflow_repo: Arc<dyn WorkflowRepository>,
}

impl AppState {
    /// Create production app state with SQLite
    pub fn new_production(db_path: &str) -> Self {
        let conn = Arc::new(Mutex::new(Connection::open(db_path).unwrap()));

        Self {
            project_repo: Arc::new(SqliteProjectRepository::new(conn.clone())),
            task_repo: Arc::new(SqliteTaskRepository::new(conn.clone())),
            artifact_repo: Arc::new(SqliteArtifactRepository::new(conn.clone())),
            workflow_repo: Arc::new(SqliteWorkflowRepository::new(conn.clone())),
        }
    }

    /// Create test app state with in-memory repositories
    pub fn new_test() -> Self {
        Self {
            project_repo: Arc::new(MemoryProjectRepository::new()),
            task_repo: Arc::new(MemoryTaskRepository::new()),
            artifact_repo: Arc::new(MemoryArtifactRepository::new()),
            workflow_repo: Arc::new(MemoryWorkflowRepository::new()),
        }
    }

    /// Create with custom repositories (for advanced testing)
    pub fn with_repos(
        project_repo: Arc<dyn ProjectRepository>,
        task_repo: Arc<dyn TaskRepository>,
        artifact_repo: Arc<dyn ArtifactRepository>,
        workflow_repo: Arc<dyn WorkflowRepository>,
    ) -> Self {
        Self { project_repo, task_repo, artifact_repo, workflow_repo }
    }
}

// Use in Tauri commands
#[tauri::command]
pub async fn get_tasks(
    project_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<Task>, String> {
    state.task_repo
        .get_by_project(&ProjectId(project_id))
        .await
        .map_err(|e| e.to_string())
}
```

### Testing with Mock Repositories

```rust
// src-tauri/tests/task_service_test.rs

use crate::domain::services::TaskService;
use crate::infrastructure::memory::MemoryTaskRepository;
use crate::application::AppState;

#[tokio::test]
async fn test_move_task_to_ready() {
    // Arrange: Use in-memory repository
    let state = AppState::new_test();
    let service = TaskService::new(state.task_repo.clone());

    // Create test task
    let task = Task::new("Test task");
    state.task_repo.create(task.clone()).await.unwrap();

    // Act: Move task to ready
    let result = service.schedule_task(&task.id).await;

    // Assert
    assert!(result.is_ok());
    let updated = state.task_repo.get_by_id(&task.id).await.unwrap().unwrap();
    assert!(matches!(updated.internal_status, State::Ready));
}

#[tokio::test]
async fn test_state_transition_persisted() {
    let repo = MemoryTaskRepository::new();
    let task = Task::new("Test");
    repo.create(task.clone()).await.unwrap();

    // Transition
    repo.persist_state_transition(
        &task.id,
        &State::Backlog,
        &State::Ready,
        &TaskEvent::Schedule,
    ).await.unwrap();

    // Verify history recorded
    let history = repo.get_state_history(&task.id).await.unwrap();
    assert_eq!(history.len(), 1);
    assert!(matches!(history[0].to, State::Ready));
}
```

### Future: PostgreSQL Implementation

```rust
// src-tauri/src/infrastructure/postgres/postgres_task_repo.rs (future)

use async_trait::async_trait;
use sqlx::PgPool;

pub struct PostgresTaskRepository {
    pool: PgPool,
}

#[async_trait]
impl TaskRepository for PostgresTaskRepository {
    async fn create(&self, task: Task) -> AppResult<Task> {
        sqlx::query!(
            r#"INSERT INTO tasks (id, project_id, title, description, internal_status)
               VALUES ($1, $2, $3, $4, $5)"#,
            task.id.0,
            task.project_id.0,
            task.title,
            task.description,
            task.internal_status.to_string(),
        )
        .execute(&self.pool)
        .await?;
        Ok(task)
    }

    // ... same interface, different implementation
}
```

---

## Agentic Client Abstraction Layer

**Goal**: Avoid vendor lock-in. Default to Claude Code/Agent SDK, but allow swapping to Codex CLI, Gemini CLI, or other agentic clients in the future.

### Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           DOMAIN LAYER                                      │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    trait AgenticClient                               │   │
│  │  + spawn_agent(config) -> AgentHandle                                │   │
│  │  + send_prompt(handle, prompt) -> Response                           │   │
│  │  + stream_response(handle) -> Stream<Chunk>                          │   │
│  │  + stop_agent(handle)                                                │   │
│  │  + capabilities() -> ClientCapabilities                              │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
└───────────────────────────────────────────────────────────────────────────────┘
                                        │ implements
                                        ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                       INFRASTRUCTURE LAYER                                  │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────────────┐ │
│  │ ClaudeCodeClient│  │  CodexClient    │  │   GeminiClient              │ │
│  │ (default)       │  │  (future)       │  │   (future)                  │ │
│  │ - claude CLI    │  │ - codex CLI     │  │ - gemini CLI                │ │
│  │ - Agent SDK     │  │ - OpenAI API    │  │ - Google AI API             │ │
│  └─────────────────┘  └─────────────────┘  └─────────────────────────────┘ │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ MockAgenticClient (testing) - predefined responses, records calls   │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Folder Structure

```
src-tauri/src/
├── domain/
│   └── agents/                 # Agent abstractions
│       ├── mod.rs
│       ├── agentic_client.rs   # trait AgenticClient
│       ├── agent_config.rs     # AgentConfig, AgentRole
│       └── capabilities.rs     # ClientCapabilities
├── infrastructure/
│   └── agents/                 # Implementations
│       ├── claude/
│       │   └── claude_code_client.rs
│       ├── codex/              # (future)
│       ├── gemini/             # (future)
│       └── mock/
│           └── mock_client.rs
```

### Core Trait Definition

```rust
// src-tauri/src/domain/agents/agentic_client.rs

use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Abstraction over agentic AI clients (Claude, Codex, Gemini, etc.)
#[async_trait]
pub trait AgenticClient: Send + Sync {
    /// Spawn a new agent with the given configuration
    async fn spawn_agent(&self, config: AgentConfig) -> AgentResult<AgentHandle>;

    /// Stop a running agent
    async fn stop_agent(&self, handle: &AgentHandle) -> AgentResult<()>;

    /// Wait for an agent to complete
    async fn wait_for_completion(&self, handle: &AgentHandle) -> AgentResult<AgentOutput>;

    /// Send a prompt and get a complete response
    async fn send_prompt(&self, handle: &AgentHandle, prompt: &str) -> AgentResult<AgentResponse>;

    /// Stream responses
    fn stream_response(
        &self,
        handle: &AgentHandle,
        prompt: &str,
    ) -> Pin<Box<dyn Stream<Item = AgentResult<ResponseChunk>> + Send>>;

    /// Get client capabilities
    fn capabilities(&self) -> &ClientCapabilities;

    /// Check if client is available (CLI installed, API key set)
    async fn is_available(&self) -> AgentResult<bool>;
}

#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub role: AgentRole,
    pub prompt: String,
    pub working_directory: PathBuf,
    pub model: Option<String>,
    pub max_tokens: Option<u32>,
    pub timeout_secs: Option<u64>,
    pub env: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentRole { Worker, Reviewer, QaPrep, QaRefiner, QaTester, Supervisor, Custom(String) }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientType { ClaudeCode, Codex, Gemini, Mock, Custom(String) }

#[derive(Debug, Clone)]
pub struct ClientCapabilities {
    pub client_type: ClientType,
    pub supports_shell: bool,
    pub supports_filesystem: bool,
    pub supports_streaming: bool,
    pub supports_mcp: bool,
    pub max_context_tokens: u32,
    pub models: Vec<ModelInfo>,
}
```

### Claude Code Implementation (Default)

```rust
// src-tauri/src/infrastructure/agents/claude/claude_code_client.rs

pub struct ClaudeCodeClient {
    cli_path: PathBuf,
    capabilities: ClientCapabilities,
}

impl ClaudeCodeClient {
    pub fn new() -> Self {
        Self {
            cli_path: which::which("claude").unwrap_or_else(|_| "claude".into()),
            capabilities: ClientCapabilities {
                client_type: ClientType::ClaudeCode,
                supports_shell: true,
                supports_filesystem: true,
                supports_streaming: true,
                supports_mcp: true,
                max_context_tokens: 200_000,
                models: vec![
                    ModelInfo { id: "claude-sonnet-4-20250514".into(), name: "Claude Sonnet 4".into(), .. },
                    ModelInfo { id: "claude-opus-4-20250514".into(), name: "Claude Opus 4".into(), .. },
                ],
            },
        }
    }
}

#[async_trait]
impl AgenticClient for ClaudeCodeClient {
    async fn spawn_agent(&self, config: AgentConfig) -> AgentResult<AgentHandle> {
        let mut args = vec!["-p".into(), config.prompt.clone(), "--output-format".into(), "stream-json".into()];
        if let Some(model) = &config.model { args.extend(["--model".into(), model.clone()]); }

        let child = Command::new(&self.cli_path)
            .args(&args)
            .current_dir(&config.working_directory)
            .stdout(Stdio::piped())
            .spawn()?;

        // Store handle for later management
        let handle = AgentHandle::new(ClientType::ClaudeCode, config.role);
        PROCESSES.lock().await.insert(handle.id.clone(), child);
        Ok(handle)
    }

    async fn wait_for_completion(&self, handle: &AgentHandle) -> AgentResult<AgentOutput> {
        let mut child = PROCESSES.lock().await.remove(&handle.id).ok_or(AgentError::NotFound)?;
        let output = child.wait_with_output().await?;
        Ok(AgentOutput {
            success: output.status.success(),
            content: String::from_utf8_lossy(&output.stdout).into(),
            ..Default::default()
        })
    }
    // ... other methods
}
```

### Mock Client (Testing)

```rust
// src-tauri/src/infrastructure/agents/mock/mock_client.rs

pub struct MockAgenticClient {
    responses: Arc<RwLock<HashMap<String, String>>>,
    call_history: Arc<RwLock<Vec<MockCall>>>,
}

impl MockAgenticClient {
    pub fn new() -> Self { /* ... */ }

    /// Set response for prompts containing pattern
    pub async fn when_prompt_contains(&self, pattern: &str, response: &str) {
        self.responses.write().await.insert(pattern.into(), response.into());
    }

    /// Get recorded calls for assertions
    pub async fn get_calls(&self) -> Vec<MockCall> {
        self.call_history.read().await.clone()
    }
}

#[async_trait]
impl AgenticClient for MockAgenticClient {
    async fn spawn_agent(&self, config: AgentConfig) -> AgentResult<AgentHandle> {
        self.call_history.write().await.push(MockCall::spawn(&config));
        Ok(AgentHandle::mock(config.role))
    }

    async fn send_prompt(&self, _: &AgentHandle, prompt: &str) -> AgentResult<AgentResponse> {
        self.call_history.write().await.push(MockCall::prompt(prompt));
        let response = self.find_matching_response(prompt).await;
        Ok(AgentResponse { content: response, ..Default::default() })
    }
    // ... returns mock data instantly, no API calls
}
```

### Updated App State with Agent Client

```rust
// src-tauri/src/application/app_state.rs

pub struct AppState {
    pub project_repo: Arc<dyn ProjectRepository>,
    pub task_repo: Arc<dyn TaskRepository>,
    pub agent_client: Arc<dyn AgenticClient>,  // ← Abstracted!
}

impl AppState {
    /// Production: SQLite + Claude Code (default)
    pub fn new_production(db_path: &str) -> Self {
        Self {
            project_repo: Arc::new(SqliteProjectRepository::new(db_path)),
            task_repo: Arc::new(SqliteTaskRepository::new(db_path)),
            agent_client: Arc::new(ClaudeCodeClient::new()),
        }
    }

    /// Testing: In-memory + Mock agent (no API calls)
    pub fn new_test() -> Self {
        Self {
            project_repo: Arc::new(MemoryProjectRepository::new()),
            task_repo: Arc::new(MemoryTaskRepository::new()),
            agent_client: Arc::new(MockAgenticClient::new()),
        }
    }

    /// Swap to different provider
    pub fn with_agent_client(mut self, client: Arc<dyn AgenticClient>) -> Self {
        self.agent_client = client;
        self
    }
}
```

### Configuration

```toml
# config.toml
[agent]
client = "claude"  # Options: "claude", "codex" (future), "gemini" (future), "mock"

[agent.claude]
cli_path = "/usr/local/bin/claude"  # Optional
default_model = "claude-sonnet-4-20250514"

[agent.codex]  # Future
api_key = "${OPENAI_API_KEY}"

[agent.gemini]  # Future
api_key = "${GOOGLE_AI_API_KEY}"
```

### Usage in Services

```rust
// Services don't know which client is being used
impl TaskService {
    pub async fn execute_task(&self, task_id: &TaskId) -> AppResult<()> {
        let task = self.task_repo.get_by_id(task_id).await?;

        // Works with ANY agentic client (Claude, Codex, Gemini, Mock)
        let handle = self.agent_client.spawn_agent(AgentConfig {
            role: AgentRole::Worker,
            prompt: task.description.clone(),
            ..Default::default()
        }).await?;

        let output = self.agent_client.wait_for_completion(&handle).await?;
        // Process output...
    }
}
```

---

#### Error Handling

```rust
// src-tauri/src/error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Task not found: {0}")]
    TaskNotFound(String),

    #[error("Invalid status transition: {from} → {to}")]
    InvalidTransition { from: String, to: String },

    #[error("VM error: {0}")]
    VmError(String),

    #[error("Agent error: {0}")]
    AgentError(String),
}

// Make errors serializable for Tauri
impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
```

#### Type Safety with Newtypes

```rust
// src-tauri/src/core/types.rs

// Prevent mixing up IDs
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProjectId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ArtifactId(pub String);

impl TaskId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

// Status as enum, not strings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InternalStatus {
    Backlog,
    Ready,
    Blocked,
    Executing,
    PendingReview,
    RevisionNeeded,
    Approved,
    Failed,
    Cancelled,
}

impl InternalStatus {
    /// Returns valid transitions from this status
    pub fn valid_transitions(&self) -> &[InternalStatus] {
        use InternalStatus::*;
        match self {
            Backlog => &[Ready, Cancelled],
            Ready => &[Executing, Blocked, Cancelled],
            Blocked => &[Ready, Cancelled],
            Executing => &[PendingReview, Failed, Blocked],
            PendingReview => &[Approved, RevisionNeeded],
            RevisionNeeded => &[Executing, Cancelled],
            Approved => &[Ready], // Re-open
            Failed => &[Ready],
            Cancelled => &[Ready],
        }
    }

    pub fn can_transition_to(&self, target: InternalStatus) -> bool {
        self.valid_transitions().contains(&target)
    }
}
```

#### Command Pattern (Thin Commands)

```rust
// src-tauri/src/commands/tasks.rs
// Commands are THIN - just validation, delegation, response formatting

use crate::core::task_service::TaskService;
use crate::error::AppResult;

#[tauri::command]
pub async fn create_task(
    state: tauri::State<'_, AppState>,
    project_id: String,
    title: String,
    description: Option<String>,
) -> AppResult<Task> {
    // Validate input
    if title.trim().is_empty() {
        return Err(AppError::Validation("Title cannot be empty".into()));
    }

    // Delegate to service
    let service = TaskService::new(state.db.clone());
    service.create(ProjectId(project_id), title, description).await
}

#[tauri::command]
pub async fn move_task(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
    task_id: String,
    to_status: String,
) -> AppResult<Task> {
    let service = TaskService::new(state.db.clone());
    let task = service.transition(TaskId(task_id), to_status.parse()?).await?;

    // Emit event for UI
    app.emit("task:status", &task)?;

    Ok(task)
}
```

#### Service Layer (Business Logic)

```rust
// src-tauri/src/core/task_service.rs
// Services contain business logic, are testable without Tauri

pub struct TaskService {
    repo: TaskRepository,
    side_effects: SideEffectExecutor,
}

impl TaskService {
    pub async fn transition(
        &self,
        task_id: TaskId,
        to_status: InternalStatus,
    ) -> AppResult<Task> {
        let task = self.repo.get(&task_id).await?
            .ok_or(AppError::TaskNotFound(task_id.0.clone()))?;

        // Validate transition
        if !task.internal_status.can_transition_to(to_status) {
            return Err(AppError::InvalidTransition {
                from: task.internal_status.to_string(),
                to: to_status.to_string(),
            });
        }

        // Execute side effects
        self.side_effects.execute(to_status, &task).await?;

        // Update and return
        self.repo.update_status(&task_id, to_status).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_valid_transition() {
        let service = TaskService::new_test();
        let task = service.create_test_task(InternalStatus::Ready).await;

        let result = service.transition(task.id, InternalStatus::Executing).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_invalid_transition() {
        let service = TaskService::new_test();
        let task = service.create_test_task(InternalStatus::Backlog).await;

        // Can't go directly from Backlog to Executing
        let result = service.transition(task.id, InternalStatus::Executing).await;
        assert!(matches!(result, Err(AppError::InvalidTransition { .. })));
    }
}
```

#### Repository Pattern (Data Access)

```rust
// src-tauri/src/database/repositories/task_repo.rs
// Repositories handle SQL, return domain types

pub struct TaskRepository {
    pool: SqlitePool,
}

impl TaskRepository {
    pub async fn get(&self, id: &TaskId) -> AppResult<Option<Task>> {
        let row = sqlx::query_as!(
            TaskRow,
            "SELECT * FROM tasks WHERE id = ?",
            id.0
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Task::from))
    }

    pub async fn list_by_status(
        &self,
        project_id: &ProjectId,
        status: InternalStatus,
    ) -> AppResult<Vec<Task>> {
        let status_str = status.to_string();
        let rows = sqlx::query_as!(
            TaskRow,
            "SELECT * FROM tasks WHERE project_id = ? AND internal_status = ? ORDER BY priority DESC",
            project_id.0,
            status_str
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Task::from).collect())
    }
}
```

---

### TypeScript Frontend Best Practices

#### Strict TypeScript Configuration

```json
// tsconfig.json
{
  "compilerOptions": {
    "strict": true,
    "noUncheckedIndexedAccess": true,
    "noImplicitReturns": true,
    "noFallthroughCasesInSwitch": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "exactOptionalPropertyTypes": true,
    "forceConsistentCasingInFileNames": true,
    "verbatimModuleSyntax": true
  }
}
```

#### Module Organization

```
src/
├── main.tsx                    # Entry point only
├── App.tsx                     # Router setup only
├── types/                      # Shared type definitions
│   ├── index.ts                # Re-exports
│   ├── task.ts                 # Task types + Zod schemas
│   ├── project.ts
│   ├── workflow.ts
│   └── events.ts
├── lib/                        # Utilities, no React
│   ├── tauri.ts                # Tauri invoke wrappers
│   ├── validation.ts           # Zod schemas
│   └── formatters.ts           # Date, number formatters
├── hooks/                      # Custom React hooks
│   ├── useProjects.ts
│   ├── useTasks.ts
│   ├── useTaskMutation.ts
│   └── useEvents.ts
├── stores/                     # Zustand stores
│   ├── projectStore.ts
│   ├── taskStore.ts
│   └── uiStore.ts
├── components/
│   ├── ui/                     # Primitive components
│   │   ├── Button.tsx
│   │   ├── Input.tsx
│   │   └── Modal.tsx
│   ├── tasks/                  # Feature components
│   │   ├── TaskBoard/
│   │   │   ├── index.tsx       # Public export
│   │   │   ├── TaskBoard.tsx   # Main component
│   │   │   ├── Column.tsx      # Sub-component
│   │   │   ├── TaskCard.tsx
│   │   │   └── hooks.ts        # Component-specific hooks
│   │   └── TaskDetail/
│   │       ├── index.tsx
│   │       ├── TaskDetail.tsx
│   │       └── StateHistory.tsx
│   └── layout/
│       ├── Sidebar.tsx
│       └── Header.tsx
└── pages/                      # Route components
    ├── ProjectPage.tsx
    └── SettingsPage.tsx
```

#### Type Definitions with Zod Runtime Validation

```typescript
// src/types/task.ts
import { z } from "zod";

// Zod schema = runtime validation + type inference
export const InternalStatusSchema = z.enum([
  "backlog",
  "ready",
  "blocked",
  "executing",
  "pending_review",
  "revision_needed",
  "approved",
  "failed",
  "cancelled",
]);

export type InternalStatus = z.infer<typeof InternalStatusSchema>;

export const TaskSchema = z.object({
  id: z.string().uuid(),
  projectId: z.string().uuid(),
  title: z.string().min(1),
  description: z.string().nullable(),
  internalStatus: InternalStatusSchema,
  externalStatus: z.string().nullable(),
  priority: z.number().int().min(0),
  wave: z.number().int().nullable(),
  checkpointType: z.enum(["auto", "human-verify", "decision", "human-action"]).nullable(),
  createdAt: z.string().datetime(),
  updatedAt: z.string().datetime(),
});

export type Task = z.infer<typeof TaskSchema>;

// Discriminated union for task events
export const TaskEventSchema = z.discriminatedUnion("type", [
  z.object({
    type: z.literal("created"),
    task: TaskSchema,
  }),
  z.object({
    type: z.literal("updated"),
    taskId: z.string().uuid(),
    changes: TaskSchema.partial(),
  }),
  z.object({
    type: z.literal("deleted"),
    taskId: z.string().uuid(),
  }),
  z.object({
    type: z.literal("status_changed"),
    taskId: z.string().uuid(),
    from: InternalStatusSchema,
    to: InternalStatusSchema,
    changedBy: z.enum(["user", "system", "agent"]),
  }),
]);

export type TaskEvent = z.infer<typeof TaskEventSchema>;
```

#### Tauri Invoke Wrappers with Type Safety

```typescript
// src/lib/tauri.ts
import { invoke } from "@tauri-apps/api/core";
import { TaskSchema, Task } from "@/types/task";
import { z } from "zod";

// Generic invoke wrapper with runtime validation
async function typedInvoke<T>(
  cmd: string,
  args: Record<string, unknown>,
  schema: z.ZodType<T>
): Promise<T> {
  const result = await invoke(cmd, args);
  return schema.parse(result);
}

// Typed API functions
export const api = {
  tasks: {
    list: (projectId: string) =>
      typedInvoke("list_tasks", { projectId }, z.array(TaskSchema)),

    get: (taskId: string) =>
      typedInvoke("get_task", { taskId }, TaskSchema),

    create: (projectId: string, title: string, description?: string) =>
      typedInvoke("create_task", { projectId, title, description }, TaskSchema),

    move: (taskId: string, toStatus: string) =>
      typedInvoke("move_task", { taskId, toStatus }, TaskSchema),
  },
  // ... other namespaces
};
```

#### Component Organization (Single Responsibility)

```typescript
// src/components/tasks/TaskBoard/index.tsx
// Public API - only export what's needed
export { TaskBoard } from "./TaskBoard";
export type { TaskBoardProps } from "./TaskBoard";
```

```typescript
// src/components/tasks/TaskBoard/TaskBoard.tsx
// Main component - orchestrates sub-components, max ~150 lines

import { Column } from "./Column";
import { useTaskBoard } from "./hooks";

export interface TaskBoardProps {
  projectId: string;
  workflowId: string;
}

export function TaskBoard({ projectId, workflowId }: TaskBoardProps) {
  const { columns, onDragEnd, isLoading } = useTaskBoard(projectId, workflowId);

  if (isLoading) {
    return <TaskBoardSkeleton />;
  }

  return (
    <DndContext onDragEnd={onDragEnd}>
      <div className="flex gap-4 overflow-x-auto p-4">
        {columns.map((column) => (
          <Column key={column.id} column={column} />
        ))}
      </div>
    </DndContext>
  );
}
```

```typescript
// src/components/tasks/TaskBoard/hooks.ts
// Component-specific hooks - data fetching, mutations, local state

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/tauri";

export function useTaskBoard(projectId: string, workflowId: string) {
  const queryClient = useQueryClient();

  const { data: tasks, isLoading } = useQuery({
    queryKey: ["tasks", projectId],
    queryFn: () => api.tasks.list(projectId),
  });

  const { data: workflow } = useQuery({
    queryKey: ["workflow", workflowId],
    queryFn: () => api.workflows.get(workflowId),
  });

  const moveMutation = useMutation({
    mutationFn: ({ taskId, toStatus }: { taskId: string; toStatus: string }) =>
      api.tasks.move(taskId, toStatus),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["tasks", projectId] });
    },
  });

  const columns = useMemo(() => {
    if (!workflow || !tasks) return [];
    return workflow.columns.map((col) => ({
      ...col,
      tasks: tasks.filter((t) => t.externalStatus === col.id),
    }));
  }, [workflow, tasks]);

  const onDragEnd = useCallback((event: DragEndEvent) => {
    const { active, over } = event;
    if (!over) return;

    const taskId = active.id as string;
    const toStatus = over.id as string;

    moveMutation.mutate({ taskId, toStatus });
  }, [moveMutation]);

  return { columns, onDragEnd, isLoading };
}
```

#### Zustand Store Pattern

```typescript
// src/stores/taskStore.ts
import { create } from "zustand";
import { immer } from "zustand/middleware/immer";
import { Task, InternalStatus } from "@/types/task";

interface TaskState {
  tasks: Record<string, Task>;
  selectedTaskId: string | null;
}

interface TaskActions {
  setTasks: (tasks: Task[]) => void;
  updateTask: (taskId: string, changes: Partial<Task>) => void;
  selectTask: (taskId: string | null) => void;
}

export const useTaskStore = create<TaskState & TaskActions>()(
  immer((set) => ({
    tasks: {},
    selectedTaskId: null,

    setTasks: (tasks) =>
      set((state) => {
        state.tasks = Object.fromEntries(tasks.map((t) => [t.id, t]));
      }),

    updateTask: (taskId, changes) =>
      set((state) => {
        const task = state.tasks[taskId];
        if (task) {
          Object.assign(task, changes);
        }
      }),

    selectTask: (taskId) =>
      set((state) => {
        state.selectedTaskId = taskId;
      }),
  }))
);

// Selectors (outside store for memoization)
export const selectTasksByStatus = (status: InternalStatus) => (state: TaskState) =>
  Object.values(state.tasks).filter((t) => t.internalStatus === status);

export const selectSelectedTask = (state: TaskState & TaskActions) =>
  state.selectedTaskId ? state.tasks[state.selectedTaskId] : null;
```

#### Event Handling with Type Safety

```typescript
// src/hooks/useEvents.ts
import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { TaskEventSchema, TaskEvent } from "@/types/events";
import { useTaskStore } from "@/stores/taskStore";

export function useTaskEvents() {
  const updateTask = useTaskStore((s) => s.updateTask);

  useEffect(() => {
    const unlisten = listen<unknown>("task:event", (event) => {
      // Runtime validation of backend events
      const parsed = TaskEventSchema.safeParse(event.payload);

      if (!parsed.success) {
        console.error("Invalid task event:", parsed.error);
        return;
      }

      const taskEvent = parsed.data;

      switch (taskEvent.type) {
        case "updated":
          updateTask(taskEvent.taskId, taskEvent.changes);
          break;
        case "status_changed":
          updateTask(taskEvent.taskId, { internalStatus: taskEvent.to });
          break;
        // ... handle other events
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [updateTask]);
}
```

---

### Shared Conventions

#### Naming Conventions

| Element | Convention | Example |
|---------|------------|---------|
| **Files** (Rust) | `snake_case.rs` | `task_service.rs` |
| **Files** (TS) | `PascalCase.tsx` / `camelCase.ts` | `TaskCard.tsx`, `formatters.ts` |
| **Functions** | `snake_case` (Rust), `camelCase` (TS) | `get_task`, `getTask` |
| **Types/Structs** | `PascalCase` | `Task`, `WorkflowSchema` |
| **Constants** | `SCREAMING_SNAKE_CASE` | `MAX_ITERATIONS` |
| **Enums** | `PascalCase` variants | `InternalStatus::Executing` |

#### File Size Limits

| File Type | Max Lines | Action When Exceeded |
|-----------|-----------|----------------------|
| Component | 200 | Extract sub-components |
| Hook | 100 | Split into focused hooks |
| Service | 300 | Split by domain |
| Store | 150 | Split into slices |
| Type definitions | 200 | Split by domain |

#### Documentation Standards

```rust
// Rust: Document public APIs, explain "why" not "what"

/// Transitions a task to a new status, executing any side effects.
///
/// # Errors
/// - `InvalidTransition` if the status change is not allowed
/// - `TaskNotFound` if the task doesn't exist
///
/// # Side Effects
/// Depending on the target status:
/// - `Executing`: Spawns a worker agent
/// - `PendingReview`: Spawns a reviewer agent
/// - `Approved`: Unblocks dependent tasks
pub async fn transition(&self, task_id: TaskId, to: InternalStatus) -> AppResult<Task>
```

```typescript
// TypeScript: JSDoc for public APIs, inline comments for complex logic

/**
 * Validates a status transition and returns the side effect to execute.
 *
 * @throws {InvalidTransitionError} if transition is not allowed
 */
export function validateTransition(
  from: InternalStatus,
  to: InternalStatus
): SideEffect | null {
  // Check basic validity first (O(1) lookup)
  if (!VALID_TRANSITIONS[from]?.includes(to)) {
    throw new InvalidTransitionError(from, to);
  }

  // Side effects only trigger on specific transitions
  return SIDE_EFFECT_MAP[`${from}->${to}`] ?? null;
}
```

#### Testing Strategy

```
Tests/
├── Unit Tests (fast, isolated)
│   ├── Pure functions
│   ├── State machine transitions
│   ├── Validators
│   └── Formatters
│
├── Integration Tests (medium speed)
│   ├── Service + Repository
│   ├── Component + Hook
│   └── Store + API
│
└── E2E Tests (slow, full stack)
    ├── Critical user flows
    ├── Task execution loop
    └── Agent communication
```

```rust
// Rust: Unit test example
#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(InternalStatus::Ready, InternalStatus::Executing, true)]
    #[case(InternalStatus::Backlog, InternalStatus::Executing, false)]
    #[case(InternalStatus::Executing, InternalStatus::PendingReview, true)]
    fn test_status_transitions(
        #[case] from: InternalStatus,
        #[case] to: InternalStatus,
        #[case] expected: bool,
    ) {
        assert_eq!(from.can_transition_to(to), expected);
    }
}
```

```typescript
// TypeScript: Component test example
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { TaskCard } from "./TaskCard";

describe("TaskCard", () => {
  const mockTask = createMockTask({ title: "Test Task" });

  it("renders task title", () => {
    render(<TaskCard task={mockTask} />);
    expect(screen.getByText("Test Task")).toBeInTheDocument();
  });

  it("calls onSelect when clicked", async () => {
    const onSelect = vi.fn();
    render(<TaskCard task={mockTask} onSelect={onSelect} />);

    await userEvent.click(screen.getByRole("button"));
    expect(onSelect).toHaveBeenCalledWith(mockTask.id);
  });
});
```

---

## Design System (Dark Theme, Anti-AI-Slop)

**Theme:** Dark only. Modern, sleek, hand-crafted feel.

### What is AI Slop? (What to AVOID)
- Blue-to-purple gradients (the #1 telltale sign)
- Inter font as the only typeface
- Three boxes with icons in a grid
- Overly saturated, bright colors on dark backgrounds
- Rounded corners everywhere
- Generic stock illustrations
- Cluttered layouts with no hierarchy

### Color Palette (NOT purple/blue)
```css
:root {
  /* Backgrounds - dark grays, NOT pure black */
  --bg-base: #0f0f0f;
  --bg-surface: #1a1a1a;
  --bg-elevated: #242424;
  --bg-hover: #2d2d2d;

  /* Text - off-white, NOT pure white */
  --text-primary: #f0f0f0;
  --text-secondary: #a0a0a0;
  --text-muted: #666666;

  /* Accent - warm, distinctive (NOT purple) */
  --accent-primary: #ff6b35;      /* Warm orange */
  --accent-secondary: #ffa94d;    /* Soft amber */

  /* Status */
  --status-success: #10b981;      /* Emerald */
  --status-warning: #f59e0b;      /* Amber */
  --status-error: #ef4444;        /* Red */
  --status-info: #3b82f6;         /* Blue (sparingly) */

  /* Borders & Dividers */
  --border-subtle: rgba(255, 255, 255, 0.06);
  --border-default: rgba(255, 255, 255, 0.1);
}
```

### Typography (NOT just Inter)
```css
:root {
  /* Display/Headers - distinctive */
  --font-display: 'SF Pro Display', -apple-system, sans-serif;

  /* Body - readable */
  --font-body: 'SF Pro Text', -apple-system, sans-serif;

  /* Mono - for code */
  --font-mono: 'JetBrains Mono', 'Fira Code', monospace;
}
```

### Spacing (8pt Grid System)
All spacing uses multiples of 4px, primarily 8px:
- `--space-1`: 4px (tight)
- `--space-2`: 8px (default)
- `--space-3`: 12px
- `--space-4`: 16px
- `--space-6`: 24px
- `--space-8`: 32px
- `--space-12`: 48px

### Visual Principles
1. **No gradients on UI elements** - solid colors only
2. **Subtle borders** - 1px, low opacity white
3. **Generous whitespace** - cramped = cheap
4. **Clear hierarchy** - one focal point per view
5. **Purposeful animations** - 150-200ms, ease-out
6. **Glassmorphism sparingly** - only for modals/overlays:
   ```css
   backdrop-filter: blur(20px);
   background: rgba(255, 255, 255, 0.05);
   border: 1px solid rgba(255, 255, 255, 0.1);
   ```

### Inspiration (Study These)
- **Linear** - Bold typography, monochrome, intentional
- **Raycast** - Developer-first, keyboard-driven
- **Warp** - Terminal with modern touches
- **Vercel Dashboard** - Clean, functional

### Anti-Slop Guardrails for AI Implementation
When implementing UI, these constraints MUST be followed:
1. **NO purple or blue-purple gradients anywhere**
2. **NO Inter font** - use SF Pro or system fonts
3. **NO generic icon grids** (3 boxes with icons)
4. **NO high-saturation colors** on dark backgrounds
5. **ALWAYS use CSS variables** - never hardcode colors
6. **ALWAYS follow 8pt grid** - no random spacing
7. **ALWAYS maintain 4.5:1 contrast ratio** for text

---

## Configuration (Settings Profile)

Minimal essential settings with good defaults:

| Setting | Default | Rationale |
|---------|---------|-----------|
| `max_concurrent_tasks` | `2` | Balance between speed and resource usage. 2 VMs use ~500MB RAM total. |
| `auto_commit` | `true` | Matches original Ralph behavior. Each completed task = one atomic commit. |
| `commit_message_prefix` | `"feat: "` | Conventional commits format. Can be `fix:`, `chore:`, etc. |
| `pause_on_failure` | `true` | Stop queue when task fails, so user can investigate before continuing. |
| `model` | `"claude-sonnet-4-20250514"` | Best balance of speed/cost/quality. User can upgrade to Opus for complex tasks. |
| `review_before_destructive` | `true` | Auto-insert review point before tasks that delete files or modify configs. |
| `ai_review_enabled` | `true` | Auto-review completed tasks with AI agent. |
| `ai_review_auto_fix` | `true` | Auto-create fix tasks for AI review failures (false = send to backlog). |
| `require_fix_approval` | `false` | If true, AI-proposed fix tasks need human approval before execution. |
| `require_human_review` | `false` | If true, AI-approved tasks still need human approval. |
| `max_fix_attempts` | `3` | Max times AI can propose fixes before giving up → backlog. |
| `supervisor_enabled` | `true` | Enable watchdog monitoring for stuck/looping agents. |
| `supervisor_loop_threshold` | `3` | Same tool call N times = potential loop, trigger check. |
| `supervisor_stuck_timeout` | `300` | Seconds without progress before stuck detection (5 min). |

**Profile System:**
- Default profile ships with app
- User can create custom profiles (e.g., "fast" with max_concurrent=4, "careful" with checkpoints everywhere)
- Per-project profile override (optional)

**Not configurable (intentionally fixed):**
- VM isolation level (always full VM for security)
- Database location (always `~/Library/Application Support/RalphX/`)
- Git operations (always in project directory, never pushes automatically)

---

## Decisions Made

| Decision | Choice | Rationale |
|----------|--------|-----------|
| **App Name** | RalphX | Keeps Ralph heritage, modern feel |
| **Agent SDK** | TypeScript | Same ecosystem as frontend |
| **Isolation** | Full VM | Cowork-level security |
| **Concurrency** | Parallel | Multiple VMs for different projects |

## Technical Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| VM startup latency | UX delay on first run | Pre-boot VM on app launch, keep warm |
| Virtualization.framework complexity | Development time | Start with simpler process isolation, upgrade to VM |
| Memory usage with multiple VMs | Resource constraints | Limit concurrent VMs, pause inactive ones |
| IPC reliability | Agent failures | Implement heartbeat, auto-restart on disconnect |
| Linux VM image size | Bundle size | Minimal Alpine-based image, lazy download |

---

## Extensibility Architecture

RalphX is designed to be extensible, supporting custom workflows, development methodologies (like BMAD, GSD), and integration with Claude Code's native plugin/skill/agent/hook system.

### Design Philosophy

**Two-layer status system:**
- **Internal statuses**: Fixed, minimal set with hardcoded side effects (the "engine")
- **External statuses**: User-defined labels that map to internal statuses (the "UI")

**Leverage Claude Code's native extension system:**
- **Plugins**: Distribution packages containing agents, skills, hooks, MCP servers
- **Skills**: Reusable capabilities (Claude Code `.claude/skills/*/SKILL.md`)
- **Agents**: Task-specific executors (Claude Code `.claude/agents/*.md`)
- **Hooks**: Event-driven automation (PreToolUse, PostToolUse, Stop, etc.)

**Support methodology extensions:**
- BMAD Method (8 agents, 4 phases, document-centric)
- GSD Method (11 agents, wave-based parallelization, checkpoint protocol)
- Custom methodologies via configuration

---

## Internal Status State Machine

### Design Philosophy

The state machine is the **core engine** of RalphX. Every status has:
1. **Granular states** - Each distinct operation has its own status (no compound states)
2. **Explicit transitions** - Only defined transitions are allowed
3. **Lifecycle hooks** - `on_enter`, `on_exit`, and transition callbacks
4. **Guards** - Conditions that must be true for a transition to occur
5. **Side effects** - Actions triggered by transitions (spawn agents, emit events, etc.)

### Internal Status Enum (14 statuses)

```typescript
enum InternalStatus {
  // ═══════════════════════════════════════════════════════════════════
  // IDLE STATES (no automatic actions, waiting for user/system trigger)
  // ═══════════════════════════════════════════════════════════════════
  BACKLOG = "backlog",        // Not ready for work, parked
  READY = "ready",            // Ready to be picked up (user moved here)
  BLOCKED = "blocked",        // Waiting on dependencies or human input

  // ═══════════════════════════════════════════════════════════════════
  // QA PREP STATES (runs in parallel with execution)
  // ═══════════════════════════════════════════════════════════════════
  QA_PREPPING = "qa_prepping",      // QA Prep agent generating acceptance criteria (background)

  // ═══════════════════════════════════════════════════════════════════
  // EXECUTION STATES (worker agent lifecycle)
  // ═══════════════════════════════════════════════════════════════════
  EXECUTING = "executing",           // Worker agent actively running
  EXECUTION_DONE = "execution_done", // Worker finished, awaiting QA or review

  // ═══════════════════════════════════════════════════════════════════
  // QA TESTING STATES (post-execution verification)
  // ═══════════════════════════════════════════════════════════════════
  QA_REFINING = "qa_refining",       // QA agent refining plan based on actual implementation
  QA_TESTING = "qa_testing",         // Browser tests executing
  QA_PASSED = "qa_passed",           // All QA tests passed
  QA_FAILED = "qa_failed",           // QA tests failed, needs attention

  // ═══════════════════════════════════════════════════════════════════
  // REVIEW STATES (AI and human review lifecycle)
  // ═══════════════════════════════════════════════════════════════════
  PENDING_REVIEW = "pending_review",     // Awaiting AI reviewer
  REVISION_NEEDED = "revision_needed",   // Review found issues, needs rework

  // ═══════════════════════════════════════════════════════════════════
  // TERMINAL STATES
  // ═══════════════════════════════════════════════════════════════════
  APPROVED = "approved",      // Complete and verified
  FAILED = "failed",          // Requires manual intervention
  CANCELLED = "cancelled",    // Intentionally abandoned
}
```

### State Machine Definition

```typescript
// ═══════════════════════════════════════════════════════════════════════════
// CORE TYPES
// ═══════════════════════════════════════════════════════════════════════════

interface StateMachineContext {
  task: Task;
  project: Project;
  qaEnabled: boolean;
  qaPrepComplete: boolean;  // Track if background QA prep finished
  services: {
    agentSpawner: AgentSpawner;
    eventEmitter: EventEmitter;
    notifier: Notifier;
  };
}

interface Transition {
  from: InternalStatus;
  to: InternalStatus;
  trigger: "user" | "agent" | "system" | "automatic";
  guard?: (ctx: StateMachineContext) => boolean;        // Must return true to allow
  onTransition?: (ctx: StateMachineContext) => Promise<void>;  // Runs during transition
}

interface StatusConfig {
  status: InternalStatus;
  category: "idle" | "qa_prep" | "execution" | "qa_test" | "review" | "terminal";
  onEnter?: (ctx: StateMachineContext) => Promise<void>;   // Runs when entering this status
  onExit?: (ctx: StateMachineContext) => Promise<void>;    // Runs when leaving this status
  autoTransition?: {
    to: InternalStatus;
    condition: (ctx: StateMachineContext) => boolean;
    delay?: number;  // ms to wait before checking condition
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// STATUS CONFIGURATIONS (on_enter / on_exit hooks)
// ═══════════════════════════════════════════════════════════════════════════

const STATUS_CONFIGS: StatusConfig[] = [
  // --- IDLE STATES ---
  {
    status: InternalStatus.BACKLOG,
    category: "idle",
    // No hooks - just parked
  },
  {
    status: InternalStatus.READY,
    category: "idle",
    onEnter: async (ctx) => {
      // When task becomes READY, spawn QA Prep in background (if enabled)
      if (ctx.qaEnabled) {
        ctx.services.agentSpawner.spawnBackground("qa-prep", ctx.task.id);
        await ctx.services.eventEmitter.emit("qa_prep_started", { taskId: ctx.task.id });
      }
    },
    autoTransition: {
      to: InternalStatus.EXECUTING,
      condition: (ctx) => !ctx.task.hasUnresolvedBlockers(),
      delay: 100,  // Small delay to allow QA prep to start
    },
  },
  {
    status: InternalStatus.BLOCKED,
    category: "idle",
    onEnter: async (ctx) => {
      await ctx.services.eventEmitter.emit("task_blocked", {
        taskId: ctx.task.id,
        blockers: ctx.task.blockers,
      });
    },
    autoTransition: {
      to: InternalStatus.READY,
      condition: (ctx) => ctx.task.blockers.every(b => b.resolved),
    },
  },

  // --- QA PREP STATE (background) ---
  {
    status: InternalStatus.QA_PREPPING,
    category: "qa_prep",
    onEnter: async (ctx) => {
      // This is a virtual status - tracks background QA prep progress
      await ctx.services.agentSpawner.spawn("qa-prep", {
        taskId: ctx.task.id,
        description: ctx.task.description,
        codebaseContext: await ctx.services.getCodebaseContext(ctx.task),
      });
    },
    onExit: async (ctx) => {
      ctx.qaPrepComplete = true;
      await ctx.services.eventEmitter.emit("qa_prep_completed", { taskId: ctx.task.id });
    },
  },

  // --- EXECUTION STATES ---
  {
    status: InternalStatus.EXECUTING,
    category: "execution",
    onEnter: async (ctx) => {
      ctx.task.startedAt = new Date();
      await ctx.services.agentSpawner.spawn("worker", {
        taskId: ctx.task.id,
        profile: ctx.project.workerProfile,
      });
      await ctx.services.eventEmitter.emit("task_execution_started", { taskId: ctx.task.id });
    },
    onExit: async (ctx) => {
      await ctx.services.eventEmitter.emit("task_execution_ended", { taskId: ctx.task.id });
    },
  },
  {
    status: InternalStatus.EXECUTION_DONE,
    category: "execution",
    onEnter: async (ctx) => {
      ctx.task.executionCompletedAt = new Date();
      await ctx.services.eventEmitter.emit("task_execution_done", { taskId: ctx.task.id });
    },
    autoTransition: {
      // If QA enabled, go to QA refining; otherwise go to review
      to: InternalStatus.QA_REFINING,
      condition: (ctx) => ctx.qaEnabled,
    },
  },

  // --- QA TESTING STATES ---
  {
    status: InternalStatus.QA_REFINING,
    category: "qa_test",
    onEnter: async (ctx) => {
      // Wait for QA prep if it hasn't completed yet
      if (!ctx.qaPrepComplete) {
        await ctx.services.agentSpawner.waitFor("qa-prep", ctx.task.id);
      }
      // Spawn QA refiner to update test plan based on actual implementation
      await ctx.services.agentSpawner.spawn("qa-refiner", {
        taskId: ctx.task.id,
        originalPlan: await ctx.services.getQaPlan(ctx.task.id),
        gitDiff: await ctx.services.getGitDiff(ctx.task.id),
      });
    },
    onExit: async (ctx) => {
      await ctx.services.eventEmitter.emit("qa_refinement_completed", { taskId: ctx.task.id });
    },
  },
  {
    status: InternalStatus.QA_TESTING,
    category: "qa_test",
    onEnter: async (ctx) => {
      await ctx.services.agentSpawner.spawn("qa-tester", {
        taskId: ctx.task.id,
        refinedPlan: await ctx.services.getRefinedQaPlan(ctx.task.id),
        browserUrl: ctx.project.browserTestingUrl,
      });
      await ctx.services.eventEmitter.emit("qa_testing_started", { taskId: ctx.task.id });
    },
    onExit: async (ctx) => {
      await ctx.services.eventEmitter.emit("qa_testing_ended", { taskId: ctx.task.id });
    },
  },
  {
    status: InternalStatus.QA_PASSED,
    category: "qa_test",
    onEnter: async (ctx) => {
      await ctx.services.eventEmitter.emit("qa_passed", { taskId: ctx.task.id });
    },
    autoTransition: {
      to: InternalStatus.PENDING_REVIEW,
      condition: () => true,  // Always proceed to review
    },
  },
  {
    status: InternalStatus.QA_FAILED,
    category: "qa_test",
    onEnter: async (ctx) => {
      await ctx.services.notifier.notify("qa_failed", {
        taskId: ctx.task.id,
        failures: ctx.task.qaFailures,
      });
    },
    // No auto-transition - requires manual intervention or retry
  },

  // --- REVIEW STATES ---
  {
    status: InternalStatus.PENDING_REVIEW,
    category: "review",
    onEnter: async (ctx) => {
      await ctx.services.agentSpawner.spawn("reviewer", {
        taskId: ctx.task.id,
        profile: ctx.project.reviewerProfile,
        artifacts: await ctx.services.getTaskArtifacts(ctx.task.id),
      });
    },
  },
  {
    status: InternalStatus.REVISION_NEEDED,
    category: "review",
    onEnter: async (ctx) => {
      await ctx.services.eventEmitter.emit("revision_needed", {
        taskId: ctx.task.id,
        feedback: ctx.task.reviewFeedback,
      });
    },
    autoTransition: {
      to: InternalStatus.EXECUTING,
      condition: () => true,
      delay: 500,  // Small delay for feedback to be processed
    },
  },

  // --- TERMINAL STATES ---
  {
    status: InternalStatus.APPROVED,
    category: "terminal",
    onEnter: async (ctx) => {
      ctx.task.completedAt = new Date();
      await ctx.services.eventEmitter.emit("task_approved", { taskId: ctx.task.id });
      // Unblock dependent tasks
      for (const dep of ctx.task.dependents) {
        await ctx.services.eventEmitter.emit("blocker_resolved", {
          blockerId: ctx.task.id,
          taskId: dep.id,
        });
      }
    },
  },
  {
    status: InternalStatus.FAILED,
    category: "terminal",
    onEnter: async (ctx) => {
      await ctx.services.notifier.notify("task_failed", {
        taskId: ctx.task.id,
        error: ctx.task.error,
      });
      await ctx.services.eventEmitter.emit("task_failed", { taskId: ctx.task.id });
    },
  },
  {
    status: InternalStatus.CANCELLED,
    category: "terminal",
    onEnter: async (ctx) => {
      await ctx.services.eventEmitter.emit("task_cancelled", { taskId: ctx.task.id });
    },
  },
];

// ═══════════════════════════════════════════════════════════════════════════
// TRANSITION DEFINITIONS
// ═══════════════════════════════════════════════════════════════════════════

const TRANSITIONS: Transition[] = [
  // ─────────────────────────────────────────────────────────────────────────
  // FROM BACKLOG
  // ─────────────────────────────────────────────────────────────────────────
  {
    from: InternalStatus.BACKLOG,
    to: InternalStatus.READY,
    trigger: "user",
  },
  {
    from: InternalStatus.BACKLOG,
    to: InternalStatus.CANCELLED,
    trigger: "user",
  },

  // ─────────────────────────────────────────────────────────────────────────
  // FROM READY
  // ─────────────────────────────────────────────────────────────────────────
  {
    from: InternalStatus.READY,
    to: InternalStatus.EXECUTING,
    trigger: "automatic",
    guard: (ctx) => !ctx.task.hasUnresolvedBlockers(),
    onTransition: async (ctx) => {
      await ctx.services.eventEmitter.emit("task_picked_up", { taskId: ctx.task.id });
    },
  },
  {
    from: InternalStatus.READY,
    to: InternalStatus.BLOCKED,
    trigger: "system",
    guard: (ctx) => ctx.task.hasUnresolvedBlockers(),
  },

  // ─────────────────────────────────────────────────────────────────────────
  // FROM BLOCKED
  // ─────────────────────────────────────────────────────────────────────────
  {
    from: InternalStatus.BLOCKED,
    to: InternalStatus.READY,
    trigger: "system",
    guard: (ctx) => ctx.task.blockers.every(b => b.resolved),
  },
  {
    from: InternalStatus.BLOCKED,
    to: InternalStatus.CANCELLED,
    trigger: "user",
  },

  // ─────────────────────────────────────────────────────────────────────────
  // FROM EXECUTING
  // ─────────────────────────────────────────────────────────────────────────
  {
    from: InternalStatus.EXECUTING,
    to: InternalStatus.EXECUTION_DONE,
    trigger: "agent",
  },
  {
    from: InternalStatus.EXECUTING,
    to: InternalStatus.FAILED,
    trigger: "agent",
    guard: (ctx) => ctx.task.hasUnrecoverableError,
  },
  {
    from: InternalStatus.EXECUTING,
    to: InternalStatus.BLOCKED,
    trigger: "agent",
    guard: (ctx) => ctx.task.needsHumanInput,
  },

  // ─────────────────────────────────────────────────────────────────────────
  // FROM EXECUTION_DONE
  // ─────────────────────────────────────────────────────────────────────────
  {
    from: InternalStatus.EXECUTION_DONE,
    to: InternalStatus.QA_REFINING,
    trigger: "automatic",
    guard: (ctx) => ctx.qaEnabled,
  },
  {
    from: InternalStatus.EXECUTION_DONE,
    to: InternalStatus.PENDING_REVIEW,
    trigger: "automatic",
    guard: (ctx) => !ctx.qaEnabled,
  },

  // ─────────────────────────────────────────────────────────────────────────
  // FROM QA_REFINING
  // ─────────────────────────────────────────────────────────────────────────
  {
    from: InternalStatus.QA_REFINING,
    to: InternalStatus.QA_TESTING,
    trigger: "agent",
  },
  {
    from: InternalStatus.QA_REFINING,
    to: InternalStatus.FAILED,
    trigger: "agent",
    guard: (ctx) => ctx.task.qaPrepFailed,
  },

  // ─────────────────────────────────────────────────────────────────────────
  // FROM QA_TESTING
  // ─────────────────────────────────────────────────────────────────────────
  {
    from: InternalStatus.QA_TESTING,
    to: InternalStatus.QA_PASSED,
    trigger: "agent",
    guard: (ctx) => ctx.task.qaResults.allPassed,
  },
  {
    from: InternalStatus.QA_TESTING,
    to: InternalStatus.QA_FAILED,
    trigger: "agent",
    guard: (ctx) => !ctx.task.qaResults.allPassed,
  },

  // ─────────────────────────────────────────────────────────────────────────
  // FROM QA_PASSED
  // ─────────────────────────────────────────────────────────────────────────
  {
    from: InternalStatus.QA_PASSED,
    to: InternalStatus.PENDING_REVIEW,
    trigger: "automatic",
  },

  // ─────────────────────────────────────────────────────────────────────────
  // FROM QA_FAILED
  // ─────────────────────────────────────────────────────────────────────────
  {
    from: InternalStatus.QA_FAILED,
    to: InternalStatus.REVISION_NEEDED,
    trigger: "system",
    onTransition: async (ctx) => {
      // Create revision task with QA failure details
      ctx.task.reviewFeedback = ctx.task.qaFailureReport;
    },
  },
  {
    from: InternalStatus.QA_FAILED,
    to: InternalStatus.PENDING_REVIEW,
    trigger: "user",  // Human can skip QA failures if needed
  },

  // ─────────────────────────────────────────────────────────────────────────
  // FROM PENDING_REVIEW
  // ─────────────────────────────────────────────────────────────────────────
  {
    from: InternalStatus.PENDING_REVIEW,
    to: InternalStatus.APPROVED,
    trigger: "agent",
    onTransition: async (ctx) => {
      await ctx.services.eventEmitter.emit("review_approved", { taskId: ctx.task.id });
    },
  },
  {
    from: InternalStatus.PENDING_REVIEW,
    to: InternalStatus.REVISION_NEEDED,
    trigger: "agent",
  },
  {
    from: InternalStatus.PENDING_REVIEW,
    to: InternalStatus.APPROVED,
    trigger: "user",  // Human override
  },

  // ─────────────────────────────────────────────────────────────────────────
  // FROM REVISION_NEEDED
  // ─────────────────────────────────────────────────────────────────────────
  {
    from: InternalStatus.REVISION_NEEDED,
    to: InternalStatus.EXECUTING,
    trigger: "automatic",
    onTransition: async (ctx) => {
      // Pass review feedback to worker
      await ctx.services.eventEmitter.emit("revision_started", {
        taskId: ctx.task.id,
        feedback: ctx.task.reviewFeedback,
      });
    },
  },

  // ─────────────────────────────────────────────────────────────────────────
  // FROM TERMINAL STATES (re-open)
  // ─────────────────────────────────────────────────────────────────────────
  {
    from: InternalStatus.FAILED,
    to: InternalStatus.READY,
    trigger: "user",
    onTransition: async (ctx) => {
      ctx.task.error = null;  // Clear error state
    },
  },
  {
    from: InternalStatus.CANCELLED,
    to: InternalStatus.READY,
    trigger: "user",
  },
  {
    from: InternalStatus.APPROVED,
    to: InternalStatus.READY,
    trigger: "user",  // Re-run task
  },
];
```

### State Machine Engine

```typescript
// ═══════════════════════════════════════════════════════════════════════════
// STATE MACHINE ENGINE
// ═══════════════════════════════════════════════════════════════════════════

class TaskStateMachine {
  private configs: Map<InternalStatus, StatusConfig>;
  private transitions: Transition[];

  constructor() {
    this.configs = new Map(STATUS_CONFIGS.map(c => [c.status, c]));
    this.transitions = TRANSITIONS;
  }

  /**
   * Attempt to transition a task to a new status.
   * @returns true if transition succeeded, false if blocked by guard
   * @throws InvalidTransitionError if transition not defined
   */
  async transition(
    ctx: StateMachineContext,
    to: InternalStatus,
    trigger: Transition["trigger"]
  ): Promise<boolean> {
    const from = ctx.task.internalStatus;

    // Find matching transition
    const trans = this.transitions.find(
      t => t.from === from && t.to === to && t.trigger === trigger
    );

    if (!trans) {
      throw new InvalidTransitionError(from, to, trigger);
    }

    // Check guard condition
    if (trans.guard && !trans.guard(ctx)) {
      return false;
    }

    // Execute lifecycle hooks
    const fromConfig = this.configs.get(from);
    const toConfig = this.configs.get(to);

    // 1. on_exit from current status
    if (fromConfig?.onExit) {
      await fromConfig.onExit(ctx);
    }

    // 2. on_transition callback
    if (trans.onTransition) {
      await trans.onTransition(ctx);
    }

    // 3. Update status
    ctx.task.internalStatus = to;
    ctx.task.statusChangedAt = new Date();
    await this.persistStatusChange(ctx, from, to, trigger);

    // 4. on_enter new status
    if (toConfig?.onEnter) {
      await toConfig.onEnter(ctx);
    }

    // 5. Check for auto-transitions
    if (toConfig?.autoTransition) {
      const { to: autoTo, condition, delay } = toConfig.autoTransition;
      if (delay) {
        setTimeout(() => this.checkAutoTransition(ctx, autoTo, condition), delay);
      } else if (condition(ctx)) {
        await this.transition(ctx, autoTo, "automatic");
      }
    }

    return true;
  }

  /**
   * Get all valid transitions from current status.
   */
  getValidTransitions(from: InternalStatus): Transition[] {
    return this.transitions.filter(t => t.from === from);
  }

  /**
   * Check if a transition is valid (exists and guard passes).
   */
  canTransition(ctx: StateMachineContext, to: InternalStatus): boolean {
    const from = ctx.task.internalStatus;
    const trans = this.transitions.find(t => t.from === from && t.to === to);
    if (!trans) return false;
    if (trans.guard && !trans.guard(ctx)) return false;
    return true;
  }

  private async checkAutoTransition(
    ctx: StateMachineContext,
    to: InternalStatus,
    condition: (ctx: StateMachineContext) => boolean
  ) {
    if (condition(ctx)) {
      await this.transition(ctx, to, "automatic");
    }
  }

  private async persistStatusChange(
    ctx: StateMachineContext,
    from: InternalStatus,
    to: InternalStatus,
    trigger: string
  ) {
    // Record in task_state_history table
    await ctx.services.db.insert("task_state_history", {
      task_id: ctx.task.id,
      from_status: from,
      to_status: to,
      trigger,
      changed_at: new Date(),
    });
  }
}
```

### Rust Implementation using statig

We use [**statig**](https://github.com/mdeloof/statig) - the most popular Rust state machine library (745+ stars, actively maintained).

**Why statig:**
- Hierarchical states (superstates) - perfect for grouping related states
- Async actions - needed for agent spawning
- State-local storage - attach context to specific states
- Compile-time validation - invalid transitions become compile errors
- Clean macro syntax - readable and maintainable
- no_std compatible

**Add to Cargo.toml:**
```toml
[dependencies]
statig = { version = "0.3", features = ["async"] }
```

```rust
// src/domain/task_state_machine.rs

use statig::prelude::*;
use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════════════════
// EVENTS (triggers for state transitions)
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub enum TaskEvent {
    // User actions
    Schedule,           // User moves task to Ready
    Cancel,             // User cancels task
    ForceApprove,       // Human override
    Retry,              // Retry from failed/cancelled
    SkipQa,             // Human skips QA failure

    // Agent signals
    ExecutionComplete,  // Worker finished
    ExecutionFailed { error: String },
    NeedsHumanInput { reason: String },
    QaRefinementComplete,
    QaTestsComplete { passed: bool },
    ReviewComplete { approved: bool, feedback: Option<String> },

    // System signals
    BlockersResolved,
    BlockerDetected { blocker_id: String },
}

// ═══════════════════════════════════════════════════════════════════════════
// SHARED CONTEXT (data available to all states)
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug)]
pub struct TaskContext {
    pub task_id: String,
    pub project_id: String,
    pub qa_enabled: bool,
    pub qa_prep_complete: bool,
    pub blockers: Vec<Blocker>,
    pub review_feedback: Option<String>,
    pub error: Option<String>,
    pub services: TaskServices,
}

#[derive(Debug)]
pub struct Blocker {
    pub id: String,
    pub resolved: bool,
}

// Services injected into the state machine
#[derive(Debug)]
pub struct TaskServices {
    pub agent_spawner: Box<dyn AgentSpawner>,
    pub event_emitter: Box<dyn EventEmitter>,
    pub notifier: Box<dyn Notifier>,
}

// ═══════════════════════════════════════════════════════════════════════════
// STATE MACHINE DEFINITION
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Default)]
pub struct TaskStateMachine;

// States that can hold data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QaFailedData {
    pub failures: Vec<QaFailure>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedData {
    pub error: String,
}

#[state_machine(
    initial = "State::backlog()",
    // Enable async for agent spawning
    state(derive(Debug, Clone, Serialize, Deserialize)),
    // Generate transition table for debugging
    on_transition = "Self::on_transition",
    on_dispatch = "Self::on_dispatch"
)]
impl TaskStateMachine {
    // ═══════════════════════════════════════════════════════════════════════
    // IDLE STATES
    // ═══════════════════════════════════════════════════════════════════════

    #[state]
    async fn backlog(context: &mut TaskContext, event: &TaskEvent) -> Response<State> {
        match event {
            TaskEvent::Schedule => Transition(State::ready()),
            TaskEvent::Cancel => Transition(State::cancelled()),
            _ => Super
        }
    }

    #[state]
    async fn ready(context: &mut TaskContext, event: &TaskEvent) -> Response<State> {
        match event {
            // Entry: spawn QA prep in background, then auto-transition to executing
            TaskEvent::BlockerDetected { blocker_id } => {
                context.blockers.push(Blocker { id: blocker_id.clone(), resolved: false });
                Transition(State::blocked())
            }
            TaskEvent::Cancel => Transition(State::cancelled()),
            _ => Super
        }
    }

    #[action]
    async fn enter_ready(context: &mut TaskContext) {
        // Spawn QA prep in background (non-blocking)
        if context.qa_enabled {
            context.services.agent_spawner.spawn_background("qa-prep", &context.task_id).await;
            context.services.event_emitter.emit("qa_prep_started", &context.task_id).await;
        }
        // Auto-transition to executing (handled by on_transition hook)
    }

    #[state]
    async fn blocked(context: &mut TaskContext, event: &TaskEvent) -> Response<State> {
        match event {
            TaskEvent::BlockersResolved => {
                context.blockers.iter_mut().for_each(|b| b.resolved = true);
                Transition(State::ready())
            }
            TaskEvent::Cancel => Transition(State::cancelled()),
            _ => Super
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // EXECUTION STATES (superstate groups executing + execution_done)
    // ═══════════════════════════════════════════════════════════════════════

    #[superstate]
    async fn execution(context: &mut TaskContext, event: &TaskEvent) -> Response<State> {
        // Common handling for all execution states
        match event {
            TaskEvent::Cancel => Transition(State::cancelled()),
            _ => Super
        }
    }

    #[state(superstate = "execution")]
    async fn executing(context: &mut TaskContext, event: &TaskEvent) -> Response<State> {
        match event {
            TaskEvent::ExecutionComplete => Transition(State::execution_done()),
            TaskEvent::ExecutionFailed { error } => {
                context.error = Some(error.clone());
                Transition(State::failed(FailedData { error: error.clone() }))
            }
            TaskEvent::NeedsHumanInput { reason } => {
                context.blockers.push(Blocker { id: reason.clone(), resolved: false });
                Transition(State::blocked())
            }
            _ => Super
        }
    }

    #[action]
    async fn enter_executing(context: &mut TaskContext) {
        context.services.agent_spawner.spawn("worker", &context.task_id).await;
        context.services.event_emitter.emit("task_execution_started", &context.task_id).await;
    }

    #[state(superstate = "execution")]
    async fn execution_done(context: &mut TaskContext, event: &TaskEvent) -> Response<State> {
        // Auto-transition based on QA setting
        if context.qa_enabled {
            Transition(State::qa_refining())
        } else {
            Transition(State::pending_review())
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // QA STATES (superstate groups all QA-related states)
    // ═══════════════════════════════════════════════════════════════════════

    #[superstate]
    async fn qa(context: &mut TaskContext, event: &TaskEvent) -> Response<State> {
        // Common handling for all QA states
        match event {
            TaskEvent::Cancel => Transition(State::cancelled()),
            TaskEvent::SkipQa => Transition(State::pending_review()),
            _ => Super
        }
    }

    #[state(superstate = "qa")]
    async fn qa_refining(context: &mut TaskContext, event: &TaskEvent) -> Response<State> {
        match event {
            TaskEvent::QaRefinementComplete => Transition(State::qa_testing()),
            _ => Super
        }
    }

    #[action]
    async fn enter_qa_refining(context: &mut TaskContext) {
        // Wait for QA prep if not complete
        if !context.qa_prep_complete {
            context.services.agent_spawner.wait_for("qa-prep", &context.task_id).await;
        }
        context.services.agent_spawner.spawn("qa-refiner", &context.task_id).await;
        context.services.event_emitter.emit("qa_refinement_started", &context.task_id).await;
    }

    #[state(superstate = "qa")]
    async fn qa_testing(context: &mut TaskContext, event: &TaskEvent) -> Response<State> {
        match event {
            TaskEvent::QaTestsComplete { passed: true } => Transition(State::qa_passed()),
            TaskEvent::QaTestsComplete { passed: false } => {
                Transition(State::qa_failed(QaFailedData { failures: vec![] }))
            }
            _ => Super
        }
    }

    #[action]
    async fn enter_qa_testing(context: &mut TaskContext) {
        context.services.agent_spawner.spawn("qa-tester", &context.task_id).await;
        context.services.event_emitter.emit("qa_testing_started", &context.task_id).await;
    }

    #[state(superstate = "qa")]
    async fn qa_passed(context: &mut TaskContext, event: &TaskEvent) -> Response<State> {
        // Auto-transition to review
        Transition(State::pending_review())
    }

    #[action]
    async fn enter_qa_passed(context: &mut TaskContext) {
        context.services.event_emitter.emit("qa_passed", &context.task_id).await;
    }

    #[state(superstate = "qa", entry_action = "enter_qa_failed")]
    async fn qa_failed(
        data: &mut QaFailedData,
        context: &mut TaskContext,
        event: &TaskEvent
    ) -> Response<State> {
        match event {
            TaskEvent::Retry => {
                context.review_feedback = Some("QA failures detected".to_string());
                Transition(State::revision_needed())
            }
            TaskEvent::SkipQa => Transition(State::pending_review()),  // Human override
            _ => Super
        }
    }

    #[action]
    async fn enter_qa_failed(context: &mut TaskContext) {
        context.services.notifier.notify("qa_failed", &context.task_id).await;
        context.services.event_emitter.emit("qa_failed", &context.task_id).await;
    }

    // ═══════════════════════════════════════════════════════════════════════
    // REVIEW STATES (superstate groups review-related states)
    // ═══════════════════════════════════════════════════════════════════════

    #[superstate]
    async fn review(context: &mut TaskContext, event: &TaskEvent) -> Response<State> {
        match event {
            TaskEvent::Cancel => Transition(State::cancelled()),
            _ => Super
        }
    }

    #[state(superstate = "review")]
    async fn pending_review(context: &mut TaskContext, event: &TaskEvent) -> Response<State> {
        match event {
            TaskEvent::ReviewComplete { approved: true, .. } => Transition(State::approved()),
            TaskEvent::ReviewComplete { approved: false, feedback } => {
                context.review_feedback = feedback.clone();
                Transition(State::revision_needed())
            }
            TaskEvent::ForceApprove => Transition(State::approved()),  // Human override
            _ => Super
        }
    }

    #[action]
    async fn enter_pending_review(context: &mut TaskContext) {
        context.services.agent_spawner.spawn("reviewer", &context.task_id).await;
        context.services.event_emitter.emit("review_started", &context.task_id).await;
    }

    #[state(superstate = "review")]
    async fn revision_needed(context: &mut TaskContext, event: &TaskEvent) -> Response<State> {
        // Auto-transition back to executing with feedback
        Transition(State::executing())
    }

    #[action]
    async fn enter_revision_needed(context: &mut TaskContext) {
        context.services.event_emitter.emit("revision_needed", &context.task_id).await;
    }

    // ═══════════════════════════════════════════════════════════════════════
    // TERMINAL STATES
    // ═══════════════════════════════════════════════════════════════════════

    #[state]
    async fn approved(context: &mut TaskContext, event: &TaskEvent) -> Response<State> {
        match event {
            TaskEvent::Retry => Transition(State::ready()),  // Re-run task
            _ => Super
        }
    }

    #[action]
    async fn enter_approved(context: &mut TaskContext) {
        context.services.event_emitter.emit("task_approved", &context.task_id).await;
        // Unblock dependent tasks would happen here
    }

    #[state(entry_action = "enter_failed")]
    async fn failed(
        data: &mut FailedData,
        context: &mut TaskContext,
        event: &TaskEvent
    ) -> Response<State> {
        match event {
            TaskEvent::Retry => {
                context.error = None;
                Transition(State::ready())
            }
            _ => Super
        }
    }

    #[action]
    async fn enter_failed(context: &mut TaskContext) {
        context.services.notifier.notify("task_failed", &context.task_id).await;
        context.services.event_emitter.emit("task_failed", &context.task_id).await;
    }

    #[state]
    async fn cancelled(context: &mut TaskContext, event: &TaskEvent) -> Response<State> {
        match event {
            TaskEvent::Retry => Transition(State::ready()),
            _ => Super
        }
    }

    #[action]
    async fn enter_cancelled(context: &mut TaskContext) {
        context.services.event_emitter.emit("task_cancelled", &context.task_id).await;
    }

    // ═══════════════════════════════════════════════════════════════════════
    // TRANSITION HOOKS (for logging, metrics, persistence)
    // ═══════════════════════════════════════════════════════════════════════

    fn on_transition(source: &State, target: &State, context: &TaskContext) {
        tracing::info!(
            task_id = %context.task_id,
            from = ?source,
            to = ?target,
            "Task state transition"
        );
        // Persist to task_state_history table
    }

    fn on_dispatch(state: StateOrSuperstate<Self>, event: &TaskEvent) {
        tracing::debug!(
            state = ?state,
            event = ?event,
            "Dispatching event"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// USAGE EXAMPLE
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_happy_path() {
        let mut context = TaskContext {
            task_id: "task-1".to_string(),
            project_id: "proj-1".to_string(),
            qa_enabled: true,
            qa_prep_complete: false,
            blockers: vec![],
            review_feedback: None,
            error: None,
            services: mock_services(),
        };

        let mut sm = TaskStateMachine::default().uninitialized_state_machine().init();

        // User schedules task
        sm.handle(&TaskEvent::Schedule).await;
        assert!(matches!(sm.state(), State::Ready));

        // Simulate the ready -> executing auto-transition
        // (In real code, this would be triggered by enter_ready action)

        // Worker completes
        sm.handle(&TaskEvent::ExecutionComplete).await;
        assert!(matches!(sm.state(), State::ExecutionDone));

        // QA refiner completes
        sm.handle(&TaskEvent::QaRefinementComplete).await;
        assert!(matches!(sm.state(), State::QaTesting));

        // QA tests pass
        sm.handle(&TaskEvent::QaTestsComplete { passed: true }).await;
        assert!(matches!(sm.state(), State::QaPassed));

        // Auto-transitions to pending_review, then reviewer approves
        sm.handle(&TaskEvent::ReviewComplete { approved: true, feedback: None }).await;
        assert!(matches!(sm.state(), State::Approved));
    }

    #[tokio::test]
    async fn test_qa_failure_retry() {
        let mut context = TaskContext { /* ... */ };
        let mut sm = TaskStateMachine::default().uninitialized_state_machine().init();

        // ... advance to qa_testing ...

        // QA fails
        sm.handle(&TaskEvent::QaTestsComplete { passed: false }).await;
        assert!(matches!(sm.state(), State::QaFailed(_)));

        // Retry creates revision
        sm.handle(&TaskEvent::Retry).await;
        assert!(matches!(sm.state(), State::RevisionNeeded));

        // Auto-transitions back to executing for rework
    }
}
```

### SQLite Integration with statig

**Pattern: SQLite as source of truth, statig for transition validation**

statig supports serde serialization, but we use a **rehydration pattern** where:
1. SQLite stores the current state (string enum)
2. On load: create state machine with that initial state
3. Process events → statig validates transitions
4. On transition: persist new state to SQLite

```rust
// src/domain/task_repository.rs

use crate::domain::task_state_machine::{TaskStateMachine, State, TaskEvent, TaskContext};
use rusqlite::{Connection, params};
use statig::prelude::*;

/// Repository that bridges SQLite persistence with statig state machine
pub struct TaskRepository {
    conn: Connection,
}

impl TaskRepository {
    /// Load a task and create its state machine from persisted state
    pub async fn load_task_with_state_machine(
        &self,
        task_id: &str,
    ) -> Result<(Task, StateMachine<TaskStateMachine>), AppError> {
        // 1. Load task from SQLite
        let task: Task = self.conn.query_row(
            "SELECT * FROM tasks WHERE id = ?",
            params![task_id],
            |row| Task::from_row(row),
        )?;

        // 2. Parse the persisted state
        let persisted_state: State = task.internal_status.parse()?;

        // 3. Create state machine with the persisted state as initial
        let sm = TaskStateMachine::default()
            .uninitialized_state_machine()
            .init_with_state(persisted_state);  // statig supports custom initial state

        Ok((task, sm))
    }

    /// Process an event and persist the new state atomically
    pub async fn process_event(
        &self,
        task_id: &str,
        event: TaskEvent,
        context: &mut TaskContext,
    ) -> Result<State, AppError> {
        // 1. Load task and state machine
        let (mut task, mut sm) = self.load_task_with_state_machine(task_id).await?;

        let old_state = sm.state().clone();

        // 2. Process event through statig (validates transition)
        sm.handle_with_context(&event, context).await;

        let new_state = sm.state().clone();

        // 3. If state changed, persist to SQLite
        if old_state != new_state {
            self.persist_state_change(&task_id, &old_state, &new_state, &event).await?;
        }

        Ok(new_state)
    }

    /// Persist state change to SQLite with audit log
    async fn persist_state_change(
        &self,
        task_id: &str,
        from: &State,
        to: &State,
        event: &TaskEvent,
    ) -> Result<(), AppError> {
        let tx = self.conn.transaction()?;

        // Update task status
        tx.execute(
            "UPDATE tasks SET internal_status = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
            params![to.to_string(), task_id],
        )?;

        // Record in audit log
        tx.execute(
            r#"INSERT INTO task_state_history (id, task_id, from_status, to_status, trigger, created_at)
               VALUES (?, ?, ?, ?, ?, CURRENT_TIMESTAMP)"#,
            params![
                uuid::Uuid::new_v4().to_string(),
                task_id,
                from.to_string(),
                to.to_string(),
                format!("{:?}", event),
            ],
        )?;

        tx.commit()?;
        Ok(())
    }
}
```

### State Serialization for SQLite

```rust
// src/domain/task_state_machine.rs (additions)

use serde::{Deserialize, Serialize};

// Enable serde for State enum
#[state_machine(
    initial = "State::backlog()",
    state(derive(Debug, Clone, Serialize, Deserialize, PartialEq)),
    // ...
)]
impl TaskStateMachine { /* ... */ }

// Implement string conversion for SQLite storage
impl std::fmt::Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Serialize state to string for SQLite TEXT column
        match self {
            State::Backlog => write!(f, "backlog"),
            State::Ready => write!(f, "ready"),
            State::Blocked => write!(f, "blocked"),
            State::Executing => write!(f, "executing"),
            State::ExecutionDone => write!(f, "execution_done"),
            State::QaRefining => write!(f, "qa_refining"),
            State::QaTesting => write!(f, "qa_testing"),
            State::QaPassed => write!(f, "qa_passed"),
            State::QaFailed(_) => write!(f, "qa_failed"),
            State::PendingReview => write!(f, "pending_review"),
            State::RevisionNeeded => write!(f, "revision_needed"),
            State::Approved => write!(f, "approved"),
            State::Failed(_) => write!(f, "failed"),
            State::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl std::str::FromStr for State {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "backlog" => Ok(State::Backlog),
            "ready" => Ok(State::Ready),
            "blocked" => Ok(State::Blocked),
            "executing" => Ok(State::Executing),
            "execution_done" => Ok(State::ExecutionDone),
            "qa_refining" => Ok(State::QaRefining),
            "qa_testing" => Ok(State::QaTesting),
            "qa_passed" => Ok(State::QaPassed),
            "qa_failed" => Ok(State::QaFailed(QaFailedData::default())),
            "pending_review" => Ok(State::PendingReview),
            "revision_needed" => Ok(State::RevisionNeeded),
            "approved" => Ok(State::Approved),
            "failed" => Ok(State::Failed(FailedData::default())),
            "cancelled" => Ok(State::Cancelled),
            _ => Err(AppError::InvalidStatus(s.to_string())),
        }
    }
}
```

### State-Local Data Persistence

States with data (`qa_failed`, `failed`) need extra columns:

```sql
-- Store state-local data in separate columns
ALTER TABLE tasks ADD COLUMN qa_failure_data TEXT;  -- JSON for QaFailedData
ALTER TABLE tasks ADD COLUMN error_data TEXT;       -- JSON for FailedData

-- Or use a dedicated state_data table
CREATE TABLE task_state_data (
    task_id TEXT PRIMARY KEY REFERENCES tasks(id),
    state_type TEXT NOT NULL,    -- 'qa_failed' | 'failed'
    data TEXT NOT NULL,          -- JSON serialized state data
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

```rust
// Loading state with data
impl TaskRepository {
    fn load_state_with_data(&self, task: &Task) -> Result<State, AppError> {
        match task.internal_status.as_str() {
            "qa_failed" => {
                let data: QaFailedData = serde_json::from_str(
                    &task.qa_failure_data.as_ref().unwrap_or(&"{}".to_string())
                )?;
                Ok(State::QaFailed(data))
            }
            "failed" => {
                let data: FailedData = serde_json::from_str(
                    &task.error_data.as_ref().unwrap_or(&"{}".to_string())
                )?;
                Ok(State::Failed(data))
            }
            other => other.parse(),
        }
    }
}
```

### Transaction Safety

All state transitions must be atomic:

```rust
/// Execute a state transition within a database transaction
pub async fn transition_atomically<F, Fut>(
    &self,
    task_id: &str,
    event: TaskEvent,
    side_effect: F,
) -> Result<State, AppError>
where
    F: FnOnce(&Task, &State) -> Fut,
    Fut: Future<Output = Result<(), AppError>>,
{
    let tx = self.conn.transaction()?;

    // 1. Load and lock the task row
    let task = tx.query_row(
        "SELECT * FROM tasks WHERE id = ? FOR UPDATE",
        params![task_id],
        |row| Task::from_row(row),
    )?;

    // 2. Create state machine and process event
    let mut sm = self.create_state_machine(&task)?;
    let mut context = self.create_context(&task).await?;

    let old_state = sm.state().clone();
    sm.handle_with_context(&event, &mut context).await;
    let new_state = sm.state().clone();

    // 3. Execute side effect (e.g., spawn agent)
    side_effect(&task, &new_state).await?;

    // 4. Persist state change
    if old_state != new_state {
        tx.execute(
            "UPDATE tasks SET internal_status = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
            params![new_state.to_string(), task_id],
        )?;
    }

    tx.commit()?;
    Ok(new_state)
}
```

### Cargo.toml Dependencies

```toml
[dependencies]
statig = { version = "0.3", features = ["async"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
rusqlite = { version = "0.31", features = ["bundled"] }
uuid = { version = "1.0", features = ["v4"] }
```

### Hierarchical State Diagram (using statig superstates)

```
┌─────────────────────────────────────────────────────────────────────────────────────────┐
│                                     TaskStateMachine                                     │
├─────────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                         │
│  ┌─────────┐         ┌───────┐         ┌─────────┐                                     │
│  │ BACKLOG │ ──────► │ READY │ ──────► │ BLOCKED │                                     │
│  └─────────┘         └───┬───┘         └────┬────┘                                     │
│                          │                  │                                           │
│                          │ auto             │ blockers_resolved                         │
│                          ▼                  │                                           │
│  ┌──────────────────────────────────────────┼────────────────────────────────────────┐ │
│  │ <<superstate>> EXECUTION                 │                                        │ │
│  │ ┌───────────┐         ┌────────────────┐ │                                        │ │
│  │ │ EXECUTING │ ──────► │ EXECUTION_DONE │◄┘                                        │ │
│  │ └───────────┘         └───────┬────────┘                                          │ │
│  └───────────────────────────────┼───────────────────────────────────────────────────┘ │
│                                  │                                                      │
│                    ┌─────────────┴─────────────┐                                       │
│                    │ [qa_enabled]               │ [!qa_enabled]                         │
│                    ▼                            │                                       │
│  ┌─────────────────────────────────────────────┼────────────────────────────────────┐ │
│  │ <<superstate>> QA                           │                                    │ │
│  │ ┌─────────────┐     ┌────────────┐          │                                    │ │
│  │ │ QA_REFINING │ ──► │ QA_TESTING │          │                                    │ │
│  │ └─────────────┘     └─────┬──────┘          │                                    │ │
│  │                     ┌─────┴─────┐           │                                    │ │
│  │                     ▼           ▼           │                                    │ │
│  │              ┌───────────┐ ┌───────────┐    │                                    │ │
│  │              │ QA_PASSED │ │ QA_FAILED │    │                                    │ │
│  │              └─────┬─────┘ └─────┬─────┘    │                                    │ │
│  └────────────────────┼─────────────┼─────────────────────────────────────────────────┘ │
│                       │             │ retry     │                                       │
│                       ▼             ▼           ▼                                       │
│  ┌────────────────────────────────────────────────────────────────────────────────────┐ │
│  │ <<superstate>> REVIEW                                                              │ │
│  │ ┌────────────────┐         ┌─────────────────┐                                    │ │
│  │ │ PENDING_REVIEW │ ──────► │ REVISION_NEEDED │ ─────► (back to EXECUTING)         │ │
│  │ └───────┬────────┘         └─────────────────┘                                    │ │
│  └─────────┼──────────────────────────────────────────────────────────────────────────┘ │
│            │ approved                                                                   │
│            ▼                                                                            │
│  ┌──────────────────────────────────────────────────────────────────────────────────┐  │
│  │ <<terminal>>                                                                      │  │
│  │ ┌──────────┐     ┌────────┐     ┌───────────┐                                    │  │
│  │ │ APPROVED │     │ FAILED │     │ CANCELLED │  ◄── (from any non-terminal state) │  │
│  │ └──────────┘     └────────┘     └───────────┘                                    │  │
│  └──────────────────────────────────────────────────────────────────────────────────┘  │
│                                                                                         │
└─────────────────────────────────────────────────────────────────────────────────────────┘
```
```

### Visual State Diagram

```
                                    ┌─────────────────────────────────────────────────────────────────┐
                                    │                    QA_PREPPING (background)                     │
                                    │                 Runs in parallel with execution                 │
                                    └─────────────────────────────────────────────────────────────────┘
                                                              ▲
                                                              │ spawned on enter
                                                              │
┌─────────┐    user    ┌───────┐    auto     ┌───────────┐   │   agent   ┌────────────────┐
│ BACKLOG │ ─────────► │ READY │ ──────────► │ EXECUTING │ ──┼─────────► │ EXECUTION_DONE │
└─────────┘            └───────┘             └───────────┘   │           └────────────────┘
     │                      │                      │         │                  │
     │                      │                      │         │                  ├─── [QA enabled] ───►  QA_REFINING ──► QA_TESTING ──┬─► QA_PASSED ──► PENDING_REVIEW
     │                      ▼                      ▼         │                  │                                                     │
     │                 ┌─────────┐             ┌────────┐    │                  └─── [QA disabled] ─────────────────────────────────────► PENDING_REVIEW
     │                 │ BLOCKED │ ◄───────── │ FAILED │ ◄──┘                                                                         │
     │                 └─────────┘             └────────┘                                                                              │
     │                      │                      ▲                               QA_FAILED ◄──────────────────────────────────────────┘
     │                      │ blockers             │                                   │
     │                      │ resolved             │                                   ▼
     │                      ▼                      │                             REVISION_NEEDED
     │                 ┌───────┐                   │                                   │
     └─────────────────│ READY │                   │                                   │ auto
                       └───────┘                   │                                   ▼
                                                   │                             ┌───────────┐
                                                   └─────────────────────────────│ EXECUTING │
                                                                                 └───────────┘
                                                                                       │
                                                                                       ▼
┌───────────┐         ┌──────────┐                                             ┌────────────────┐
│ CANCELLED │ ◄────── │ APPROVED │ ◄─────────────────────────────────────────── │ PENDING_REVIEW │
└───────────┘         └──────────┘                                             └────────────────┘
```

---

## Custom Workflow Schemas

Users can define custom boards that map to internal statuses, enabling Jira-style, GitHub-style, or methodology-specific workflows.

### Workflow Definition

```typescript
interface WorkflowSchema {
  id: string;
  name: string;
  description: string;
  columns: WorkflowColumn[];
  externalSync?: ExternalSyncConfig;
  defaults: {
    workerProfile?: string;
    reviewerProfile?: string;
  };
}

interface WorkflowColumn {
  id: string;
  name: string;              // Display: "In QA", "Ready for Dev", "Selected"
  color?: string;
  icon?: string;
  mapsTo: InternalStatus;    // Maps to internal status for side effects
  behavior?: {
    skipReview?: boolean;
    autoAdvance?: boolean;
    agentProfile?: string;   // Override agent for this column
  };
}
```

### Built-in Workflows

**Default RalphX:**
```typescript
const defaultWorkflow: WorkflowSchema = {
  id: "ralphx-default",
  name: "RalphX Default",
  columns: [
    { id: "draft", name: "Draft", mapsTo: "backlog" },
    { id: "backlog", name: "Backlog", mapsTo: "backlog" },
    { id: "todo", name: "To Do", mapsTo: "ready" },
    { id: "planned", name: "Planned", mapsTo: "ready" },
    { id: "in_progress", name: "In Progress", mapsTo: "executing" },
    { id: "in_review", name: "In Review", mapsTo: "pending_review" },
    { id: "done", name: "Done", mapsTo: "approved" },
  ],
};
```

**Jira-Compatible:**
```typescript
const jiraWorkflow: WorkflowSchema = {
  id: "jira-compat",
  name: "Jira Compatible",
  columns: [
    { id: "backlog", name: "Backlog", mapsTo: "backlog" },
    { id: "selected", name: "Selected for Dev", mapsTo: "ready" },
    { id: "in_progress", name: "In Progress", mapsTo: "executing" },
    { id: "in_qa", name: "In QA", mapsTo: "pending_review" },
    { id: "done", name: "Done", mapsTo: "approved" },
  ],
  externalSync: { provider: "jira", direction: "bidirectional" },
};
```

### External Sync (Future)

```typescript
interface ExternalSyncConfig {
  provider: "jira" | "github" | "linear" | "notion";
  mapping: Record<string, ExternalStatusMapping>;
  sync: {
    direction: "pull" | "push" | "bidirectional";
    webhook?: boolean;
  };
  conflictResolution: "external_wins" | "internal_wins" | "manual";
}
```

---

## Agent Profiles (Using Claude Code Components)

RalphX agents are **compositions of Claude Code native components**:
- **Claude Code Agents** (`.claude/agents/*.md`) - execution environment
- **Claude Code Skills** (`.claude/skills/*/SKILL.md`) - capabilities/knowledge
- **Claude Code Hooks** - lifecycle automation
- **MCP Servers** - external tool integration

### Agent Profile Schema

```typescript
interface AgentProfile {
  id: string;
  name: string;
  description: string;
  role: "worker" | "reviewer" | "supervisor" | "orchestrator" | "researcher";

  // Claude Code component references
  claudeCode: {
    agentDefinition: string;     // Path to .claude/agents/*.md
    skills: string[];            // Skills to inject at startup
    hooks?: HooksConfig;         // Agent-scoped hooks
    mcpServers?: string[];       // MCP servers to enable
  };

  // Execution configuration
  execution: {
    model: "opus" | "sonnet" | "haiku";
    maxIterations: number;
    timeoutMinutes: number;
    permissionMode: "default" | "acceptEdits" | "bypassPermissions";
  };

  // Artifact I/O
  io: {
    inputArtifactTypes: ArtifactType[];
    outputArtifactTypes: ArtifactType[];
  };

  // Behavioral flags
  behavior: {
    canSpawnSubAgents: boolean;
    autoCommit: boolean;
    autonomyLevel: "supervised" | "semi_autonomous" | "fully_autonomous";
  };
}
```

### Built-in Agent Profiles

| Profile | Role | Model | Max Iterations | Key Skills |
|---------|------|-------|----------------|------------|
| `worker` | Task execution | Sonnet | 30 | coding-standards, testing-patterns, git-workflow |
| `reviewer` | Code review | Sonnet | 10 | code-review-checklist, security-patterns |
| `supervisor` | Watchdog | Haiku | 100 | anomaly-detection, intervention-patterns |
| `orchestrator` | Planning | Opus | 50 | planning, delegation, synthesis |
| `deep-researcher` | Research | Opus | 200 | research-methodology, source-verification |

### Claude Code Agent Definition Example

`.claude/agents/worker.md`:
```markdown
---
name: ralphx-worker
description: Executes implementation tasks autonomously
tools: Read, Write, Edit, Bash, Grep, Glob, Git
permissionMode: acceptEdits
skills:
  - coding-standards
  - testing-patterns
  - git-workflow
hooks:
  PostToolUse:
    - matcher: "Write|Edit"
      hooks:
        - type: command
          command: "npm run lint:fix"
---

You are a focused developer agent executing a specific task.

## Your Mission
Complete the assigned task by:
1. Understanding requirements fully
2. Writing clean, tested code
3. Committing atomic changes

## Constraints
- Only modify files directly related to the task
- Run tests before marking complete
- Keep changes minimal and focused
```

### Claude Code Skill Definition Example

`.claude/skills/coding-standards/SKILL.md`:
```markdown
---
name: coding-standards
description: Project coding standards and patterns
disable-model-invocation: true
user-invocable: false
---

## Coding Standards

### TypeScript
- Use strict mode
- Prefer const over let
- Use explicit return types on functions

### React
- Functional components only
- Use hooks for state management
- Props interfaces above component

### Testing
- Test file next to source: `Component.test.tsx`
- Use React Testing Library
- Mock external dependencies
```

---

## Artifact System

Artifacts are typed documents that flow between processes - outputs from one process become inputs to another.

### Artifact Types

```typescript
type ArtifactType =
  // Documents
  | "prd" | "research_document" | "design_doc" | "specification"
  // Code
  | "code_change" | "diff" | "test_result"
  // Process
  | "task_spec" | "review_feedback" | "approval" | "findings" | "recommendations"
  // Context
  | "context" | "previous_work" | "research_brief"
  // Logs
  | "activity_log" | "alert" | "intervention";

interface Artifact {
  id: string;
  type: ArtifactType;
  name: string;
  content: ArtifactContent;
  metadata: {
    createdAt: Date;
    createdBy: string;  // Agent profile ID or "user"
    taskId?: string;
    processId?: string;
    version: number;
  };
  derivedFrom?: string[];  // Parent artifact IDs
}
```

### Artifact Buckets

Buckets organize artifacts by purpose with access control:

| Bucket | Accepted Types | Writers | Readers |
|--------|---------------|---------|---------|
| `research-outputs` | research_document, findings, recommendations | deep-researcher, orchestrator | all |
| `work-context` | context, task_spec, previous_work | orchestrator, system | worker, reviewer |
| `code-changes` | code_change, diff, test_result | worker | reviewer |
| `prd-library` | prd, specification, design_doc | orchestrator, user | all |

### Artifact Flow Engine

Automate artifact routing between processes:

```typescript
interface ArtifactFlow {
  id: string;
  trigger: {
    event: "artifact_created" | "task_completed" | "process_completed";
    filter?: { artifactTypes?: ArtifactType[]; sourceBucket?: string };
  };
  steps: ArtifactFlowStep[];
}

// Example: Research → Task Decomposition
const researchToDevFlow: ArtifactFlow = {
  id: "research-to-dev",
  trigger: {
    event: "artifact_created",
    filter: { artifactTypes: ["recommendations"], sourceBucket: "research-outputs" },
  },
  steps: [
    { type: "copy", toBucket: "prd-library" },
    { type: "spawn_process", processType: "task_decomposition", agentProfile: "orchestrator" },
  ],
};
```

---

## Methodology Support

RalphX can support external development methodologies as extensions.

### Methodology = Workflow + Agents + Artifacts

**Key insight**: A methodology brings its own Kanban board structure. When a user activates a methodology, the Kanban columns change to reflect that methodology's workflow while still mapping to our internal statuses for side effects.

### BMAD Method Integration

**BMAD** (Breakthrough Method for Agile AI-Driven Development) uses:
- **8 agents**: Analyst, PM, Architect, UX Designer, Developer, Scrum Master, TEA, Tech Writer
- **4 phases**: Analysis → Planning → Solutioning → Implementation
- **Document-centric**: PRD, Architecture Doc, UX Design, Stories/Epics

**BMAD Kanban Workflow:**
```typescript
const bmadWorkflow: WorkflowSchema = {
  id: "bmad-method",
  name: "BMAD Method",
  description: "Breakthrough Method for Agile AI-Driven Development",
  columns: [
    // Phase 1: Analysis
    { id: "brainstorm", name: "Brainstorm", mapsTo: "backlog",
      behavior: { agentProfile: "bmad-analyst" } },
    { id: "research", name: "Research", mapsTo: "executing",
      behavior: { agentProfile: "bmad-analyst" } },

    // Phase 2: Planning
    { id: "prd-draft", name: "PRD Draft", mapsTo: "executing",
      behavior: { agentProfile: "bmad-pm" } },
    { id: "prd-review", name: "PRD Review", mapsTo: "pending_review",
      behavior: { agentProfile: "bmad-pm" } },
    { id: "ux-design", name: "UX Design", mapsTo: "executing",
      behavior: { agentProfile: "bmad-ux" } },

    // Phase 3: Solutioning
    { id: "architecture", name: "Architecture", mapsTo: "executing",
      behavior: { agentProfile: "bmad-architect" } },
    { id: "stories", name: "Stories", mapsTo: "ready",
      behavior: { agentProfile: "bmad-pm" } },

    // Phase 4: Implementation
    { id: "sprint", name: "Sprint", mapsTo: "executing",
      behavior: { agentProfile: "bmad-developer" } },
    { id: "code-review", name: "Code Review", mapsTo: "pending_review",
      behavior: { agentProfile: "bmad-developer" } },
    { id: "done", name: "Done", mapsTo: "approved" },
  ],
};
```

**Mapping to RalphX:**
| BMAD Concept | RalphX Equivalent |
|--------------|-------------------|
| Agent personas | Agent profiles with different skills |
| Workflows (BP, CP, CA, DS) | Skills with step-based execution |
| Documents (PRD, Architecture) | Artifacts in buckets |
| Phase progression | Workflow columns (each phase = column group) |
| Validation checklists | Review hooks |

### GSD Method Integration

**GSD** (Get Shit Done) uses:
- **11 agents**: project-researcher, phase-researcher, planner, executor, verifier, debugger, etc.
- **Wave-based parallelization**: Plans grouped into waves for parallel execution
- **Checkpoint protocol**: human-verify, decision, human-action types
- **Goal-backward verification**: must-haves derived from phase goals

**GSD Kanban Workflow:**
```typescript
const gsdWorkflow: WorkflowSchema = {
  id: "gsd-method",
  name: "GSD (Get Shit Done)",
  description: "Spec-driven development with wave-based parallelization",
  columns: [
    // Initialize
    { id: "initialize", name: "Initialize", mapsTo: "backlog",
      behavior: { agentProfile: "gsd-project-researcher" } },

    // Discuss (optional)
    { id: "discuss", name: "Discuss", mapsTo: "blocked",
      behavior: { agentProfile: "gsd-orchestrator" } },

    // Plan
    { id: "research", name: "Research", mapsTo: "executing",
      behavior: { agentProfile: "gsd-phase-researcher" } },
    { id: "planning", name: "Planning", mapsTo: "executing",
      behavior: { agentProfile: "gsd-planner" } },
    { id: "plan-check", name: "Plan Check", mapsTo: "pending_review",
      behavior: { agentProfile: "gsd-plan-checker" } },

    // Execute (wave-based)
    { id: "queued", name: "Queued", mapsTo: "ready" },
    { id: "executing", name: "Executing", mapsTo: "executing",
      behavior: { agentProfile: "gsd-executor" } },
    { id: "checkpoint", name: "Checkpoint", mapsTo: "blocked" },

    // Verify
    { id: "verifying", name: "Verifying", mapsTo: "pending_review",
      behavior: { agentProfile: "gsd-verifier" } },
    { id: "debugging", name: "Debugging", mapsTo: "revision_needed",
      behavior: { agentProfile: "gsd-debugger" } },

    // Complete
    { id: "done", name: "Done", mapsTo: "approved" },
  ],
};
```

**GSD-specific task fields:**
```typescript
// Extended task for GSD methodology
interface GSDTask extends Task {
  wave?: number;           // Wave 1, 2, 3... for parallel execution
  checkpoint_type?: "auto" | "human-verify" | "decision" | "human-action";
  phase_id?: string;       // "01-setup", "02-core", etc.
  plan_id?: string;        // "01-01", "01-02" within phase
  must_haves?: {
    truths: string[];      // Observable behaviors
    artifacts: string[];   // Required file paths
    key_links: string[];   // Component connections to verify
  };
}
```

**Mapping to RalphX:**
| GSD Concept | RalphX Equivalent |
|-------------|-------------------|
| Phases + Plans | Tasks with `phase_id` and `plan_id` fields |
| Waves | Task `wave` field for parallel execution grouping |
| Checkpoints | Task `checkpoint_type` + `blocked` internal status |
| Must-haves | Task `must_haves` field + verification hooks |
| Model profiles | Agent profile `execution.model` setting |
| STATE.md | Activity log + task state history |

### How Methodology Switching Works

When user activates a methodology:
1. **Workflow changes** - Kanban columns update to methodology's workflow
2. **Agent profiles load** - Methodology's agents become available
3. **Skills inject** - Methodology's skills available to agents
4. **Artifact templates ready** - Document templates in buckets
5. **Hooks activate** - Methodology-specific lifecycle hooks

```
User selects "BMAD Method" for project
       ↓
Load bmadWorkflow → Update Kanban columns
       ↓
Load BMAD agent profiles (analyst, pm, architect, etc.)
       ↓
Inject BMAD skills into agents
       ↓
Create artifact buckets (prd-drafts, architecture-docs, etc.)
       ↓
Activate BMAD hooks (validation checklists, phase gates)
       ↓
Project now uses BMAD workflow with all side effects intact
```

### Methodology Extension Schema

```typescript
interface MethodologyExtension {
  id: string;
  name: string;
  description: string;

  // Agent profiles this methodology provides
  agentProfiles: AgentProfile[];

  // Skills bundled with methodology
  skills: string[];  // Paths to skill directories

  // Custom workflow for this methodology
  workflow: WorkflowSchema;

  // Phase/stage definitions
  phases?: {
    id: string;
    name: string;
    order: number;
    agentProfiles: string[];  // Which agents work in this phase
  }[];

  // Document templates
  templates?: {
    type: ArtifactType;
    templatePath: string;
  }[];

  // Hooks for methodology-specific behavior
  hooks?: HooksConfig;
}
```

---

## Deep Research Loops

Support for long-running research agents with configurable depth.

### Research Process Configuration

```typescript
interface ResearchProcess {
  id: string;
  name: string;
  brief: {
    question: string;
    context?: string;
    scope?: string;
    constraints?: string[];
  };
  depth: ResearchDepthPreset | CustomDepth;
  agentProfileId: string;
  output: {
    targetBucket: string;
    artifactTypes: ArtifactType[];
  };
  progress: {
    currentIteration: number;
    status: "pending" | "running" | "paused" | "completed" | "failed";
    lastCheckpoint?: string;  // Artifact ID
  };
}

interface CustomDepth {
  maxIterations: number;
  timeoutHours: number;
  checkpointInterval: number;  // Save progress every N iterations
}
```

### Research Depth Presets

| Preset | Iterations | Timeout | Use Case |
|--------|------------|---------|----------|
| `quick-scan` | 10 | 30 min | Fast overview |
| `standard` | 50 | 2 hrs | Thorough investigation |
| `deep-dive` | 200 | 8 hrs | Comprehensive analysis |
| `exhaustive` | 500 | 24 hrs | Leave no stone unturned |

### Integration with Orchestrator

The Orchestrator can spawn research before creating tasks:

```markdown
## Planning Phase
Before creating tasks, spawn deep-researcher if:
- Task requires technology decision
- Domain is unfamiliar
- User explicitly requests research

Research outputs become:
1. Context artifacts for workers
2. Input for PRD refinement
3. Basis for task decomposition
```

---

## Extensibility Database Schema

Additional tables for the extensibility layer:

```sql
-- Workflows
CREATE TABLE workflows (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  description TEXT,
  schema_json TEXT NOT NULL,  -- Full WorkflowSchema as JSON
  is_default BOOLEAN DEFAULT FALSE,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Agent Profiles
CREATE TABLE agent_profiles (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  role TEXT NOT NULL,
  profile_json TEXT NOT NULL,  -- Full AgentProfile as JSON
  is_builtin BOOLEAN DEFAULT FALSE,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Artifacts
CREATE TABLE artifacts (
  id TEXT PRIMARY KEY,
  type TEXT NOT NULL,
  name TEXT NOT NULL,
  content_type TEXT NOT NULL,  -- "inline" | "file"
  content_text TEXT,
  content_path TEXT,
  bucket_id TEXT REFERENCES artifact_buckets(id),
  task_id TEXT REFERENCES tasks(id),
  process_id TEXT REFERENCES processes(id),
  created_by TEXT NOT NULL,
  version INTEGER DEFAULT 1,
  previous_version_id TEXT,
  metadata_json TEXT,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Artifact Buckets
CREATE TABLE artifact_buckets (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  config_json TEXT NOT NULL,
  is_system BOOLEAN DEFAULT FALSE
);

-- Artifact Relations (derivedFrom, relatedTo)
CREATE TABLE artifact_relations (
  id TEXT PRIMARY KEY,
  from_artifact_id TEXT NOT NULL REFERENCES artifacts(id),
  to_artifact_id TEXT NOT NULL REFERENCES artifacts(id),
  relation_type TEXT NOT NULL  -- "derived_from" | "related_to"
);

-- Processes (Research loops, etc.)
CREATE TABLE processes (
  id TEXT PRIMARY KEY,
  type TEXT NOT NULL,  -- "research" | "development" | "review"
  name TEXT NOT NULL,
  config_json TEXT NOT NULL,
  status TEXT NOT NULL,
  current_iteration INTEGER DEFAULT 0,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  started_at DATETIME,
  completed_at DATETIME
);

-- Task extensions for methodology support
ALTER TABLE tasks ADD COLUMN external_status TEXT;
ALTER TABLE tasks ADD COLUMN internal_status TEXT;
ALTER TABLE tasks ADD COLUMN wave INTEGER;  -- For parallel execution grouping
ALTER TABLE tasks ADD COLUMN checkpoint_type TEXT;  -- "auto" | "human-verify" | "decision" | "human-action"
ALTER TABLE tasks ADD COLUMN phase_id TEXT;
ALTER TABLE tasks ADD COLUMN plan_id TEXT;

-- Task dependencies
CREATE TABLE task_dependencies (
  id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(id),
  depends_on_task_id TEXT NOT NULL REFERENCES tasks(id),
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Methodology extensions
CREATE TABLE methodology_extensions (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  description TEXT,
  config_json TEXT NOT NULL,
  is_active BOOLEAN DEFAULT FALSE,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Indexes
CREATE INDEX idx_artifacts_bucket ON artifacts(bucket_id);
CREATE INDEX idx_artifacts_type ON artifacts(type);
CREATE INDEX idx_tasks_internal_status ON tasks(internal_status);
CREATE INDEX idx_tasks_wave ON tasks(wave);
CREATE INDEX idx_processes_status ON processes(status);
```

---

## RalphX Plugin Structure

RalphX ships with a Claude Code plugin containing agents, skills, and hooks:

```
ralphx-plugin/
├── .claude-plugin/
│   └── plugin.json
├── agents/
│   ├── worker.md
│   ├── reviewer.md
│   ├── supervisor.md
│   ├── orchestrator.md
│   └── deep-researcher.md
├── skills/
│   ├── coding-standards/SKILL.md
│   ├── testing-patterns/SKILL.md
│   ├── code-review-checklist/SKILL.md
│   ├── research-methodology/SKILL.md
│   └── git-workflow/SKILL.md
├── hooks/
│   └── hooks.json
└── .mcp.json
```

### plugin.json

```json
{
  "name": "ralphx",
  "description": "Autonomous development loop with extensible workflows",
  "version": "1.0.0",
  "author": { "name": "RalphX" },
  "agents": "./agents/",
  "skills": "./skills/",
  "hooks": "./hooks/hooks.json",
  "mcpServers": "./.mcp.json"
}
```

### hooks.json

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Write|Edit",
        "hooks": [
          {
            "type": "command",
            "command": "${CLAUDE_PLUGIN_ROOT}/scripts/lint-fix.sh",
            "timeout": 30
          }
        ]
      }
    ],
    "Stop": [
      {
        "hooks": [
          {
            "type": "prompt",
            "prompt": "Verify task completion: check acceptance criteria and update task status"
          }
        ]
      }
    ]
  }
}
```

---

## Extension Points Summary

| Extension Point | Description | Implementation |
|-----------------|-------------|----------------|
| **Custom Workflows** | Define board layouts with custom columns | `WorkflowSchema` JSON in database |
| **Status Mappings** | Map external statuses to internal ones | `WorkflowColumn.mapsTo` field |
| **External Sync** | Bidirectional sync with Jira/GitHub/etc | `ExternalSyncConfig` + provider adapters |
| **Agent Profiles** | Create specialized agents | `AgentProfile` JSON + Claude Code agents |
| **Skills** | Add capabilities to agents | Claude Code `.claude/skills/*/SKILL.md` |
| **Hooks** | Lifecycle automation | Claude Code hooks (PreToolUse, PostToolUse, Stop) |
| **MCP Servers** | External tool integration | `.mcp.json` configuration |
| **Artifact Types** | Define new document categories | Type enum extension |
| **Artifact Buckets** | Create storage/routing buckets | `ArtifactBucket` config |
| **Artifact Flows** | Automate artifact routing | `ArtifactFlow` trigger rules |
| **Research Presets** | Custom research depth configs | `ResearchDepthPreset` |
| **Methodologies** | BMAD, GSD, custom methods | `MethodologyExtension` packages |

---

## Key Architecture Principles

1. **Leverage Claude Code's native system** - Use plugins, skills, agents, hooks directly instead of reinventing
2. **Internal status = side effects** - 9 internal statuses with documented, predictable behavior
3. **External status = UI flexibility** - Custom workflows map to internal statuses
4. **Agents = Claude Code components** - Composed of agent definitions, skills, hooks
5. **Artifacts = typed I/O** - Documents flow between processes through typed buckets
6. **Methodologies = configuration** - BMAD, GSD, etc. are configuration packages, not code changes

This architecture enables:
- Adding new methodologies without code changes
- Custom workflows that still trigger correct side effects
- Agent specialization through skill composition
- Research → Planning → Execution artifact flow
- Third-party plugin ecosystem via Claude Code marketplace

---

## Chat & Ideation System

### Design Philosophy

The chat interface is the primary way users interact with the Orchestrator. Rather than manually creating tasks, users **converse** to brainstorm, explore, and refine ideas. The output of these conversations are **Task Proposals** that can be reviewed and applied to the Kanban.

**Key insight**: Ideation and execution are fundamentally different activities. Mixing them in the same space creates friction. RalphX separates them:
- **Ideation View**: Open-ended brainstorming with the Orchestrator → produces proposals
- **Kanban View**: Structured execution → consumes tasks

### Chat Interface Location

The chat is implemented as a **contextual side panel** that's always accessible:

```
┌─────────────────────────────────────────────────────────────────────────────────────┐
│ RalphX                                                        [⌘K] Toggle Chat     │
├───────────────┬─────────────────────────────────────────────────────┬───────────────┤
│               │                                                     │               │
│  PROJECT NAV  │              MAIN VIEW AREA                         │  CHAT PANEL   │
│               │                                                     │  (Resizable)  │
│  ┌─────────┐  │   ┌─────────────────────────────────────────────┐   │               │
│  │Ideation │  │   │                                             │   │  ┌─────────┐  │
│  └─────────┘  │   │  Current View Content                       │   │  │ Context │  │
│  ┌─────────┐  │   │  (Kanban / Ideation / Settings / etc.)     │   │  │ Aware   │  │
│  │ Kanban  │  │   │                                             │   │  │         │  │
│  └─────────┘  │   │                                             │   │  │ Chat    │  │
│  ┌─────────┐  │   │                                             │   │  │ History │  │
│  │Activity │  │   │                                             │   │  │         │  │
│  └─────────┘  │   │                                             │   │  └─────────┘  │
│  ┌─────────┐  │   │                                             │   │               │
│  │Settings │  │   │                                             │   │  ┌─────────┐  │
│  └─────────┘  │   │                                             │   │  │ Input   │  │
│               │   └─────────────────────────────────────────────┘   │  └─────────┘  │
│               │                                                     │               │
└───────────────┴─────────────────────────────────────────────────────┴───────────────┘
```

**Chat Panel Behaviors:**
- **Toggle**: ⌘+K (or ⌘+J) to show/hide
- **Resizable**: Drag edge to adjust width (min 280px, max 50% of window)
- **Persistent**: Conversation history maintained across view changes
- **Context-aware**: Knows current view and selected items

### Context Awareness

The chat panel adapts based on what the user is viewing/selecting:

| Current Context | Chat Behavior |
|-----------------|---------------|
| **Kanban View** (nothing selected) | General project chat, can suggest tasks |
| **Kanban View** (task selected) | Chat about selected task, can modify it |
| **Ideation View** | Full ideation mode, generates proposals |
| **Task Detail Modal** | Focused on that specific task |
| **Settings** | Can help configure settings |

```typescript
interface ChatContext {
  view: "kanban" | "ideation" | "activity" | "settings" | "task_detail";
  projectId: string;
  selectedTaskId?: string;
  selectedProposalIds?: string[];
  ideationSessionId?: string;
}
```

---

## Ideation View

A dedicated space for brainstorming that produces **Task Proposals** (not real tasks).

### Ideation View Layout

```
┌─────────────────────────────────────────────────────────────────────────────────────┐
│  Ideation: MyProject                                    [New Session] [Archive]     │
├───────────────────────────────────────────────┬─────────────────────────────────────┤
│  CONVERSATION                                 │  TASK PROPOSALS                      │
│                                               │                                      │
│  ┌───────────────────────────────────────┐   │  ┌────────────────────────────────┐  │
│  │ You: I need user authentication       │   │  │ ☑ 1. Setup auth database       │  │
│  └───────────────────────────────────────┘   │  │    Priority: HIGH (blocks 2)   │  │
│                                               │  │    Category: setup             │  │
│  ┌───────────────────────────────────────┐   │  │    [Edit] [Remove]             │  │
│  │ Orchestrator: I'll help design that.  │   │  └────────────────────────────────┘  │
│  │ Based on the tech stack (React +      │   │                                      │
│  │ Tauri), I suggest these approaches... │   │  ┌────────────────────────────────┐  │
│  └───────────────────────────────────────┘   │  │ ☑ 2. Implement JWT service     │  │
│                                               │  │    Priority: HIGH              │  │
│  ┌───────────────────────────────────────┐   │  │    Depends on: #1              │  │
│  │ You: Use JWT, not sessions            │   │  │    [Edit] [Remove]             │  │
│  └───────────────────────────────────────┘   │  └────────────────────────────────┘  │
│                                               │                                      │
│  ┌───────────────────────────────────────┐   │  ┌────────────────────────────────┐  │
│  │ Orchestrator: Updated. JWT is a good  │   │  │ ☑ 3. Create login UI           │  │
│  │ choice for Tauri. Here are the        │   │  │    Priority: MEDIUM            │  │
│  │ proposed tasks with dependencies...   │   │  │    Depends on: #2              │  │
│  │                                        │   │  │    [Edit] [Remove]             │  │
│  └───────────────────────────────────────┘   │  └────────────────────────────────┘  │
│                                               │                                      │
│                                               │  ┌────────────────────────────────┐  │
│                                               │  │ ☐ 4. Add password reset flow   │  │
│                                               │  │    Priority: LOW               │  │
│                                               │  │    Category: feature           │  │
│                                               │  │    [Edit] [Remove]             │  │
│                                               │  └────────────────────────────────┘  │
│                                               │                                      │
│                                               │  ─────────────────────────────────   │
│                                               │  Selected: 3 of 4                    │
│                                               │                                      │
│                                               │  [Apply to Draft ▼] [Clear All]     │
├───────────────────────────────────────────────┴─────────────────────────────────────┤
│  [Send message...]                                                     [Attach ▼]  │
└─────────────────────────────────────────────────────────────────────────────────────┘
```

### Ideation Sessions

Each ideation conversation is a **session** that can be:
- **Active**: Currently being worked on
- **Archived**: Completed or paused for later
- **Converted**: All proposals applied to Kanban

```typescript
interface IdeationSession {
  id: string;
  projectId: string;
  title: string;                    // Auto-generated or user-defined
  status: "active" | "archived" | "converted";
  messages: ChatMessage[];
  proposals: TaskProposal[];
  createdAt: Date;
  updatedAt: Date;
  archivedAt?: Date;
  convertedAt?: Date;
}
```

### Task Proposals

Proposals are **draft tasks** that exist only within ideation until applied:

```typescript
interface TaskProposal {
  id: string;
  sessionId: string;

  // Core fields
  title: string;
  description: string;
  category: "setup" | "feature" | "integration" | "styling" | "testing" | "documentation";

  // Steps (like PRD tasks)
  steps?: string[];
  acceptanceCriteria?: string[];

  // Priority assessment (auto-calculated)
  suggestedPriority: Priority;
  priorityScore: number;           // 0-100 for sorting
  priorityReason: string;          // Human-readable explanation

  // Dependencies (references other proposals in same session)
  dependsOn: string[];             // Proposal IDs this depends on
  blocks: string[];                // Proposal IDs this would unblock

  // Complexity estimate
  estimatedComplexity: "trivial" | "simple" | "moderate" | "complex" | "very_complex";

  // User modifications
  userPriority?: Priority;         // Override if user disagrees
  userModified: boolean;           // True if user edited any field

  // Status
  status: "pending" | "accepted" | "rejected" | "modified";
  selected: boolean;               // Checkbox state in UI

  // Link to created task (after apply)
  createdTaskId?: string;

  createdAt: Date;
  updatedAt: Date;
}

type Priority = "critical" | "high" | "medium" | "low";
```

### Apply Proposals to Kanban

When user clicks "Apply", selected proposals become real tasks:

```typescript
interface ApplyProposalsOptions {
  proposalIds: string[];
  targetColumn: "draft" | "backlog" | "todo";
  preserveDependencies: boolean;    // Create task_dependencies records
  assignWave?: number;              // For parallel execution grouping
}

interface ApplyProposalsResult {
  createdTasks: Task[];
  dependenciesCreated: number;
  warnings?: string[];              // e.g., "Circular dependency detected"
}
```

**Apply Options in UI:**
- **Apply to Draft**: Tasks go to Draft column (needs more refinement)
- **Apply to Backlog**: Tasks go to Backlog (confirmed, not scheduled)
- **Apply to Todo**: Tasks go to Todo (ready to be planned)
- **Apply with Dependencies**: Preserves the dependency graph

---

## Priority Assessment System

### Priority Calculation Algorithm

The Orchestrator calculates priority using multiple factors:

```typescript
interface PriorityAssessment {
  proposalId: string;

  // Final results
  suggestedPriority: Priority;
  priorityScore: number;           // 0-100
  priorityReason: string;

  // Factor breakdown
  factors: {
    dependencyFactor: {
      score: number;               // 0-30 points
      blocksCount: number;
      reason: string;              // "Blocks 3 other tasks"
    };
    criticalPathFactor: {
      score: number;               // 0-25 points
      isOnCriticalPath: boolean;
      pathLength: number;
      reason: string;
    };
    businessValueFactor: {
      score: number;               // 0-20 points
      keywords: string[];          // ["MVP", "core", "essential"]
      reason: string;
    };
    complexityFactor: {
      score: number;               // 0-15 points (inverse - simpler = higher)
      complexity: string;
      reason: string;              // "Quick win - simple task"
    };
    userHintFactor: {
      score: number;               // 0-10 points
      hints: string[];             // ["urgent", "blocker", "ASAP"]
      reason: string;
    };
  };
}
```

### Priority Scoring Breakdown

| Factor | Max Points | Description |
|--------|------------|-------------|
| **Dependency** | 30 | Tasks that unblock others get higher priority |
| **Critical Path** | 25 | Tasks on the longest path to completion |
| **Business Value** | 20 | Keywords from conversation indicating importance |
| **Complexity** | 15 | Simpler tasks scored higher (quick wins first) |
| **User Hints** | 10 | Explicit urgency signals from user |
| **Total** | 100 | |

### Score to Priority Mapping

| Score Range | Priority |
|-------------|----------|
| 80-100 | Critical |
| 60-79 | High |
| 40-59 | Medium |
| 0-39 | Low |

### Dependency Analysis

```typescript
interface DependencyGraph {
  nodes: {
    proposalId: string;
    title: string;
    inDegree: number;              // Number of dependencies
    outDegree: number;             // Number of tasks this blocks
  }[];
  edges: {
    from: string;                  // Depends on
    to: string;                    // Is dependency of
  }[];
  criticalPath: string[];          // Ordered list of proposal IDs
  hasCycles: boolean;
  cycles?: string[][];             // If cycles detected, list them
}
```

### Priority Keywords Detection

The Orchestrator scans conversation for priority signals:

```typescript
const PRIORITY_KEYWORDS = {
  critical: ["critical", "blocker", "blocking", "urgent", "ASAP", "emergency", "must have"],
  high: ["important", "priority", "essential", "core", "MVP", "key", "crucial"],
  low: ["nice to have", "optional", "future", "later", "eventually", "if time"],
};
```

---

## Orchestrator Tools for Ideation

The Orchestrator agent needs specific tools for the ideation workflow:

### Tool Definitions

```typescript
// ═══════════════════════════════════════════════════════════════════════════════
// IDEATION SESSION MANAGEMENT
// ═══════════════════════════════════════════════════════════════════════════════

interface CreateIdeationSessionTool {
  name: "create_ideation_session";
  description: "Start a new ideation session for brainstorming";
  parameters: {
    projectId: string;
    title?: string;
  };
  returns: IdeationSession;
}

interface GetIdeationSessionTool {
  name: "get_ideation_session";
  description: "Get current ideation session with all proposals";
  parameters: {
    sessionId: string;
  };
  returns: IdeationSession;
}

// ═══════════════════════════════════════════════════════════════════════════════
// TASK PROPOSAL CRUD
// ═══════════════════════════════════════════════════════════════════════════════

interface CreateTaskProposalTool {
  name: "create_task_proposal";
  description: "Create a new task proposal in the current ideation session";
  parameters: {
    sessionId: string;
    title: string;
    description: string;
    category: TaskCategory;
    steps?: string[];
    acceptanceCriteria?: string[];
    dependsOn?: string[];          // Other proposal IDs
    estimatedComplexity?: Complexity;
  };
  returns: TaskProposal;
}

interface UpdateTaskProposalTool {
  name: "update_task_proposal";
  description: "Update an existing task proposal";
  parameters: {
    proposalId: string;
    changes: Partial<TaskProposal>;
  };
  returns: TaskProposal;
}

interface DeleteTaskProposalTool {
  name: "delete_task_proposal";
  description: "Remove a task proposal from the session";
  parameters: {
    proposalId: string;
  };
  returns: { success: boolean };
}

// ═══════════════════════════════════════════════════════════════════════════════
// PRIORITY & DEPENDENCY ANALYSIS
// ═══════════════════════════════════════════════════════════════════════════════

interface AssessPriorityTool {
  name: "assess_priority";
  description: "Calculate priority for a proposal based on dependencies and context";
  parameters: {
    proposalId: string;
    conversationContext?: string;  // Recent messages for keyword analysis
  };
  returns: PriorityAssessment;
}

interface AssessAllPrioritiesTool {
  name: "assess_all_priorities";
  description: "Recalculate priorities for all proposals in session";
  parameters: {
    sessionId: string;
  };
  returns: PriorityAssessment[];
}

interface AnalyzeDependenciesTool {
  name: "analyze_dependencies";
  description: "Build dependency graph for proposals in session";
  parameters: {
    sessionId: string;
  };
  returns: DependencyGraph;
}

interface SuggestDependenciesTool {
  name: "suggest_dependencies";
  description: "AI suggests likely dependencies based on proposal content";
  parameters: {
    sessionId: string;
  };
  returns: {
    suggestions: {
      from: string;              // Proposal ID
      to: string;                // Proposal ID
      confidence: number;        // 0-1
      reason: string;            // "Login UI likely needs auth service"
    }[];
  };
}

// ═══════════════════════════════════════════════════════════════════════════════
// APPLY TO KANBAN
// ═══════════════════════════════════════════════════════════════════════════════

interface ApplyProposalsTool {
  name: "apply_proposals_to_kanban";
  description: "Convert selected proposals into real tasks on the Kanban";
  parameters: {
    proposalIds: string[];
    targetColumn: "draft" | "backlog" | "todo";
    preserveDependencies: boolean;
  };
  returns: ApplyProposalsResult;
}

// ═══════════════════════════════════════════════════════════════════════════════
// CONTEXT RETRIEVAL
// ═══════════════════════════════════════════════════════════════════════════════

interface GetProjectContextTool {
  name: "get_project_context";
  description: "Get project info, tech stack, existing tasks for context";
  parameters: {
    projectId: string;
  };
  returns: {
    project: Project;
    existingTasks: Task[];
    techStack?: string[];
    prdSummary?: string;
  };
}

interface GetExistingTasksTool {
  name: "get_existing_tasks";
  description: "Get existing tasks that might be related to current ideation";
  parameters: {
    projectId: string;
    filter?: {
      status?: InternalStatus[];
      category?: string;
      searchQuery?: string;
    };
  };
  returns: Task[];
}
```

### Orchestrator Agent Definition for Ideation

`.claude/agents/orchestrator-ideation.md`:

```markdown
---
name: orchestrator-ideation
description: Facilitates ideation sessions and generates task proposals
tools:
  - create_ideation_session
  - get_ideation_session
  - create_task_proposal
  - update_task_proposal
  - delete_task_proposal
  - assess_priority
  - assess_all_priorities
  - analyze_dependencies
  - suggest_dependencies
  - apply_proposals_to_kanban
  - get_project_context
  - get_existing_tasks
model: sonnet
---

You are the Ideation Orchestrator for RalphX. Your role is to help users brainstorm, explore ideas, and transform them into well-structured task proposals.

## Your Responsibilities

1. **Active Listening**: Understand what the user wants to build
2. **Clarifying Questions**: Ask questions to refine vague ideas
3. **Task Decomposition**: Break features into atomic, actionable tasks
4. **Dependency Detection**: Identify which tasks depend on others
5. **Priority Assessment**: Calculate and explain task priorities
6. **Proposal Generation**: Create task proposals with all necessary detail

## Workflow

### Phase 1: Understand
- Ask clarifying questions about the feature/idea
- Understand constraints, requirements, and preferences
- Get context about existing work and tech stack

### Phase 2: Decompose
- Break the feature into atomic tasks
- Each task should be completable in one focused session
- Include clear acceptance criteria

### Phase 3: Organize
- Identify dependencies between tasks
- Calculate priorities based on the dependency graph
- Order tasks optimally for execution

### Phase 4: Present
- Show the user the proposed tasks
- Explain your priority reasoning
- Allow them to modify, remove, or reorder

## Guidelines

- **Be conversational**: This is brainstorming, not a form
- **Think out loud**: Share your reasoning as you decompose
- **Stay focused**: Keep proposals actionable and scoped
- **Respect user expertise**: They know their domain better
- **Iterate**: Refine proposals based on feedback

## Priority Reasoning

When explaining priority, mention:
- Dependency relationships ("This blocks 3 other tasks")
- Quick wins ("Simple task, good for momentum")
- Critical path ("On the longest path to completion")
- Business value ("Core MVP feature")

## Example Interaction

User: "I need to add user authentication"

You: "I'll help you plan that out. A few questions first:
1. What's your tech stack? (I see this is a Tauri app)
2. Should users be able to register, or just login?
3. Any social login (Google, GitHub)?
4. Password reset flow needed?"

[After discussion]

You: "Based on our discussion, here are the proposed tasks:

**1. Setup authentication database tables** (HIGH priority)
- Creates the foundation other tasks need
- Blocks: #2, #3

**2. Implement JWT token service** (HIGH priority)
- Core auth logic
- Depends on: #1

**3. Create login page UI** (MEDIUM priority)
- Frontend implementation
- Depends on: #2

**4. Add password reset flow** (LOW priority)
- Nice-to-have, not blocking anything

Want me to adjust any of these?"
```

---

## Ideation Database Schema

Additional tables for the ideation system:

```sql
-- ═══════════════════════════════════════════════════════════════════════════════
-- IDEATION SESSIONS
-- ═══════════════════════════════════════════════════════════════════════════════

CREATE TABLE ideation_sessions (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  title TEXT,                                    -- Auto-generated or user-defined
  status TEXT NOT NULL DEFAULT 'active',         -- 'active' | 'archived' | 'converted'
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  archived_at DATETIME,
  converted_at DATETIME
);

CREATE INDEX idx_ideation_sessions_project ON ideation_sessions(project_id);
CREATE INDEX idx_ideation_sessions_status ON ideation_sessions(status);

-- ═══════════════════════════════════════════════════════════════════════════════
-- TASK PROPOSALS
-- ═══════════════════════════════════════════════════════════════════════════════

CREATE TABLE task_proposals (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL REFERENCES ideation_sessions(id) ON DELETE CASCADE,

  -- Core fields
  title TEXT NOT NULL,
  description TEXT,
  category TEXT NOT NULL,                        -- 'setup' | 'feature' | etc.

  -- Detailed content (JSON arrays)
  steps TEXT,                                    -- JSON array of step strings
  acceptance_criteria TEXT,                      -- JSON array of criteria strings

  -- Priority assessment
  suggested_priority TEXT NOT NULL,              -- 'critical' | 'high' | 'medium' | 'low'
  priority_score INTEGER NOT NULL DEFAULT 50,   -- 0-100
  priority_reason TEXT,
  priority_factors TEXT,                         -- JSON: full PriorityAssessment.factors

  -- Complexity
  estimated_complexity TEXT DEFAULT 'moderate',  -- 'trivial' | 'simple' | 'moderate' | 'complex' | 'very_complex'

  -- User overrides
  user_priority TEXT,                            -- User's override if they disagree
  user_modified BOOLEAN DEFAULT FALSE,

  -- Status
  status TEXT NOT NULL DEFAULT 'pending',        -- 'pending' | 'accepted' | 'rejected' | 'modified'
  selected BOOLEAN DEFAULT TRUE,                 -- Checkbox state in UI

  -- Link to created task (after apply)
  created_task_id TEXT REFERENCES tasks(id),

  -- Ordering within session
  sort_order INTEGER NOT NULL DEFAULT 0,

  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_task_proposals_session ON task_proposals(session_id);
CREATE INDEX idx_task_proposals_status ON task_proposals(status);
CREATE INDEX idx_task_proposals_priority ON task_proposals(priority_score DESC);

-- ═══════════════════════════════════════════════════════════════════════════════
-- PROPOSAL DEPENDENCIES
-- ═══════════════════════════════════════════════════════════════════════════════

CREATE TABLE proposal_dependencies (
  id TEXT PRIMARY KEY,
  proposal_id TEXT NOT NULL REFERENCES task_proposals(id) ON DELETE CASCADE,
  depends_on_proposal_id TEXT NOT NULL REFERENCES task_proposals(id) ON DELETE CASCADE,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,

  UNIQUE(proposal_id, depends_on_proposal_id),
  CHECK(proposal_id != depends_on_proposal_id)   -- No self-dependencies
);

CREATE INDEX idx_proposal_deps_proposal ON proposal_dependencies(proposal_id);
CREATE INDEX idx_proposal_deps_depends_on ON proposal_dependencies(depends_on_proposal_id);

-- ═══════════════════════════════════════════════════════════════════════════════
-- CHAT MESSAGES
-- ═══════════════════════════════════════════════════════════════════════════════

CREATE TABLE chat_messages (
  id TEXT PRIMARY KEY,

  -- Context (one of these should be set)
  session_id TEXT REFERENCES ideation_sessions(id) ON DELETE CASCADE,  -- For ideation chat
  project_id TEXT REFERENCES projects(id) ON DELETE CASCADE,           -- For general project chat
  task_id TEXT REFERENCES tasks(id) ON DELETE CASCADE,                 -- For task-specific chat

  -- Message content
  role TEXT NOT NULL,                            -- 'user' | 'orchestrator' | 'system'
  content TEXT NOT NULL,

  -- Rich metadata
  metadata TEXT,                                 -- JSON: mentioned task IDs, artifacts, etc.

  -- For threading (future)
  parent_message_id TEXT REFERENCES chat_messages(id),

  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_chat_messages_session ON chat_messages(session_id);
CREATE INDEX idx_chat_messages_project ON chat_messages(project_id);
CREATE INDEX idx_chat_messages_task ON chat_messages(task_id);
CREATE INDEX idx_chat_messages_created ON chat_messages(created_at);

-- ═══════════════════════════════════════════════════════════════════════════════
-- TASK DEPENDENCIES (for applied tasks)
-- ═══════════════════════════════════════════════════════════════════════════════

-- Note: This table already exists in the main schema, but documenting here for clarity
-- When proposals are applied, proposal_dependencies become task_dependencies

CREATE TABLE IF NOT EXISTS task_dependencies (
  id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
  depends_on_task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,

  UNIQUE(task_id, depends_on_task_id),
  CHECK(task_id != depends_on_task_id)
);

CREATE INDEX IF NOT EXISTS idx_task_deps_task ON task_dependencies(task_id);
CREATE INDEX IF NOT EXISTS idx_task_deps_depends_on ON task_dependencies(depends_on_task_id);
```

---

## Ideation → Kanban Transition Flow

Complete flow from ideation to execution:

```
┌─────────────────────────────────────────────────────────────────────────────────────┐
│  IDEATION PHASE                                                                      │
├─────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                      │
│  1. User opens Ideation view                                                        │
│     └─► New IdeationSession created (status: active)                                │
│                                                                                      │
│  2. User converses with Orchestrator                                                │
│     └─► ChatMessages stored with session_id                                         │
│                                                                                      │
│  3. Orchestrator creates TaskProposals                                              │
│     └─► Proposals created with suggested_priority                                   │
│     └─► Dependencies inferred and stored in proposal_dependencies                   │
│                                                                                      │
│  4. User reviews proposals in side panel                                            │
│     └─► Can edit title, description, priority                                       │
│     └─► Can remove unwanted proposals                                               │
│     └─► Can reorder manually                                                        │
│                                                                                      │
│  5. User selects proposals to apply                                                 │
│     └─► Checkboxes set selected = true/false                                        │
│                                                                                      │
└────────────────────────────────────────┬────────────────────────────────────────────┘
                                         │
                                         ▼
┌─────────────────────────────────────────────────────────────────────────────────────┐
│  TRANSITION                                                                          │
├─────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                      │
│  6. User clicks "Apply to [Column]"                                                 │
│     └─► System validates no circular dependencies                                   │
│     └─► For each selected proposal:                                                 │
│         ├─► Create Task in target column                                            │
│         ├─► Copy priority, description, steps, acceptance_criteria                  │
│         ├─► Set created_task_id on proposal                                         │
│         └─► Update proposal status = 'accepted'                                     │
│     └─► For each proposal_dependency where both proposals accepted:                 │
│         └─► Create task_dependency record                                           │
│                                                                                      │
│  7. Session status updated                                                          │
│     └─► If all proposals accepted/rejected: status = 'converted'                    │
│     └─► If some pending: status remains 'active'                                    │
│                                                                                      │
└────────────────────────────────────────┬────────────────────────────────────────────┘
                                         │
                                         ▼
┌─────────────────────────────────────────────────────────────────────────────────────┐
│  EXECUTION PHASE (existing Kanban flow)                                             │
├─────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                      │
│  8. Tasks appear in target column                                                   │
│     └─► Dependencies shown as blockers                                              │
│     └─► Priority affects sort order within column                                   │
│                                                                                      │
│  9. Normal Kanban workflow continues                                                │
│     └─► User moves tasks to Planned                                                 │
│     └─► State machine triggers side effects                                         │
│     └─► Agents execute tasks                                                        │
│                                                                                      │
└─────────────────────────────────────────────────────────────────────────────────────┘
```

---

## UI Components for Ideation

### ProposalCard Component

```typescript
interface ProposalCardProps {
  proposal: TaskProposal;
  onSelect: (id: string, selected: boolean) => void;
  onEdit: (id: string) => void;
  onRemove: (id: string) => void;
  dependencyInfo?: {
    dependsOnCount: number;
    blocksCount: number;
  };
}

// Visual states:
// - Default: Dark surface with subtle border
// - Selected: Accent border (orange)
// - High priority: Critical/High badge with warm color
// - Has dependencies: Dependency icon with count
// - User modified: Small "edited" indicator
```

### ProposalList Component

```typescript
interface ProposalListProps {
  proposals: TaskProposal[];
  onSelectAll: () => void;
  onDeselectAll: () => void;
  onApply: (targetColumn: string) => void;
  onClearAll: () => void;
}

// Features:
// - Drag to reorder
// - Multi-select with Shift+click
// - Bulk actions (select all, deselect all)
// - Dependency visualization (lines between cards)
// - Priority-based auto-sort option
```

### ApplyModal Component

```typescript
interface ApplyModalProps {
  selectedProposals: TaskProposal[];
  onApply: (options: ApplyProposalsOptions) => void;
  onCancel: () => void;
}

// Shows:
// - List of selected proposals
// - Dependency graph preview
// - Target column selector
// - "Preserve dependencies" checkbox
// - Warnings (circular deps, missing deps)
```

---

## Key Architecture Additions

Add to the Key Architecture Principles:

7. **Ideation ≠ Execution** - Separate brainstorming from task management to reduce friction
8. **Proposals before Tasks** - Ideas become proposals, proposals become tasks (two-stage commitment)
9. **Automatic Priority** - System suggests, user confirms (reduce manual work)
10. **Context-Aware Chat** - Chat adapts to current view and selection
11. **Dependency-First Planning** - Priority derived from dependency graph, not arbitrary assignment
