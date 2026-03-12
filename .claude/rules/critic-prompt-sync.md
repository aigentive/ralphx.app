> **Maintainer note:** This file optimizes for LLM context efficiency. Rules: (1) Tables > prose (2) One example max per concept (3) No redundant explanations (4) Use symbols: → = leads to, | = or, ❌/✅ = wrong/right (5) Before adding content, ask: "Can this be a single line?" If yes, make it one line.

# Critic Prompt Sync Rule

## Overview

The "proposed vs existing" framing text injected into adversarial critic prompts lives in **3 locations that must stay identical**. Any change to the framing must be applied to all 3.

## Sync Locations

| # | File | Location |
|---|------|----------|
| 1 | `src-tauri/src/http_server/handlers/artifacts.rs` | `PROPOSED_VS_EXISTING_FRAMING` const (line ~31) |
| 2 | `ralphx-plugin/agents/orchestrator-ideation.md` | Phase 3.5 VERIFY section |
| 3 | `ralphx-plugin/agents/ideation-team-lead.md` | Phase 4.5 VERIFY section |

## Sync Rule (NON-NEGOTIABLE)

❌ Never edit the framing text in one location without updating the other two.
✅ Use the grep sentinel below to verify all 3 contain matching framing before committing.

## Grep Sentinel

The first 50 characters of `PROPOSED_VS_EXISTING_FRAMING` serve as the sync check string:

```
CRITICAL INSTRUCTION — Proposed vs Existing State:
```

Run to verify sync:
```bash
grep -rn "CRITICAL INSTRUCTION — Proposed vs Existing State:" \
  src-tauri/src/http_server/handlers/artifacts.rs \
  ralphx-plugin/agents/orchestrator-ideation.md \
  ralphx-plugin/agents/ideation-team-lead.md
```

All 3 files must match. If any file is missing the sentinel → framing is out of sync → fix before committing.
