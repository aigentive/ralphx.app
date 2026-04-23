/**
 * ExternalMcpSettingsPanel - Settings for External MCP server configuration.
 *
 * All fields require app restart to take effect (runtime config stored in ralphx.yaml).
 * Auth token is masked on load — shows "••••••••" placeholder when set, empty when unset.
 */

import { useState, useEffect } from "react";
import { Server, RefreshCw } from "lucide-react";
import { Card } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import { Switch } from "@/components/ui/switch";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import {
  getExternalMcpConfig,
  updateExternalMcpConfig,
  type ExternalMcpConfigView,
  type ExternalMcpConfigUpdate,
} from "@/api/external-mcp";

// ============================================================================
// Restart Notice (card-level)
// ============================================================================

function RestartNotice() {
  return (
    <div
      data-testid="restart-required-badge"
      className="mx-5 mb-2 flex items-center gap-2 rounded-md border border-[var(--accent-border)] bg-[var(--accent-muted)] px-3 py-2 text-xs text-[var(--accent-primary)]"
    >
      <RefreshCw className="h-3 w-3 shrink-0" />
      <span>All fields below require an app restart to take effect.</span>
    </div>
  );
}

// ============================================================================
// Field Row
// ============================================================================

interface FieldRowProps {
  id: string;
  label: string;
  description: string;
  children: React.ReactNode;
}

function FieldRow({ id, label, description, children }: FieldRowProps) {
  return (
    <div className="flex items-start justify-between py-3 border-b border-[var(--border-subtle)] last:border-0 -mx-2 px-2 rounded-md transition-colors hover:bg-[var(--bg-hover)]">
      <div className="flex-1 min-w-0 pr-4">
        <Label
          htmlFor={id}
          className="text-sm font-medium text-[var(--text-primary)]"
        >
          {label}
        </Label>
        <p id={`${id}-desc`} className="text-xs text-[var(--text-muted)] mt-0.5">
          {description}
        </p>
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  );
}

// ============================================================================
// ExternalMcpSettingsPanel
// ============================================================================

type FormState = {
  enabled: boolean;
  port: string;
  host: string;
  authToken: string;
  nodePath: string;
};

const AUTH_TOKEN_MASK = "••••••••";

