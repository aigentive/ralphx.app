/**
 * TeamFindingsSection - Collapsible team research findings display
 *
 * Shows specialist findings from team-ideated plans in a table format.
 * macOS Tahoe glass-morphism, warm orange accent.
 */

import { useState } from "react";
import { Users, ChevronDown } from "lucide-react";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import { cn } from "@/lib/utils";

// ============================================================================
// Types
// ============================================================================

export interface TeamFinding {
  specialist: string;
  keyFinding: string;
  color?: string;
}

export interface TeamFindingsSectionProps {
  findings: TeamFinding[];
  teamMode: "research" | "debate";
  teammateCount: number;
  defaultExpanded?: boolean;
}

// ============================================================================
// Component
// ============================================================================

export function TeamFindingsSection({
  findings,
  teamMode,
  teammateCount,
  defaultExpanded = false,
}: TeamFindingsSectionProps) {
  const [isOpen, setIsOpen] = useState(defaultExpanded);

  if (findings.length === 0) return null;

  const title = teamMode === "research" ? "Team Research Summary" : "Team Debate Summary";

  return (
    <Collapsible open={isOpen} onOpenChange={setIsOpen}>
      <div
        className="rounded-lg mb-4"
        style={{
          background: "hsla(220 10% 100% / 0.02)",
          border: "1px solid hsla(220 10% 100% / 0.06)",
        }}
      >
        <CollapsibleTrigger asChild>
          <button
            className="flex items-center gap-2.5 w-full px-3 py-2.5 text-left"
            aria-label={title}
          >
            <Users
              className="w-3.5 h-3.5 flex-shrink-0"
              style={{ color: "hsl(14 100% 60%)" }}
            />
            <span
              className="text-[12px] font-medium flex-1"
              style={{ color: "hsl(220 10% 80%)" }}
            >
              {title}
            </span>
            <span
              className="text-[10px] font-medium w-5 h-5 rounded-full flex items-center justify-center flex-shrink-0"
              style={{
                background: "hsla(14 100% 60% / 0.15)",
                color: "hsl(14 100% 60%)",
              }}
            >
              {teammateCount}
            </span>
            <ChevronDown
              className={cn(
                "w-3.5 h-3.5 transition-transform duration-200 flex-shrink-0",
                !isOpen && "-rotate-90"
              )}
              style={{ color: "hsl(220 10% 50%)" }}
            />
          </button>
        </CollapsibleTrigger>

        <CollapsibleContent>
          <div className="px-3 pb-3">
            <div
              className="overflow-hidden rounded-md"
              style={{ border: "1px solid hsla(220 10% 100% / 0.06)" }}
            >
              <table className="w-full text-[12px]">
                <thead>
                  <tr style={{ background: "hsla(220 10% 100% / 0.02)" }}>
                    <th
                      className="px-3 py-2 text-left text-[10px] font-medium uppercase tracking-wider"
                      style={{ color: "hsl(220 10% 50%)" }}
                    >
                      Specialist
                    </th>
                    <th
                      className="px-3 py-2 text-left text-[10px] font-medium uppercase tracking-wider"
                      style={{ color: "hsl(220 10% 50%)" }}
                    >
                      Key Finding
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {findings.map((finding) => (
                    <tr
                      key={finding.specialist}
                      className="border-t"
                      style={{ borderColor: "hsla(220 10% 100% / 0.06)" }}
                    >
                      <td className="px-3 py-2 whitespace-nowrap">
                        <span className="flex items-center gap-2">
                          {finding.color && (
                            <span
                              className="w-2 h-2 rounded-full flex-shrink-0"
                              style={{ background: finding.color }}
                            />
                          )}
                          <span style={{ color: "hsl(220 10% 85%)" }}>
                            {finding.specialist}
                          </span>
                        </span>
                      </td>
                      <td
                        className="px-3 py-2"
                        style={{ color: "hsl(220 10% 65%)" }}
                      >
                        {finding.keyFinding}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        </CollapsibleContent>
      </div>
    </Collapsible>
  );
}
