/**
 * SectionTitle - Consistent section header styling for task detail views
 */

import type { ReactNode } from "react";

export function SectionTitle({ children }: { children: ReactNode }) {
  return (
    <h3 className="text-[13px] font-medium mb-2.5 text-white/80">{children}</h3>
  );
}
