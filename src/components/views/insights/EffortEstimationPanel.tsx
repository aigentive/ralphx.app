import { useState } from "react";
import { ChevronDown, ChevronRight } from "lucide-react";
import { DetailCard } from "@/components/tasks/detail-views/shared/DetailCard";
import { useMetricsConfig, useSaveMetricsConfig } from "@/hooks/useMetricsConfig";
import { DEFAULT_METRICS_CONFIG } from "@/types/project-stats";
import type { MetricsConfig } from "@/types/project-stats";

const ACCENT = "#ff6b35";

interface EffortEstimationPanelProps {
  lowHours: number;
  highHours: number;
  taskCount: number;
  projectId: string;
}

const CALIBRATION_FIELDS = [
  { field: "simpleBaseHours" as const, label: "Simple base hours" },
  { field: "mediumBaseHours" as const, label: "Medium base hours" },
  { field: "complexBaseHours" as const, label: "Complex base hours" },
  { field: "calendarFactor" as const, label: "Calendar factor" },
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

  function handleFieldBlur(field: keyof MetricsConfig, value: string) {
    const num = parseFloat(value);
    if (isNaN(num)) return;
    saveConfig({ ...currentConfig, [field]: num });
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
              {CALIBRATION_FIELDS.map(({ field, label }) => (
                <div key={field} className="flex items-center justify-between gap-2">
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
