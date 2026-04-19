/**
 * EmptyState — shared layout for "nothing-to-show" screens.
 *
 * Each view has its own personality (lightbulb in Ideation, doc-with-sparkle
 * in Kanban, dashed check in Reviews), so this component standardises the
 * layout rhythm (centred column, icon / title / description / optional CTA)
 * without dictating the glyph itself. Pick a variant to style the icon
 * container; pass the icon as a child.
 *
 * Spec: specs/design/page-by-page-review.md (cross-view pattern #5)
 */

import type { ReactNode } from "react";
import { cn } from "@/lib/utils";

export type EmptyStateVariant =
  | "neutral" // text-muted icon, dashed border — e.g. "no pending reviews"
  | "info" // accent-muted fill, accent icon — e.g. "ready to start"
  | "warning" // warning tint — e.g. "something needs your input"
  | "error"; // error tint — e.g. "something broke"

export interface EmptyStateProps {
  icon: ReactNode;
  title: string;
  description?: string;
  /** Optional CTA (button, link, etc.) rendered below the description. */
  action?: ReactNode;
  variant?: EmptyStateVariant;
  /**
   * Render the icon as-is (skip the default 64×64 rounded container).
   * Use when a view has a bespoke icon treatment (gradient tile, sparkle
   * overlay, animated glyph) that shouldn't be wrapped.
   */
  iconBleed?: boolean;
  /** Pass through test id for targeted e2e selection. */
  "data-testid"?: string;
  className?: string;
}

const ICON_CONTAINER_VARIANT: Record<EmptyStateVariant, string> = {
  neutral: "border-2 border-dashed border-[var(--border-subtle)]",
  info: "bg-[var(--accent-muted)] border border-[var(--accent-border)]",
  warning: "bg-[var(--status-warning-muted)] border border-[var(--status-warning-border)]",
  error: "bg-[var(--status-error-muted)] border border-[var(--status-error-border)]",
};

const ICON_COLOR_VARIANT: Record<EmptyStateVariant, string> = {
  neutral: "text-[var(--text-secondary)]",
  info: "text-[var(--accent-primary)]",
  warning: "text-[var(--status-warning)]",
  error: "text-[var(--status-error)]",
};

export function EmptyState({
  icon,
  title,
  description,
  action,
  variant = "neutral",
  iconBleed = false,
  "data-testid": testId,
  className,
}: EmptyStateProps) {
  return (
    <div
      data-testid={testId}
      className={cn(
        "flex flex-col items-center justify-center px-6 py-12 text-center",
        className,
      )}
    >
      {iconBleed ? (
        <div className="mb-4" aria-hidden>{icon}</div>
      ) : (
        <div
          className={cn(
            "w-16 h-16 mb-4 rounded-xl flex items-center justify-center",
            "[&>svg]:w-8 [&>svg]:h-8",
            ICON_CONTAINER_VARIANT[variant],
            ICON_COLOR_VARIANT[variant],
          )}
          aria-hidden
        >
          {icon}
        </div>
      )}
      <p className="text-sm font-medium text-[var(--text-primary)]">{title}</p>
      {description && (
        <p className="mt-1 text-xs text-[var(--text-muted)]">{description}</p>
      )}
      {action && <div className="mt-4">{action}</div>}
    </div>
  );
}
