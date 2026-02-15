/**
 * TeamConfigPanel - Configuration panel for team mode ideation sessions
 *
 * Shows when Research or Debate team mode is selected.
 * Allows configuring max teammates, model ceiling, budget, and composition mode.
 */

import type { TeamConfig, CompositionMode } from "@/types/ideation";

interface TeamConfigPanelProps {
  config: TeamConfig;
  onChange: (config: TeamConfig) => void;
}

const MAX_TEAMMATES_OPTIONS = [2, 3, 4, 5, 6, 7, 8];
const MODEL_OPTIONS = [
  { value: "haiku", label: "Haiku" },
  { value: "sonnet", label: "Sonnet" },
  { value: "opus", label: "Opus" },
];
const BUDGET_OPTIONS = [
  { value: "", label: "None" },
  { value: "5", label: "$5" },
  { value: "10", label: "$10" },
  { value: "25", label: "$25" },
];

const selectStyle: React.CSSProperties = {
  background: "hsla(220 10% 100% / 0.04)",
  border: "1px solid hsla(220 10% 100% / 0.08)",
  color: "hsl(220 10% 90%)",
  borderRadius: "8px",
  padding: "6px 10px",
  fontSize: "13px",
  outline: "none",
};

export function TeamConfigPanel({ config, onChange }: TeamConfigPanelProps) {
  const update = (partial: Partial<TeamConfig>) => {
    onChange({ ...config, ...partial });
  };

  return (
    <div
      className="w-full rounded-xl p-4 mt-4 text-left"
      style={{
        background: "hsla(220 10% 100% / 0.02)",
        border: "1px solid hsla(220 10% 100% / 0.06)",
      }}
    >
      {/* Row 1: Max teammates + Model ceiling */}
      <div className="flex items-center gap-4 mb-3">
        <label className="flex items-center gap-2 text-[13px]" style={{ color: "hsl(220 10% 60%)" }}>
          <span>Max teammates:</span>
          <select
            value={config.maxTeammates}
            onChange={(e) => update({ maxTeammates: Number(e.target.value) })}
            style={selectStyle}
          >
            {MAX_TEAMMATES_OPTIONS.map((n) => (
              <option key={n} value={n}>{n}</option>
            ))}
          </select>
        </label>

        <label className="flex items-center gap-2 text-[13px]" style={{ color: "hsl(220 10% 60%)" }}>
          <span>Model ceiling:</span>
          <select
            value={config.modelCeiling}
            onChange={(e) => update({ modelCeiling: e.target.value })}
            style={selectStyle}
          >
            {MODEL_OPTIONS.map((m) => (
              <option key={m.value} value={m.value}>{m.label}</option>
            ))}
          </select>
        </label>
      </div>

      {/* Row 2: Budget + Composition */}
      <div className="flex items-center gap-4 mb-3">
        <label className="flex items-center gap-2 text-[13px]" style={{ color: "hsl(220 10% 60%)" }}>
          <span>Budget limit:</span>
          <select
            value={config.budgetLimit ?? ""}
            onChange={(e) => update({ budgetLimit: e.target.value ? Number(e.target.value) : undefined })}
            style={selectStyle}
          >
            {BUDGET_OPTIONS.map((b) => (
              <option key={b.value} value={b.value}>{b.label}</option>
            ))}
          </select>
        </label>

        <div className="flex items-center gap-3 text-[13px]" style={{ color: "hsl(220 10% 60%)" }}>
          <span>Composition:</span>
          <CompositionRadio
            value={config.compositionMode}
            onChange={(mode) => update({ compositionMode: mode })}
          />
        </div>
      </div>

      {/* Constrained info */}
      {config.compositionMode === "constrained" && (
        <p
          className="text-[12px] mt-2 pl-1"
          style={{ color: "hsl(220 10% 50%)" }}
        >
          Lead will select from preset roles only.
        </p>
      )}
    </div>
  );
}

function CompositionRadio({
  value,
  onChange,
}: {
  value: CompositionMode;
  onChange: (v: CompositionMode) => void;
}) {
  const options: { value: CompositionMode; label: string }[] = [
    { value: "dynamic", label: "Dynamic" },
    { value: "constrained", label: "Constrained" },
  ];

  return (
    <div className="flex items-center gap-3">
      {options.map((opt) => (
        <label key={opt.value} className="flex items-center gap-1.5 cursor-pointer" onClick={() => onChange(opt.value)}>
          <span
            className="w-3.5 h-3.5 rounded-full border flex items-center justify-center"
            style={{
              borderColor: value === opt.value ? "hsl(14 100% 60%)" : "hsla(220 10% 100% / 0.15)",
              background: value === opt.value ? "hsl(14 100% 60%)" : "transparent",
            }}
          >
            {value === opt.value && (
              <span
                className="w-1.5 h-1.5 rounded-full"
                style={{ background: "white" }}
              />
            )}
          </span>
          <span style={{ color: value === opt.value ? "hsl(220 10% 90%)" : "hsl(220 10% 60%)" }}>
            {opt.label}
          </span>
        </label>
      ))}
    </div>
  );
}
