/**
 * ResearchPanel - Launch and manage research sessions
 */

import { useState, useCallback } from "react";
import {
  Search,
  Sliders,
  Rocket,
  Loader2,
} from "lucide-react";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import type { ResearchDepthPreset, ResearchDepth, ResearchBrief } from "@/types/research";
import { RESEARCH_PRESET_INFO } from "@/types/research";
import { getDepthIcon } from "./ExtensibilityView.utils";

type DepthSelection = ResearchDepthPreset | "custom";

export function ResearchPanel() {
  const [question, setQuestion] = useState("");
  const [context, setContext] = useState("");
  const [scope, setScope] = useState("");
  const [selectedPreset, setSelectedPreset] = useState<DepthSelection>("standard");
  const [customIterations, setCustomIterations] = useState(100);
  const [customTimeout, setCustomTimeout] = useState(4);
  const [isLaunching, setIsLaunching] = useState(false);

  const isValid = question.trim().length > 0;
  const isCustom = selectedPreset === "custom";

  const handleLaunch = useCallback(() => {
    // Note: brief and depth are prepared for API integration but not yet used
    // @ts-expect-error Prepared for API integration
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    const brief: ResearchBrief = {
      question,
      constraints: [],
      ...(context && { context }),
      ...(scope && { scope }),
    };
    // @ts-expect-error Prepared for API integration
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    const depth: ResearchDepth = isCustom
      ? {
          type: "custom",
          config: {
            maxIterations: customIterations,
            timeoutHours: customTimeout,
            checkpointInterval: Math.ceil(customIterations / 10),
          },
        }
      : { type: "preset", preset: selectedPreset as ResearchDepthPreset };

    setIsLaunching(true);
    // Simulate launch - TODO: needs actual command call with brief/depth
    setTimeout(() => setIsLaunching(false), 2000);
  }, [question, context, scope, isCustom, selectedPreset, customIterations, customTimeout]);

  // Recent sessions mock
  const recentSessions = [
    {
      id: "1",
      question: "Best practices for state management in React",
      status: "complete" as const,
      preset: "standard",
      iterations: 45,
      duration: "2h 15m",
      date: "Jan 24",
    },
  ];

  return (
    <div data-testid="research-panel" className="space-y-6">
      {/* Research Launcher Card */}
      <Card
        className="max-w-xl mx-auto"
        style={{
          background: "rgba(255,255,255,0.04)",
          backdropFilter: "blur(20px)",
          WebkitBackdropFilter: "blur(20px)",
          border: "1px solid rgba(255,255,255,0.08)",
          boxShadow: "0 1px 3px rgba(0,0,0,0.12)",
        }}
      >
        <CardContent className="p-6 space-y-5">
          <h2
            className="text-lg font-semibold text-center"
            style={{
              color: "var(--text-primary)",
              letterSpacing: "-0.02em",
            }}
          >
            Launch New Research
          </h2>

          {/* Question */}
          <div className="space-y-2">
            <Label htmlFor="question" className="font-medium">
              Research Question
            </Label>
            <Textarea
              id="question"
              data-testid="question-input"
              value={question}
              onChange={(e) => setQuestion(e.target.value)}
              placeholder="What do you want to research?"
              rows={3}
              disabled={isLaunching}
            />
          </div>

          {/* Context & Scope */}
          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-2">
              <Label
                htmlFor="context"
                className="text-xs"
                style={{ color: "var(--text-secondary)" }}
              >
                Context (optional)
              </Label>
              <Input
                id="context"
                data-testid="context-input"
                value={context}
                onChange={(e) => setContext(e.target.value)}
                placeholder="Background information"
                disabled={isLaunching}
              />
            </div>
            <div className="space-y-2">
              <Label
                htmlFor="scope"
                className="text-xs"
                style={{ color: "var(--text-secondary)" }}
              >
                Scope (optional)
              </Label>
              <Input
                id="scope"
                data-testid="scope-input"
                value={scope}
                onChange={(e) => setScope(e.target.value)}
                placeholder="Limit the scope"
                disabled={isLaunching}
              />
            </div>
          </div>

          {/* Depth Preset Selector */}
          <div className="space-y-3">
            <Label className="font-medium">Research Depth</Label>
            <div
              data-testid="depth-preset-selector"
              role="radiogroup"
              aria-label="Research depth"
              className="grid grid-cols-2 gap-2"
            >
              {RESEARCH_PRESET_INFO.map((info) => {
                const isSelected = selectedPreset === info.preset;
                const IconComponent = getDepthIcon(info.preset);
                return (
                  <button
                    key={info.preset}
                    type="button"
                    role="radio"
                    aria-checked={isSelected}
                    data-testid={`preset-${info.preset}`}
                    onClick={() => setSelectedPreset(info.preset)}
                    disabled={isLaunching}
                    className="p-3 rounded-lg text-left transition-all duration-150"
                    style={{
                      backgroundColor: "var(--bg-base)",
                      border: isSelected
                        ? "2px solid var(--accent-primary)"
                        : "1px solid var(--border-subtle)",
                    }}
                  >
                    <IconComponent
                      className="w-4 h-4 mb-1"
                      style={{
                        color: isSelected
                          ? "var(--accent-primary)"
                          : "var(--text-muted)",
                      }}
                    />
                    <div
                      className="text-sm font-medium"
                      style={{ color: "var(--text-primary)" }}
                    >
                      {info.name}
                    </div>
                    <div className="text-xs" style={{ color: "var(--text-muted)" }}>
                      {info.config.maxIterations} iter, {info.config.timeoutHours}h
                    </div>
                  </button>
                );
              })}
              <button
                type="button"
                role="radio"
                aria-checked={isCustom}
                data-testid="preset-custom"
                onClick={() => setSelectedPreset("custom")}
                disabled={isLaunching}
                className="p-3 rounded-lg text-left transition-all duration-150"
                style={{
                  backgroundColor: "var(--bg-base)",
                  border: isCustom
                    ? "2px solid var(--accent-primary)"
                    : "1px solid var(--border-subtle)",
                }}
              >
                <Sliders
                  className="w-4 h-4 mb-1"
                  style={{
                    color: isCustom
                      ? "var(--accent-primary)"
                      : "var(--text-muted)",
                  }}
                />
                <div
                  className="text-sm font-medium"
                  style={{ color: "var(--text-primary)" }}
                >
                  Custom
                </div>
                <div className="text-xs" style={{ color: "var(--text-muted)" }}>
                  Set your own limits
                </div>
              </button>
            </div>
          </div>

          {/* Custom Depth Inputs */}
          {isCustom && (
            <div className="grid grid-cols-2 gap-3 animate-in slide-in-from-top-2 duration-200">
              <div className="space-y-2">
                <Label
                  htmlFor="iterations"
                  className="text-xs"
                  style={{ color: "var(--text-secondary)" }}
                >
                  Max Iterations
                </Label>
                <Input
                  id="iterations"
                  data-testid="custom-iterations-input"
                  type="number"
                  value={customIterations}
                  onChange={(e) => setCustomIterations(Number(e.target.value))}
                  min={1}
                  disabled={isLaunching}
                />
              </div>
              <div className="space-y-2">
                <Label
                  htmlFor="timeout"
                  className="text-xs"
                  style={{ color: "var(--text-secondary)" }}
                >
                  Timeout (hours)
                </Label>
                <Input
                  id="timeout"
                  data-testid="custom-timeout-input"
                  type="number"
                  value={customTimeout}
                  onChange={(e) => setCustomTimeout(Number(e.target.value))}
                  min={0.5}
                  step={0.5}
                  disabled={isLaunching}
                />
              </div>
            </div>
          )}

          {/* Actions */}
          <div className="flex justify-end gap-2 pt-2">
            <Button
              data-testid="cancel-button"
              variant="ghost"
              disabled={isLaunching}
            >
              Cancel
            </Button>
            <Button
              data-testid="launch-button"
              onClick={handleLaunch}
              disabled={!isValid || isLaunching}
              className="gap-1.5"
            >
              {isLaunching ? (
                <>
                  <Loader2 className="w-4 h-4 animate-spin" />
                  Launching...
                </>
              ) : (
                <>
                  <Rocket className="w-4 h-4" />
                  Launch Research
                </>
              )}
            </Button>
          </div>
        </CardContent>
      </Card>

      {/* Recent Sessions */}
      {recentSessions.length > 0 && (
        <div className="max-w-xl mx-auto space-y-3">
          <h3
            className="text-sm font-medium"
            style={{ color: "var(--text-secondary)" }}
          >
            Recent Research Sessions
          </h3>
          {recentSessions.map((session) => (
            <Card
              key={session.id}
              data-testid="session-card"
              className="cursor-pointer transition-all duration-180 hover:-translate-y-px"
              style={{
                background: "rgba(255,255,255,0.04)",
                backdropFilter: "blur(20px)",
                WebkitBackdropFilter: "blur(20px)",
                border: "1px solid rgba(255,255,255,0.08)",
                boxShadow: "0 1px 3px rgba(0,0,0,0.12)",
              }}
            >
              <CardContent className="p-4">
                <div className="flex items-center justify-between mb-2">
                  <div className="flex items-center gap-2">
                    <Search
                      className="w-4 h-4"
                      style={{ color: "var(--text-muted)" }}
                    />
                    <span
                      className="text-sm font-medium truncate"
                      style={{ color: "var(--text-primary)" }}
                    >
                      {session.question}
                    </span>
                  </div>
                  <Badge
                    variant={
                      session.status === "complete" ? "default" : "secondary"
                    }
                    className={
                      session.status === "complete"
                        ? "bg-emerald-500/10 text-emerald-400"
                        : ""
                    }
                  >
                    {session.status === "complete" ? "Complete" : session.status}
                  </Badge>
                </div>
                <div
                  className="text-xs flex items-center gap-2"
                  style={{ color: "var(--text-muted)" }}
                >
                  <span>{session.preset}</span>
                  <span>·</span>
                  <span>{session.iterations} iterations</span>
                  <span>·</span>
                  <span>{session.duration}</span>
                  <span>·</span>
                  <span>{session.date}</span>
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      )}
    </div>
  );
}
