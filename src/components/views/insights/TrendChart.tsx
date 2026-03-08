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
}

export function TrendChart({ title, data, valueFormatter, color = ACCENT }: TrendChartProps) {
  const chartData = data.map((pt) => ({
    week: formatWeek(pt.weekStart),
    value: pt.value,
  }));

  const fmt = valueFormatter ?? ((v: number) => String(v));

  if (chartData.length === 0) {
    return (
      <div>
        <p
          className="text-[12px] font-medium mb-3"
          style={{ color: "rgba(255,255,255,0.7)" }}
        >
          {title}
        </p>
        <p className="text-[12px]" style={{ color: "rgba(255,255,255,0.3)" }}>
          No data yet
        </p>
      </div>
    );
  }

  return (
    <div>
      <p
        className="text-[12px] font-medium mb-3"
        style={{ color: "rgba(255,255,255,0.7)" }}
      >
        {title}
      </p>
      <ResponsiveContainer width="100%" height={120}>
        <LineChart data={chartData} margin={{ top: 4, right: 4, left: -20, bottom: 0 }}>
          <CartesianGrid
            strokeDasharray="3 3"
            stroke="rgba(255,255,255,0.06)"
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
            formatter={(val) =>
              typeof val === "number" ? [fmt(val), title] : [String(val), title]
            }
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
        </LineChart>
      </ResponsiveContainer>
    </div>
  );
}
