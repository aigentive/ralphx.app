# Test Extraction Manifest — 2026-02-20

## Summary
Total files: 125 | Total test lines to extract: ~36,924
- Group A (domain): 75 files, ~21,581 test lines
- Group B (application): 19 files, ~5,695 test lines
- Group C (infrastructure/commands/http): 31 files, ~9,648 test lines

## Extraction Pattern

**Confirmed from `application/reconciliation.rs` (line 113-114):**
```rust
#[cfg(test)]
mod tests;
```
Companion file `reconciliation/tests.rs` starts with `use super::*;`

**Rules:**
- For standalone `foo.rs` → create `foo_tests.rs`, replace inline block with:
  ```rust
  #[cfg(test)]
  #[path = "foo_tests.rs"]
  mod tests;
  ```
- For `foo/mod.rs` → create `foo/tests.rs`, replace inline block with:
  ```rust
  #[cfg(test)]
  mod tests;
  ```
- New test file starts with `use super::*;` then all the original test module contents (imports, helpers, test fns)
- Do NOT include the outer `mod tests {` or its closing `}` — just the contents

## Priority
Process files top-to-bottom (largest test blocks first). Files < 30 test lines are low priority — skip if time-constrained.

---

## Group A — Domain Files (75 files, ~21,581 test lines)

### Entities (22 files)
| File | Total | Test Lines | Tests | Target |
|------|-------|------------|-------|--------|
| domain/entities/task.rs | 1,238 | 962 | 56 | domain/entities/task_tests.rs |
| domain/entities/status.rs | 1,099 | 835 | 87 | domain/entities/status_tests.rs |
| domain/entities/types.rs | 1,060 | 740 | 95 | domain/entities/types_tests.rs |
| domain/entities/task_metadata.rs | 955 | 662 | 38 | domain/entities/task_metadata_tests.rs |
| domain/entities/project.rs | 872 | 594 | 47 | domain/entities/project_tests.rs |
| domain/entities/workflow.rs | 1,080 | 582 | 43 | domain/entities/workflow_tests.rs |
| domain/entities/review.rs | 1,059 | 483 | 38 | domain/entities/review_tests.rs |
| domain/entities/review_issue.rs | 1,015 | 411 | 25 | domain/entities/review_issue_tests.rs |
| domain/entities/merge_progress_event.rs | 347 | 221 | 20 | domain/entities/merge_progress_event_tests.rs |
| domain/entities/task_qa.rs | 397 | 204 | 11 | domain/entities/task_qa_tests.rs |
| domain/entities/task_context.rs | 276 | 188 | 6 | domain/entities/task_context_tests.rs |
| domain/entities/task_step.rs | 481 | 166 | 8 | domain/entities/task_step_tests.rs |
| domain/entities/activity_event.rs | 461 | 154 | 13 | domain/entities/activity_event_tests.rs |
| domain/entities/agent_run.rs | 385 | 150 | 12 | domain/entities/agent_run_tests.rs |
| domain/entities/ideation/session_context.rs | 234 | 147 | 6 | domain/entities/ideation/session_context_tests.rs |
| domain/entities/plan_branch.rs | 297 | 133 | 15 | domain/entities/plan_branch_tests.rs |
| domain/entities/chat_conversation.rs | 395 | 121 | 10 | domain/entities/chat_conversation_tests.rs |
| domain/entities/memory_archive.rs | 359 | 114 | 7 | domain/entities/memory_archive_tests.rs |
| domain/entities/memory_entry.rs | 327 | 104 | 6 | domain/entities/memory_entry_tests.rs |
| domain/entities/team.rs | 252 | 101 | 10 | domain/entities/team_tests.rs |
| domain/entities/chat_attachment.rs | 225 | 90 | 6 | domain/entities/chat_attachment_tests.rs |
| domain/entities/ideation/session_link.rs | 207 | 84 | 7 | domain/entities/ideation/session_link_tests.rs |

