/**
 * Handler for v1_get_agent_guide — returns static onboarding guide content.
 * Pure static content: no backend dependency, no state dependency, instant response.
 */
import { GUIDE_SECTIONS, FULL_GUIDE, VALID_SECTIONS } from "./guide-content.js";
export async function handleGetAgentGuide(args, _context) {
    const section = args.section;
    if (section) {
        if (!VALID_SECTIONS.includes(section)) {
            return JSON.stringify({
                error: "invalid_section",
                valid_sections: VALID_SECTIONS,
                message: `Unknown section "${section}". Valid: ${VALID_SECTIONS.join(", ")}`,
            }, null, 2);
        }
        return GUIDE_SECTIONS[section];
    }
    return FULL_GUIDE;
}
//# sourceMappingURL=guide.js.map