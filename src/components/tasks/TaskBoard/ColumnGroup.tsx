/**
 * ColumnGroup - Collapsible group within a kanban column
 *
 * Design: macOS Tahoe Liquid Glass
 * - Frosted glass header with backdrop-blur
 * - Chevron rotation on collapse
 * - Accent color left border when expanded
 * - Smooth transitions
 */

import { type ReactNode } from "react";
import { ChevronDown } from "lucide-react";
import { Badge } from "@/components/ui/badge";
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
    >
      <div
        className="rounded-md overflow-hidden transition-all duration-200"
        style={{
          borderLeft: isExpanded && accentColor
            ? `2px solid ${accentColor}`
            : "2px solid transparent",
        }}
      >
        {/* Group header - Liquid Glass style */}
        <CollapsibleTrigger asChild>
          <button
            type="button"
            className={cn(
              "w-full flex items-center gap-2 px-2 py-1.5 text-left",
              "rounded-md transition-all duration-150",
              "hover:bg-white/[0.04] focus:outline-none focus-visible:ring-1 focus-visible:ring-white/20"
            )}
            style={{
              background: "rgba(255,255,255,0.02)",
            }}
          >
            {/* Chevron indicator */}
            <ChevronDown
              className={cn(
                "w-3.5 h-3.5 text-white/40 transition-transform duration-200 flex-shrink-0",
                collapsed && "-rotate-90"
              )}
            />

            {/* Optional icon */}
            {icon && (
              <span className="flex-shrink-0 text-white/50">
                {icon}
              </span>
            )}

            {/* Group label */}
            <span className="text-[11px] font-medium text-white/60 flex-1 truncate tracking-tight">
              {label}
            </span>

            {/* Count badge */}
            <Badge
              variant="secondary"
              className="text-[9px] px-1 py-0 min-w-[16px] text-center bg-white/[0.03] text-white/40 border-white/[0.06]"
            >
              {count}
            </Badge>
          </button>
        </CollapsibleTrigger>

        {/* Group content */}
        <CollapsibleContent className="pt-1.5">
          <div className="flex flex-col gap-2 pl-1">
            {children}
          </div>
        </CollapsibleContent>
      </div>
    </Collapsible>
  );
}
