/**
 * PermissionsBitmask - Toggle read/write/admin permissions for API keys.
 *
 * Permissions are stored as integer bitmask: 1=read, 2=write, 4=admin.
 * Renders three toggle pills. Admin implies read+write.
 */

import { PERM_READ, PERM_WRITE, PERM_ADMIN, PERM_CREATE_PROJECT, hasPermission } from "@/types/api-key";

// ============================================================================
// Props
// ============================================================================

export interface PermissionsBitmaskProps {
  value: number;
  onChange: (value: number) => void;
  disabled?: boolean;
  /** Read-only display mode — no toggle interaction */
  readOnly?: boolean;
}

// ============================================================================
// Constants
// ============================================================================

const PERMISSION_BITS = [
  { bit: PERM_READ, label: "Read", description: "List projects & task status" },
  { bit: PERM_WRITE, label: "Write", description: "Create tasks & send messages" },
  { bit: PERM_ADMIN, label: "Admin", description: "Manage keys & settings" },
  { bit: PERM_CREATE_PROJECT, label: "Create Project", description: "Register new projects via external API" },
] as const;

// ============================================================================
// Component
// ============================================================================

export function PermissionsBitmask({
  value,
  onChange,
  disabled = false,
  readOnly = false,
}: PermissionsBitmaskProps) {
  const handleToggle = (bit: number) => {
    if (disabled || readOnly) return;
    onChange(value ^ bit);
  };

  return (
    <div
      className="flex flex-wrap gap-1.5"
      data-testid="permissions-bitmask"
    >
      {PERMISSION_BITS.map(({ bit, label, description }) => {
        const active = hasPermission(value, bit);
        const interactive = !disabled && !readOnly;
        return (
          <button
            key={bit}
            type="button"
            title={description}
            onClick={() => handleToggle(bit)}
            disabled={disabled}
            data-testid={`perm-toggle-${label.toLowerCase()}`}
            className={[
              "px-2.5 py-1 rounded-md text-xs font-medium transition-colors select-none",
              interactive ? "cursor-pointer" : "cursor-default",
              active
                ? "bg-[var(--accent-muted)] text-[var(--accent-primary)] border border-[rgba(255,107,53,0.3)]"
                : "bg-[var(--bg-surface)] text-[var(--text-muted)] border border-[var(--border-subtle)]",
              disabled ? "opacity-50" : "",
            ].join(" ")}
          >
            {label}
          </button>
        );
      })}
    </div>
  );
}
