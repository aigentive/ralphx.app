/**
 * Shared widget primitives for tool call rendering.
 *
 * Design reference: mockups/tool-call-widgets.html
 * Palette: macOS Tahoe flat, blue-gray + orange accent (#ff6b35)
 *
 * Constants, types, and utilities are in shared.constants.ts
 */

import React, { useState, useCallback } from "react";
import { ChevronRight, Check } from "lucide-react";

// Re-export types from constants (values imported locally to avoid react-refresh warnings)
export type {
  ToolCall,
  ToolCallWidgetProps,
  BadgeVariant,
  StepLineVariant,
} from "./shared.constants";

import {
  COLLAPSED_HEIGHT_COMPACT,
  COLLAPSED_HEIGHT,
  GRADIENT_HEIGHT,
  TRANSITION_SPEED,
  colors,
  shortenPath,
  badgeStyles,
  stepVariantConfig,
  type WidgetCardProps,
  type WidgetHeaderProps,
  type GradientFadeProps,
  type CodePreviewProps,
  type InlineIndicatorProps,
  type StepLineProps,
  type FilePathProps,
  type BadgeProps,
} from "./shared.constants";

// ============================================================================
// WidgetCard — Collapsible card with chevron, gradient-faded body
// ============================================================================

export const WidgetCard = React.memo(function WidgetCard({
  header,
  children,
  defaultExpanded = false,
  compact = false,
  className = "",
  alwaysExpanded = false,
}: WidgetCardProps) {
  const [isOpen, setIsOpen] = useState(defaultExpanded || alwaysExpanded);

  const toggle = useCallback(() => {
    if (!alwaysExpanded) setIsOpen((prev: boolean) => !prev);
  }, [alwaysExpanded]);

  const collapseHeight = compact ? COLLAPSED_HEIGHT_COMPACT : COLLAPSED_HEIGHT;

  return (
    <div
      className={className}
      style={{
        background: colors.bgSurface,
        borderRadius: 10,
        overflow: "hidden",
        border: `1px solid ${colors.borderSubtle}`,
      }}
    >
      {/* Clickable header */}
      <div
        onClick={toggle}
        role="button"
        tabIndex={0}
        onKeyDown={(e: React.KeyboardEvent) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); toggle(); } }}
        style={{
          display: "flex",
          alignItems: "center",
          gap: 7,
          padding: compact ? "5px 8px" : "7px 10px",
          cursor: alwaysExpanded ? "default" : "pointer",
          userSelect: "none",
          transition: `background ${TRANSITION_SPEED}`,
          minHeight: compact ? 28 : 32,
        }}
        onMouseEnter={(e: React.MouseEvent<HTMLDivElement>) => { if (!alwaysExpanded) e.currentTarget.style.background = colors.bgHover; }}
        onMouseLeave={(e: React.MouseEvent<HTMLDivElement>) => { e.currentTarget.style.background = "transparent"; }}
      >
        {!alwaysExpanded && (
          <ChevronRight
            size={10}
            style={{
              color: colors.textMuted,
              flexShrink: 0,
              transition: `transform ${TRANSITION_SPEED}`,
              transform: isOpen ? "rotate(90deg)" : "rotate(0deg)",
            }}
          />
        )}
        {header}
      </div>

      {/* Collapsible body with gradient fade */}
      <div
        style={{
          maxHeight: isOpen ? 2000 : collapseHeight,
          overflow: "hidden",
          position: "relative",
          transition: `max-height ${TRANSITION_SPEED} ease`,
        }}
      >
        <div
          style={{
            padding: "0 10px 8px",
            borderTop: `1px solid ${colors.borderSubtle}`,
            paddingTop: 8,
          }}
        >
          {children}
        </div>

        {/* Gradient fade overlay */}
        {!alwaysExpanded && (
          <div
            style={{
              position: "absolute",
              bottom: 0,
              left: 0,
              right: 0,
              height: GRADIENT_HEIGHT,
              background: `linear-gradient(to bottom, transparent, ${colors.bgSurface})`,
              pointerEvents: "none",
              transition: `opacity ${TRANSITION_SPEED}`,
              opacity: isOpen ? 0 : 1,
            }}
          />
        )}
      </div>
    </div>
  );
});

// ============================================================================
// WidgetHeader — Icon + title + badge layout (used inside WidgetCard header)
// ============================================================================

export function WidgetHeader({ icon, title, badge, compact = false, mono = false }: WidgetHeaderProps) {
  return (
    <>
      {icon && (
        <span style={{ width: 14, height: 14, color: colors.textMuted, flexShrink: 0, display: "flex", alignItems: "center" }}>
          {icon}
        </span>
      )}
      <span
        style={{
          fontSize: compact ? 11 : 11.5,
          fontWeight: 500,
          color: colors.textSecondary,
          flex: 1,
          overflow: "hidden",
          textOverflow: "ellipsis",
          whiteSpace: "nowrap",
          fontFamily: mono ? "var(--font-mono)" : undefined,
        }}
      >
        {title}
      </span>
      {badge}
    </>
  );
}

