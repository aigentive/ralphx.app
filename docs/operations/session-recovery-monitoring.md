# Session Recovery Monitoring Guide

## Overview
This guide provides instructions for monitoring the session recovery feature during the 1-week internal testing period to collect metrics and validate the success rate.

## Monitoring Period
**Duration:** 1 week from feature flag enablement
**Target:** Collect sufficient data to calculate recovery success rate
**Success Criteria:** ≥95% success rate

## Log Events

The session recovery system emits three key structured log events:

| Event | Level | Purpose | Key Fields |
|-------|-------|---------|-----------|
| `stale_session_detected` | WARN | Triggered when a stale session is detected | `session_id`, `conversation_id`, `context_type` |
| `rehydrate_success` | INFO | Recovery completed successfully | `new_session_id`, `replay_turns`, `estimated_tokens`, `duration_ms` |
| `rehydrate_failure` | ERROR | Recovery failed | `error`, `duration_ms` |

## Monitoring Methods

### Method 1: Live Monitoring (Development)

Monitor logs in real-time while using RalphX:

```bash
# If running with npm run tauri dev, logs appear in terminal
# Pipe to monitoring script:
npm run tauri dev 2>&1 | ./scripts/monitor-session-recovery.sh
```

### Method 2: Log File Analysis

If logs are written to a file:

```bash
# Analyze log file
./scripts/monitor-session-recovery.sh path/to/ralphx.log
```

### Method 3: Manual Grep

Extract events manually:

```bash
# Count stale session detections
grep "event=stale_session_detected" logs/*.log | wc -l

# Count successful recoveries
grep "event=rehydrate_success" logs/*.log | wc -l

# Count failed recoveries
grep "event=rehydrate_failure" logs/*.log | wc -l

# View recent events with context
grep -E "event=(stale_session_detected|rehydrate_success|rehydrate_failure)" logs/*.log | tail -20
```

## Collecting Metrics

### Required Metrics

1. **Stale Session Detection Rate**
   - How many stale sessions were detected
   - Purpose: Baseline measurement of how often this issue occurs

2. **Recovery Success Rate**
   - Formula: `successes / (successes + failures) × 100%`
   - Target: ≥95%

3. **Average Recovery Duration**
   - Extract from `duration_ms` field in `rehydrate_success` events
   - Target: <5 seconds

4. **User-Visible Errors**
   - Count of `[Agent error: No conversation found]` messages
   - Compare with historical data (before feature)
   - Target: 50% reduction

### Sample Data Collection

Create a daily log entry:

```markdown
## YYYY-MM-DD

### Metrics
- Stale sessions detected: X
- Successful recoveries: Y
- Failed recoveries: Z
- Success rate: Y/(Y+Z) = XX.X%
- Avg duration: XXXms

### Notes
- [Any issues observed]
- [User feedback]
- [Notable patterns]
```

## Using the Monitoring Script

### Basic Usage

```bash
# Analyze a log file
./scripts/monitor-session-recovery.sh ralphx.log

# Live monitoring
tail -f ralphx.log | ./scripts/monitor-session-recovery.sh
```

### Example Output

```
=== Session Recovery Statistics ===

Stale Sessions Detected: 12
Successful Recoveries:   11
Failed Recoveries:       1

Success Rate: 91.67% (11/12)
Average Duration: 2341ms

✗ SUCCESS RATE BELOW CRITERIA (< 95%)

=== Recent Events (last 10) ===

[DETECTED] WARN ralphx: event=stale_session_detected session_id=abc123 conversation_id=conv-456
[SUCCESS]  INFO ralphx: event=rehydrate_success new_session_id=def789 replay_turns=15 duration_ms=2100
...
```

## Investigation Triggers

### Trigger 1: Low Success Rate (<95%)
**Action Required:**
1. Analyze failure logs for patterns
2. Check error messages in `rehydrate_failure` events
3. Investigate specific failure scenarios
4. Consider fixes before full rollout

### Trigger 2: High Duration (>5s average)
**Action Required:**
1. Check `estimated_tokens` in success events
2. Identify conversations with large history
3. Consider token budget adjustment
4. Profile recovery performance

### Trigger 3: Frequent Failures (Same Error)
**Action Required:**
1. Extract error messages: `grep "event=rehydrate_failure" | grep -oE 'error="[^"]+"'`
2. Group by error type
3. Prioritize fixes for most common errors

## Failure Analysis

When investigating failures:

```bash
# Extract all failure events with full context
grep "event=rehydrate_failure" logs/*.log > failures.txt

# Look for patterns in error messages
cat failures.txt | grep -oE 'error="[^"]+"' | sort | uniq -c | sort -rn

# Find related conversation IDs
grep "event=rehydrate_failure" logs/*.log | grep -oE 'conversation_id=[^ ]+' | sort | uniq

# Check database for conversation details
sqlite3 src-tauri/ralphx.db "
  SELECT c.id, c.context_type, COUNT(m.id) as message_count
  FROM chat_conversations c
  LEFT JOIN chat_messages m ON m.chat_conversation_id = c.id
  WHERE c.id IN ('conv-id-1', 'conv-id-2')
  GROUP BY c.id;
"
```

## Weekly Summary Template

At the end of the monitoring period, compile a summary:

```markdown
# Session Recovery - Week 1 Monitoring Summary

## Date Range
Start: YYYY-MM-DD
End: YYYY-MM-DD

## Overall Metrics
- Total stale sessions detected: XXX
- Successful recoveries: XXX
- Failed recoveries: XXX
- **Success rate: XX.X%**
- Average recovery duration: XXXms
- Median recovery duration: XXXms

## Success Criteria Assessment
- [✓/✗] Success rate ≥95%: XX.X%
- [✓/✗] Average duration <5s: XXXms
- [✓/✗] No critical bugs identified

## Failure Analysis
### Most Common Failures
1. Error type 1: XX occurrences
2. Error type 2: XX occurrences
3. Error type 3: XX occurrences

### Root Causes Identified
- [List any patterns found]

## User Impact
- User-visible errors: [Before vs After comparison]
- Manual retry incidents: [Count]
- User feedback: [Summary]

## Recommendations
- [ ] Proceed with full rollout (remove feature flag)
- [ ] Additional fixes required before rollout
- [ ] Extend monitoring period
- [ ] Adjust token budget or other parameters

## Next Steps
1. [Action item 1]
2. [Action item 2]
3. [Action item 3]
```

## Continuous Monitoring Checklist

During the 1-week period:

- [ ] Day 1: Initial test and baseline measurement
- [ ] Day 2-3: Monitor for any immediate issues
- [ ] Day 4: Mid-week review of metrics
- [ ] Day 5-6: Continue monitoring
- [ ] Day 7: Compile weekly summary and assess criteria

## Troubleshooting

### No Events in Logs
- Verify `ENABLE_SESSION_RECOVERY=true` in `.env`
- Restart application after changing `.env`
- Trigger a manual stale session (see manual testing guide)
- Check log level: ensure WARN and INFO are enabled

### Cannot Find Log File
- In dev mode, logs go to terminal/Console.app
- Consider redirecting logs to a file:
  ```bash
  npm run tauri dev 2>&1 | tee ralphx-dev.log
  ```

### Script Not Working
- Verify script is executable: `chmod +x scripts/monitor-session-recovery.sh`
- Check bash version: `bash --version` (requires bash 4+)
- Install `bc` if not present: `brew install bc`

## Related Documents
- [Session Recovery Manual Test](../testing/session-recovery-manual-test.md) - How to manually trigger recovery
- [Session Recovery Rollout Plan](session-recovery-rollout.md) - Production deployment plan
