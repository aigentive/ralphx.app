import { useState } from "react";
import { ChevronDown, ChevronRight } from "lucide-react";
import { DetailCard } from "@/components/tasks/detail-views/shared/DetailCard";

const ACCENT = "#ff6b35";

interface EffortEstimationPanelProps {
  lowHours: number;
  highHours: number;
  taskCount: number;
}

export function EffortEstimationPanel({ lowHours, highHours, taskCount }: EffortEstimationPanelProps) {
  const [expanded, setExpanded] = useState(false);

  return (
    <DetailCard variant="accent">
      <div className="flex flex-col gap-3">
        <div className="flex items-start justify-between gap-4">
          <div className="flex flex-col gap-1">
            <span
              className="text-[11px] font-semibold uppercase tracking-wider"
              style={{ color: "rgba(255,255,255,0.4)", letterSpacing: "0.08em" }}
            >
              Remaining Work Estimate
            </span>
            <span
              className="text-[24px] font-semibold"
              style={{ color: ACCENT, fontFamily: "system-ui" }}
            >
              ~{lowHours}–{highHours}h
            </span>
            <span className="text-[12px]" style={{ color: "rgba(255,255,255,0.4)" }}>
              Based on {taskCount} completed task{taskCount !== 1 ? "s" : ""}
            </span>
          </div>
        </div>

        <button
          onClick={() => setExpanded((v) => !v)}
          className="flex items-center gap-1.5 text-[12px] w-fit"
          style={{ color: "rgba(255,255,255,0.45)" }}
        >
          {expanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
          Methodology
        </button>

        {expanded && (
          <div
            className="text-[12px] leading-relaxed rounded-lg p-3"
            style={{
              color: "rgba(255,255,255,0.55)",
              backgroundColor: "rgba(255,255,255,0.04)",
            }}
          >
            <p className="mb-2 font-medium" style={{ color: "rgba(255,255,255,0.7)" }}>
              Effort Model Estimate (EME)
            </p>
            <ul className="flex flex-col gap-1.5">
              <li>
                <span style={{ color: "rgba(255,255,255,0.7)" }}>Simple</span> (≤3 steps):
                2h base × 1.5 calendar factor
              </li>
              <li>
                <span style={{ color: "rgba(255,255,255,0.7)" }}>Medium</span> (4–7 steps
                or 1 review): 4h base × 1.5 calendar factor
              </li>
              <li>
                <span style={{ color: "rgba(255,255,255,0.7)" }}>Complex</span> (8+ steps
                or 2+ reviews): 8h base × 1.5 calendar factor
              </li>
            </ul>
            <p className="mt-2" style={{ color: "rgba(255,255,255,0.35)" }}>
              Range reflects uncertainty: low = 80% of estimate, high = 120%.
            </p>
          </div>
        )}
      </div>
    </DetailCard>
  );
}
