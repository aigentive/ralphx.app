/**
 * Unit tests for MCP tool definitions and authorization logic
 * Tests agent team coordination features
 */
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { getAllowedToolNames, getFilteredTools, isToolAllowed, setAgentType, getAllTools, TOOL_ALLOWLIST, parseAllowedToolsFromArgs, } from '../tools.js';
import { IDEATION_TEAM_LEAD, IDEATION_TEAM_MEMBER, WORKER_TEAM_MEMBER, } from '../agentNames.js';
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
});
describe('getFilteredTools', () => {
    beforeEach(() => {
        delete process.env.RALPHX_ALLOWED_MCP_TOOLS;
    });
    afterEach(() => {
        delete process.env.RALPHX_ALLOWED_MCP_TOOLS;
    });
    it('should return correct tool set for ideation-team-lead', () => {
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
        expect(toolNames).toContain('get_plan_artifact');
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
    it('ideation-team-lead should have all team coordination tools', () => {
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
//# sourceMappingURL=tools.test.js.map