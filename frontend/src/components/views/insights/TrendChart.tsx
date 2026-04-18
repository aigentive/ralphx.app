import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from "recharts";
import type { WeeklyDataPoint } from "@/types/project-stats";

const ACCENT = "#ff6b35";
const SECONDARY_DEFAULT = "rgba(148, 163, 184, 0.7)";

const tooltipStyle = {
  backgroundColor: "hsl(220 10% 12%)",
  border: "none",
  borderRadius: "8px",
  fontSize: "12px",
  color: "rgba(255,255,255,0.85)",
};

function formatWeek(weekStart: string): string {
  const date = new Date(weekStart + "T00:00:00");
  return date.toLocaleDateString("en-US", { month: "short", day: "numeric" });
}

interface TrendChartProps {
  title: string;
  data: WeeklyDataPoint[];
  valueFormatter?: (v: number) => string;
  color?: string;
  currentValue?: string;
  timeWindow?: string;
  secondaryData?: WeeklyDataPoint[];
  secondaryLabel?: string;
  secondaryColor?: string;
  primaryLabel?: string;
  secondaryValueFormatter?: (v: number) => string;
}

export function TrendChart({
  title,
  data,
  valueFormatter,
  color = ACCENT,
  currentValue,
  timeWindow,
  secondaryData,
  secondaryLabel,
  secondaryColor = SECONDARY_DEFAULT,
  primaryLabel,
  secondaryValueFormatter,
}: TrendChartProps) {
  const hasSecondary = secondaryData !== undefined && secondaryData.length > 0;
  const secondaryMap = new Map(
    hasSecondary ? secondaryData.map((pt) => [formatWeek(pt.weekStart), pt.value]) : [],
  );

  const chartData = data.map((pt) => {
    const week = formatWeek(pt.weekStart);
    const point: Record<string, string | number> = { week, value: pt.value };
    if (hasSecondary) {
      const sec = secondaryMap.get(week);
      if (sec !== undefined) point.secondary = sec;
    }
    return point;
  });

  const fmt = valueFormatter ?? ((v: number) => String(v));
  const fmtSecondary = secondaryValueFormatter ?? ((v: number) => String(v));

  const header = (
    <div className={timeWindow !== undefined ? "mb-2" : "mb-3"}>
      <div className="flex items-center justify-between">
        <p
          className="text-[12px] font-medium"
          style={{ color: "rgba(255,255,255,0.7)" }}
        >
          {title}
        </p>
        {currentValue !== undefined && (
          <span
            className="text-[12px]"
            style={{ color: "rgba(255,255,255,0.5)" }}
          >
            {currentValue}
          </span>
        )}
      </div>
      {timeWindow !== undefined && (
        <p
          className="text-[10px] mt-0.5"
          style={{ color: "rgba(255,255,255,0.35)" }}
        >
          {timeWindow}
        </p>
      )}
    </div>
  );

  if (chartData.length === 0) {
    return (
      <div>
        {header}
        <p className="text-[12px]" style={{ color: "rgba(255,255,255,0.3)" }}>
          No data yet
        </p>
      </div>
    );
  }

  const pLabel = primaryLabel ?? title;
  const sLabel = secondaryLabel ?? "Secondary";

  return (
    <div>
      {header}
      <ResponsiveContainer width="100%" height={120}>
        <LineChart data={chartData} margin={{ top: 4, right: 4, left: -20, bottom: 0 }}>
          <CartesianGrid
            strokeDasharray="3 3"
            stroke="var(--overlay-weak)"
            vertical={false}
          />
          <XAxis
            dataKey="week"
            tick={{ fontSize: 10, fill: "rgba(255,255,255,0.35)" }}
            axisLine={false}
            tickLine={false}
          />
          <YAxis
            tick={{ fontSize: 10, fill: "rgba(255,255,255,0.35)" }}
            axisLine={false}
            tickLine={false}
            tickFormatter={fmt}
          />
          <Tooltip
            contentStyle={tooltipStyle}
            formatter={(val, name) => {
              if (name === "secondary") {
                return typeof val === "number"
                  ? [fmtSecondary(val), sLabel]
                  : [String(val), sLabel];
              }
              return typeof val === "number"
                ? [fmt(val), pLabel]
                : [String(val), pLabel];
            }}
            labelStyle={{ color: "rgba(255,255,255,0.5)" }}
          />
          <Line
            type="monotone"
            dataKey="value"
            stroke={color}
            strokeWidth={2}
            dot={false}
            activeDot={{ r: 4, fill: color }}
          />
          {hasSecondary && (
            <Line
              type="monotone"
              dataKey="secondary"
              stroke={secondaryColor}
              strokeWidth={1.5}
              dot={false}
              activeDot={{ r: 3, fill: secondaryColor }}
              strokeDasharray="4 3"
            />
          )}
        </LineChart>
      </ResponsiveContainer>
      {hasSecondary && (
        <div className="flex items-center gap-3 mt-1.5">
          <span className="flex items-center gap-1">
            <span
              className="inline-block w-[6px] h-[6px] rounded-full"
              style={{ backgroundColor: color }}
            />
            <span
              className="text-[10px]"
              style={{ color: "rgba(255,255,255,0.5)" }}
            >
              {pLabel}
            </span>
          </span>
          <span className="flex items-center gap-1">
            <span
              className="inline-block w-[6px] h-[6px] rounded-full"
              style={{ backgroundColor: secondaryColor }}
            />
            <span
              className="text-[10px]"
              style={{ color: "rgba(255,255,255,0.5)" }}
            >
              {sLabel}
            </span>
          </span>
        </div>
      )}
    </div>
  );
}
