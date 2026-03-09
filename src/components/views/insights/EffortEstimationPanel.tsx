import { useMemo } from "react";
import { HelpCircle } from "lucide-react";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { useMetricsConfig, useSaveMetricsConfig } from "@/hooks/useMetricsConfig";
import { DEFAULT_METRICS_CONFIG } from "@/types/project-stats";
import type { MetricsConfig } from "@/types/project-stats";

const ACCENT = "#ff6b35";
const HOURS_PER_DAY = 8;

type ExperienceLevel = "junior" | "mid" | "senior" | "staff" | "custom";

interface ExperiencePreset {
  label: string;
  description: string;
  simpleBaseHours: number;
  mediumBaseHours: number;
  complexBaseHours: number;
  calendarFactor: number;
}

const EXPERIENCE_PRESETS: Record<Exclude<ExperienceLevel, "custom">, ExperiencePreset> = {
  junior: { label: "Junior", description: "1-2 yrs exp", simpleBaseHours: 4, mediumBaseHours: 10, complexBaseHours: 20, calendarFactor: 2.0 },
  mid: { label: "Mid", description: "3-5 yrs exp", simpleBaseHours: 2, mediumBaseHours: 4, complexBaseHours: 8, calendarFactor: 1.5 },
  senior: { label: "Senior", description: "5-8 yrs exp", simpleBaseHours: 1, mediumBaseHours: 2, complexBaseHours: 4, calendarFactor: 1.3 },
  staff: { label: "Staff+", description: "8+ yrs exp", simpleBaseHours: 0.5, mediumBaseHours: 1, complexBaseHours: 2, calendarFactor: 1.2 },
};

const FIELD_TOOLTIPS: Record<string, string> = {
  simpleBaseHours: "Tasks with 3 or fewer steps. Typically small bug fixes, config changes, or single-file edits that a developer would handle quickly.",
  mediumBaseHours: "Tasks with 4-7 steps. Multi-file changes, feature additions, or moderate refactors requiring some planning and testing.",
  complexBaseHours: "Tasks with 8+ steps. Large features, cross-module changes, or architectural work requiring significant design and coordination.",
  calendarFactor: "Accounts for meetings, context switching, code review, and other overhead that stretches pure coding time. 1.0 = no overhead, 2.0 = half the day is non-coding.",
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
  earliestTaskDate: string | null;
  latestTaskDate: string | null;
  projectId: string;
}

const CALIBRATION_FIELDS = [
  { field: "simpleBaseHours" as const, label: "Simple", sub: "\u22643 steps" },
  { field: "mediumBaseHours" as const, label: "Medium", sub: "4-7 steps" },
  { field: "complexBaseHours" as const, label: "Complex", sub: "8+ steps" },
  { field: "calendarFactor" as const, label: "Overhead", sub: "multiplier" },
] as const;

function formatEstimate(hours: number): string {
  return Math.round(hours).toLocaleString();
}

function formatWorkTime(hours: number, workingDaysPerWeek: number): string {
  const totalDays = hours / HOURS_PER_DAY;
  const weeks = Math.floor(totalDays / workingDaysPerWeek);
  const remainingDays = Math.round(totalDays % workingDaysPerWeek);

  if (weeks === 0) {
    return `${Math.round(totalDays)}d`;
  }
  if (remainingDays === 0) {
    return `${weeks}w`;
  }
  return `${weeks}w ${remainingDays}d`;
}

function formatDateRange(earliest: string | null, latest: string | null): string | null {
  if (!earliest || !latest) return null;
  const fmt = (d: string) => {
    const date = new Date(d + "T00:00:00");
    return date.toLocaleDateString("en-US", { month: "short", year: "numeric" });
  };
  const e = fmt(earliest);
  const l = fmt(latest);
  return e === l ? e : `${e} — ${l}`;
}

function computeCalendarSpan(earliest: string | null, latest: string | null, workingDaysPerWeek: number): string | null {
  if (!earliest || !latest) return null;
  const start = new Date(earliest + "T00:00:00");
  const end = new Date(latest + "T00:00:00");
  const diffMs = end.getTime() - start.getTime();
  const diffDays = Math.max(1, Math.round(diffMs / (1000 * 60 * 60 * 24)));
  const calendarWeeks = Math.round(diffDays / 7);
  const workWeeks = Math.round((diffDays / 7) * (workingDaysPerWeek / 7));
  if (calendarWeeks < 1) return `${diffDays} days`;
  return `${calendarWeeks} calendar weeks (~${workWeeks} work weeks)`;
}

