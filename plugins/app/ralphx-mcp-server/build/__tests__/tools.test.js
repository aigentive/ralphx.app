/**
 * Unit tests for MCP tool definitions and authorization logic
 * Tests agent team coordination features
 */
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { getAllowedToolNames, getFilteredTools, isToolAllowed, setAgentType, getAllTools, getToolRecoveryHint, formatToolErrorMessage, TOOL_ALLOWLIST, parseAllowedToolsFromArgs, } from '../tools.js';
import { PLAN_TOOLS } from '../plan-tools.js';
import { IDEATION_TEAM_LEAD, IDEATION_TEAM_MEMBER, WORKER_TEAM_MEMBER, ORCHESTRATOR_IDEATION, ORCHESTRATOR_IDEATION_READONLY, IDEATION_SPECIALIST_BACKEND, IDEATION_SPECIALIST_FRONTEND, IDEATION_SPECIALIST_INFRA, IDEATION_SPECIALIST_CODE_QUALITY, IDEATION_SPECIALIST_PROMPT_QUALITY, IDEATION_SPECIALIST_INTENT, IDEATION_SPECIALIST_PIPELINE_SAFETY, IDEATION_SPECIALIST_STATE_MACHINE, IDEATION_CRITIC, IDEATION_ADVOCATE, PLAN_VERIFIER, PLAN_CRITIC_COMPLETENESS, PLAN_CRITIC_IMPLEMENTATION_FEASIBILITY, REVIEWER, WORKER, MERGER, } from '../agentNames.js';
describe('getAllowedToolNames', () => {
    beforeEach(() => {
        // Clear env var before each test
        delete process.env.RALPHX_ALLOWED_MCP_TOOLS;
    });
    afterEach(() => {
        // Clean up env var after each test
        delete process.env.RALPHX_ALLOWED_MCP_TOOLS;
    });
    it('should return parsed list when RALPHX_ALLOWED_MCP_TOOLS env var is set', () => {
        process.env.RALPHX_ALLOWED_MCP_TOOLS = 'get_session_plan,create_team_artifact';
        const tools = getAllowedToolNames();
        expect(tools).toEqual(['get_session_plan', 'create_team_artifact']);
    });
    it('should handle spaces in env var', () => {
        process.env.RALPHX_ALLOWED_MCP_TOOLS = '  get_session_plan  ,  create_team_artifact  ';
        const tools = getAllowedToolNames();
        expect(tools).toEqual(['get_session_plan', 'create_team_artifact']);
    });
    it('should handle trailing commas in env var', () => {
        process.env.RALPHX_ALLOWED_MCP_TOOLS = 'get_session_plan,create_team_artifact,';
        const tools = getAllowedToolNames();
        expect(tools).toEqual(['get_session_plan', 'create_team_artifact']);
    });
    it('should filter out empty entries in env var', () => {
        process.env.RALPHX_ALLOWED_MCP_TOOLS = 'get_session_plan,,create_team_artifact,  ,';
        const tools = getAllowedToolNames();
        expect(tools).toEqual(['get_session_plan', 'create_team_artifact']);
    });
    it('should return TOOL_ALLOWLIST entry when env var is unset and agent type is valid', () => {
        setAgentType(IDEATION_TEAM_LEAD);
        const tools = getAllowedToolNames();
        expect(tools).toEqual(TOOL_ALLOWLIST[IDEATION_TEAM_LEAD]);
    });
    it('should return empty array when env var is unset and agent type is unknown', () => {
        setAgentType('unknown-agent-type');
        const tools = getAllowedToolNames();
        expect(tools).toEqual([]);
    });
    it('should return empty array when env var is empty string', () => {
        process.env.RALPHX_ALLOWED_MCP_TOOLS = '';
        const tools = getAllowedToolNames();
        expect(tools).toEqual([]);
    });
    it('should prioritize env var over agent type allowlist', () => {
        setAgentType(IDEATION_TEAM_LEAD);
        process.env.RALPHX_ALLOWED_MCP_TOOLS = 'get_session_plan';
        const tools = getAllowedToolNames();
        // Should return env var list, not agent type allowlist
        expect(tools).toEqual(['get_session_plan']);
        expect(tools).not.toEqual(TOOL_ALLOWLIST[IDEATION_TEAM_LEAD]);
    });
    it('should strip delegation tools from env override for non-delegating agents', () => {
        setAgentType(IDEATION_TEAM_LEAD);
        process.env.RALPHX_ALLOWED_MCP_TOOLS = 'delegate_start,get_session_plan,delegate_wait';
        const tools = getAllowedToolNames();
        expect(tools).toEqual(['get_session_plan']);
    });
});
describe('getToolRecoveryHint', () => {
    it('returns parent-session and example guidance for update_plan_verification', () => {
        const hint = getToolRecoveryHint('update_plan_verification');
        expect(hint).toContain('PARENT ideation session_id');
        expect(hint).toContain('backend remaps it automatically');
        expect(hint).toContain('prefer those narrower helpers');
        expect(hint).toContain('status=reviewing');
        expect(hint).toContain('Example reviewing payload:');
        expect(hint).toContain('Example terminal payload:');
    });
    it('returns narrower verifier-helper guidance for report_verification_round', () => {
        const hint = getToolRecoveryHint('report_verification_round');
        expect(hint).toContain('verifier-friendly helper');
        expect(hint).toContain('backend remaps it to the parent automatically');
        expect(hint).toContain('status=reviewing and in_progress=true are filled in automatically');
        expect(hint).toContain('Example payload:');
    });
    it('returns narrower verifier-helper guidance for complete_plan_verification', () => {
        const hint = getToolRecoveryHint('complete_plan_verification');
        expect(hint).toContain('terminal verification updates');
        expect(hint).toContain('backend remaps it to the parent automatically');
        expect(hint).toContain('in_progress=false is filled in automatically');
        expect(hint).toContain('External sessions cannot use status=skipped');
    });
    it('returns artifact-collection guidance for get_verification_round_artifacts', () => {
        const hint = getToolRecoveryHint('get_verification_round_artifacts');
        expect(hint).toContain('verifier helper');
        expect(hint).toContain('get_team_artifacts + get_artifact');
        expect(hint).toContain('created_after');
    });
    it('returns verifier-debugging guidance for get_child_session_status', () => {
        const hint = getToolRecoveryHint('get_child_session_status');
        expect(hint).toContain('include_recent_messages=true');
        expect(hint).toContain('Example payload:');
    });
    it('returns invariant-context guidance for send_ideation_session_message', () => {
        const hint = getToolRecoveryHint('send_ideation_session_message');
        expect(hint).toContain('SESSION_ID, ROUND, artifact prefix/schema');
        expect(hint).toContain('Example payload:');
    });
    it('returns null for an unknown tool', () => {
        expect(getToolRecoveryHint('not_a_real_tool')).toBeNull();
    });
});
describe('formatToolErrorMessage', () => {
    it('appends details and a usage hint for known high-friction tools', () => {
        const text = formatToolErrorMessage('update_plan_verification', 'Cannot update verification state on a verification child session.', 'Use the parent session id instead.');
        expect(text).toContain('ERROR: Cannot update verification state on a verification child session.');
        expect(text).toContain('Details: Use the parent session id instead.');
        expect(text).toContain('Usage hint for update_plan_verification:');
        expect(text).toContain('Example reviewing payload:');
    });
    it('leaves unknown tools without a usage-hint section', () => {
        const text = formatToolErrorMessage('not_a_real_tool', 'boom');
        expect(text).toBe('ERROR: boom');
    });
});
describe('getFilteredTools', () => {
    beforeEach(() => {
        delete process.env.RALPHX_ALLOWED_MCP_TOOLS;
    });
    afterEach(() => {
        delete process.env.RALPHX_ALLOWED_MCP_TOOLS;
    });
    it('should return correct tool set for ralphx-ideation-team-lead', () => {
        setAgentType(IDEATION_TEAM_LEAD);
        const tools = getFilteredTools();
        const toolNames = tools.map((t) => t.name);
        // Should include team coordination tools
        expect(toolNames).toContain('request_team_plan');
        expect(toolNames).toContain('request_teammate_spawn');
        expect(toolNames).toContain('create_team_artifact');
        expect(toolNames).toContain('get_team_artifacts');
        expect(toolNames).toContain('get_team_session_state');
        expect(toolNames).toContain('save_team_session_state');
        // Should include ideation tools
        expect(toolNames).toContain('create_task_proposal');
        expect(toolNames).toContain('update_task_proposal');
        expect(toolNames).toContain('get_session_plan');
        // Should match allowlist count
        expect(tools.length).toBe(TOOL_ALLOWLIST[IDEATION_TEAM_LEAD].length);
    });
    it('should return only allowed tools for ideation-team-member (read-only)', () => {
        setAgentType(IDEATION_TEAM_MEMBER);
        const tools = getFilteredTools();
        const toolNames = tools.map((t) => t.name);
        // Should include artifact tools
        expect(toolNames).toContain('create_team_artifact');
        expect(toolNames).toContain('get_team_artifacts');
        // Should include read-only access tools
        expect(toolNames).toContain('get_session_plan');
        expect(toolNames).toContain('list_session_proposals');
        expect(toolNames).toContain('get_artifact');
        // Should NOT include lead-only tools
        expect(toolNames).not.toContain('request_team_plan');
        expect(toolNames).not.toContain('request_teammate_spawn');
        expect(toolNames).not.toContain('create_task_proposal');
        expect(toolNames).not.toContain('save_team_session_state');
        // Should match allowlist count
        expect(tools.length).toBe(TOOL_ALLOWLIST[IDEATION_TEAM_MEMBER].length);
    });
    it('should return correct tool set for worker-team-member', () => {
        setAgentType(WORKER_TEAM_MEMBER);
        const tools = getFilteredTools();
        const toolNames = tools.map((t) => t.name);
        // Should include artifact tools (document decisions)
        expect(toolNames).toContain('create_team_artifact');
        expect(toolNames).toContain('get_team_artifacts');
        // Should include worker step tools
        expect(toolNames).toContain('start_step');
        expect(toolNames).toContain('complete_step');
        expect(toolNames).toContain('get_task_context');
        // Should NOT include lead-only tools
        expect(toolNames).not.toContain('request_team_plan');
        expect(toolNames).not.toContain('request_teammate_spawn');
        expect(toolNames).not.toContain('save_team_session_state');
        // Should match allowlist count
        expect(tools.length).toBe(TOOL_ALLOWLIST[WORKER_TEAM_MEMBER].length);
    });
    it('should scope ralphx-plan-verifier to the narrower verification helpers', () => {
        setAgentType('ralphx-plan-verifier');
        const tools = getFilteredTools();
        const toolNames = tools.map((t) => t.name);
        expect(toolNames).toContain('fs_read_file');
        expect(toolNames).toContain('fs_list_dir');
        expect(toolNames).toContain('fs_grep');
        expect(toolNames).toContain('fs_glob');
        expect(toolNames).toContain('report_verification_round');
        expect(toolNames).toContain('complete_plan_verification');
        expect(toolNames).toContain('get_verification_round_artifacts');
        expect(toolNames).toContain('get_plan_verification');
        expect(toolNames).not.toContain('update_plan_verification');
        expect(toolNames).not.toContain('get_team_artifacts');
        expect(toolNames).not.toContain('get_artifact');
    });
    it('should expose read-only filesystem tools for qa prep', () => {
        setAgentType('qa-prep');
        const tools = getFilteredTools();
        const toolNames = tools.map((t) => t.name);
        expect(toolNames).toEqual(['fs_read_file', 'fs_list_dir', 'fs_grep', 'fs_glob']);
    });
    it('should return no tools for unknown agent type', () => {
        setAgentType('unknown-agent-type');
        const tools = getFilteredTools();
        expect(tools).toEqual([]);
    });
    it('should return only env var tools when env var is set', () => {
        setAgentType(IDEATION_TEAM_LEAD);
        process.env.RALPHX_ALLOWED_MCP_TOOLS = 'get_session_plan,create_team_artifact';
        const tools = getFilteredTools();
        const toolNames = tools.map((t) => t.name);
        // Check contents without caring about order
        expect(toolNames).toContain('get_session_plan');
        expect(toolNames).toContain('create_team_artifact');
        expect(tools.length).toBe(2);
    });
});
describe('isToolAllowed', () => {
    beforeEach(() => {
        delete process.env.RALPHX_ALLOWED_MCP_TOOLS;
    });
    afterEach(() => {
        delete process.env.RALPHX_ALLOWED_MCP_TOOLS;
    });
    it('should return true for allowed tool', () => {
        setAgentType(IDEATION_TEAM_LEAD);
        expect(isToolAllowed('request_team_plan')).toBe(true);
        expect(isToolAllowed('create_task_proposal')).toBe(true);
    });
    it('should return false for disallowed tool', () => {
        setAgentType(IDEATION_TEAM_MEMBER);
        expect(isToolAllowed('request_team_plan')).toBe(false);
        expect(isToolAllowed('create_task_proposal')).toBe(false);
    });
    it('should return false for unknown tool', () => {
        setAgentType(IDEATION_TEAM_LEAD);
        expect(isToolAllowed('nonexistent_tool')).toBe(false);
    });
    it('should respect env var override for allowed tool', () => {
        setAgentType(IDEATION_TEAM_LEAD);
        process.env.RALPHX_ALLOWED_MCP_TOOLS = 'get_session_plan';
        expect(isToolAllowed('get_session_plan')).toBe(true);
        expect(isToolAllowed('request_team_plan')).toBe(false); // Not in env var
    });
    it('should respect env var override for disallowed tool', () => {
        setAgentType(IDEATION_TEAM_MEMBER);
        process.env.RALPHX_ALLOWED_MCP_TOOLS = 'request_team_plan'; // Normally not allowed
        expect(isToolAllowed('request_team_plan')).toBe(true);
    });
});
describe('New team tool definitions', () => {
    const allTools = getAllTools();
    describe('request_team_plan', () => {
        const tool = allTools.find((t) => t.name === 'request_team_plan');
        it('should exist in ALL_TOOLS', () => {
            expect(tool).toBeDefined();
        });
        it('should have correct inputSchema with required fields', () => {
            expect(tool?.inputSchema).toBeDefined();
            expect(tool?.inputSchema.type).toBe('object');
            expect(tool?.inputSchema.properties).toHaveProperty('process');
            expect(tool?.inputSchema.properties).toHaveProperty('teammates');
            expect(tool?.inputSchema.required).toContain('process');
            expect(tool?.inputSchema.required).toContain('teammates');
        });
        it('should have teammates array with correct item schema', () => {
            const teammates = tool?.inputSchema.properties?.teammates;
            expect(teammates).toBeDefined();
            expect(teammates.type).toBe('array');
            expect(teammates.items).toBeDefined();
            expect(teammates.items.type).toBe('object');
            expect(teammates.items.properties).toHaveProperty('role');
            expect(teammates.items.properties).toHaveProperty('tools');
            expect(teammates.items.properties).toHaveProperty('mcp_tools');
            expect(teammates.items.properties).toHaveProperty('model');
            expect(teammates.items.properties).toHaveProperty('prompt_summary');
        });
    });
    describe('request_teammate_spawn', () => {
        const tool = allTools.find((t) => t.name === 'request_teammate_spawn');
        it('should exist in ALL_TOOLS', () => {
            expect(tool).toBeDefined();
        });
        it('should have correct inputSchema with required fields', () => {
            expect(tool?.inputSchema).toBeDefined();
            expect(tool?.inputSchema.type).toBe('object');
            expect(tool?.inputSchema.properties).toHaveProperty('role');
            expect(tool?.inputSchema.properties).toHaveProperty('prompt');
            expect(tool?.inputSchema.properties).toHaveProperty('model');
            expect(tool?.inputSchema.properties).toHaveProperty('tools');
            expect(tool?.inputSchema.properties).toHaveProperty('mcp_tools');
            expect(tool?.inputSchema.required).toContain('role');
            expect(tool?.inputSchema.required).toContain('prompt');
            expect(tool?.inputSchema.required).toContain('model');
            expect(tool?.inputSchema.required).toContain('tools');
            expect(tool?.inputSchema.required).toContain('mcp_tools');
        });
        it('should have model enum constraint', () => {
            const model = tool?.inputSchema.properties?.model;
            expect(model).toBeDefined();
            expect(model.enum).toEqual(['haiku', 'sonnet', 'opus']);
        });
    });
    describe('create_team_artifact', () => {
        const tool = allTools.find((t) => t.name === 'create_team_artifact');
        it('should exist in ALL_TOOLS', () => {
            expect(tool).toBeDefined();
        });
        it('should have correct inputSchema with required fields', () => {
            expect(tool?.inputSchema).toBeDefined();
            expect(tool?.inputSchema.type).toBe('object');
            expect(tool?.inputSchema.properties).toHaveProperty('session_id');
            expect(tool?.inputSchema.properties).toHaveProperty('title');
            expect(tool?.inputSchema.properties).toHaveProperty('content');
            expect(tool?.inputSchema.properties).toHaveProperty('artifact_type');
            expect(tool?.inputSchema.required).toContain('session_id');
            expect(tool?.inputSchema.required).toContain('title');
            expect(tool?.inputSchema.required).toContain('content');
            expect(tool?.inputSchema.required).toContain('artifact_type');
        });
        it('should have artifact_type enum constraint', () => {
            const artifactType = tool?.inputSchema.properties?.artifact_type;
            expect(artifactType).toBeDefined();
            expect(artifactType.enum).toEqual(['TeamResearch', 'TeamAnalysis', 'TeamSummary']);
        });
        it('should document parent-session targeting for verification flows', () => {
            expect(tool?.description).toContain('PARENT ideation session_id');
            expect(tool?.description).toContain('backend remaps it to the parent ideation session automatically');
            expect(tool?.description).toContain('Example critic artifact');
            expect(tool?.inputSchema.properties?.session_id?.description).toContain('auto-remapped to that parent');
            expect(tool?.inputSchema.properties?.title?.description).toContain('Completeness: ');
            expect((tool?.inputSchema).examples?.[0]).toMatchObject({
                session_id: 'parent-session-id',
                artifact_type: 'TeamResearch',
            });
        });
    });
    describe('get_team_artifacts', () => {
        const tool = allTools.find((t) => t.name === 'get_team_artifacts');
        it('should exist in ALL_TOOLS', () => {
            expect(tool).toBeDefined();
        });
        it('should have correct inputSchema with required fields', () => {
            expect(tool?.inputSchema).toBeDefined();
            expect(tool?.inputSchema.type).toBe('object');
            expect(tool?.inputSchema.properties).toHaveProperty('session_id');
            expect(tool?.inputSchema.required).toContain('session_id');
        });
        it('should document round-oriented verification lookup guidance', () => {
            expect(tool?.description).toContain('PARENT ideation session_id');
            expect(tool?.description).toContain('backend remaps it to the parent ideation session automatically');
            expect(tool?.description).toContain('prefer get_verification_round_artifacts');
            expect(tool?.description).toContain('get_team_artifacts({"session_id":"<parent-session>"})');
            expect((tool?.inputSchema).examples?.[0]).toMatchObject({
                session_id: 'parent-session-id',
            });
        });
    });
    describe('update_plan_verification', () => {
        const tool = PLAN_TOOLS.find((t) => t.name === 'update_plan_verification');
        it('should document parent-session targeting and terminal usage', () => {
            expect(tool).toBeDefined();
            expect(tool?.description).toContain('PARENT ideation session_id');
            expect(tool?.description).toContain('backend remaps it automatically');
            expect(tool?.description).toContain("status='reviewing'");
            expect(tool?.description).toContain("External sessions cannot use status='skipped'");
            expect(tool?.description).toContain('Example reviewing payload');
            expect(tool?.description).toContain('Example terminal payload');
        });
        it('should document generation and child-session constraints in schema descriptions', () => {
            const sessionId = tool?.inputSchema.properties?.session_id;
            const status = tool?.inputSchema.properties?.status;
            const generation = tool?.inputSchema.properties?.generation;
            expect(sessionId.description).toContain('auto-remapped');
            expect(status.description).toContain('Use reviewing for in-progress rounds');
            expect(generation.description).toContain('Pass on every verifier call');
            expect((tool?.inputSchema).examples?.[0]).toMatchObject({
                session_id: 'parent-session-id',
                status: 'reviewing',
                in_progress: true,
            });
            expect((tool?.inputSchema).examples?.[1]).toMatchObject({
                status: 'verified',
                in_progress: false,
                convergence_reason: 'zero_blocking',
            });
        });
    });
    describe('report_verification_round', () => {
        const tool = PLAN_TOOLS.find((t) => t.name === 'report_verification_round');
        it('should expose the verifier-friendly round helper with fixed semantics', () => {
            expect(tool).toBeDefined();
            expect(tool?.description).toContain('Verifier-friendly helper');
            expect(tool?.description).toContain('backend remaps it automatically');
            expect(tool?.description).toContain('status fixed to reviewing');
            expect(tool?.description).toContain('in_progress fixed to true');
            expect((tool?.inputSchema).examples?.[0]).toMatchObject({
                session_id: 'parent-session-id',
                round: 1,
                generation: 3,
            });
            expect(tool?.inputSchema.required).toEqual(['session_id', 'round', 'generation']);
        });
    });
    describe('complete_plan_verification', () => {
        const tool = PLAN_TOOLS.find((t) => t.name === 'complete_plan_verification');
        it('should expose the verifier-friendly terminal helper with fixed semantics', () => {
            expect(tool).toBeDefined();
            expect(tool?.description).toContain('Verifier-friendly helper');
            expect(tool?.description).toContain('backend remaps it automatically');
            expect(tool?.description).toContain('in_progress fixed to false');
            expect(tool?.description).toContain("External sessions cannot use status='skipped'");
            expect((tool?.inputSchema).examples?.[0]).toMatchObject({
                session_id: 'parent-session-id',
                status: 'verified',
                convergence_reason: 'zero_blocking',
            });
            expect((tool?.inputSchema).examples?.[1]).toMatchObject({
                status: 'reviewing',
                convergence_reason: 'agent_error',
            });
            expect(tool?.inputSchema.required).toEqual(['session_id', 'status', 'generation']);
        });
    });
    describe('get_verification_round_artifacts', () => {
        const tool = allTools.find((t) => t.name === 'get_verification_round_artifacts');
        it('should expose the verifier artifact collection helper', () => {
            expect(tool).toBeDefined();
            expect(tool?.description).toContain('Verifier-oriented helper');
            expect(tool?.description).toContain('attach full artifact content');
            expect((tool?.inputSchema).examples?.[0]).toMatchObject({
                session_id: 'parent-session-id',
                prefixes: ['Completeness: ', 'Feasibility: '],
                include_full_content: true,
            });
            expect(tool?.inputSchema.required).toEqual(['session_id', 'prefixes']);
        });
    });
    describe('get_child_session_status', () => {
        const tool = allTools.find((t) => t.name === 'get_child_session_status');
        it('should document verifier debugging guidance and example payload', () => {
            expect(tool).toBeDefined();
            expect(tool?.description).toContain('include_recent_messages=true');
            expect(tool?.description).toContain('last assistant/tool outputs');
            expect((tool?.inputSchema).examples?.[0]).toMatchObject({
                session_id: 'verification-child-session-id',
                include_recent_messages: true,
                message_limit: 10,
            });
        });
    });
    describe('send_ideation_session_message', () => {
        const tool = allTools.find((t) => t.name === 'send_ideation_session_message');
        it('should document full-context verifier nudges and example payload', () => {
            expect(tool).toBeDefined();
            expect(tool?.description).toContain('repeat the full invariant context');
            expect(tool?.description).toContain('SESSION_ID, ROUND, expected artifact prefix/schema');
            expect((tool?.inputSchema).examples?.[0]).toMatchObject({
                session_id: 'verification-child-session-id',
            });
            expect((tool?.inputSchema).examples?.[0]?.message).toContain('SESSION_ID');
            expect((tool?.inputSchema).examples?.[0]?.message).toContain('ROUND: 2');
            expect((tool?.inputSchema).examples?.[0]?.message).toContain('TeamResearch artifact');
        });
    });
    describe('get_team_session_state', () => {
        const tool = allTools.find((t) => t.name === 'get_team_session_state');
        it('should exist in ALL_TOOLS', () => {
            expect(tool).toBeDefined();
        });
        it('should have correct inputSchema with required fields', () => {
            expect(tool?.inputSchema).toBeDefined();
            expect(tool?.inputSchema.type).toBe('object');
            expect(tool?.inputSchema.properties).toHaveProperty('session_id');
            expect(tool?.inputSchema.required).toContain('session_id');
        });
    });
    describe('save_team_session_state', () => {
        const tool = allTools.find((t) => t.name === 'save_team_session_state');
        it('should exist in ALL_TOOLS', () => {
            expect(tool).toBeDefined();
        });
        it('should have correct inputSchema with required fields', () => {
            expect(tool?.inputSchema).toBeDefined();
            expect(tool?.inputSchema.type).toBe('object');
            expect(tool?.inputSchema.properties).toHaveProperty('session_id');
            expect(tool?.inputSchema.properties).toHaveProperty('team_composition');
            expect(tool?.inputSchema.properties).toHaveProperty('phase');
            expect(tool?.inputSchema.required).toContain('session_id');
            expect(tool?.inputSchema.required).toContain('team_composition');
            expect(tool?.inputSchema.required).toContain('phase');
        });
        it('should have team_composition array with correct item schema', () => {
            const teamComp = tool?.inputSchema.properties?.team_composition;
            expect(teamComp).toBeDefined();
            expect(teamComp.type).toBe('array');
            expect(teamComp.items).toBeDefined();
            expect(teamComp.items.type).toBe('object');
            expect(teamComp.items.properties).toHaveProperty('name');
            expect(teamComp.items.properties).toHaveProperty('role');
            expect(teamComp.items.properties).toHaveProperty('prompt');
            expect(teamComp.items.properties).toHaveProperty('model');
            expect(teamComp.items.required).toContain('name');
            expect(teamComp.items.required).toContain('role');
            expect(teamComp.items.required).toContain('prompt');
            expect(teamComp.items.required).toContain('model');
        });
    });
});
describe('Tool allowlist for new agent types', () => {
    it('ralphx-ideation-team-lead should have all team coordination tools', () => {
        const allowlist = TOOL_ALLOWLIST[IDEATION_TEAM_LEAD];
        expect(allowlist).toContain('request_team_plan');
        expect(allowlist).toContain('request_teammate_spawn');
        expect(allowlist).toContain('create_team_artifact');
        expect(allowlist).toContain('get_team_artifacts');
        expect(allowlist).toContain('get_team_session_state');
        expect(allowlist).toContain('save_team_session_state');
    });
    it('ideation-team-member should have limited team tools', () => {
        const allowlist = TOOL_ALLOWLIST[IDEATION_TEAM_MEMBER];
        // Should have artifact tools
        expect(allowlist).toContain('create_team_artifact');
        expect(allowlist).toContain('get_team_artifacts');
        // Should NOT have lead-only tools
        expect(allowlist).not.toContain('request_team_plan');
        expect(allowlist).not.toContain('request_teammate_spawn');
        expect(allowlist).not.toContain('save_team_session_state');
    });
    it('worker-team-member should have artifact tools', () => {
        const allowlist = TOOL_ALLOWLIST[WORKER_TEAM_MEMBER];
        // Should have artifact tools (document decisions)
        expect(allowlist).toContain('create_team_artifact');
        expect(allowlist).toContain('get_team_artifacts');
        // Should NOT have lead-only tools
        expect(allowlist).not.toContain('request_team_plan');
        expect(allowlist).not.toContain('request_teammate_spawn');
        expect(allowlist).not.toContain('save_team_session_state');
    });
});
// ===========================================================================
// TDD tests for --allowed-tools CLI arg parsing (Wave 1)
// These tests FAIL until Wave 2 implementation is complete.
// ===========================================================================
describe('parseAllowedToolsFromArgs', () => {
    let originalArgv;
    beforeEach(() => {
        originalArgv = [...process.argv];
        // Start clean — no --allowed-tools arg
        process.argv = process.argv.filter((a) => !a.startsWith('--allowed-tools'));
    });
    afterEach(() => {
        process.argv = originalArgv;
    });
    it('returns ["tool1", "tool2"] when --allowed-tools=tool1,tool2', () => {
        process.argv = [...process.argv, '--allowed-tools=tool1,tool2'];
        const result = parseAllowedToolsFromArgs();
        expect(result).toEqual(['tool1', 'tool2']);
    });
    it('returns [] when --allowed-tools=__NONE__ (explicit empty sentinel)', () => {
        process.argv = [...process.argv, '--allowed-tools=__NONE__'];
        const result = parseAllowedToolsFromArgs();
        expect(result).toEqual([]);
    });
    it('returns undefined when --allowed-tools= (empty value falls through)', () => {
        process.argv = [...process.argv, '--allowed-tools='];
        const result = parseAllowedToolsFromArgs();
        expect(result).toBeUndefined();
    });
    it('returns undefined when --allowed-tools is absent', () => {
        // argv already cleaned in beforeEach
        const result = parseAllowedToolsFromArgs();
        expect(result).toBeUndefined();
    });
    it('skips invalid tool names (uppercase, spaces) and emits warning', () => {
        const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => { });
        process.argv = [
            ...process.argv,
            '--allowed-tools=valid_tool,INVALID_UPPER,has space',
        ];
        const result = parseAllowedToolsFromArgs();
        expect(result).toContain('valid_tool');
        expect(result).not.toContain('INVALID_UPPER');
        expect(result).not.toContain('has space');
        expect(consoleSpy).toHaveBeenCalledWith(expect.stringContaining('INVALID_UPPER'));
        consoleSpy.mockRestore();
    });
    it('includes unknown tool names (not in ALL_TOOLS) and emits warning', () => {
        const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => { });
        process.argv = [
            ...process.argv,
            '--allowed-tools=get_session_plan,xyz_not_in_registry',
        ];
        const result = parseAllowedToolsFromArgs();
        expect(result).toContain('get_session_plan');
        expect(result).toContain('xyz_not_in_registry'); // included, NOT dropped
        expect(consoleSpy).toHaveBeenCalledWith(expect.stringContaining('xyz_not_in_registry'));
        consoleSpy.mockRestore();
    });
});
describe('getAllowedToolNames - CLI arg priority chain', () => {
    let originalArgv;
    beforeEach(() => {
        originalArgv = [...process.argv];
        delete process.env.RALPHX_ALLOWED_MCP_TOOLS;
        process.argv = process.argv.filter((a) => !a.startsWith('--allowed-tools'));
    });
    afterEach(() => {
        process.argv = originalArgv;
        delete process.env.RALPHX_ALLOWED_MCP_TOOLS;
    });
    it('uses --allowed-tools CLI arg when RALPHX_ALLOWED_MCP_TOOLS env var is not set', () => {
        process.argv = [...process.argv, '--allowed-tools=get_session_plan,create_team_artifact'];
        const tools = getAllowedToolNames();
        expect(tools).toEqual(['get_session_plan', 'create_team_artifact']);
    });
    it('env var takes priority over --allowed-tools CLI arg', () => {
        process.env.RALPHX_ALLOWED_MCP_TOOLS = 'get_session_plan';
        process.argv = [...process.argv, '--allowed-tools=create_team_artifact'];
        const tools = getAllowedToolNames();
        expect(tools).toEqual(['get_session_plan']); // env var wins
        expect(tools).not.toContain('create_team_artifact');
    });
    it('--allowed-tools takes priority over TOOL_ALLOWLIST fallback', () => {
        setAgentType(IDEATION_TEAM_LEAD);
        process.argv = [...process.argv, '--allowed-tools=get_session_plan'];
        const tools = getAllowedToolNames();
        expect(tools).toEqual(['get_session_plan']);
        expect(tools).not.toEqual(TOOL_ALLOWLIST[IDEATION_TEAM_LEAD]);
    });
    it('fallback to TOOL_ALLOWLIST emits deprecation warning when --allowed-tools absent', () => {
        const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => { });
        setAgentType(IDEATION_TEAM_LEAD);
        // No env var, no --allowed-tools in argv
        const tools = getAllowedToolNames();
        expect(tools).toEqual(TOOL_ALLOWLIST[IDEATION_TEAM_LEAD]);
        expect(consoleSpy).toHaveBeenCalledWith(expect.stringContaining('WARN'));
        consoleSpy.mockRestore();
    });
});
// ===========================================================================
// delete_task_proposal MCP tool — alias for archive_task_proposal
// ===========================================================================
describe('delete_task_proposal tool', () => {
    const allTools = getAllTools();
    const tool = allTools.find((t) => t.name === 'delete_task_proposal');
    it('should exist in ALL_TOOLS', () => {
        expect(tool).toBeDefined();
    });
    it('should have correct inputSchema with required proposal_id field', () => {
        expect(tool?.inputSchema).toBeDefined();
        expect(tool?.inputSchema.type).toBe('object');
        expect(tool?.inputSchema.properties).toHaveProperty('proposal_id');
        expect(tool?.inputSchema.required).toContain('proposal_id');
    });
    it('should be in TOOL_ALLOWLIST for ralphx-ideation', () => {
        expect(TOOL_ALLOWLIST[ORCHESTRATOR_IDEATION]).toContain('delete_task_proposal');
    });
    it('should be in TOOL_ALLOWLIST for ralphx-ideation-team-lead', () => {
        expect(TOOL_ALLOWLIST[IDEATION_TEAM_LEAD]).toContain('delete_task_proposal');
    });
    it('should NOT be in TOOL_ALLOWLIST for ralphx-ideation-readonly', () => {
        expect(TOOL_ALLOWLIST[ORCHESTRATOR_IDEATION_READONLY]).not.toContain('delete_task_proposal');
    });
    it('should be returned by getFilteredTools for ralphx-ideation', () => {
        setAgentType(ORCHESTRATOR_IDEATION);
        const toolNames = getFilteredTools().map((t) => t.name);
        expect(toolNames).toContain('delete_task_proposal');
    });
    it('should be returned by getFilteredTools for ralphx-ideation-team-lead', () => {
        setAgentType(IDEATION_TEAM_LEAD);
        const toolNames = getFilteredTools().map((t) => t.name);
        expect(toolNames).toContain('delete_task_proposal');
    });
});
// ===========================================================================
// revert_and_skip MCP tool — tool definition, allowlist, and dispatch coverage
// ===========================================================================
describe('revert_and_skip tool', () => {
    const allTools = getAllTools();
    const tool = allTools.find((t) => t.name === 'revert_and_skip');
    it('should exist in ALL_TOOLS', () => {
        expect(tool).toBeDefined();
    });
    it('should have correct inputSchema with required fields', () => {
        expect(tool?.inputSchema).toBeDefined();
        expect(tool?.inputSchema.type).toBe('object');
        expect(tool?.inputSchema.properties).toHaveProperty('session_id');
        expect(tool?.inputSchema.properties).toHaveProperty('plan_version_to_restore');
        expect(tool?.inputSchema.required).toContain('session_id');
        expect(tool?.inputSchema.required).toContain('plan_version_to_restore');
    });
    it('should be in TOOL_ALLOWLIST for ralphx-ideation', () => {
        expect(TOOL_ALLOWLIST[ORCHESTRATOR_IDEATION]).toContain('revert_and_skip');
    });
    it('should be in TOOL_ALLOWLIST for ralphx-ideation-team-lead', () => {
        expect(TOOL_ALLOWLIST[IDEATION_TEAM_LEAD]).toContain('revert_and_skip');
    });
    it('should NOT be in TOOL_ALLOWLIST for ralphx-ideation-readonly', () => {
        expect(TOOL_ALLOWLIST[ORCHESTRATOR_IDEATION_READONLY]).not.toContain('revert_and_skip');
    });
    it('should be returned by getFilteredTools for ralphx-ideation', () => {
        setAgentType(ORCHESTRATOR_IDEATION);
        const toolNames = getFilteredTools().map((t) => t.name);
        expect(toolNames).toContain('revert_and_skip');
    });
    it('should be returned by getFilteredTools for ralphx-ideation-team-lead', () => {
        setAgentType(IDEATION_TEAM_LEAD);
        const toolNames = getFilteredTools().map((t) => t.name);
        expect(toolNames).toContain('revert_and_skip');
    });
});
// ===========================================================================
// get_acceptance_status + get_pending_confirmations tool definitions + allowlist
// ===========================================================================
describe('acceptance gate tools', () => {
    const allTools = getAllTools();
    describe('get_acceptance_status', () => {
        const tool = allTools.find((t) => t.name === 'get_acceptance_status');
        it('should exist in ALL_TOOLS', () => {
            expect(tool).toBeDefined();
        });
        it('should have correct inputSchema with required session_id', () => {
            expect(tool?.inputSchema).toBeDefined();
            expect(tool?.inputSchema.type).toBe('object');
            expect(tool?.inputSchema.properties).toHaveProperty('session_id');
            expect(tool?.inputSchema.required).toContain('session_id');
        });
        it('should be in TOOL_ALLOWLIST for ralphx-ideation', () => {
            expect(TOOL_ALLOWLIST[ORCHESTRATOR_IDEATION]).toContain('get_acceptance_status');
        });
        it('should be in TOOL_ALLOWLIST for ralphx-ideation-team-lead', () => {
            expect(TOOL_ALLOWLIST[IDEATION_TEAM_LEAD]).toContain('get_acceptance_status');
        });
        it('should NOT be in TOOL_ALLOWLIST for ralphx-ideation-readonly', () => {
            expect(TOOL_ALLOWLIST[ORCHESTRATOR_IDEATION_READONLY]).not.toContain('get_acceptance_status');
        });
        it('should be returned by getFilteredTools for ralphx-ideation', () => {
            setAgentType(ORCHESTRATOR_IDEATION);
            const toolNames = getFilteredTools().map((t) => t.name);
            expect(toolNames).toContain('get_acceptance_status');
        });
        it('should be returned by getFilteredTools for ralphx-ideation-team-lead', () => {
            setAgentType(IDEATION_TEAM_LEAD);
            const toolNames = getFilteredTools().map((t) => t.name);
            expect(toolNames).toContain('get_acceptance_status');
        });
    });
    describe('get_pending_confirmations', () => {
        const tool = allTools.find((t) => t.name === 'get_pending_confirmations');
        it('should exist in ALL_TOOLS', () => {
            expect(tool).toBeDefined();
        });
        it('should have an object inputSchema with no required fields', () => {
            expect(tool?.inputSchema).toBeDefined();
            expect(tool?.inputSchema.type).toBe('object');
            expect(tool?.inputSchema.required).toEqual([]);
        });
        it('should be in TOOL_ALLOWLIST for ralphx-ideation', () => {
            expect(TOOL_ALLOWLIST[ORCHESTRATOR_IDEATION]).toContain('get_pending_confirmations');
        });
        it('should be in TOOL_ALLOWLIST for ralphx-ideation-team-lead', () => {
            expect(TOOL_ALLOWLIST[IDEATION_TEAM_LEAD]).toContain('get_pending_confirmations');
        });
        it('should NOT be in TOOL_ALLOWLIST for ralphx-ideation-readonly', () => {
            expect(TOOL_ALLOWLIST[ORCHESTRATOR_IDEATION_READONLY]).not.toContain('get_pending_confirmations');
        });
        it('should be returned by getFilteredTools for ralphx-ideation', () => {
            setAgentType(ORCHESTRATOR_IDEATION);
            const toolNames = getFilteredTools().map((t) => t.name);
            expect(toolNames).toContain('get_pending_confirmations');
        });
        it('should be returned by getFilteredTools for ralphx-ideation-team-lead', () => {
            setAgentType(IDEATION_TEAM_LEAD);
            const toolNames = getFilteredTools().map((t) => t.name);
            expect(toolNames).toContain('get_pending_confirmations');
        });
    });
});
// ===========================================================================
// RalphX native delegation bridge tools
// ===========================================================================
describe('delegation bridge tools', () => {
    const allTools = getAllTools();
    it.each(['delegate_start', 'delegate_wait', 'delegate_cancel'])('%s should exist in ALL_TOOLS', (toolName) => {
        expect(allTools.find((tool) => tool.name === toolName)).toBeDefined();
    });
    it('delegate_start should expose optional parent_session_id plus required agent_name and prompt', () => {
        const tool = allTools.find((entry) => entry.name === 'delegate_start');
        expect(tool?.inputSchema.type).toBe('object');
        expect(tool?.inputSchema.properties).toHaveProperty('parent_session_id');
        expect(tool?.inputSchema.properties).toHaveProperty('parent_turn_id');
        expect(tool?.inputSchema.properties).toHaveProperty('parent_message_id');
        expect(tool?.inputSchema.properties).toHaveProperty('parent_conversation_id');
        expect(tool?.inputSchema.properties).toHaveProperty('parent_tool_use_id');
        expect(tool?.inputSchema.properties).toHaveProperty('delegated_session_id');
        expect(tool?.inputSchema.required).toEqual(expect.arrayContaining(['agent_name', 'prompt']));
        expect(tool?.inputSchema.required).not.toContain('parent_session_id');
    });
    it('delegate_wait should support delegated-status hydration options', () => {
        const tool = allTools.find((entry) => entry.name === 'delegate_wait');
        expect(tool?.inputSchema.properties).toHaveProperty('include_delegated_status');
        expect(tool?.inputSchema.properties).toHaveProperty('include_child_status');
        expect(tool?.inputSchema.properties).toHaveProperty('include_messages');
        expect(tool?.inputSchema.properties).toHaveProperty('message_limit');
    });
    it.each([ORCHESTRATOR_IDEATION, ORCHESTRATOR_IDEATION_READONLY, PLAN_VERIFIER])('%s should expose delegation bridge tools', (agent) => {
        expect(TOOL_ALLOWLIST[agent]).toContain('delegate_start');
        expect(TOOL_ALLOWLIST[agent]).toContain('delegate_wait');
        expect(TOOL_ALLOWLIST[agent]).toContain('delegate_cancel');
    });
    it.each([WORKER, REVIEWER, MERGER])('%s should expose delegation bridge tools in the fallback allowlist', (agent) => {
        expect(TOOL_ALLOWLIST[agent]).toContain('delegate_start');
        expect(TOOL_ALLOWLIST[agent]).toContain('delegate_wait');
        expect(TOOL_ALLOWLIST[agent]).toContain('delegate_cancel');
    });
    it.each([ORCHESTRATOR_IDEATION, ORCHESTRATOR_IDEATION_READONLY, PLAN_VERIFIER, WORKER, REVIEWER, MERGER])('%s should return delegate_start from getFilteredTools', (agent) => {
        setAgentType(agent);
        const toolNames = getFilteredTools().map((tool) => tool.name);
        expect(toolNames).toContain('delegate_start');
    });
    it('ideation team lead should not receive delegation bridge tools from getFilteredTools', () => {
        setAgentType(IDEATION_TEAM_LEAD);
        const toolNames = getFilteredTools().map((tool) => tool.name);
        expect(toolNames).not.toContain('delegate_start');
        expect(toolNames).not.toContain('delegate_wait');
        expect(toolNames).not.toContain('delegate_cancel');
    });
});
// ===========================================================================
// Specialist / Critic / Advocate TOOL_ALLOWLIST assertions + YAML parity
// ===========================================================================
describe('TOOL_ALLOWLIST specialist entries', () => {
    const artifactSpecialists = [
        IDEATION_SPECIALIST_BACKEND,
        IDEATION_SPECIALIST_FRONTEND,
        IDEATION_SPECIALIST_INFRA,
        IDEATION_SPECIALIST_CODE_QUALITY,
        IDEATION_SPECIALIST_PROMPT_QUALITY,
        IDEATION_SPECIALIST_INTENT,
        IDEATION_SPECIALIST_PIPELINE_SAFETY,
        IDEATION_SPECIALIST_STATE_MACHINE,
    ];
    const parentContextSpecialists = [
        IDEATION_SPECIALIST_BACKEND,
        IDEATION_SPECIALIST_FRONTEND,
        IDEATION_SPECIALIST_INFRA,
    ];
    it.each(artifactSpecialists)('%s should include create_team_artifact', (agent) => {
        expect(TOOL_ALLOWLIST[agent]).toContain('create_team_artifact');
    });
    it.each(artifactSpecialists)('%s should include get_team_artifacts', (agent) => {
        expect(TOOL_ALLOWLIST[agent]).toContain('get_team_artifacts');
    });
    it.each(parentContextSpecialists)('%s should include get_parent_session_context', (agent) => {
        expect(TOOL_ALLOWLIST[agent]).toContain('get_parent_session_context');
    });
    it('IDEATION_TEAM_MEMBER should include get_parent_session_context', () => {
        expect(TOOL_ALLOWLIST[IDEATION_TEAM_MEMBER]).toContain('get_parent_session_context');
    });
    it('IDEATION_CRITIC should include create_team_artifact', () => {
        expect(TOOL_ALLOWLIST[IDEATION_CRITIC]).toContain('create_team_artifact');
    });
    it('IDEATION_CRITIC should include get_team_artifacts', () => {
        expect(TOOL_ALLOWLIST[IDEATION_CRITIC]).toContain('get_team_artifacts');
    });
    it('IDEATION_ADVOCATE should include create_team_artifact', () => {
        expect(TOOL_ALLOWLIST[IDEATION_ADVOCATE]).toContain('create_team_artifact');
    });
    it('IDEATION_ADVOCATE should include get_team_artifacts', () => {
        expect(TOOL_ALLOWLIST[IDEATION_ADVOCATE]).toContain('get_team_artifacts');
    });
    it.each([
        PLAN_CRITIC_COMPLETENESS,
        PLAN_CRITIC_IMPLEMENTATION_FEASIBILITY,
    ])('%s should include create_team_artifact', (agent) => {
        expect(TOOL_ALLOWLIST[agent]).toContain('create_team_artifact');
    });
    it.each([
        PLAN_CRITIC_COMPLETENESS,
        PLAN_CRITIC_IMPLEMENTATION_FEASIBILITY,
    ])('%s should stay bounded to direct read tools', (agent) => {
        expect(TOOL_ALLOWLIST[agent]).not.toContain('get_team_artifacts');
    });
});
//# sourceMappingURL=tools.test.js.map