export function ExternalMcpSettingsPanel() {
  const [config, setConfig] = useState<ExternalMcpConfigView | null>(null);
  const [form, setForm] = useState<FormState>({
    enabled: false,
    port: "3848",
    host: "127.0.0.1",
    authToken: "",
    nodePath: "",
  });
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [saveStatus, setSaveStatus] = useState<"idle" | "success" | "error">("idle");
  const [errorMessage, setErrorMessage] = useState("");

  useEffect(() => {
    setIsLoading(true);
    getExternalMcpConfig()
      .then((data) => {
        setConfig(data);
        setForm({
          enabled: data.enabled,
          port: String(data.port),
          host: data.host,
          // Show mask placeholder if token is set; otherwise empty
          authToken: data.authToken != null ? AUTH_TOKEN_MASK : "",
          nodePath: data.nodePath ?? "",
        });
      })
      .catch((err: unknown) => {
        setErrorMessage(err instanceof Error ? err.message : "Failed to load config");
        setSaveStatus("error");
      })
      .finally(() => setIsLoading(false));
  }, []);

  const handleSave = async () => {
    setIsSaving(true);
    setSaveStatus("idle");
    setErrorMessage("");

    const update: ExternalMcpConfigUpdate = {};

    if (form.enabled !== config?.enabled) {
      update.enabled = form.enabled;
    }

    const portNum = parseInt(form.port, 10);
    if (!isNaN(portNum) && portNum !== config?.port) {
      update.port = portNum;
    }

    if (form.host !== config?.host) {
      update.host = form.host;
    }

    // Only send authToken if it was changed (i.e., not still showing the mask)
    if (form.authToken !== AUTH_TOKEN_MASK) {
      update.authToken = form.authToken;
    }

    const configNodePath = config?.nodePath ?? "";
    if (form.nodePath !== configNodePath && form.nodePath !== "") {
      update.nodePath = form.nodePath;
    }

    try {
      await updateExternalMcpConfig(update);
      setSaveStatus("success");
      // Refresh config to get updated masked state
      const updated = await getExternalMcpConfig();
      setConfig(updated);
      setForm((f) => ({
        ...f,
        authToken: updated.authToken != null ? AUTH_TOKEN_MASK : "",
      }));
    } catch (err: unknown) {
      setErrorMessage(err instanceof Error ? err.message : "Failed to save config");
      setSaveStatus("error");
    } finally {
      setIsSaving(false);
    }
  };

  if (isLoading) {
    return (
      <Card className="bg-[var(--bg-elevated)] border-[var(--border-default)] p-5">
        <p className="text-sm text-[var(--text-muted)]">Loading...</p>
      </Card>
    );
  }

  return (
    <Card
      className={cn(
        "bg-[var(--bg-elevated)] border-[var(--border-default)] shadow-[var(--shadow-xs)]",
        "border border-transparent",
        "bg-[var(--bg-elevated)]"
      )}
    >
      <div className="flex items-start gap-3 p-5 pb-0">
        <div className="p-2 rounded-lg bg-[var(--accent-muted)] shrink-0">
          <Server className="w-[18px] h-[18px] text-[var(--card-icon-color)]" />
        </div>
        <div>
          <h3 className="text-sm font-semibold tracking-tight text-[var(--text-primary)]">
            External MCP
          </h3>
          <p className="text-xs text-[var(--text-muted)] mt-0.5">
            Configure external MCP server access (restart required to apply)
          </p>
        </div>
      </div>
      <Separator className="my-4 bg-[var(--border-subtle)]" />
      <RestartNotice />
      <div className="px-5 pb-5 space-y-1">
        {/* Enabled toggle */}
        <FieldRow
          id="ext-mcp-enabled"
          label="Enabled"
          description="Allow external agents to connect via the MCP server"
        >
          <Switch
            id="ext-mcp-enabled"
            data-testid="ext-mcp-enabled"
            checked={form.enabled}
            onCheckedChange={(checked) =>
              setForm((f) => ({ ...f, enabled: checked }))
            }
            disabled={isSaving}
            aria-describedby="ext-mcp-enabled-desc"
            className="data-[state=checked]:bg-[var(--accent-primary)]"
          />
        </FieldRow>

        {/* Host */}
        <FieldRow
          id="ext-mcp-host"
          label="Host"
          description="Bind address for the external MCP server"
        >
          <Input
            id="ext-mcp-host"
            data-testid="ext-mcp-host"
            aria-describedby="ext-mcp-host-desc"
            value={form.host}
            onChange={(e) => setForm((f) => ({ ...f, host: e.target.value }))}
            disabled={isSaving}
            className="w-[200px] bg-[var(--bg-surface)] border-[var(--border-default)] focus:border-[var(--accent-primary)] focus:ring-[var(--accent-primary)]"
          />
        </FieldRow>

        {/* Port */}
        <FieldRow
          id="ext-mcp-port"
          label="Port"
          description="TCP port the external MCP server listens on"
        >
          <Input
            id="ext-mcp-port"
            data-testid="ext-mcp-port"
            aria-describedby="ext-mcp-port-desc"
            type="number"
            value={form.port}
            onChange={(e) => setForm((f) => ({ ...f, port: e.target.value }))}
            disabled={isSaving}
            min={1}
            max={65535}
            className="w-[120px] bg-[var(--bg-surface)] border-[var(--border-default)] focus:border-[var(--accent-primary)] focus:ring-[var(--accent-primary)] [appearance:textfield] [&::-webkit-outer-spin-button]:appearance-none [&::-webkit-inner-spin-button]:appearance-none"
          />
        </FieldRow>

        {/* Auth Token */}
        <FieldRow
          id="ext-mcp-auth-token"
          label="Auth Token"
          description="Bearer token required for external agent connections (leave blank to disable)"
        >
          <Input
            id="ext-mcp-auth-token"
            data-testid="ext-mcp-auth-token"
            aria-describedby="ext-mcp-auth-token-desc"
            type="password"
            value={form.authToken}
            placeholder={config?.authToken != null ? AUTH_TOKEN_MASK : ""}
            onChange={(e) => setForm((f) => ({ ...f, authToken: e.target.value }))}
            disabled={isSaving}
            className="w-[200px] bg-[var(--bg-surface)] border-[var(--border-default)] focus:border-[var(--accent-primary)] focus:ring-[var(--accent-primary)]"
          />
        </FieldRow>

        {/* Node Path */}
        <FieldRow
          id="ext-mcp-node-path"
          label="Node Path"
          description="Path to Node.js binary used to launch the MCP server (leave blank for system default)"
        >
          <Input
            id="ext-mcp-node-path"
            data-testid="ext-mcp-node-path"
            aria-describedby="ext-mcp-node-path-desc"
            value={form.nodePath}
            placeholder="/usr/local/bin/node"
            onChange={(e) => setForm((f) => ({ ...f, nodePath: e.target.value }))}
            disabled={isSaving}
            className="w-[200px] bg-[var(--bg-surface)] border-[var(--border-default)] focus:border-[var(--accent-primary)] focus:ring-[var(--accent-primary)]"
          />
        </FieldRow>

        {/* Save button and feedback */}
        <div className="flex items-center justify-between pt-3">
          <div className="text-xs">
            {saveStatus === "success" && (
              <span className="text-[var(--status-success)]">
                Saved — restart the app to apply changes
              </span>
            )}
            {saveStatus === "error" && (
              <span className="text-[var(--status-error)]">
                {errorMessage || "Failed to save"}
              </span>
            )}
          </div>
          <Button
            data-testid="ext-mcp-save"
            onClick={() => void handleSave()}
            disabled={isSaving}
            size="sm"
            className="bg-[var(--accent-primary)] hover:bg-[var(--accent-hover)] text-[var(--text-on-accent)]"
          >
            {isSaving ? "Saving..." : "Save"}
          </Button>
        </div>
      </div>
    </Card>
  );
}
