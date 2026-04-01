/**
 * SectionTitle - macOS Tahoe-inspired section header
 *
 * Features Apple's characteristic spacing and typography hierarchy
 * with SF Pro Display characteristics.
 */

import type { ReactNode } from "react";

interface SectionTitleProps {
  children: ReactNode;
  muted?: boolean;
}

export function SectionTitle({ children, muted = false }: SectionTitleProps) {
  return (
    <h3
      className="text-[11px] font-semibold uppercase tracking-wider mb-3"
      style={{
        color: muted ? "rgba(255,255,255,0.35)" : "rgba(255,255,255,0.5)",
        letterSpacing: "0.08em",
      }}
    >
      {children}
    </h3>
  );
}
