# Features Stream Activity

> Log entries for PRD task completion and P0 gap fixes.

---

### 2026-01-28 18:45:00 - Update ralph-streams.sh for stream argument and model selection
**What:**
- Added STREAM argument parsing (first arg) with validation for: features, refactor, polish, verify, hygiene
- Added ANTHROPIC_MODEL env var support (default: opus) with --model flag passed to claude
- Changed prompt source from hardcoded PROMPT.md to streams/${STREAM}/PROMPT.md
- Maintained backward compatibility: if first arg is a number, uses legacy PROMPT.md mode
- Stream-specific log files: logs/iteration_${STREAM}_$i.json
- Stream-specific activity file paths and completion messages
- Only require specs/prd.md for legacy mode and features stream

**Commands:**
- `bash -n ralph-streams.sh` - syntax validation passed

**Result:** Success
