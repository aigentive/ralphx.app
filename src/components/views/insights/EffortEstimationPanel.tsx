import { useMemo, useState } from "react";
import { ChevronDown, ChevronRight } from "lucide-react";
import { DetailCard } from "@/components/tasks/detail-views/shared/DetailCard";
import { useMetricsConfig, useSaveMetricsConfig } from "@/hooks/useMetricsConfig";
import { DEFAULT_METRICS_CONFIG } from "@/types/project-stats";
import type { MetricsConfig } from "@/types/project-stats";

const ACCENT = "#ff6b35";

type ExperienceLevel = "junior" | "mid" | "senior" | "staff" | "custom";

interface ExperiencePreset {
  label: string;
  simpleBaseHours: number;
  mediumBaseHours: number;
  complexBaseHours: number;
  calendarFactor: number;
}

const EXPERIENCE_PRESETS: Record<Exclude<ExperienceLevel, "custom">, ExperiencePreset> = {
  junior: { label: "Junior", simpleBaseHours: 4, mediumBaseHours: 10, complexBaseHours: 20, calendarFactor: 2.0 },
  mid: { label: "Mid-level", simpleBaseHours: 2, mediumBaseHours: 4, complexBaseHours: 8, calendarFactor: 1.5 },
  senior: { label: "Senior", simpleBaseHours: 1, mediumBaseHours: 2, complexBaseHours: 4, calendarFactor: 1.3 },
  staff: { label: "Staff/Principal", simpleBaseHours: 0.5, mediumBaseHours: 1, complexBaseHours: 2, calendarFactor: 1.2 },
};

function detectExperienceLevel(config: MetricsConfig): ExperienceLevel {
  for (const [key, preset] of Object.entries(EXPERIENCE_PRESETS)) {
    if (
      config.simpleBaseHours === preset.simpleBaseHours &&
      config.mediumBaseHours === preset.mediumBaseHours &&
      config.complexBaseHours === preset.complexBaseHours &&
      config.calendarFactor === preset.calendarFactor
    ) {
      return key as Exclude<ExperienceLevel, "custom">;
    }
  }
  return "custom";
}

interface EffortEstimationPanelProps {
  lowHours: number;
  highHours: number;
  taskCount: number;
  projectId: string;
}

const CALIBRATION_FIELDS = [
  { field: "simpleBaseHours" as const, label: "Simple (≤3 steps)", hint: "e.g., small bug fix, config change" },
  { field: "mediumBaseHours" as const, label: "Medium (4-7 steps or 1 review)", hint: "e.g., feature implementation, refactor" },
  { field: "complexBaseHours" as const, label: "Complex (8+ steps or 2+ reviews)", hint: "e.g., multi-file architecture change" },
  { field: "calendarFactor" as const, label: "Calendar factor", hint: "Accounts for context-switching, meetings, code review overhead" },
] as const;