### State Machine (7 files)
| File | Total | Test Lines | Tests | Target |
|------|-------|------------|-------|--------|
| domain/state_machine/types.rs | 818 | 539 | 55 | domain/state_machine/types_tests.rs |
| domain/state_machine/mocks.rs | 855 | 403 | 4 | domain/state_machine/mocks_tests.rs |
| domain/state_machine/events.rs | 585 | 366 | 30 | domain/state_machine/events_tests.rs |
| domain/state_machine/persistence.rs | 472 | 350 | 29 | domain/state_machine/persistence_tests.rs |
| domain/state_machine/services.rs | 309 | 105 | 13 | domain/state_machine/services_tests.rs |
| domain/state_machine/context.rs | 419 | 32 | 3 | domain/state_machine/context_tests.rs |
| domain/state_machine/mod.rs | 52 | 20 | 2 | domain/state_machine/mod_tests.rs |

### QA (3 files)
| File | Total | Test Lines | Tests | Target |
|------|-------|------------|-------|--------|
| domain/qa/results.rs | 866 | 393 | 35 | domain/qa/results_tests.rs |
| domain/qa/criteria.rs | 638 | 351 | 29 | domain/qa/criteria_tests.rs |
| domain/qa/config.rs | 526 | 323 | 37 | domain/qa/config_tests.rs |

### Review (2 files)
| File | Total | Test Lines | Tests | Target |
|------|-------|------------|-------|--------|
| domain/review/review_points.rs | 686 | 415 | 52 | domain/review/review_points_tests.rs |
| domain/review/config.rs | 293 | 175 | 15 | domain/review/config_tests.rs |

### Agents (5 files)
| File | Total | Test Lines | Tests | Target |
|------|-------|------------|-------|--------|
| domain/agents/types.rs | 636 | 328 | 44 | domain/agents/types_tests.rs |
| domain/agents/agent_profile.rs | 709 | 314 | 31 | domain/agents/agent_profile_tests.rs |
| domain/agents/capabilities.rs | 201 | 108 | 13 | domain/agents/capabilities_tests.rs |
| domain/agents/error.rs | 135 | 95 | 12 | domain/agents/error_tests.rs |
| domain/agents/mod.rs | 63 | 41 | 4 | domain/agents/mod_tests.rs |

### Services (6 files)
| File | Total | Test Lines | Tests | Target |
|------|-------|------------|-------|--------|
| domain/services/methodology_service.rs | 897 | 656 | 3 | domain/services/methodology_service_tests.rs |
| domain/services/workflow_service.rs | 671 | 479 | 2 | domain/services/workflow_service_tests.rs |
| domain/services/message_queue.rs | 569 | 310 | 11 | domain/services/message_queue_tests.rs |
| domain/services/index_rewriter.rs | 353 | 153 | 5 | domain/services/index_rewriter_tests.rs |
| domain/services/rule_parser.rs | 251 | 76 | 4 | domain/services/rule_parser_tests.rs |
| domain/services/bucket_classifier.rs | 186 | 50 | 5 | domain/services/bucket_classifier_tests.rs |

### Tools (1 file)
| File | Total | Test Lines | Tests | Target |
|------|-------|------------|-------|--------|
| domain/tools/complete_review.rs | 916 | 503 | 35 | domain/tools/complete_review_tests.rs |

### Supervisor (3 files)
| File | Total | Test Lines | Tests | Target |
|------|-------|------------|-------|--------|
| domain/supervisor/patterns.rs | 478 | 180 | 19 | domain/supervisor/patterns_tests.rs |
| domain/supervisor/events.rs | 382 | 153 | 16 | domain/supervisor/events_tests.rs |
| domain/supervisor/actions.rs | 342 | 143 | 18 | domain/supervisor/actions_tests.rs |