export function EffortEstimationPanel({ lowHours, highHours, taskCount, earliestTaskDate, latestTaskDate, projectId }: EffortEstimationPanelProps) {
  const { data: config } = useMetricsConfig(projectId);
  const { mutate: saveConfig } = useSaveMetricsConfig(projectId);

  const currentConfig = config ?? DEFAULT_METRICS_CONFIG;
  const isDefault =
    currentConfig.simpleBaseHours === DEFAULT_METRICS_CONFIG.simpleBaseHours &&
    currentConfig.mediumBaseHours === DEFAULT_METRICS_CONFIG.mediumBaseHours &&
    currentConfig.complexBaseHours === DEFAULT_METRICS_CONFIG.complexBaseHours &&
    currentConfig.calendarFactor === DEFAULT_METRICS_CONFIG.calendarFactor &&
    currentConfig.workingDaysPerWeek === DEFAULT_METRICS_CONFIG.workingDaysPerWeek;

  const currentLevel = useMemo(() => detectExperienceLevel(currentConfig), [currentConfig]);
  const workDays = currentConfig.workingDaysPerWeek;

  function handleFieldBlur(field: keyof MetricsConfig, value: string) {
    const num = parseFloat(value);
    if (isNaN(num)) return;
    if (field === "workingDaysPerWeek") {
      const clamped = Math.max(1, Math.min(7, Math.round(num)));
      saveConfig({ ...currentConfig, [field]: clamped });
    } else {
      saveConfig({ ...currentConfig, [field]: num });
    }
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
    <div
      className="rounded-xl"
      style={{ backgroundColor: "hsla(14 100% 60% / 0.08)" }}
    >
      {/* Two-column layout: estimate left, calibration right */}
      <div className="grid grid-cols-1 lg:grid-cols-[1fr_auto] gap-0">
        {/* Left: Estimate display */}
        <div className="p-4 flex flex-col justify-center gap-1">
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
          <div className="flex items-baseline gap-1.5">
            <span
              className="text-[28px] font-semibold tabular-nums"
              style={{ color: ACCENT, fontFamily: "system-ui", lineHeight: 1.1 }}
            >
              ~{formatEstimate(lowHours)}&ndash;{formatEstimate(highHours)}
            </span>
            <span
              className="text-[16px] font-medium"
              style={{ color: "rgba(255,107,53,0.6)" }}
            >
              active hours
            </span>
          </div>

          {/* Work weeks/days conversion */}
          <span
            className="text-[13px] tabular-nums"
            style={{ color: "rgba(255,255,255,0.5)" }}
          >
            {formatWorkTime(lowHours, workDays)}&ndash;{formatWorkTime(highHours, workDays)}{" "}
            <span style={{ color: "rgba(255,255,255,0.3)" }}>
              ({HOURS_PER_DAY}h/day, {workDays}d/week)
            </span>
          </span>

          <span className="text-[12px]" style={{ color: "rgba(255,255,255,0.4)" }}>
            Based on {taskCount} completed task{taskCount !== 1 ? "s" : ""}
            {formatDateRange(earliestTaskDate, latestTaskDate) != null && (
              <> · {formatDateRange(earliestTaskDate, latestTaskDate)}</>
            )}
            {" "}· Equivalent manual effort without AI
          </span>
          {computeCalendarSpan(earliestTaskDate, latestTaskDate, workDays) != null && (
            <span className="text-[11px]" style={{ color: "rgba(255,255,255,0.3)" }}>
              Completed over {computeCalendarSpan(earliestTaskDate, latestTaskDate, workDays)}
            </span>
          )}

          {/* Range explanation */}
          <span className="text-[11px]" style={{ color: "rgba(255,255,255,0.3)" }}>
            Low = pure coding time per task · High = coding + overhead ({currentConfig.calendarFactor}× for meetings, reviews, context switching)
          </span>

          {/* Methodology inline — compact */}
          <div
            className="mt-1 flex gap-4 text-[11px]"
            style={{ color: "rgba(255,255,255,0.35)" }}
          >
            <span>
              <span style={{ color: "rgba(255,255,255,0.5)" }}>Simple:</span>{" "}
              {currentConfig.simpleBaseHours}h × {currentConfig.calendarFactor}
            </span>
            <span>
              <span style={{ color: "rgba(255,255,255,0.5)" }}>Medium:</span>{" "}
              {currentConfig.mediumBaseHours}h × {currentConfig.calendarFactor}
            </span>
            <span>
              <span style={{ color: "rgba(255,255,255,0.5)" }}>Complex:</span>{" "}
              {currentConfig.complexBaseHours}h × {currentConfig.calendarFactor}
            </span>
          </div>
        </div>

        {/* Right: Calibration panel */}
        <div
          className="p-4 flex flex-col gap-3"
          style={{
            borderLeft: "1px solid rgba(255,255,255,0.06)",
            minWidth: "280px",
          }}
          data-testid="calibration-section"
        >
          {/* Level selector as segmented buttons */}
          <div className="flex flex-col gap-1.5">
            <span
              className="text-[10px] uppercase tracking-wide"
              style={{ color: "rgba(255,255,255,0.3)", letterSpacing: "0.06em" }}
            >
              Team Level
            </span>
            <div className="flex gap-1" data-testid="experience-level-select">
              {Object.entries(EXPERIENCE_PRESETS).map(([key, preset]) => {
                const isActive = currentLevel === key;
                return (
                  <button
                    key={key}
                    onClick={() => handlePresetChange(key as ExperienceLevel)}
                    className="flex-1 flex flex-col items-center rounded-md px-2 py-1.5 transition-colors"
                    style={{
                      backgroundColor: isActive
                        ? "rgba(255,107,53,0.15)"
                        : "rgba(255,255,255,0.04)",
                      color: isActive
                        ? ACCENT
                        : "rgba(255,255,255,0.5)",
                    }}
                  >
                    <span className="text-[11px] font-medium">{preset.label}</span>
                    <span
                      className="text-[9px]"
                      style={{
                        color: isActive
                          ? "rgba(255,107,53,0.6)"
                          : "rgba(255,255,255,0.25)",
                      }}
                    >
                      {preset.description}
                    </span>
                  </button>
                );
              })}
            </div>
            {currentLevel === "custom" && (
              <span className="text-[10px]" style={{ color: "rgba(255,107,53,0.5)" }}>
                Custom values
              </span>
            )}
          </div>

          {/* Calibration inputs in 2×2 grid + working days */}
          <TooltipProvider delayDuration={200}>
            <div className="grid grid-cols-2 gap-x-3 gap-y-2">
              {CALIBRATION_FIELDS.map(({ field, label, sub }) => (
                <div key={field} className="flex items-center justify-between gap-2">
                  <div className="flex items-center gap-1">
                    <div className="flex flex-col">
                      <span
                        className="text-[11px]"
                        style={{ color: "rgba(255,255,255,0.5)" }}
                      >
                        {label}
                      </span>
                      <span
                        className="text-[9px]"
                        style={{ color: "rgba(255,255,255,0.25)" }}
                      >
                        {sub}
                      </span>
                    </div>
                    {FIELD_TOOLTIPS[field] !== undefined && (
                      <Tooltip>
                        <TooltipTrigger asChild>
                          <HelpCircle className="w-3 h-3 shrink-0 text-muted-foreground" />
                        </TooltipTrigger>
                        <TooltipContent
                          side="top"
                          className="max-w-[220px] text-[11px]"
                        >
                          {FIELD_TOOLTIPS[field]}
                        </TooltipContent>
                      </Tooltip>
                    )}
                  </div>
                  <input
                    id={`insights-calibrate-${field}`}
                    type="number"
                    min={field === "calendarFactor" ? 1 : 0.5}
                    max={field === "calendarFactor" ? 3 : 40}
                    step={0.5}
                    defaultValue={currentConfig[field]}
                    key={currentConfig[field]}
                    onBlur={(e) => handleFieldBlur(field, e.target.value)}
                    className="w-14 rounded px-1.5 py-0.5 text-[12px] text-right tabular-nums outline-none ring-0 focus:ring-0 focus:outline-none focus-visible:outline-none border-0"
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

              {/* Working days per week */}
              <div className="flex items-center justify-between gap-2 col-span-2 pt-1"
                style={{ borderTop: "1px solid rgba(255,255,255,0.06)" }}
              >
                <div className="flex items-center gap-1">
                  <div className="flex flex-col">
                    <span
                      className="text-[11px]"
                      style={{ color: "rgba(255,255,255,0.5)" }}
                    >
                      Work days/week
                    </span>
                    <span
                      className="text-[9px]"
                      style={{ color: "rgba(255,255,255,0.25)" }}
                    >
                      for time conversion
                    </span>
                  </div>
                  <Tooltip>
                    <TooltipTrigger asChild>
                      <HelpCircle className="w-3 h-3 shrink-0 text-muted-foreground" />
                    </TooltipTrigger>
                    <TooltipContent
                      side="top"
                      className="max-w-[220px] text-[11px]"
                    >
                      Number of working days per week. Used to convert hours into work weeks and days (8h/day).
                    </TooltipContent>
                  </Tooltip>
                </div>
                <input
                  id="insights-calibrate-workingDaysPerWeek"
                  type="number"
                  min={1}
                  max={7}
                  step={1}
                  defaultValue={currentConfig.workingDaysPerWeek}
                  key={currentConfig.workingDaysPerWeek}
                  onBlur={(e) => handleFieldBlur("workingDaysPerWeek", e.target.value)}
                  className="w-14 rounded px-1.5 py-0.5 text-[12px] text-right tabular-nums outline-none ring-0 focus:ring-0 focus:outline-none focus-visible:outline-none border-0"
                  style={{
                    backgroundColor: "rgba(255,255,255,0.06)",
                    color: "rgba(255,255,255,0.7)",
                    boxShadow: "none",
                    outline: "none",
                  }}
                  data-testid="calibrate-workingDaysPerWeek"
                  aria-label="Working days per week"
                />
              </div>
            </div>
          </TooltipProvider>

          {/* Reset */}
          {!isDefault && (
            <button
              onClick={handleReset}
              className="text-[11px] transition-colors self-start"
              style={{ color: "rgba(255,255,255,0.35)" }}
              data-testid="calibration-reset"
            >
              Reset to defaults
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
