# Metrics Schema Mapping

> Column name corrections for the Engineering Metrics feature.
> Verified via `PRAGMA table_info()` against production DB (schema v57 → v58).

## Column Name Corrections (Plan → Reality)

The implementation plan used assumed column names. These are the **actual column names** to use in all SQL queries.

| Table | Plan Assumed | Actual Column | Notes |
|-------|-------------|---------------|-------|
| `tasks` | `status` | `internal_status` | Values: `'backlog'`, `'executing'`, `'merged'`, etc. |
| `task_state_history` | `from_state` | `from_status` | Can be NULL for first transition |
| `task_state_history` | `to_state` | `to_status` | NOT NULL |
| `reviews` | `outcome` | `status` | Values: `'pending'`, `'approved'`, `'changes_requested'` |

## Verified Tables and Required Columns

### tasks
| Column | Type | Required for Metrics |
|--------|------|---------------------|
| `id` | TEXT PK | ✅ Task counts |
| `project_id` | TEXT | ✅ Project scoping |
| `internal_status` | TEXT | ✅ Success rate (completed vs failed/cancelled) |
| `created_at` | DATETIME | ✅ Throughput date filtering |
| `updated_at` | DATETIME | ✅ Recency |

### task_state_history
| Column | Type | Required for Metrics |
|--------|------|---------------------|
| `task_id` | TEXT | ✅ Join key |
| `from_status` | TEXT | ✅ Cycle time phase identification |
| `to_status` | TEXT | ✅ Cycle time phase identification |
| `created_at` | DATETIME | ✅ Cycle time duration calculation |

### task_steps
| Column | Type | Required for Metrics |
|--------|------|---------------------|
| `task_id` | TEXT | ✅ Step count for EME complexity |
| `id` | TEXT PK | ✅ Step counting |
| `status` | TEXT | ✅ Step completion tracking |

### reviews
| Column | Type | Required for Metrics |
|--------|------|---------------------|
| `task_id` | TEXT | ✅ Join key |
| `status` | TEXT | ✅ Review pass rate (`'approved'` vs `'changes_requested'`) |

## Corrected SQL Query Snippets

### Task Success Rate
```sql
-- Use internal_status, NOT "status"
SELECT
  COUNT(CASE WHEN internal_status = 'merged' THEN 1 END) as completed,
  COUNT(CASE WHEN internal_status IN ('cancelled', 'failed') THEN 1 END) as failed,
  COUNT(*) as total
FROM tasks
WHERE project_id = ?1
```

### Cycle Time (LAG window function)
```sql
-- Use from_status/to_status, NOT "from_state"/"to_state"
WITH transitions AS (
  SELECT
    task_id,
    to_status,
    created_at,
    LAG(created_at) OVER (PARTITION BY task_id ORDER BY created_at) as prev_at,
    LAG(to_status) OVER (PARTITION BY task_id ORDER BY created_at) as prev_status
  FROM task_state_history
  WHERE task_id IN (SELECT id FROM tasks WHERE project_id = ?1 AND internal_status = 'merged')
)
SELECT
  prev_status as phase,
  AVG(julianday(created_at) - julianday(prev_at)) * 24 * 60 as avg_minutes,
  COUNT(*) as sample_size
FROM transitions
WHERE prev_at IS NOT NULL
GROUP BY prev_status
```

### Review Pass Rate
```sql
-- Use reviews.status, NOT "outcome"
SELECT
  COUNT(CASE WHEN status = 'approved' THEN 1 END) as approved,
  COUNT(*) as total
FROM reviews r
JOIN tasks t ON t.id = r.task_id
WHERE t.project_id = ?1
```

### EME Complexity (step count + review cycles)
```sql
-- Unchanged — task_steps.status and reviews.status both correctly named
SELECT
  t.id,
  COALESCE(s.step_count, 0) as step_count,
  COALESCE(r.review_count, 0) as review_cycles
FROM tasks t
LEFT JOIN (SELECT task_id, COUNT(*) as step_count FROM task_steps GROUP BY task_id) s ON s.task_id = t.id
LEFT JOIN (SELECT task_id, COUNT(*) as review_count FROM reviews GROUP BY task_id) r ON r.task_id = t.id
WHERE t.project_id = ?1 AND t.internal_status = 'merged'
```

## Index Added (v58)

```sql
CREATE INDEX IF NOT EXISTS idx_task_state_history_task_created
  ON task_state_history(task_id, created_at);
```

Complements the existing `idx_task_state_history_task_id` (single-column) by covering the `ORDER BY created_at` needed for the `LAG()` window function in cycle time queries.