### Execution (1 file)
| File | Total | Test Lines | Tests | Target |
|------|-------|------------|-------|--------|
| domain/execution/settings.rs | 131 | 78 | 7 | domain/execution/settings_tests.rs |

### Ideation (1 file)
| File | Total | Test Lines | Tests | Target |
|------|-------|------------|-------|--------|
| domain/ideation/config.rs | 94 | 51 | 4 | domain/ideation/config_tests.rs |

### Repositories (24 files)
| File | Total | Test Lines | Tests | Target |
|------|-------|------------|-------|--------|
| domain/repositories/ideation_session_repository.rs | 655 | 567 | 1 | domain/repositories/ideation_session_repository_tests.rs |
| domain/repositories/task_proposal_repository.rs | 614 | 535 | 1 | domain/repositories/task_proposal_repository_tests.rs |
| domain/repositories/review_repository.rs | 550 | 461 | 1 | domain/repositories/review_repository_tests.rs |
| domain/repositories/proposal_dependency_repository.rs | 523 | 445 | 1 | domain/repositories/proposal_dependency_repository_tests.rs |
| domain/repositories/chat_message_repository.rs | 490 | 421 | 1 | domain/repositories/chat_message_repository_tests.rs |
| domain/repositories/artifact_repository.rs | 506 | 409 | 1 | domain/repositories/artifact_repository_tests.rs |
| domain/repositories/task_dependency_repository.rs | 462 | 408 | 1 | domain/repositories/task_dependency_repository_tests.rs |
| domain/repositories/process_repo.rs | 410 | 359 | 1 | domain/repositories/process_repo_tests.rs |
| domain/repositories/methodology_repo.rs | 389 | 347 | 1 | domain/repositories/methodology_repo_tests.rs |
| domain/repositories/artifact_bucket_repository.rs | 379 | 343 | 1 | domain/repositories/artifact_bucket_repository_tests.rs |
| domain/repositories/task_repository.rs | 640 | 326 | 1 | domain/repositories/task_repository_tests.rs |
| domain/repositories/task_qa_repository.rs | 378 | 314 | 1 | domain/repositories/task_qa_repository_tests.rs |
| domain/repositories/artifact_flow_repository.rs | 318 | 279 | 1 | domain/repositories/artifact_flow_repository_tests.rs |
| domain/repositories/agent_profile_repository.rs | 355 | 272 | 4 | domain/repositories/agent_profile_repository_tests.rs |
| domain/repositories/question_repository.rs | 255 | 221 | 1 | domain/repositories/question_repository_tests.rs |
| domain/repositories/activity_event_repository.rs | 393 | 221 | 6 | domain/repositories/activity_event_repository_tests.rs |
| domain/repositories/permission_repository.rs | 246 | 211 | 1 | domain/repositories/permission_repository_tests.rs |
| domain/repositories/workflow_repository.rs | 243 | 207 | 1 | domain/repositories/workflow_repository_tests.rs |
| domain/repositories/session_link_repository.rs | 206 | 176 | 1 | domain/repositories/session_link_repository_tests.rs |
| domain/repositories/project_repository.rs | 201 | 168 | 1 | domain/repositories/project_repository_tests.rs |
| domain/repositories/chat_conversation_repository.rs | 180 | 113 | 2 | domain/repositories/chat_conversation_repository_tests.rs |
| domain/repositories/agent_run_repository.rs | 215 | 132 | 2 | domain/repositories/agent_run_repository_tests.rs |
| domain/repositories/chat_attachment_repository.rs | 165 | 107 | 2 | domain/repositories/chat_attachment_repository_tests.rs |
| domain/repositories/status_transition.rs | 179 | 103 | 8 | domain/repositories/status_transition_tests.rs |

---

## Group B — Application Files (19 files, ~5,695 test lines)

