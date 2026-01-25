#!/bin/bash
# Seed test data for visual audits
# Usage: ./scripts/seed-test-data.sh [profile]
# Profiles: minimal, kanban (default), ideation, full

set -e

DB_PATH="src-tauri/ralphx.db"
PROFILE="${1:-kanban}"

echo "Seeding test data with profile: $PROFILE"

# Check if database exists
if [ ! -f "$DB_PATH" ]; then
    echo "Database not found at $DB_PATH"
    echo "Start the app once to create the database, then run this script."
    exit 1
fi

# Check if data already exists
PROJECT_COUNT=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM projects;")
if [ "$PROJECT_COUNT" -gt 0 ]; then
    echo "Data already exists ($PROJECT_COUNT projects). Use --clear to remove first."
    if [ "$2" = "--clear" ]; then
        echo "Clearing existing data..."
        sqlite3 "$DB_PATH" "DELETE FROM tasks; DELETE FROM projects;"
    else
        exit 0
    fi
fi

# Generate UUIDs
PROJECT_ID=$(uuidgen | tr '[:upper:]' '[:lower:]')
NOW=$(date -u +"%Y-%m-%dT%H:%M:%S.000000+00:00")

# Create project
sqlite3 "$DB_PATH" "
INSERT INTO projects (id, name, working_directory, git_mode, created_at, updated_at)
VALUES ('$PROJECT_ID', 'Visual Audit Test', '$(pwd)', 'local', '$NOW', '$NOW');
"
echo "Created project: Visual Audit Test ($PROJECT_ID)"

if [ "$PROFILE" = "minimal" ]; then
    echo "Minimal profile - no tasks created"
    exit 0
fi

# Create tasks for kanban profile
create_task() {
    local TASK_ID=$(uuidgen | tr '[:upper:]' '[:lower:]')
    local TITLE="$1"
    local DESC="$2"
    local CATEGORY="$3"
    local PRIORITY="$4"
    local STATUS="$5"
    local STARTED="$6"
    local COMPLETED="$7"

    sqlite3 "$DB_PATH" "
    INSERT INTO tasks (id, project_id, category, title, description, priority, internal_status, needs_review_point, created_at, updated_at, started_at, completed_at)
    VALUES ('$TASK_ID', '$PROJECT_ID', '$CATEGORY', '$TITLE', '$DESC', $PRIORITY, '$STATUS', 0, '$NOW', '$NOW', $STARTED, $COMPLETED);
    "
    echo "  Created task: $TITLE ($STATUS)"
}

# Priority values: 1=Critical, 2=High, 3=Medium, 4=Low, 0=None
# These must match TaskCard.tsx getPriorityColor() switch cases

# Backlog task - Low priority
create_task "Add notifications" "Toast notifications for actions" "feature" 4 "backlog" "NULL" "NULL"

# Ready tasks - various priorities
create_task "Implement dark mode" "Add dark mode support to the application" "feature" 3 "ready" "NULL" "NULL"
create_task "Fix sidebar scroll" "Sidebar content overflows on small screens" "bug" 1 "ready" "NULL" "NULL"

# Executing task - High priority
create_task "Add keyboard shortcuts" "Implement Cmd+K for quick actions" "feature" 2 "executing" "'$NOW'" "NULL"

# Completed task - Medium priority
create_task "Setup project structure" "Initial Tauri + React setup" "setup" 3 "approved" "'$NOW'" "'$NOW'"

echo ""
echo "Test data seeded successfully!"
echo "Projects: 1"
echo "Tasks: 5"
