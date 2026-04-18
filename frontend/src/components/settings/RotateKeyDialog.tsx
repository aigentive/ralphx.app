/**
 * RotateKeyDialog - Three-step dialog for rotating an API key.
 *
 * Step 1 (confirm):  Warning about old key invalidation (after 60s grace).
 * Step 2 (rotating): Loading state while rotation runs.
 * Step 3 (reveal):   New raw key displayed ONCE with copy button (extends CreateKeyDialog pattern).
 */

import { useState, useCallback, useEffect } from "react";
import { Key, Copy, Check, AlertTriangle, RotateCcw } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { useRotateApiKey } from "@/hooks/useApiKeys";

// ============================================================================
// Props
// ============================================================================

export interface RotateKeyDialogProps {
  open: boolean;
  keyId: string;
  keyName: string;
  onClose: () => void;
  onRotated: () => void;
}

// ============================================================================
// Component
// ============================================================================

type Step = "confirm" | "rotating" | "reveal";

export function RotateKeyDialog({
  open,
  keyId,
  keyName,
  onClose,
  onRotated,
}: RotateKeyDialogProps) {
  const [step, setStep] = useState<Step>("confirm");
  const [rawKey, setRawKey] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const [hasCopied, setHasCopied] = useState(false);

  const rotateMutation = useRotateApiKey();

  // Reset on open
  useEffect(() => {
    if (open) {
      setStep("confirm");
      setRawKey(null);
      setError(null);
      setCopied(false);
      setHasCopied(false);
    }
  }, [open]);

  const handleRotate = useCallback(async () => {
    setStep("rotating");
    setError(null);
    try {
      const result = await rotateMutation.mutateAsync(keyId);
      setRawKey(result.rawKey);
      setStep("reveal");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Rotation failed");
      setStep("confirm");
    }
  }, [keyId, rotateMutation]);

  const handleCopy = useCallback(() => {
    if (!rawKey) return;
    void navigator.clipboard.writeText(rawKey).then(() => {
      setCopied(true);
      setHasCopied(true);
      setTimeout(() => setCopied(false), 2000);
    });
  }, [rawKey]);

  const handleDone = useCallback(() => {
    onRotated();
    onClose();
  }, [onRotated, onClose]);

  const handleOpenChange = useCallback(
    (isOpen: boolean) => {
      if (!isOpen) {
        if (step === "reveal") onRotated();
        onClose();
      }
    },
    [onClose, onRotated, step]
  );

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent
        data-testid="rotate-key-dialog"
        className="max-w-md"
        style={{
          background: "var(--bg-elevated)",
          border: "1px solid var(--border-subtle)",
        }}
      >
        {/* Header */}
        <DialogHeader>
          <div className="flex items-center gap-3">
            <div
              className="p-2 rounded-lg shrink-0"
              style={{
                background: "var(--accent-muted)",
                border: "1px solid var(--accent-border)",
              }}
            >
              <RotateCcw className="h-5 w-5 text-[var(--accent-primary)]" />
            </div>
            <div>
              <DialogTitle className="text-[var(--text-primary)]">
                {step === "reveal" ? "Key Rotated" : "Rotate API Key"}
              </DialogTitle>
              <DialogDescription className="mt-0.5 text-[var(--text-muted)]">
                {step === "reveal"
                  ? "Copy your new key — it will not be shown again"
                  : `Rotating: ${keyName}`}
              </DialogDescription>
            </div>
          </div>
        </DialogHeader>

        {/* Body */}
        <div className="px-6 py-4 space-y-4">
          {step === "confirm" && (
            <>
              {/* Warning */}
              <div
                className="rounded-lg px-3 py-2.5 flex items-start gap-2.5"
                style={{
                  background: "var(--accent-muted)",
                  border: "1px solid var(--accent-border)",
                }}
              >
                <AlertTriangle
                  className="w-4 h-4 shrink-0 mt-0.5"
                  style={{ color: "#ff6b35" }}
                />
                <p className="text-sm" style={{ color: "#ff6b35" }}>
                  The old key will remain valid for 60 seconds after rotation,
                  then expire. Any clients must update immediately.
                </p>
              </div>
              {error && (
                <p className="text-xs text-red-400 flex items-center gap-1.5">
                  <AlertTriangle className="w-3.5 h-3.5 shrink-0" />
                  {error}
                </p>
              )}
            </>
          )}

          {step === "rotating" && (
            <div className="py-4 flex items-center justify-center gap-3">
              <div className="w-4 h-4 border-2 border-[var(--accent-primary)] border-t-transparent rounded-full animate-spin" />
              <span className="text-sm text-[var(--text-muted)]">
                Rotating key…
              </span>
            </div>
          )}

          {step === "reveal" && (
            <>
              {/* Warning */}
              <div
                className="rounded-lg px-3 py-2.5 flex items-start gap-2.5"
                style={{
                  background: "var(--accent-muted)",
                  border: "1px solid var(--accent-border)",
                }}
              >
                <AlertTriangle
                  className="w-4 h-4 shrink-0 mt-0.5"
                  style={{ color: "#ff6b35" }}
                />
                <p className="text-sm" style={{ color: "#ff6b35" }}>
                  This new key will only be shown once. Copy it now and store
                  it securely.
                </p>
              </div>

              {/* Key display */}
              <div className="flex items-center gap-2">
                <div
                  className="flex-1 px-3 py-2 rounded-md font-mono text-sm text-[var(--text-primary)] break-all select-all"
                  style={{
                    background: "var(--alpha-black-30)",
                    border: "1px solid var(--border-subtle)",
                  }}
                >
                  {rawKey}
                </div>
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={handleCopy}
                  data-testid="copy-rotated-key-button"
                  className="shrink-0 h-9 w-9 hover:bg-[var(--bg-hover)]"
                  title="Copy to clipboard"
                >
                  {copied ? (
                    <Check className="w-4 h-4 text-green-400" />
                  ) : (
                    <Copy className="w-4 h-4 text-[var(--text-muted)]" />
                  )}
                </Button>
              </div>
            </>
          )}
        </div>

        {/* Footer */}
        <DialogFooter>
          {step === "confirm" && (
            <>
              <Button
                data-testid="cancel-rotate-button"
                type="button"
                variant="ghost"
                onClick={onClose}
                className="text-[var(--text-primary)] hover:bg-[var(--bg-hover)]"
              >
                Cancel
              </Button>
              <Button
                data-testid="confirm-rotate-button"
                type="button"
                onClick={() => void handleRotate()}
                style={{ background: "#ff6b35", color: "white" }}
                className="hover:opacity-90"
              >
                <Key className="w-3.5 h-3.5 mr-1.5" />
                Rotate Key
              </Button>
            </>
          )}

          {step === "reveal" && (
            <Button
              data-testid="done-rotate-button"
              type="button"
              onClick={handleDone}
              disabled={!hasCopied}
              title={!hasCopied ? "Copy the key before closing" : undefined}
              style={hasCopied ? { background: "#ff6b35", color: "white" } : undefined}
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