| File | Total | Test Lines | Tests | Target |
|------|-------|------------|-------|--------|
| application/chat_service/chat_service_errors.rs | 1,927 | 1,379 | 78 | application/chat_service/chat_service_errors_tests.rs |
| application/chat_service/chat_service_context.rs | 1,392 | 676 | 6 | application/chat_service/chat_service_context_tests.rs |
| application/review_issue_service.rs | 886 | 647 | 4 | application/review_issue_service_tests.rs |
| application/qa_service.rs | 991 | 547 | 9 | application/qa_service_tests.rs |
| application/supervisor_service.rs | 794 | 349 | 1 | application/supervisor_service_tests.rs |
| application/chat_resumption.rs | 600 | 310 | 2 | application/chat_resumption_tests.rs |
| application/memory_orchestration.rs | 635 | 245 | 7 | application/memory_orchestration_tests.rs |
| application/memory_archive_service.rs | 637 | 232 | 4 | application/memory_archive_service_tests.rs |
| application/chat_service/chat_service_replay.rs | 411 | 203 | 7 | application/chat_service/chat_service_replay_tests.rs |
| application/chat_service/chat_service_handlers.rs | 1,368 | 171 | 4 | application/chat_service/chat_service_handlers_tests.rs |
| application/chat_service/chat_service_streaming.rs | 1,313 | 169 | 11 | application/chat_service/chat_service_streaming_tests.rs |
| application/resume_validator.rs | 490 | 156 | 6 | application/resume_validator_tests.rs |
| application/diff_service.rs | 842 | 146 | 5 | application/diff_service_tests.rs |
| application/plan_ranking/mod.rs | 277 | 142 | 11 | application/plan_ranking/tests.rs |
| application/git_service/git_cmd.rs | 344 | 139 | 13 | application/git_service/git_cmd_tests.rs |
| application/chat_service/chat_service_helpers.rs | 196 | 81 | 11 | application/chat_service/chat_service_helpers_tests.rs |
| application/team_stream_processor.rs | 503 | 40 | 2 | application/team_stream_processor_tests.rs |
| application/chat_service/chat_service_send_background.rs | 601 | 37 | 5 | application/chat_service/chat_service_send_background_tests.rs |
| application/team_events.rs | 217 | 26 | 2 | application/team_events_tests.rs |

---

## Group C — Infrastructure/Commands/HTTP Files (31 files, ~9,648 test lines)

### Infrastructure (11 files)
| File | Total | Test Lines | Tests | Target |
|------|-------|------------|-------|--------|
| infrastructure/agents/claude/stream_processor.rs | 2,842 | 1,865 | 60 | infrastructure/agents/claude/stream_processor_tests.rs |
| infrastructure/agents/claude/agent_config/team_config.rs | 1,564 | 1,021 | 68 | infrastructure/agents/claude/agent_config/team_config_tests.rs |
| infrastructure/agents/claude/agent_config/mod.rs | 1,681 | 915 | 37 | infrastructure/agents/claude/agent_config/tests.rs |
| infrastructure/agents/claude/claude_code_client.rs | 1,658 | 775 | 45 | infrastructure/agents/claude/claude_code_client_tests.rs |
| infrastructure/sqlite/state_machine_repository.rs | 859 | 533 | 26 | infrastructure/sqlite/state_machine_repository_tests.rs |
| infrastructure/sqlite/sqlite_activity_event_repo.rs | 1,154 | 517 | 2 | infrastructure/sqlite/sqlite_activity_event_repo_tests.rs |
| infrastructure/supervisor/event_bus.rs | 405 | 304 | 16 | infrastructure/supervisor/event_bus_tests.rs |
| infrastructure/agents/claude/agent_config/runtime_config.rs | 431 | 140 | 7 | infrastructure/agents/claude/agent_config/runtime_config_tests.rs |
| infrastructure/memory/memory_question_repo.rs | 195 | 117 | 1 | infrastructure/memory/memory_question_repo_tests.rs |
| infrastructure/memory/memory_permission_repo.rs | 192 | 111 | 1 | infrastructure/memory/memory_permission_repo_tests.rs |
| infrastructure/sqlite/connection.rs | 105 | 66 | 6 | infrastructure/sqlite/connection_tests.rs |

