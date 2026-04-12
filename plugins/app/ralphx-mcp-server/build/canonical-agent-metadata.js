import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { parse as parseYaml } from "yaml";
const canonicalAgentDefinitionCache = new Map();
export function resolveRepoRoot() {
    let current = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../../../");
    while (!fs.existsSync(path.join(current, "agents"))) {
        const parent = path.dirname(current);
        if (parent === current) {
            break;
        }
        current = parent;
    }
    return current;
}
export function canonicalAgentName(agentType) {
    const shortName = agentType.startsWith("ralphx:") ? agentType.slice("ralphx:".length) : agentType;
    switch (shortName) {
        case "orchestrator-ideation":
            return "ralphx-ideation";
        case "orchestrator-ideation-readonly":
            return "ralphx-ideation-readonly";
        case "ideation-team-lead":
            return "ralphx-ideation-team-lead";
        case "ideation-advocate":
            return "ralphx-ideation-advocate";
        case "ideation-critic":
            return "ralphx-ideation-critic";
        case "ideation-specialist-backend":
            return "ralphx-ideation-specialist-backend";
        case "ideation-specialist-code-quality":
            return "ralphx-ideation-specialist-code-quality";
        case "ideation-specialist-frontend":
            return "ralphx-ideation-specialist-frontend";
        case "ideation-specialist-infra":
            return "ralphx-ideation-specialist-infra";
        case "ideation-specialist-intent":
            return "ralphx-ideation-specialist-intent";
        case "ideation-specialist-pipeline-safety":
            return "ralphx-ideation-specialist-pipeline-safety";
        case "ideation-specialist-prompt-quality":
            return "ralphx-ideation-specialist-prompt-quality";
        case "ideation-specialist-state-machine":
            return "ralphx-ideation-specialist-state-machine";
        case "ideation-specialist-ux":
            return "ralphx-ideation-specialist-ux";
        case "plan-verifier":
            return "ralphx-plan-verifier";
        case "plan-critic-completeness":
            return "ralphx-plan-critic-completeness";
        case "plan-critic-implementation-feasibility":
            return "ralphx-plan-critic-implementation-feasibility";
        case "chat-task":
            return "ralphx-chat-task";
        case "chat-project":
            return "ralphx-chat-project";
        case "ralphx-worker-team":
            return "ralphx-execution-team-lead";
        case "ralphx-worker":
            return "ralphx-execution-worker";
        case "ralphx-coder":
            return "ralphx-execution-coder";
        case "ralphx-reviewer":
            return "ralphx-execution-reviewer";
        case "ralphx-merger":
            return "ralphx-execution-merger";
        case "ralphx-orchestrator":
            return "ralphx-execution-orchestrator";
        case "ralphx-supervisor":
            return "ralphx-execution-supervisor";
        case "ralphx-deep-researcher":
            return "ralphx-research-deep-researcher";
        case "project-analyzer":
            return "ralphx-project-analyzer";
        case "memory-capture":
            return "ralphx-memory-capture";
        case "memory-maintainer":
            return "ralphx-memory-maintainer";
        case "session-namer":
            return "ralphx-utility-session-namer";
        case "qa-prep":
            return "ralphx-qa-prep";
        case "qa-tester":
            return "ralphx-qa-executor";
        case "supervisor":
            return "ralphx-execution-supervisor";
        default:
            return shortName;
    }
}
export function clearCanonicalAgentDefinitionCache() {
    canonicalAgentDefinitionCache.clear();
}
export function loadCanonicalAgentDefinition(agentType) {
    const canonicalName = canonicalAgentName(agentType);
    if (canonicalAgentDefinitionCache.has(canonicalName)) {
        return canonicalAgentDefinitionCache.get(canonicalName) ?? null;
    }
    const definitionPath = path.join(resolveRepoRoot(), "agents", canonicalName, "agent.yaml");
    try {
        const raw = fs.readFileSync(definitionPath, "utf8");
        const parsed = parseYaml(raw);
        const definition = parsed && parsed.name === canonicalName ? parsed : null;
        canonicalAgentDefinitionCache.set(canonicalName, definition);
        return definition;
    }
    catch {
        canonicalAgentDefinitionCache.set(canonicalName, null);
        return null;
    }
}
export function loadCanonicalMcpTools(agentType) {
    const definition = loadCanonicalAgentDefinition(agentType);
    const tools = definition?.capabilities?.mcp_tools;
    return tools && tools.length > 0 ? [...tools] : undefined;
}
//# sourceMappingURL=canonical-agent-metadata.js.map