/**
 * CreateKeyDialog - Two-step dialog for creating a new API key.
 *
 * Step 1 (input): Name field + Create button.
 * Step 2 (reveal): Shows the raw key ONCE with copy button and warning.
 *
 * The raw key is displayed only in the reveal step and never stored.
 */

import { useState, useCallback, useEffect } from "react";
import { Key, Copy, Check, AlertTriangle } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useCreateApiKey } from "@/hooks/useApiKeys";

// ============================================================================
// Props
// ============================================================================

export interface CreateKeyDialogProps {
  open: boolean;
  onClose: () => void;
  onCreated: () => void;
}

// ============================================================================
// Component
// ============================================================================

type Step = "input" | "reveal";

export function CreateKeyDialog({ open, onClose, onCreated }: CreateKeyDialogProps) {
  const [step, setStep] = useState<Step>("input");
  const [name, setName] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [rawKey, setRawKey] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const [hasCopied, setHasCopied] = useState(false);

  const createMutation = useCreateApiKey();

  // Reset on open
  useEffect(() => {
    if (open) {
      setStep("input");
      setName("");
      setError(null);
      setRawKey(null);
      setCopied(false);
      setHasCopied(false);
      createMutation.reset();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);

  const handleCreate = useCallback(async () => {
    const trimmedName = name.trim();
    if (!trimmedName) {
      setError("Key name is required");
      return;
    }

    setError(null);

    try {
      const result = await createMutation.mutateAsync({
        name: trimmedName,
        projectIds: [],
        permissions: 7,
      });
      setRawKey(result.rawKey);
      setStep("reveal");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to create key");
    }
  }, [name, createMutation]);

  const handleCopy = useCallback(() => {
    if (!rawKey) return;
    void navigator.clipboard.writeText(rawKey).then(() => {
      setCopied(true);
      setHasCopied(true);
      setTimeout(() => setCopied(false), 2000);
    });
  }, [rawKey]);

  const handleDone = useCallback(() => {
    onCreated();
    onClose();
  }, [onCreated, onClose]);

  const handleOpenChange = useCallback(
    (isOpen: boolean) => {
      if (!isOpen) {
        // If on reveal step, treat close as "done"
        if (step === "reveal") {
          onCreated();
        }
        onClose();
      }
    },
    [onClose, onCreated, step]
  );

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Enter" && step === "input" && !createMutation.isPending) {
        e.preventDefault();
        void handleCreate();
      }
    },
    [step, createMutation.isPending, handleCreate]
  );

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent
        data-testid="create-key-dialog"
        className="max-w-md"
        style={{
          background: "var(--bg-elevated)",
          border: "1px solid rgba(255,255,255,0.08)",
        }}
      >
        {/* Header */}
        <DialogHeader>
          <div className="flex items-center gap-3">
            <div
              className="p-2 rounded-lg shrink-0"
              style={{
                background: "rgba(255,107,53,0.1)",
                border: "1px solid rgba(255,107,53,0.2)",
              }}
            >
              <Key className="h-5 w-5 text-[var(--accent-primary)]" />
            </div>
            <div>
              <DialogTitle className="text-[var(--text-primary)]">
                {step === "input" ? "Create API Key" : "Key Created"}
              </DialogTitle>
              <DialogDescription className="mt-0.5 text-[var(--text-muted)]">
                {step === "input"
                  ? "API keys allow external tools to access RalphX"
                  : "Copy your key now — it will not be shown again"}
              </DialogDescription>
            </div>
          </div>
        </DialogHeader>

        {/* Body */}
        <div className="px-6 py-4 space-y-4">
          {step === "input" ? (
            <>
              <div className="space-y-1.5">
                <Label
                  htmlFor="key-name"
                  className="text-sm font-medium text-[var(--text-secondary)]"
                >
                  Key Name
                </Label>
                <Input
                  id="key-name"
                  data-testid="key-name-input"
                  value={name}
                  onChange={(e) => {
                    setName(e.target.value);
                    setError(null);
                  }}
                  onKeyDown={handleKeyDown}
                  placeholder="e.g. CI / Staging server"
                  disabled={createMutation.isPending}
                  autoFocus
                  className="bg-[var(--bg-surface)] border-[var(--border-default)] focus:border-[var(--accent-primary)] focus:ring-[var(--accent-primary)] text-sm"
                />
              </div>
              {error && (
                <p className="text-xs text-red-400 flex items-center gap-1.5">
                  <AlertTriangle className="w-3.5 h-3.5 shrink-0" />
                  {error}
                </p>
              )}
            </>
          ) : (
            <>
              {/* Warning banner */}
              <div
                className="rounded-lg px-3 py-2.5 flex items-start gap-2.5"
                style={{
                  background: "rgba(255,107,53,0.08)",
                  border: "1px solid rgba(255,107,53,0.25)",
                }}
              >
                <AlertTriangle
                  className="w-4 h-4 shrink-0 mt-0.5"
                  style={{ color: "#ff6b35" }}
                />
                <p className="text-sm" style={{ color: "#ff6b35" }}>
                  This key will only be shown once. Copy it now and store it
                  securely.
                </p>
              </div>

              {/* Key display */}
              <div className="space-y-1.5">
                <Label className="text-sm font-medium text-[var(--text-secondary)]">
                  Your API Key
                </Label>
                <div className="flex items-center gap-2">
                  <div
                    className="flex-1 px-3 py-2 rounded-md font-mono text-sm text-[var(--text-primary)] break-all select-all"
                    style={{
                      background: "rgba(0,0,0,0.3)",
                      border: "1px solid rgba(255,255,255,0.08)",
                    }}
                  >
                    {rawKey}
                  </div>
                  <Button
                    variant="ghost"
                    size="icon"
                    onClick={handleCopy}
                    data-testid="copy-key-button"
                    className="shrink-0 h-9 w-9 hover:bg-[var(--bg-surface-hover)]"
                    title="Copy to clipboard"
                  >
                    {copied ? (
                      <Check className="w-4 h-4 text-green-400" />
                    ) : (
                      <Copy className="w-4 h-4 text-[var(--text-muted)]" />
                    )}
                  </Button>
                </div>
              </div>
            </>
          )}
        </div>

        {/* Footer */}
        <DialogFooter>
          {step === "input" ? (
            <>
              <Button
                data-testid="cancel-button"
                type="button"
                variant="ghost"
                onClick={onClose}
                disabled={createMutation.isPending}
                className="text-[var(--text-primary)] hover:bg-[var(--bg-hover)]"
              >
                Cancel
              </Button>
              <Button
                data-testid="create-button"
                type="button"
                onClick={() => void handleCreate()}
                disabled={createMutation.isPending || !name.trim()}
                style={{
                  background: "#ff6b35",
                  color: "white",
                }}
                className="hover:opacity-90"
              >
                {createMutation.isPending ? "Creating..." : "Create Key"}
              </Button>
            </>
          ) : (
            <Button
              data-testid="done-button"
              type="button"
              onClick={handleDone}
              disabled={!hasCopied}
              title={!hasCopied ? "Copy the key before closing" : undefined}
              style={
                hasCopied
                  ? { background: "#ff6b35", color: "white" }
                  : undefined
              }
              className={hasCopied ? "hover:opacity-90" : "opacity-60"}
            >
              Done
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