### Commands (14 files, skip execution_commands.rs)
| File | Total | Test Lines | Tests | Target |
|------|-------|------------|-------|--------|
| commands/ideation_commands/mod.rs | 1,563 | 1,539 | 3 | commands/ideation_commands/tests.rs |
| commands/methodology_commands.rs | 498 | 221 | 3 | commands/methodology_commands_tests.rs |
| commands/activity_commands.rs | 373 | 98 | 7 | commands/activity_commands_tests.rs |
| commands/review_helpers.rs | 174 | 64 | 5 | commands/review_helpers_tests.rs |
| commands/unified_chat_commands.rs | 591 | 63 | 4 | commands/unified_chat_commands_tests.rs |
| commands/chat_attachment_commands.rs | 295 | 57 | 3 | commands/chat_attachment_commands_tests.rs |
| commands/plan_branch_commands.rs | 452 | 43 | 5 | commands/plan_branch_commands_tests.rs |
| commands/git_commands.rs | 790 | 35 | 2 | commands/git_commands_tests.rs |
| commands/task_context_commands.rs | 218 | 35 | 3 | commands/task_context_commands_tests.rs |
| commands/question_commands.rs | 96 | 33 | 3 | commands/question_commands_tests.rs |
| commands/permission_commands.rs | 109 | 33 | 3 | commands/permission_commands_tests.rs |
| commands/team_commands.rs | 362 | 31 | 3 | commands/team_commands_tests.rs |
| commands/health.rs | 36 | 17 | 2 | commands/health_tests.rs |
| commands/test_data_commands.rs | 294 | 16 | 1 | commands/test_data_commands_tests.rs |

### HTTP Server (4 files)
| File | Total | Test Lines | Tests | Target |
|------|-------|------------|-------|--------|
| http_server/handlers/teams.rs | 1,481 | 350 | 7 | http_server/handlers/teams_tests.rs |
| http_server/helpers.rs | 1,179 | 316 | 10 | http_server/helpers_tests.rs |
| http_server/handlers/git.rs | 884 | 108 | 14 | http_server/handlers/git_tests.rs |
| http_server/handlers/session_linking.rs | 589 | 48 | 4 | http_server/handlers/session_linking_tests.rs |

### Other (2 files)
| File | Total | Test Lines | Tests | Target |
|------|-------|------------|-------|--------|
| error.rs | 164 | 98 | 13 | error_tests.rs |
| testing/test_prompts.rs | 149 | 79 | 11 | testing/test_prompts_tests.rs |

---

## Skip List (already extracted or out of scope)

| File | Reason |
|------|--------|
| application/reconciliation/ | Already extracted (tests.rs companion) |
| domain/state_machine/transition_handler/ | Already extracted (tests/ subdirectory) |
| commands/execution_commands.rs | Separate refactor (4,717 lines) |
| infrastructure/sqlite/migrations/v37-v44_*_tests.rs | Already separate test files |
| application/ideation_service/tests.rs | Already a test file |
| commands/task_commands/tests.rs | Already a test file |
| infrastructure/sqlite/sqlite_task_proposal_repo/tests.rs | Already a test file |
| tests/hardening/concurrent_merge_guard_tests.rs | Already in tests/ directory |
| Files with #[cfg(test)] but 0 #[test] fns | Test helpers/utils only, no extraction needed |

## Notes
- All paths are relative to `src-tauri/src/`
- "Test Lines" = estimated lines from first `#[cfg(test)]` to end of file
- Repository files often show 1 test count but contain macro-generated test suites
- `ideation_commands/mod.rs` has 1,539 test lines (98% of file is tests)
- For `plan_ranking/mod.rs` and `agent_config/mod.rs`, use `mod tests;` pattern (already have companion dirs)
