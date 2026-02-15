/**
 * TargetSelector — "Send to" dropdown for choosing message recipient
 *
 * Dropdown showing: Lead (default), teammates, All (broadcast).
 * Uses teammate colors as indicators.
 */

import React, { useState, useRef, useEffect, useCallback } from "react";
import { ChevronDown } from "lucide-react";
import type { TeammateState } from "@/stores/teamStore";

export type TargetValue = "lead" | "*" | string;

interface TargetSelectorProps {
  teammates: TeammateState[];
  value: TargetValue;
  onChange: (target: TargetValue) => void;
}

export const TargetSelector = React.memo(function TargetSelector({
  teammates,
  value,
  onChange,
}: TargetSelectorProps) {
  const [isOpen, setIsOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  // Close on outside click
  useEffect(() => {
    if (!isOpen) return;

    const handleClickOutside = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setIsOpen(false);
      }
    };

    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, [isOpen]);

  const handleSelect = useCallback((target: TargetValue) => {
    onChange(target);
    setIsOpen(false);
  }, [onChange]);

  // Find display label for current value
  const displayLabel = value === "lead"
    ? "Lead"
    : value === "*"
      ? "All"
      : value;

  const displayColor = teammates.find((m) => m.name === value)?.color;

  return (
    <div ref={containerRef} className="relative">
      <button
        type="button"
        onClick={() => setIsOpen(!isOpen)}
        className="flex items-center gap-1 px-2 py-1 rounded text-[11px] transition-colors"
        style={{
          backgroundColor: "hsl(220 10% 14%)",
          color: "hsl(220 10% 60%)",
          border: "1px solid hsl(220 10% 18%)",
        }}
      >
        <span className="text-[10px]" style={{ color: "hsl(220 10% 40%)" }}>
          Send to:
        </span>
        {displayColor && (
          <span
            className="w-1.5 h-1.5 rounded-full"
            style={{ backgroundColor: displayColor }}
          />
        )}
        <span>{displayLabel}</span>
        <ChevronDown className="w-3 h-3 opacity-50" />
      </button>

      {isOpen && (
        <div
          className="absolute bottom-full left-0 mb-1 rounded-lg py-1 z-50 min-w-[140px]"
          style={{
            backgroundColor: "hsl(220 10% 12%)",
            border: "1px solid hsl(220 10% 18%)",
            boxShadow: "0 4px 12px hsla(220 20% 0% / 0.5)",
          }}
        >
          {/* Lead */}
          <DropdownItem
            label="Lead"
            isSelected={value === "lead"}
            onClick={() => handleSelect("lead")}
          />
          {/* Teammates */}
          {teammates
            .filter((m) => m.status !== "shutdown")
            .map((mate) => (
              <DropdownItem
                key={mate.name}
                label={mate.name}
                color={mate.color}
                isSelected={value === mate.name}
                onClick={() => handleSelect(mate.name)}
              />
            ))}
          {/* Broadcast */}
          <div style={{ borderTop: "1px solid hsl(220 10% 18%)", margin: "2px 0" }} />
          <DropdownItem
            label="All (broadcast)"
            isSelected={value === "*"}
            onClick={() => handleSelect("*")}
          />
        </div>
      )}
    </div>
  );
});

// ============================================================================
// DropdownItem
// ============================================================================

interface DropdownItemProps {
  label: string;
  color?: string;
  isSelected: boolean;
  onClick: () => void;
}

function DropdownItem({ label, color, isSelected, onClick }: DropdownItemProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="flex items-center gap-2 w-full px-3 py-1 text-[11px] text-left transition-colors"
      style={{
        backgroundColor: isSelected ? "hsl(220 10% 18%)" : "transparent",
        color: isSelected ? "hsl(220 10% 85%)" : "hsl(220 10% 60%)",
      }}
    >
      {color && (
        <span
          className="w-2 h-2 rounded-full shrink-0"
          style={{ backgroundColor: color }}
        />
      )}
      {label}
    </button>
  );
}
