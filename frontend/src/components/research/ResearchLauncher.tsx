/**
 * ResearchLauncher - Form for starting a research process
 *
 * Features:
 * - Question, context, scope inputs
 * - Depth preset selector (quick-scan, standard, deep-dive, exhaustive)
 * - Custom depth option with iteration/timeout inputs
 * - Form validation
 */

import { useState, useCallback } from "react";
import type { ResearchDepthPreset, ResearchDepth, ResearchBrief } from "@/types/research";
import { RESEARCH_PRESET_INFO } from "@/types/research";

// ============================================================================
// Types
// ============================================================================

interface ResearchLauncherProps {
  onLaunch: (data: { brief: ResearchBrief; depth: ResearchDepth }) => void;
  onCancel: () => void;
  isLaunching?: boolean;
}

type DepthSelection = ResearchDepthPreset | "custom";

// ============================================================================
// Component
// ============================================================================

export function ResearchLauncher({ onLaunch, onCancel, isLaunching = false }: ResearchLauncherProps) {
  const [question, setQuestion] = useState("");
  const [context, setContext] = useState("");
  const [scope, setScope] = useState("");
  const [selectedPreset, setSelectedPreset] = useState<DepthSelection>("standard");
  const [customIterations, setCustomIterations] = useState(100);
  const [customTimeout, setCustomTimeout] = useState(4);

  const isValid = question.trim().length > 0;
  const isCustom = selectedPreset === "custom";

  const handleLaunch = useCallback(() => {
    const brief: ResearchBrief = { question, constraints: [], ...(context && { context }), ...(scope && { scope }) };
    const depth: ResearchDepth = isCustom
      ? { type: "custom", config: { maxIterations: customIterations, timeoutHours: customTimeout, checkpointInterval: Math.ceil(customIterations / 10) } }
      : { type: "preset", preset: selectedPreset as ResearchDepthPreset };
    onLaunch({ brief, depth });
  }, [question, context, scope, isCustom, selectedPreset, customIterations, customTimeout, onLaunch]);

  return (
    <div data-testid="research-launcher" className="p-4 rounded space-y-4" style={{ backgroundColor: "var(--bg-surface)" }}>
      {/* Question */}
      <div className="space-y-1">
        <label htmlFor="question" className="block text-sm font-medium" style={{ color: "var(--text-primary)" }}>Research Question</label>
        <textarea id="question" data-testid="question-input" value={question} onChange={(e) => setQuestion(e.target.value)} disabled={isLaunching}
          placeholder="What do you want to research?" rows={3} className="w-full px-3 py-2 rounded border text-sm resize-none"
          style={{ backgroundColor: "var(--bg-base)", borderColor: "var(--border-default)", color: "var(--text-primary)" }} />
      </div>

      {/* Context & Scope */}
      <div className="grid grid-cols-2 gap-3">
        <div className="space-y-1">
          <label htmlFor="context" className="block text-xs" style={{ color: "var(--text-secondary)" }}>Context (optional)</label>
          <input id="context" data-testid="context-input" type="text" value={context} onChange={(e) => setContext(e.target.value)} disabled={isLaunching}
            placeholder="Background information" className="w-full px-2 py-1.5 rounded border text-sm"
            style={{ backgroundColor: "var(--bg-base)", borderColor: "var(--border-subtle)", color: "var(--text-primary)" }} />
        </div>
        <div className="space-y-1">
          <label htmlFor="scope" className="block text-xs" style={{ color: "var(--text-secondary)" }}>Scope (optional)</label>
          <input id="scope" data-testid="scope-input" type="text" value={scope} onChange={(e) => setScope(e.target.value)} disabled={isLaunching}
            placeholder="Limit the scope" className="w-full px-2 py-1.5 rounded border text-sm"
            style={{ backgroundColor: "var(--bg-base)", borderColor: "var(--border-subtle)", color: "var(--text-primary)" }} />
        </div>
      </div>

      {/* Depth Preset Selector */}
      <div className="space-y-2">
        <span className="block text-sm font-medium" style={{ color: "var(--text-primary)" }}>Research Depth</span>
        <div data-testid="depth-preset-selector" role="radiogroup" aria-label="Research depth" className="grid grid-cols-2 gap-2">
          {RESEARCH_PRESET_INFO.map((info) => (
            <button key={info.preset} type="button" role="radio" aria-checked={selectedPreset === info.preset}
              data-testid={`preset-${info.preset}`} data-selected={selectedPreset === info.preset ? "true" : "false"}
              onClick={() => setSelectedPreset(info.preset)} disabled={isLaunching}
              className="p-2 rounded border text-left text-sm transition-colors"
              style={{ backgroundColor: "var(--bg-base)", borderColor: selectedPreset === info.preset ? "var(--accent-primary)" : "var(--border-subtle)" }}>
              <div className="font-medium" style={{ color: "var(--text-primary)" }}>{info.name}</div>
              <div className="text-xs" style={{ color: "var(--text-muted)" }}>{info.config.maxIterations} iterations, {info.config.timeoutHours}h</div>
            </button>
          ))}
          <button type="button" role="radio" aria-checked={isCustom} data-testid="preset-custom" data-selected={isCustom ? "true" : "false"}
            onClick={() => setSelectedPreset("custom")} disabled={isLaunching} className="p-2 rounded border text-left text-sm transition-colors"
            style={{ backgroundColor: "var(--bg-base)", borderColor: isCustom ? "var(--accent-primary)" : "var(--border-subtle)" }}>
            <div className="font-medium" style={{ color: "var(--text-primary)" }}>Custom</div>
            <div className="text-xs" style={{ color: "var(--text-muted)" }}>Set your own limits</div>
          </button>
        </div>
      </div>

      {/* Custom Depth Inputs */}
      {isCustom && (
        <div className="grid grid-cols-2 gap-3">
          <div className="space-y-1">
            <label htmlFor="iterations" className="block text-xs" style={{ color: "var(--text-secondary)" }}>Max Iterations</label>
            <input id="iterations" data-testid="custom-iterations-input" type="number" value={customIterations}
              onChange={(e) => setCustomIterations(Number(e.target.value))} disabled={isLaunching} min={1}
              className="w-full px-2 py-1.5 rounded border text-sm" style={{ backgroundColor: "var(--bg-base)", borderColor: "var(--border-subtle)", color: "var(--text-primary)" }} />
          </div>
          <div className="space-y-1">
            <label htmlFor="timeout" className="block text-xs" style={{ color: "var(--text-secondary)" }}>Timeout (hours)</label>
            <input id="timeout" data-testid="custom-timeout-input" type="number" value={customTimeout}
              onChange={(e) => setCustomTimeout(Number(e.target.value))} disabled={isLaunching} min={0.5} step={0.5}
              className="w-full px-2 py-1.5 rounded border text-sm" style={{ backgroundColor: "var(--bg-base)", borderColor: "var(--border-subtle)", color: "var(--text-primary)" }} />
          </div>
        </div>
      )}

      {/* Actions */}
      <div className="flex justify-end gap-2 pt-2">
        <button data-testid="cancel-button" onClick={onCancel} disabled={isLaunching} className="px-4 py-2 rounded text-sm disabled:opacity-50"
          style={{ backgroundColor: "var(--bg-hover)", color: "var(--text-primary)" }}>Cancel</button>
        <button data-testid="launch-button" onClick={handleLaunch} disabled={!isValid || isLaunching}
          className="px-4 py-2 rounded text-sm font-medium disabled:opacity-50"
          style={{ backgroundColor: "var(--accent-primary)", color: "var(--bg-base)" }}>{isLaunching ? "Launching..." : "Launch Research"}</button>
      </div>
    </div>
  );
}
