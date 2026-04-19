/**
 * RequiredMark — renders a `*` next to a form label with screen-reader
 * "required" semantics. Use instead of red-only borders to indicate that a
 * field must be filled in.
 *
 * Spec: specs/design/color-blind-design.md §4 rule 5 + §9
 */

import type { ComponentPropsWithoutRef } from "react";

export function RequiredMark(props: ComponentPropsWithoutRef<"span">) {
  return (
    <span
      {...props}
      className={[
        "text-[var(--accent-primary)] ml-0.5 select-none",
        props.className ?? "",
      ]
        .join(" ")
        .trim()}
      aria-label="required"
    >
      *
    </span>
  );
}