// ============================================================================
// GradientFade — Standalone gradient overlay for custom layouts
// ============================================================================

export function GradientFade({ visible, bgColor = colors.bgSurface }: GradientFadeProps) {
  return (
    <div
      style={{
        position: "absolute",
        bottom: 0,
        left: 0,
        right: 0,
        height: GRADIENT_HEIGHT,
        background: `linear-gradient(to bottom, transparent, ${bgColor})`,
        pointerEvents: "none",
        transition: `opacity ${TRANSITION_SPEED}`,
        opacity: visible ? 1 : 0,
      }}
    />
  );
}

// ============================================================================
// CodePreview — Monospace text with line numbers
// ============================================================================

export function CodePreview({ lines, startLine = 1, compact = false }: CodePreviewProps) {
  return (
    <div
      style={{
        fontFamily: "var(--font-mono)",
        fontSize: compact ? 10 : 11,
        lineHeight: 1.55,
        color: colors.textSecondary,
        whiteSpace: "pre",
        overflowX: "hidden",
        padding: "6px 0",
      }}
    >
      {lines.map((line, i) => (
        <div key={i}>
          <span
            style={{
              display: "inline-block",
              width: 28,
              textAlign: "right",
              color: "hsl(220 10% 28%)",
              marginRight: 12,
              userSelect: "none",
            }}
          >
            {startLine + i}
          </span>
          {line}
        </div>
      ))}
    </div>
  );
}

// ============================================================================
// InlineIndicator — Single-line minimal indicator (for empty states)
// ============================================================================

export function InlineIndicator({ icon, text }: InlineIndicatorProps) {
  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 5,
        padding: "2px 0",
        margin: "2px 0",
      }}
    >
      {icon && <span style={{ width: 12, height: 12, display: "flex", alignItems: "center" }}>{icon}</span>}
      <span style={{ fontSize: 10.5, color: colors.textMuted }}>{text}</span>
    </div>
  );
}

// ============================================================================
// StepLine — Ultra-compact step started/completed indicator
// ============================================================================

export function StepLine({ variant, title, note, compact = false }: StepLineProps) {
  const config = stepVariantConfig[variant];
  const isActive = variant === "started";
  const isDone = variant === "completed" || variant === "skipped";

  return (
    <div
      style={{
        display: "flex",
        alignItems: "flex-start",
        gap: 7,
        padding: compact ? "2px 10px" : "4px 10px",
        margin: "2px 0",
      }}
    >
      {/* Dot / check icon */}
      <div
        style={{
          width: 14,
          height: 14,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          flexShrink: 0,
          marginTop: 1,
        }}
      >
        {variant === "completed" ? (
          <Check size={12} style={{ color: config.color }} />
        ) : (
          <svg
            viewBox="0 0 16 16"
            width={12}
            height={12}
            style={isActive ? { animation: "pulse-dot 1.5s ease-in-out infinite" } : undefined}
          >
            <circle cx={8} cy={8} r={5} fill={config.color} />
          </svg>
        )}
      </div>

      {/* Step info */}
      <div style={{ flex: 1, minWidth: 0 }}>
        <div
          style={{
            fontSize: compact ? 10.5 : 11,
            color: isDone ? colors.textMuted : colors.textSecondary,
            display: "flex",
            alignItems: "center",
            gap: 6,
            lineHeight: 1.3,
          }}
        >
          <span
            style={{
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
              flex: 1,
              fontWeight: isActive ? 500 : undefined,
              color: isActive ? colors.textPrimary : undefined,
            }}
          >
            {title}
          </span>
          <Badge variant={config.badgeVariant} compact>{config.label}</Badge>
        </div>
        {note && (
          <div
            style={{
              fontSize: 10,
              color: colors.textMuted,
              marginTop: 2,
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
            }}
          >
            {note}
          </div>
        )}
      </div>
    </div>
  );
}

// ============================================================================
// FilePath — Smart path shortening
// ============================================================================

export function FilePath({ path, maxLength = 50 }: FilePathProps) {
  const shortened = shortenPath(path, maxLength);
  return (
    <span
      style={{
        fontFamily: "var(--font-mono)",
        fontSize: 11,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
      }}
      title={path}
    >
      {shortened}
    </span>
  );
}

// ============================================================================
// Badge — Small status badges
// ============================================================================

export function Badge({ variant, children, compact = false }: BadgeProps) {
  const style = badgeStyles[variant];
  return (
    <span
      style={{
        fontSize: compact ? 9 : 9.5,
        padding: "1px 6px",
        borderRadius: 6,
        fontWeight: 500,
        flexShrink: 0,
        whiteSpace: "nowrap",
        background: style.bg,
        color: style.color,
      }}
    >
      {children}
    </span>
  );
}
