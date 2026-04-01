/**
 * Handler for v1_get_agent_guide — returns static onboarding guide content.
 * Pure static content: no backend dependency, no state dependency, instant response.
 */

import { GUIDE_SECTIONS, FULL_GUIDE, VALID_SECTIONS } from "./guide-content.js";
import type { GuideSection } from "./guide-content.js";
import type { ApiKeyContext } from "../types.js";

export async function handleGetAgentGuide(
  args: Record<string, unknown>,
  _context: ApiKeyContext
): Promise<string> {
  const section = args.section as string | undefined;

  if (section) {
    if (!VALID_SECTIONS.includes(section as GuideSection)) {
      return JSON.stringify(
        {
          error: "invalid_section",
          valid_sections: VALID_SECTIONS,
          message: `Unknown section "${section}". Valid: ${VALID_SECTIONS.join(", ")}`,
        },
        null,
        2
      );
    }
    return GUIDE_SECTIONS[section as GuideSection];
  }

  return FULL_GUIDE;
}
