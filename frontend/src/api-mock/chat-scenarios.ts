import {
  normalizeConversationProviderMetadata,
  type ChatConversation,
  type ContextType,
} from "@/types/chat-conversation";
import type {
  ChatMessageResponse,
  QueuedMessageResponse,
} from "@/api/chat";

export type MockChatScenarioName =
  | "ideation_db_widget_mix"
  | "execution_db_compact"
  | "review_db_compact"
  | "merge_db_compact";

export type MockChatScenario = {
  conversations: ChatConversation[];
  messages: Record<string, ChatMessageResponse[]>;
  queuedMessages?: Record<string, QueuedMessageResponse[]>;
};

function createConversation(
  conversation: Omit<ChatConversation, "claudeSessionId" | "providerSessionId" | "providerHarness"> &
    Parameters<typeof normalizeConversationProviderMetadata>[0]
): ChatConversation {
  return {
    id: conversation.id,
    contextType: conversation.contextType,
    contextId: conversation.contextId,
    ...normalizeConversationProviderMetadata({
      claudeSessionId: conversation.claudeSessionId,
      providerSessionId: conversation.providerSessionId,
      providerHarness: conversation.providerHarness,
    }),
    title: conversation.title,
    messageCount: conversation.messageCount,
    lastMessageAt: conversation.lastMessageAt,
    createdAt: conversation.createdAt,
    updatedAt: conversation.updatedAt,
  };
}

export function cloneMockChatMessage(
  message: ChatMessageResponse
): ChatMessageResponse {
  return {
    ...message,
    toolCalls: message.toolCalls ? [...message.toolCalls] : null,
    contentBlocks: message.contentBlocks ? [...message.contentBlocks] : null,
  };
}

export function listMockChatScenarios(): MockChatScenarioName[] {
  return Object.keys(chatScenarioFixtures) as MockChatScenarioName[];
}

export function getMockChatScenario(name: MockChatScenarioName): MockChatScenario {
  return chatScenarioFixtures[name];
}

export const IDEATION_REPLAY_CONTEXT: {
  contextType: Extract<ContextType, "ideation">;
  contextId: string;
  conversationId: string;
} = {
  contextType: "ideation",
  contextId: "session-mock-1",
  conversationId: "conv-ideation-db-widget-mix",
};

export const TASK_REPLAY_CONTEXTS: Record<
  Exclude<MockChatScenarioName, "ideation_db_widget_mix">,
  {
    contextType: Extract<ContextType, "task_execution" | "review" | "merge">;
    contextId: string;
    conversationId: string;
  }
> = {
  execution_db_compact: {
    contextType: "task_execution",
    contextId: "task-mock-4",
    conversationId: "conv-execution-db-compact",
  },
  review_db_compact: {
    contextType: "review",
    contextId: "task-mock-5",
    conversationId: "conv-review-db-compact",
  },
  merge_db_compact: {
    contextType: "merge",
    contextId: "task-mock-merge-incomplete",
    conversationId: "conv-merge-db-compact",
  },
};

const ideationReplayConversationId = IDEATION_REPLAY_CONTEXT.conversationId;
const executionReplayConversationId =
  TASK_REPLAY_CONTEXTS.execution_db_compact.conversationId;
const reviewReplayConversationId =
  TASK_REPLAY_CONTEXTS.review_db_compact.conversationId;
const mergeReplayConversationId =
  TASK_REPLAY_CONTEXTS.merge_db_compact.conversationId;

