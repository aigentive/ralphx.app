/**
 * MethodologiesPanel - Methodology management with activation
 */

import { BookOpen } from "lucide-react";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import type { MethodologyExtension } from "@/types/methodology";

interface MethodologiesPanelProps {
  methodologies: MethodologyExtension[];
  onActivate: (id: string) => void;
  onDeactivate: (id: string) => void;
}

export function MethodologiesPanel({
  methodologies,
  onActivate,
  onDeactivate,
}: MethodologiesPanelProps) {
  const isEmpty = methodologies.length === 0;

  return (
    <div data-testid="methodologies-panel" className="space-y-4">
      {/* Header */}
      <div>
        <h2
          className="text-lg font-semibold"
          style={{
            color: "var(--text-primary)",
            letterSpacing: "-0.02em",
          }}
        >
          Development Methodologies
        </h2>
        <p className="text-sm" style={{ color: "var(--text-secondary)" }}>
          Choose how RalphX organizes work
        </p>
      </div>

      {/* Empty State */}
      {isEmpty ? (
        <div className="flex flex-col items-center justify-center py-16">
          <div
            className="w-16 h-16 rounded-xl flex items-center justify-center mb-4"
            style={{ border: "2px dashed var(--border-subtle)" }}
          >
            <BookOpen
              className="w-8 h-8"
              style={{ color: "var(--text-muted)" }}
            />
          </div>
          <p
            className="text-sm font-medium mb-1"
            style={{ color: "var(--text-secondary)" }}
          >
            No methodologies available
          </p>
          <p className="text-xs" style={{ color: "var(--text-muted)" }}>
            Configure methodologies in the plugin directory
          </p>
        </div>
      ) : (
        /* Methodology Cards */
        <div className="space-y-4">
          {methodologies.map((methodology) => (
            <Card
              key={methodology.id}
              data-testid="methodology-card"
              data-active={methodology.isActive ? "true" : "false"}
              className="cursor-pointer transition-all duration-180 hover:-translate-y-px"
              style={{
                background: methodology.isActive
                  ? "rgba(255,107,53,0.08)"
                  : "rgba(255,255,255,0.04)",
                backdropFilter: "blur(20px)",
                WebkitBackdropFilter: "blur(20px)",
                border: methodology.isActive
                  ? "1px solid rgba(255,107,53,0.25)"
                  : "1px solid rgba(255,255,255,0.08)",
                boxShadow: methodology.isActive
                  ? "0 0 0 1px rgba(255,107,53,0.15), 0 2px 8px rgba(0,0,0,0.15)"
                  : "0 1px 3px rgba(0,0,0,0.12)",
              }}
            >
              <CardContent className="p-5">
                {/* Header Row */}
                <div className="flex items-center justify-between mb-3">
                  <div className="flex items-center gap-3">
                    <div
                      className={`w-2.5 h-2.5 rounded-full ${
                        methodology.isActive ? "animate-pulse" : ""
                      }`}
                      style={{
                        backgroundColor: methodology.isActive
                          ? "var(--accent-primary)"
                          : "var(--border-subtle)",
                        boxShadow: methodology.isActive
                          ? "0 0 0 4px rgba(255, 107, 53, 0.1)"
                          : undefined,
                      }}
                    />
                    <span
                      className="text-base font-semibold"
                      style={{ color: "var(--text-primary)" }}
                    >
                      {methodology.name}
                    </span>
                    {methodology.isActive && (
                      <Badge className="bg-emerald-500/10 text-emerald-400 border-0">
                        ACTIVE
                      </Badge>
                    )}
                  </div>
                  {methodology.isActive ? (
                    <Button
                      data-testid="deactivate-button"
                      variant="ghost"
                      size="sm"
                      onClick={(e) => {
                        e.stopPropagation();
                        onDeactivate(methodology.id);
                      }}
                    >
                      Deactivate
                    </Button>
                  ) : (
                    <Button
                      data-testid="activate-button"
                      size="sm"
                      onClick={(e) => {
                        e.stopPropagation();
                        onActivate(methodology.id);
                      }}
                    >
                      Activate
                    </Button>
                  )}
                </div>

                {/* Description */}
                <p
                  className="text-sm leading-relaxed mb-3"
                  style={{ color: "var(--text-secondary)" }}
                >
                  {methodology.description}
                </p>

                {/* Stats */}
                <div
                  className="text-xs flex items-center gap-3"
                  style={{ color: "var(--text-muted)" }}
                >
                  <span data-testid="phase-count">
                    {methodology.phases.length} phases
                  </span>
                  <span>·</span>
                  <span data-testid="agent-count">
                    {methodology.agentProfiles.length} agents
                  </span>
                  <span>·</span>
                  <span>{methodology.workflow.name} workflow</span>
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      )}
    </div>
  );
}
