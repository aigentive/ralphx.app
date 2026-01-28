/**
 * Shared components and utilities for detail-views
 *
 * Extracted common components used across multiple state-specific
 * task detail views to reduce duplication.
 */

import type { ReactNode } from "react";

/**
 * SectionTitle - Consistent section header styling for task detail views
 */
export function SectionTitle({ children }: { children: ReactNode }) {
  return (
    <h3 className="text-[13px] font-medium mb-2.5 text-white/80">{children}</h3>
  );
}
