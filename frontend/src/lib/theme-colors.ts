/**
 * theme-colors — helpers for composing alpha variants on top of design-token
 * colors inside component code.
 *
 * When you need a translucent color that doesn't map to an existing
 * `--status-*-muted / -border / -strong` token, use `withAlpha()` to compose
 * the alpha channel via `color-mix()`. This keeps components off primitives
 * and theme-flippable.
 *
 * Use sparingly — prefer the discrete tokens (`--accent-muted`,
 * `--status-success-border`, etc.) where the intent matches. Template-literal
 * hex concatenation (`"#ff6b3550"`) is the anti-pattern this replaces.
 *
 * Spec: specs/design/styleguide.md §0, §11
 */

/**
 * Compose a translucent color from a CSS variable token.
 *
 * @param token CSS variable reference — e.g. `"var(--status-success)"` or
 *              `"var(--accent-primary)"`. Must be a design-system token; do
 *              NOT pass a raw hex/rgba string.
 * @param alpha Opacity percentage, 0–100. Typical values: 10, 20, 30, 50, 80.
 * @returns A `color-mix()` CSS string suitable for `background`, `border`,
 *          `color`, etc. Resolves at render time against the active theme.
 *
 * @example
 *   style={{ backgroundColor: withAlpha("var(--status-success)", 15) }}
 *   // → color-mix(in srgb, var(--status-success) 15%, transparent)
 */
export function withAlpha(token: string, alpha: number): string {
  const pct = Math.max(0, Math.min(100, Math.round(alpha)));
  return `color-mix(in srgb, ${token} ${pct}%, transparent)`;
}

/**
 * Token reference for the four canonical status colors plus the brand accent.
 * Use as a discriminated type when a component needs to map a status string
 * (e.g. `"success" | "warning" | "error" | "info"`) to a token.
 */
export const STATUS_TOKEN_REFS = {
  success: "var(--status-success)",
  warning: "var(--status-warning)",
  error: "var(--status-error)",
  info: "var(--status-info)",
  accent: "var(--accent-primary)",
} as const;

export type StatusTokenKey = keyof typeof STATUS_TOKEN_REFS;

/**
 * Resolve a status key + alpha percentage to a theme-flippable translucent
 * color string. Replaces the `${config.color}${alphaHex}` pattern in
 * STATUS_CONFIG / PHASE_TYPE_COLORS dicts.
 *
 * @example
 *   // before: backgroundColor: `${statusHex}33`
 *   // after:  backgroundColor: statusTint(status, 20)
 */
export function statusTint(status: StatusTokenKey, alpha: number): string {
  return withAlpha(STATUS_TOKEN_REFS[status], alpha);
}
