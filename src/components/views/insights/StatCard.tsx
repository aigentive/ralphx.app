interface StatCardProps {
  label: string;
  value: string;
  sub?: string;
}

export function StatCard({ label, value, sub }: StatCardProps) {
  return (
    <div
      className="flex flex-col gap-1 rounded-xl"
      style={{ backgroundColor: "hsl(220 10% 12%)", padding: "14px 16px" }}
    >
      <span
        className="text-[11px] font-semibold uppercase tracking-wider"
        style={{ color: "rgba(255,255,255,0.4)", letterSpacing: "0.08em" }}
      >
        {label}
      </span>
      <span
        className="text-[22px] font-semibold"
        style={{ color: "rgba(255,255,255,0.92)", fontFamily: "system-ui" }}
      >
        {value}
      </span>
      {sub !== undefined && (
        <span className="text-[12px]" style={{ color: "rgba(255,255,255,0.4)" }}>
          {sub}
        </span>
      )}
    </div>
  );
}