const chatScenarioFixtures: Record<MockChatScenarioName, MockChatScenario> = {
  ideation_db_widget_mix: {
    conversations: [
      createConversation({
        id: ideationReplayConversationId,
        contextType: "ideation",
        contextId: IDEATION_REPLAY_CONTEXT.contextId,
        claudeSessionId: "db-ideation-claude-session-1",
        title: "GitHub task-pipeline integration",
        messageCount: 6,
        lastMessageAt: "2026-03-11T21:58:43.308888Z",
        createdAt: "2026-03-11T21:51:34.604754Z",
        updatedAt: "2026-03-11T21:58:43.308888Z",
      }),
    ],
    messages: {
      [ideationReplayConversationId]: [
        {
          id: "msg-ideation-user-1",
          sessionId: "session-mock-1",
          projectId: "project-mock-1",
          taskId: null,
          role: "user",
          content:
            "we’ll be working on a github integration with the task pipeline. will need a new configurable option to create a PR when merge succeeds.",
          metadata: null,
          parentMessageId: null,
          conversationId: ideationReplayConversationId,
          toolCalls: null,
          contentBlocks: null,
          sender: null,
          createdAt: "2026-03-11T21:51:34.589194Z",
        },
        {
          id: "msg-ideation-orchestrator-1",
          sessionId: "session-mock-1",
          projectId: "project-mock-1",
          taskId: null,
          role: "assistant",
          content:
            "Let me start by checking the session state and pulling in specialists. The real DB sample behind this replay had a compact user prompt and a large orchestrator response dominated by Agent, TaskCreate, TeamCreate, and session-context MCP calls.",
          metadata: null,
          parentMessageId: "msg-ideation-user-1",
          conversationId: ideationReplayConversationId,
          toolCalls: null,
          contentBlocks: [
            {
              type: "text",
              text: "Fresh session. I’m gathering context and setting up the team plan.",
            },
            {
              type: "tool_use",
              id: "tool-team-create-1",
              name: "TeamCreate",
              arguments: {
                team_name: "pipeline-research",
                description: "Parallel GitHub integration specialists",
              },
              result: { success: true, team_id: "team-1" },
            },
            {
              type: "tool_use",
              id: "tool-parent-context-1",
              name: "mcp__ralphx__get_parent_session_context",
              arguments: { session_id: "session-mock-1" },
              result: [
                {
                  type: "text",
                  text: "{\"ok\":true,\"title\":\"Demo Ideation Session\"}",
                },
              ],
            },
            {
              type: "tool_use",
              id: "tool-request-plan-1",
              name: "mcp__ralphx__request_team_plan",
              arguments: {
                process: "research",
                teammates: [
                  {
                    role: "pipeline-researcher",
                    model: "sonnet",
                    prompt_summary: "Audit merge pipeline",
                  },
                  {
                    role: "settings-researcher",
                    model: "sonnet",
                    prompt_summary: "Audit settings surface",
                  },
                ],
              },
              result: {
                plan_id: "plan-1",
                teammates_spawned: [{ id: "pipeline-1" }, { id: "settings-1" }],
              },
            },
            {
              type: "tool_use",
              id: "tool-task-create-1",
              name: "TaskCreate",
              arguments: {
                subject: "Support optional PR creation on merge",
                description: "Add a configurable merge-time GitHub PR path.",
              },
              result: { task_id: "task-github-pr" },
            },
            {
              type: "tool_use",
              id: "tool-agent-1",
              name: "Agent",
              arguments: {
                description: "Inspect merge pipeline hooks",
                subagent_type: "research",
              },
              result: [
                {
                  type: "tool_use",
                  id: "tool-agent-1-read-1",
                  name: "Read",
                  input: {
                    file_path: "src-tauri/src/application/chat_service/mod.rs",
                  },
                },
                {
                  type: "tool_result",
                  tool_use_id: "tool-agent-1-read-1",
                  content: "inspected merge pipeline",
                },
              ],
            },
          ],
          sender: "lead",
          createdAt: "2026-03-11T21:51:34.604754Z",
        },
        {
          id: "msg-ideation-orchestrator-2",
          sessionId: "session-mock-1",
          projectId: "project-mock-1",
          taskId: null,
          role: "assistant",
          content:
            "Good, first findings are in. The state machine can stay the same while the merge implementation picks up an optional GitHub PR action at the end.",
          metadata: null,
          parentMessageId: "msg-ideation-orchestrator-1",
          conversationId: ideationReplayConversationId,
          toolCalls: null,
          contentBlocks: [
            {
              type: "text",
              text: "I’m updating the task and pulling the current verification state.",
            },
            {
              type: "tool_use",
              id: "tool-task-update-1",
              name: "TaskUpdate",
              arguments: {
                taskId: "task-github-pr",
                status: "in_progress",
                owner: "codex",
                subject: "Replay chat UI from DB samples",
              },
              result: { success: true },
            },
            {
              type: "tool_use",
              id: "tool-plan-verification-1",
              name: "mcp__ralphx__get_plan_verification",
              arguments: { session_id: "session-mock-1" },
              result: [{ type: "text", text: "{\"round\":2,\"gapScore\":3}" }],
            },
            {
              type: "tool_use",
              id: "tool-create-artifact-1",
              name: "mcp__ralphx__create_plan_artifact",
              arguments: {
                title: "GitHub PR merge workflow",
                artifact_type: "design_doc",
              },
              result: [
                {
                  type: "text",
                  text: "{\"title\":\"GitHub PR merge workflow\",\"artifact_type\":\"design_doc\",\"version\":3}",
                },
              ],
            },
            {
              type: "tool_use",
              id: "tool-team-artifacts-1",
              name: "mcp__ralphx__get_team_artifacts",
              arguments: { session_id: "session-mock-1" },
              result: [
                {
                  type: "text",
                  text: "{\"artifacts\":[{\"title\":\"Pipeline audit\"},{\"title\":\"Settings audit\"}]}",
                },
              ],
            },
          ],
          sender: "lead",
          createdAt: "2026-03-11T21:54:55.491580Z",
        },
        {
          id: "msg-ideation-orchestrator-3",
          sessionId: "session-mock-1",
          projectId: "project-mock-1",
          taskId: null,
          role: "assistant",
          content:
            "Critics are reporting back. I’m nudging the remaining teammates while cross-checking repository usage with direct file reads and searches.",
          metadata: null,
          parentMessageId: "msg-ideation-orchestrator-2",
          conversationId: ideationReplayConversationId,
          toolCalls: null,
          contentBlocks: [
            {
              type: "text",
              text: "This bundle mirrors the real DB pattern where one orchestrator message mixes SendMessage, file reads, and verification updates.",
            },
            {
              type: "tool_use",
              id: "tool-send-message-1",
              name: "SendMessage",
              arguments: {
                recipient: "layer1-critic",
                content: "Nudged for findings.",
              },
              result: { success: true },
            },
            {
              type: "tool_use",
              id: "tool-read-1",
              name: "Read",
              arguments: {
                file_path: "src-tauri/src/application/chat_service/mod.rs",
              },
              result: "merge pipeline helper logic",
            },
            {
              type: "tool_use",
              id: "tool-grep-1",
              name: "Grep",
              arguments: { pattern: "create_pr", path: "src-tauri" },
              result: "found in merge workflow dialog",
            },
            {
              type: "tool_use",
              id: "tool-glob-1",
              name: "Glob",
              arguments: { pattern: "**/*merge*.rs", path: "src-tauri" },
              result: [
                "src-tauri/src/domain/state_machine/transition_handler/on_enter_states/merge.rs",
              ],
            },
            {
              type: "tool_use",
              id: "tool-ask-user-1",
              name: "mcp__ralphx__ask_user_question",
              arguments: {
                question: "Preferred default for automatic PR creation?",
                options: ["Always", "Configurable", "Never"],
              },
              result: { request_id: "question-1", status: "sent" },
            },
          ],
          sender: "lead",
          createdAt: "2026-03-11T21:58:43.308888Z",
        },
      ],
    },
  },
  execution_db_compact: {
    conversations: [
      createConversation({
        id: executionReplayConversationId,
        contextType: "task_execution",
        contextId: TASK_REPLAY_CONTEXTS.execution_db_compact.contextId,
        claudeSessionId: "db-execution-claude-session-1",
        title: "Execute task-mock-4",
        messageCount: 2,
        lastMessageAt: "2026-04-07T01:13:52.882129Z",
        createdAt: "2026-04-07T01:13:52.876318Z",
        updatedAt: "2026-04-07T01:13:52.882129Z",
      }),
    ],
    messages: {
      [executionReplayConversationId]: [
        {
          id: "msg-exec-user-1",
          sessionId: null,
          projectId: "project-mock-1",
          taskId: "task-mock-4",
          role: "user",
          content: "Execute task: task-mock-4",
          metadata: null,
          parentMessageId: null,
          conversationId: executionReplayConversationId,
          toolCalls: null,
          contentBlocks: null,
          sender: null,
          createdAt: "2026-04-07T01:13:52.876318Z",
        },
        {
          id: "msg-exec-worker-1",
          sessionId: null,
          projectId: "project-mock-1",
          taskId: "task-mock-4",
          role: "assistant",
          content:
            "The blocker task is merged. I’m checking the current state of the modified files and validating the pipeline before making changes.",
          metadata: null,
          parentMessageId: "msg-exec-user-1",
          conversationId: executionReplayConversationId,
          toolCalls: null,
          contentBlocks: [
            {
              type: "text",
              text: "Execution replay sampled from a compact two-message worker conversation.",
            },
            {
              type: "tool_use",
              id: "exec-read-1",
              name: "Read",
              arguments: {
                file_path: "frontend/src/components/Chat/MessageItem.tsx",
              },
              result: "provider metadata rendering",
            },
            {
              type: "tool_use",
              id: "exec-grep-1",
              name: "Grep",
              arguments: {
                pattern: "providerHarness",
                path: "frontend/src/components/Chat",
              },
              result: "multiple matches",
            },
            {
              type: "tool_use",
              id: "exec-bash-1",
              name: "bash",
              arguments: {
                command:
                  "npm --prefix frontend run test:run -- src/components/Chat/MessageItem.test.tsx",
              },
              result: "tests passed",
            },
          ],
          sender: "worker",
          createdAt: "2026-04-07T01:13:52.882129Z",
        },
      ],
    },
  },
  review_db_compact: {
    conversations: [
      createConversation({
        id: reviewReplayConversationId,
        contextType: "review",
        contextId: TASK_REPLAY_CONTEXTS.review_db_compact.contextId,
        claudeSessionId: "db-review-claude-session-1",
        title: "Review task-mock-5",
        messageCount: 2,
        lastMessageAt: "2026-04-07T01:16:57.845453Z",
        createdAt: "2026-04-07T01:16:57.831419Z",
        updatedAt: "2026-04-07T01:16:57.845453Z",
      }),
    ],
    messages: {
      [reviewReplayConversationId]: [
        {
          id: "msg-review-user-1",
          sessionId: null,
          projectId: "project-mock-1",
          taskId: "task-mock-5",
          role: "user",
          content: "Review task: task-mock-5",
          metadata: null,
          parentMessageId: null,
          conversationId: reviewReplayConversationId,
          toolCalls: null,
          contentBlocks: null,
          sender: null,
          createdAt: "2026-04-07T01:16:57.831419Z",
        },
        {
          id: "msg-reviewer-1",
          sessionId: null,
          projectId: "project-mock-1",
          taskId: "task-mock-5",
          role: "assistant",
          content:
            "This is a first review. I’m examining the diff and changed files before deciding whether to request changes or approve.",
          metadata: null,
          parentMessageId: "msg-review-user-1",
          conversationId: reviewReplayConversationId,
          toolCalls: null,
          contentBlocks: [
            {
              type: "text",
              text: "Reviewer replay sampled from a compact two-message real conversation.",
            },
            {
              type: "tool_use",
              id: "review-complete-1",
              name: "complete_review",
              arguments: {
                decision: "changes_requested",
                feedback: "Need stronger widget replay coverage.",
                issues: [
                  {
                    severity: "major",
                    description: "Missing DB-derived chat replay fixtures",
                  },
                ],
              },
              result: {
                success: true,
                new_status: "reviewing",
                followup_session_id: "followup-1",
              },
            },
          ],
          sender: "reviewer",
          createdAt: "2026-04-07T01:16:57.845453Z",
        },
      ],
    },
  },
  merge_db_compact: {
    conversations: [
      createConversation({
        id: mergeReplayConversationId,
        contextType: "merge",
        contextId: TASK_REPLAY_CONTEXTS.merge_db_compact.contextId,
        claudeSessionId: null,
        providerHarness: "codex",
        providerSessionId: "thread-merge-codex-1",
        title: "Merge task-mock-merge-incomplete",
        messageCount: 2,
        lastMessageAt: "2026-04-07T01:14:03.573602Z",
        createdAt: "2026-04-07T01:14:03.561142Z",
        updatedAt: "2026-04-07T01:14:03.573602Z",
      }),
    ],
    messages: {
      [mergeReplayConversationId]: [
        {
          id: "msg-merge-user-1",
          sessionId: null,
          projectId: "project-mock-1",
          taskId: "task-mock-merge-incomplete",
          role: "user",
          content:
            "Resolve the merge conflict for task-mock-merge-incomplete",
          metadata: null,
          parentMessageId: null,
          conversationId: mergeReplayConversationId,
          toolCalls: null,
          contentBlocks: null,
          sender: null,
          createdAt: "2026-04-07T01:14:03.561142Z",
        },
        {
          id: "msg-merge-merger-1",
          sessionId: null,
          projectId: "project-mock-1",
          taskId: "task-mock-merge-incomplete",
          role: "assistant",
          content:
            "On the plan branch. Conflict file is src/commands/gateway.ts. I’m resolving and validating before finalizing the merge.",
          metadata: null,
          parentMessageId: "msg-merge-user-1",
          conversationId: mergeReplayConversationId,
          toolCalls: null,
          contentBlocks: [
            {
              type: "text",
              text: "Merge replay sampled from a compact two-message merger conversation.",
            },
            {
              type: "tool_use",
              id: "merge-target-1",
              name: "get_merge_target",
              arguments: {},
              result: { source_branch: "task/mock-merge", target_branch: "main" },
            },
            {
              type: "tool_use",
              id: "merge-complete-1",
              name: "mcp__ralphx__complete_merge",
              arguments: { commit_sha: "abcdef1234567" },
              result: {
                success: true,
                message: "Merged cleanly",
                new_status: "merged",
              },
            },
          ],
          sender: "merger",
          createdAt: "2026-04-07T01:14:03.573602Z",
        },
      ],
    },
  },
};