export function EffortEstimationPanel({ lowHours, highHours, taskCount, projectId }: EffortEstimationPanelProps) {
  const [expanded, setExpanded] = useState(false);

  const { data: config } = useMetricsConfig(projectId);
  const { mutate: saveConfig, isPending: isSaving } = useSaveMetricsConfig(projectId);

  const currentConfig = config ?? DEFAULT_METRICS_CONFIG;
  const isDefault =
    currentConfig.simpleBaseHours === DEFAULT_METRICS_CONFIG.simpleBaseHours &&
    currentConfig.mediumBaseHours === DEFAULT_METRICS_CONFIG.mediumBaseHours &&
    currentConfig.complexBaseHours === DEFAULT_METRICS_CONFIG.complexBaseHours &&
    currentConfig.calendarFactor === DEFAULT_METRICS_CONFIG.calendarFactor;

  const currentLevel = useMemo(() => detectExperienceLevel(currentConfig), [currentConfig]);

  function handleFieldBlur(field: keyof MetricsConfig, value: string) {
    const num = parseFloat(value);
    if (isNaN(num)) return;
    saveConfig({ ...currentConfig, [field]: num });
  }

  function handlePresetChange(level: ExperienceLevel) {
    if (level === "custom") return;
    const preset = EXPERIENCE_PRESETS[level];
    saveConfig({
      ...currentConfig,
      simpleBaseHours: preset.simpleBaseHours,
      mediumBaseHours: preset.mediumBaseHours,
      complexBaseHours: preset.complexBaseHours,
      calendarFactor: preset.calendarFactor,
    });
  }

  function handleReset() {
    saveConfig(DEFAULT_METRICS_CONFIG);
  }

  return (
    <DetailCard variant="accent">
      <div className="flex flex-col gap-3">
        <div className="flex items-start justify-between gap-4">
          <div className="flex flex-col gap-1">
            <div className="flex items-center gap-2">
              <span
                className="text-[11px] font-semibold uppercase tracking-wider"
                style={{ color: "rgba(255,255,255,0.4)", letterSpacing: "0.08em" }}
              >
                Estimated Manual Effort
              </span>
              {!isDefault && (
                <span
                  className="text-[10px] px-1.5 py-0.5 rounded"
                  style={{ backgroundColor: "rgba(255,107,53,0.15)", color: ACCENT }}
                >
                  calibrated
                </span>
              )}
            </div>
            <span
              className="text-[24px] font-semibold"
              style={{ color: ACCENT, fontFamily: "system-ui" }}
            >
              ~{lowHours}–{highHours}h
            </span>
            <span className="text-[12px]" style={{ color: "rgba(255,255,255,0.4)" }}>
              Based on {taskCount} completed task{taskCount !== 1 ? "s" : ""}
            </span>
            <span className="text-[11px]" style={{ color: "rgba(255,255,255,0.3)" }}>
              Equivalent manual effort for completed work
            </span>
          </div>
        </div>

        <button
          onClick={() => setExpanded((v) => !v)}
          className="flex items-center gap-1.5 text-[12px] w-fit"
          style={{ color: "rgba(255,255,255,0.45)" }}
          data-testid="methodology-toggle"
        >
          {expanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
          Methodology
        </button>

        {expanded && (
          <div
            className="text-[12px] leading-relaxed rounded-lg p-3 space-y-3"
            style={{
              color: "rgba(255,255,255,0.55)",
              backgroundColor: "rgba(255,255,255,0.04)",
            }}
          >
            <p className="font-medium" style={{ color: "rgba(255,255,255,0.7)" }}>
              Effort Model Estimate (EME)
            </p>
            <ul className="flex flex-col gap-1.5">
              <li>
                <span style={{ color: "rgba(255,255,255,0.7)" }}>Simple</span> (≤3 steps):
                {" "}{currentConfig.simpleBaseHours}h base × {currentConfig.calendarFactor} calendar factor
              </li>
              <li>
                <span style={{ color: "rgba(255,255,255,0.7)" }}>Medium</span> (4–7 steps
                or 1 review): {currentConfig.mediumBaseHours}h base × {currentConfig.calendarFactor} calendar factor
              </li>
              <li>
                <span style={{ color: "rgba(255,255,255,0.7)" }}>Complex</span> (8+ steps
                or 2+ reviews): {currentConfig.complexBaseHours}h base × {currentConfig.calendarFactor} calendar factor
              </li>
            </ul>
            <p style={{ color: "rgba(255,255,255,0.35)" }}>
              Range reflects uncertainty: low = 80% of estimate, high = 120%.
            </p>

            {/* Calibration inputs */}
            <div
              className="space-y-2 pt-2"
              style={{ borderTop: "1px solid rgba(255,255,255,0.06)" }}
              data-testid="calibration-section"
            >
              <div
                className="text-[10px] uppercase tracking-wide"
                style={{ color: "rgba(255,255,255,0.3)" }}
              >
                Calibrate
              </div>

              {/* Experience level preset selector */}
              <div className="flex items-center gap-2">
                <label
                  htmlFor="experience-level-select"
                  className="text-[11px] shrink-0"
                  style={{ color: "rgba(255,255,255,0.45)" }}
                >
                  Team level
                </label>
                <select
                  id="experience-level-select"
                  value={currentLevel}
                  onChange={(e) => handlePresetChange(e.target.value as ExperienceLevel)}
                  className="flex-1 rounded px-2 py-1 text-[12px] outline-none ring-0 focus:ring-0 focus:outline-none focus-visible:outline-none border-0 cursor-pointer"
                  style={{
                    backgroundColor: "rgba(255,255,255,0.06)",
                    color: "rgba(255,255,255,0.7)",
                    boxShadow: "none",
                    outline: "none",
                  }}
                  data-testid="experience-level-select"
                >
                  {Object.entries(EXPERIENCE_PRESETS).map(([key, preset]) => (
                    <option key={key} value={key} style={{ backgroundColor: "#1a1a1a" }}>
                      {preset.label}
                    </option>
                  ))}
                  {currentLevel === "custom" && (
                    <option value="custom" style={{ backgroundColor: "#1a1a1a" }}>
                      Custom
                    </option>
                  )}
                </select>
              </div>

              <p className="text-[11px]" style={{ color: "rgba(255,255,255,0.35)" }}>
                Estimated hours a developer would need to complete each task type manually, without AI assistance.
              </p>
              {CALIBRATION_FIELDS.map(({ field, label, hint }) => (
                <div key={field} className="flex flex-col gap-0.5">
                  <div className="flex items-center justify-between gap-2">
                    <label
                      htmlFor={`insights-calibrate-${field}`}
                      className="text-[12px]"
                      style={{ color: "rgba(255,255,255,0.45)" }}
                    >
                      {label}
                    </label>
                    <input
                      id={`insights-calibrate-${field}`}
                      type="number"
                      min={field === "calendarFactor" ? 1 : 0.5}
                      max={field === "calendarFactor" ? 3 : 40}
                      step={0.5}
                      defaultValue={currentConfig[field]}
                      key={currentConfig[field]}
                      onBlur={(e) => handleFieldBlur(field, e.target.value)}
                      className="w-16 rounded px-1.5 py-0.5 text-[12px] text-right tabular-nums outline-none ring-0 focus:ring-0 focus:outline-none focus-visible:outline-none border-0"
                      style={{
                        backgroundColor: "rgba(255,255,255,0.06)",
                        color: "rgba(255,255,255,0.7)",
                        boxShadow: "none",
                        outline: "none",
                      }}
                      data-testid={`calibrate-${field}`}
                      aria-label={label}
                    />
                  </div>
                  <span className="text-[10px]" style={{ color: "rgba(255,255,255,0.25)" }}>
                    {hint}
                  </span>
                </div>
              ))}

              {!isDefault && (
                <button
                  onClick={handleReset}
                  disabled={isSaving}
                  className="text-[12px] underline transition-colors"
                  style={{ color: "rgba(255,255,255,0.4)" }}
                  data-testid="calibration-reset"
                >
                  Reset to defaults
                </button>
              )}
            </div>
          </div>
        )}
      </div>
    </DetailCard>
  );
}
