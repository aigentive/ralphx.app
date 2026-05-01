/**
 * LeftNavRail — narrow vertical app navigation.
 *
 * Hosts the same primary views as the legacy top-bar Navigation
 * (Agents, Ideation, Graph, Kanban, Insights, plus feature-flagged items)
 * and the Settings entry, in a compact icon-and-label rail.
 */

import { SlidersHorizontal, Users } from "lucide-react";
import ralphxLogo from "@/assets/ralphx-logo.png";
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";
import { useFeatureFlags } from "@/hooks/useFeatureFlags";
import { useProjectStats } from "@/hooks/useProjectStats";
import { useProjectStore } from "@/stores/projectStore";
import {
  selectHasAnyActiveTeam,
  selectTotalTeammateCount,
  useTeamStore,
} from "@/stores/teamStore";
import { ALL_NAV_ITEMS } from "./nav-items";
import type { ViewType } from "@/types/chat";

export const LEFT_NAV_RAIL_WIDTH = 72;

interface LeftNavRailProps {
  currentView: ViewType;
  onViewChange: (view: ViewType) => void;
  onOpenSettings?: () => void;
  /** Hide primary view items (e.g. during welcome screen). Settings stays. */
  hideViews?: boolean;
}

interface RailItemProps {
  view?: ViewType;
  label: string;
  icon: React.ElementType;
  shortcut?: string | undefined;
  isActive: boolean;
  onClick: () => void;
  testId?: string;
}

function RailItem({
  label,
  icon: Icon,
  shortcut,
  isActive,
  onClick,
  testId,
}: RailItemProps) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button
          variant="ghost"
          size="sm"
          onClick={onClick}
          aria-label={label}
          aria-current={isActive ? "page" : undefined}
          data-testid={testId}
          className={cn(
            "flex flex-col items-center justify-center gap-1 w-14 h-14 px-1 rounded-lg",
            "transition-all duration-150 active:scale-[0.98] outline-none ring-0 focus:outline-none focus-visible:outline-none"
          )}
          style={{
            background: isActive ? "var(--accent-muted)" : "transparent",
            border: isActive
              ? "1px solid var(--accent-border)"
              : "1px solid transparent",
            color: isActive ? "var(--accent-primary)" : "var(--text-muted)",
          }}
        >
          <Icon className="w-[18px] h-[18px] flex-shrink-0" />
          <span className="text-[10px] font-medium leading-none tracking-tight">
            {label}
          </span>
        </Button>
      </TooltipTrigger>
      <TooltipContent side="right" className="text-xs">
        {label}
        {shortcut && <kbd className="ml-1 opacity-70">{shortcut}</kbd>}
      </TooltipContent>
    </Tooltip>
  );
}

export function LeftNavRail({
  currentView,
  onViewChange,
  onOpenSettings,
  hideViews = false,
}: LeftNavRailProps) {
  const hasActiveTeam = useTeamStore(selectHasAnyActiveTeam);
  const teammateCount = useTeamStore(selectTotalTeammateCount);
  const activeProjectId = useProjectStore((s) => s.activeProjectId);
  const { data: stats } = useProjectStats(activeProjectId ?? undefined);
  const { data: featureFlags } = useFeatureFlags();

  const taskCount = stats?.taskCount ?? 0;
  const visibleItems = hideViews
    ? []
    : ALL_NAV_ITEMS.filter((item) => item.visible(featureFlags, taskCount));

  return (
    <aside
      className="flex flex-col items-center justify-between py-3 shrink-0 border-r overflow-hidden"
      style={{
        width: LEFT_NAV_RAIL_WIDTH,
        background: "var(--nav-rail-bg)",
        borderColor: "var(--border-subtle)",
        WebkitAppRegion: "no-drag",
      } as React.CSSProperties}
      role="navigation"
      aria-label="Primary"
      data-testid="left-nav-rail"
    >
      <div className="flex flex-col items-center gap-3">
        <div
          className="flex flex-col items-center gap-1 select-none"
          data-testid="left-nav-brand"
        >
          <img
            src={ralphxLogo}
            alt=""
            aria-hidden="true"
            className="h-9 w-9 rounded-md object-contain"
          />
          <span
            className="text-[10px] font-semibold tracking-tight leading-none"
            style={{ color: "var(--text-primary)" }}
          >
            RalphX
          </span>
        </div>

        <nav className="flex flex-col items-center gap-1">
        {visibleItems.map(({ view, label, icon, shortcut }) => (
          <RailItem
            key={view}
            view={view}
            label={label}
            icon={icon}
            shortcut={shortcut}
            isActive={currentView === view}
            onClick={() => onViewChange(view)}
            testId={`nav-${view}`}
          />
          ))}
        </nav>
      </div>

      <div className="flex flex-col items-center gap-2">
        {hasActiveTeam && !hideViews && (
          <Tooltip>
            <TooltipTrigger asChild>
              <div
                className="flex items-center gap-1 h-7 px-2 rounded-full"
                style={{
                  background: "var(--accent-muted)",
                  border: "1px solid var(--accent-border)",
                }}
                data-testid="left-nav-team-indicator"
              >
                <Users
                  className="w-3 h-3"
                  style={{ color: "var(--accent-primary)" }}
                />
                <span
                  className="text-[10px] font-semibold leading-none"
                  style={{ color: "var(--accent-primary)" }}
                >
                  {teammateCount}
                </span>
              </div>
            </TooltipTrigger>
            <TooltipContent side="right" className="text-xs">
              {teammateCount} active teammate{teammateCount === 1 ? "" : "s"}
            </TooltipContent>
          </Tooltip>
        )}

        <RailItem
          label="Settings"
          icon={SlidersHorizontal}
          shortcut="⌘,"
          isActive={false}
          onClick={() => onOpenSettings?.()}
          testId="nav-settings"
        />
      </div>
    </aside>
  );
}
