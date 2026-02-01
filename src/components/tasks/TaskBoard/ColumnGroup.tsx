/**
 * ColumnGroup - Collapsible group within a kanban column
 *
 * Design: macOS Tahoe (2025)
 * - Clean, minimal section header like Finder
 * - Simple chevron rotation
 * - Subtle left accent when expanded
 */

import { type ReactNode } from "react";
import { ChevronDown } from "lucide-react";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import { cn } from "@/lib/utils";

interface ColumnGroupProps {
  /** Group label (e.g., 'Fresh Tasks', 'Needs Revision') */
  label: string;
  /** Number of tasks in this group */
  count: number;
  /** Optional icon to display next to the label */
  icon?: ReactNode;
  /** Accent color for the left border when expanded (CSS color value) */
  accentColor?: string;
  /** Whether the group is collapsed */
  collapsed?: boolean;
  /** Callback when collapse state changes */
  onToggle?: () => void;
  /** Group contents (task cards) */
  children: ReactNode;
}

export function ColumnGroup({
  label,
  count,
  icon,
  accentColor,
  collapsed = false,
  onToggle,
  children,
}: ColumnGroupProps) {
  const isExpanded = !collapsed;

  // Build props conditionally to avoid exactOptionalPropertyTypes issues
  const handleOpenChange = onToggle ? () => onToggle() : undefined;

  return (
    <Collapsible
      open={isExpanded}
      {...(handleOpenChange && { onOpenChange: handleOpenChange })}
      className="mt-1 first:mt-0"
    >
      <div
        className="overflow-hidden"
        style={{
          borderLeft: isExpanded && accentColor
            ? `2px solid ${accentColor}`
            : "2px solid transparent",
          paddingLeft: "4px",
        }}
      >
        {/* Group header - simple like Finder section headers */}
        <CollapsibleTrigger asChild>
          <button
            type="button"
            className={cn(
              "w-full flex items-center gap-1.5 px-2 py-1.5 text-left",
              "transition-colors rounded-md",
              "focus:outline-none focus-visible:ring-1 focus-visible:ring-[hsl(14_100%_60%)]/50"
            )}
            style={{
              background: "transparent",
            }}
            onMouseEnter={(e) => {
              e.currentTarget.style.background = "hsla(220 10% 100% / 0.04)";
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.background = "transparent";
            }}
          >
            {/* Chevron indicator */}
            <ChevronDown
              className={cn(
                "w-3 h-3 flex-shrink-0 transition-transform duration-200",
                collapsed && "-rotate-90"
              )}
              style={{ color: "hsl(220 10% 45%)" }}
            />

            {/* Optional icon */}
            {icon && (
              <span className="flex-shrink-0" style={{ color: "hsl(220 10% 50%)" }}>
                {icon}
              </span>
            )}

            {/* Group label - small, gray like Finder */}
            <span
              className="flex-1 truncate"
              style={{
                fontSize: "11px",
                fontWeight: 500,
                color: "hsl(220 10% 50%)",
              }}
            >
              {label}
            </span>

            {/* Count - simple */}
            <span
              style={{
                fontSize: "10px",
                fontWeight: 500,
                color: "hsl(220 10% 40%)",
                fontVariantNumeric: "tabular-nums",
              }}
            >
              {count}
            </span>
          </button>
        </CollapsibleTrigger>

        {/* Group content */}
        <CollapsibleContent className="pt-1">
          <div className="flex flex-col gap-1.5 pl-1">
            {children}
          </div>
        </CollapsibleContent>
      </div>
    </Collapsible>
  );
}
