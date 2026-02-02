/**
 * Navigation - Main view navigation bar
 */

import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import {
  LayoutGrid,
  Network,
  Lightbulb,
  Puzzle,
  Activity,
  SlidersHorizontal,
} from "lucide-react";
import { cn } from "@/lib/utils";
import type { ViewType } from "@/types/chat";

// Navigation items configuration
// Order reflects workflow: plan ideas → visualize dependencies → execute tasks
const NAV_ITEMS: {
  view: ViewType;
  label: string;
  icon: React.ElementType;
  shortcut: string;
}[] = [
  { view: "ideation", label: "Ideation", icon: Lightbulb, shortcut: "⌘1" },
  { view: "graph", label: "Graph", icon: Network, shortcut: "⌘2" },
  { view: "kanban", label: "Kanban", icon: LayoutGrid, shortcut: "⌘3" },
  { view: "extensibility", label: "Extensibility", icon: Puzzle, shortcut: "⌘4" },
  { view: "activity", label: "Activity", icon: Activity, shortcut: "⌘5" },
  { view: "settings", label: "Settings", icon: SlidersHorizontal, shortcut: "⌘6" },
];

interface NavigationProps {
  currentView: ViewType;
  onViewChange: (view: ViewType) => void;
}

export function Navigation({ currentView, onViewChange }: NavigationProps) {
  return (
    <nav
      className="flex items-center gap-1"
      role="navigation"
      aria-label="Main views"
      style={{ WebkitAppRegion: "no-drag" } as React.CSSProperties}
    >
      {NAV_ITEMS.map(({ view, label, icon: Icon, shortcut }) => {
        const isActive = currentView === view;
        return (
          <Tooltip key={view}>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => onViewChange(view)}
                className={cn(
                  "gap-2 h-8 transition-all duration-150 active:scale-[0.98]",
                  isActive ? "px-3" : "px-2 xl:px-3"
                )}
                style={{
                  background: isActive
                    ? "hsla(14 100% 60% / 0.1)"
                    : "transparent",
                  border: isActive ? "1px solid hsla(14 100% 60% / 0.15)" : "1px solid transparent",
                  color: isActive ? "hsl(14 100% 60%)" : "hsl(220 10% 55%)",
                }}
                data-testid={`nav-${view}`}
                aria-current={isActive ? "page" : undefined}
              >
                <Icon className="w-[18px] h-[18px] flex-shrink-0" />
                <span className={cn(
                  "text-sm font-medium whitespace-nowrap",
                  isActive ? "inline" : "hidden xl:inline"
                )}>
                  {label}
                </span>
              </Button>
            </TooltipTrigger>
            <TooltipContent side="bottom" className="text-xs">
              {label} <kbd className="ml-1 opacity-70">{shortcut}</kbd>
            </TooltipContent>
          </Tooltip>
        );
      })}
    </nav>
  );
}
