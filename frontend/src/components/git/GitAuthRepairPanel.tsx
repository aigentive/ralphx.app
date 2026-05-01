import {
  AlertTriangle,
  CheckCircle2,
  Copy,
  GitBranch,
  KeyRound,
  Loader2,
  RefreshCw,
} from "lucide-react";
import type { ReactNode } from "react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { useConfirmation } from "@/hooks/useConfirmation";
import {
  useGhAuthStatus,
  useGitAuthDiagnostics,
  useResumeDeferredGitStartup,
  useSetupGhGitAuth,
  useSwitchGitOriginToSsh,
} from "@/hooks/useGithubSettings";
import type { GitAuthDiagnostics } from "@/hooks/useGithubSettings";

function authModeLabel(fetchKind: string | null | undefined, pushKind: string | null | undefined) {
  if (!fetchKind && !pushKind) {
    return "No origin";
  }
  return `Fetch ${fetchKind ?? "unknown"} / Push ${pushKind ?? "unknown"}`;
}

function errorMessage(error: unknown, fallback: string) {
  return error instanceof Error ? error.message : fallback;
}

function isGithubHttpsRemote(url: string | null | undefined) {
  return url?.trim().startsWith("https://github.com/") ?? false;
}

function hasDeferredStartupBlockingIssue(
  diagnostics: GitAuthDiagnostics | undefined,
  ghAuthenticated: boolean | undefined,
  diagnosticsFailed = false,
) {
  if (diagnosticsFailed) {
    return true;
  }
  if (!diagnostics) {
    return false;
  }
  if (diagnostics.mixedAuthModes) {
    return true;
  }
  if (ghAuthenticated === false) {
    return true;
  }
  return false;
}

export function GitAuthRepairPanel({
  projectId,
  surface = "settings",
  showWhenHealthy = false,
}: {
  projectId: string | null;
  surface?: "settings" | "publish";
  showWhenHealthy?: boolean;
}) {
  const diagnosticsQuery = useGitAuthDiagnostics(projectId);
  const ghAuthQuery = useGhAuthStatus();
  const switchToSshMutation = useSwitchGitOriginToSsh();
  const setupGhGitAuthMutation = useSetupGhGitAuth();
  const resumeDeferredGitStartupMutation = useResumeDeferredGitStartup();
  const { confirm, confirmationDialogProps, ConfirmationDialog } = useConfirmation();

  if (!projectId) {
    return null;
  }

  const diagnostics = diagnosticsQuery.data;
  const isGhAuthed = ghAuthQuery.data === true;
  const isGhMissing = ghAuthQuery.data === false;
  const isChecking = diagnosticsQuery.isLoading || ghAuthQuery.isLoading;
  const hasHttpsRemote =
    diagnostics?.fetchKind === "HTTPS" || diagnostics?.pushKind === "HTTPS";
  const hasGithubHttpsRemote =
    isGithubHttpsRemote(diagnostics?.fetchUrl) ||
    isGithubHttpsRemote(diagnostics?.pushUrl);
  const canSetupGithubHttps = isGhAuthed && hasGithubHttpsRemote;
  const canCopyGithubHttpsSetup = isGhMissing && hasGithubHttpsRemote;
  const hasRepairAction =
    Boolean(diagnostics?.canSwitchToSsh) || canSetupGithubHttps || canCopyGithubHttpsSetup;
  const hasVisibleIssue =
    diagnosticsQuery.isError ||
    ghAuthQuery.isError ||
    diagnostics?.mixedAuthModes ||
    hasHttpsRemote ||
    isGhMissing;

  if (!showWhenHealthy && !isChecking && !hasVisibleIssue && !hasRepairAction) {
    return null;
  }

  const messages: ReactNode[] = [];
  if (diagnosticsQuery.isError) {
    messages.push("Could not inspect the git origin for this project.");
  }
  if (diagnostics?.mixedAuthModes) {
    messages.push("Fetch and push use different auth modes. Installed app fetches can fail even when terminal pushes work.");
  }
  if (isGhMissing) {
    messages.push(
      <>
        GitHub CLI is not authenticated. Run{" "}
        <span className="font-mono">gh auth login</span>, then recheck.
      </>,
    );
  }
  if (canSetupGithubHttps) {
    messages.push("HTTPS remotes need a non-interactive credential. Configure GitHub CLI credentials or switch origin to SSH.");
  } else if (hasHttpsRemote) {
    messages.push("HTTPS remotes need a non-interactive credential before the installed app can fetch or push.");
  }
  if (canCopyGithubHttpsSetup) {
    messages.push("HTTPS is still available: authenticate GitHub CLI, then let RalphX configure Git credentials.");
  }
  if (messages.length === 0 && !isChecking) {
    messages.push("Git remote auth and GitHub CLI status look ready.");
  }

  const handleRecheck = async () => {
    const [diagnosticsResult, ghAuthResult] = await Promise.all([
      diagnosticsQuery.refetch(),
      ghAuthQuery.refetch(),
    ]);
    return {
      diagnostics: diagnosticsResult.data,
      ghAuthenticated: ghAuthResult.data,
      diagnosticsFailed: diagnosticsResult.isError,
    };
  };

  const resumeDeferredStartupIfHealthy = async () => {
    const current = await handleRecheck();
    if (
      hasDeferredStartupBlockingIssue(
        current.diagnostics,
        current.ghAuthenticated,
        current.diagnosticsFailed,
      )
    ) {
      return;
    }

    const resumed = await resumeDeferredGitStartupMutation.mutateAsync();
    if (resumed) {
      toast.success("Deferred startup recovery resumed");
    }
  };

  const handleCopyGithubHttpsSetup = async () => {
    try {
      await navigator.clipboard.writeText("gh auth login && gh auth setup-git");
      toast.success("GitHub HTTPS setup command copied");
    } catch {
      toast.error("Failed to copy GitHub HTTPS setup command");
    }
  };

  const handleSwitchToSsh = async () => {
    const suggestedUrl = diagnostics?.suggestedSshUrl ?? "the SSH origin URL";
    const confirmed = await confirm({
      title: "Switch origin to SSH?",
      description: `This updates this project's origin fetch and push URLs to ${suggestedUrl}. Future git operations for this checkout will use SSH keys.`,
      confirmText: "Use SSH",
    });
    if (!confirmed) {
      return;
    }

    try {
      await switchToSshMutation.mutateAsync({ projectId });
      toast.success("Git origin switched to SSH");
      await resumeDeferredStartupIfHealthy();
    } catch (error) {
      toast.error(errorMessage(error, "Failed to switch git origin to SSH"));
    }
  };

  const handleSetupGhGitAuth = async () => {
    const confirmed = await confirm({
      title: "Set up GitHub HTTPS credentials?",
      description:
        "This runs gh auth setup-git with the app's resolved GitHub CLI and updates git credential configuration for GitHub HTTPS remotes.",
      confirmText: "Setup credentials",
    });
    if (!confirmed) {
      return;
    }

    try {
      await setupGhGitAuthMutation.mutateAsync();
      toast.success("GitHub HTTPS credentials configured");
      await resumeDeferredStartupIfHealthy();
    } catch (error) {
      toast.error(errorMessage(error, "Failed to configure GitHub HTTPS credentials"));
    }
  };

  const panelClassName =
    surface === "publish"
      ? "rounded-lg border p-4"
      : "rounded-md border px-3 py-2";

  return (
    <div
      className={panelClassName}
      style={{
        background: surface === "publish" ? "var(--bg-surface)" : "var(--bg-subtle)",
        borderColor: "var(--border-subtle)",
      }}
      data-testid="git-auth-repair-panel"
    >
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="flex min-w-0 items-center gap-2">
          {isChecking ? (
            <Loader2 className="h-4 w-4 shrink-0 animate-spin text-[var(--text-muted)]" />
          ) : hasVisibleIssue ? (
            <AlertTriangle className="h-4 w-4 shrink-0 text-[var(--status-warning)]" />
          ) : (
            <CheckCircle2 className="h-4 w-4 shrink-0 text-status-success" />
          )}
          <div className="min-w-0">
            <div className="text-xs font-semibold text-[var(--text-primary)]">
              Git & GitHub Access
            </div>
            <div className="truncate text-[11px] text-[var(--text-muted)]">
              {isChecking
                ? "Checking repository credentials..."
                : authModeLabel(diagnostics?.fetchKind, diagnostics?.pushKind)}
            </div>
          </div>
        </div>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="h-7 gap-1 px-2 text-[11px]"
          onClick={() => void resumeDeferredStartupIfHealthy()}
          disabled={isChecking || resumeDeferredGitStartupMutation.isPending}
          data-testid="git-auth-recheck"
        >
          <RefreshCw className="h-3.5 w-3.5" />
          Recheck
        </Button>
      </div>

      <div className="mt-2 space-y-1 text-xs leading-relaxed text-[var(--text-secondary)]">
        {messages.map((message, index) => (
          <div key={index}>{message}</div>
        ))}
        {diagnosticsQuery.isError && (
          <div className="text-[var(--status-warning)]">
            {errorMessage(diagnosticsQuery.error, "Git diagnostics failed")}
          </div>
        )}
      </div>

      {(diagnostics?.canSwitchToSsh || canSetupGithubHttps || canCopyGithubHttpsSetup) && (
        <div className="mt-3 flex flex-wrap items-center gap-2">
          {diagnostics?.canSwitchToSsh && (
            <Button
              type="button"
              size="sm"
              className="h-8 gap-2 px-3 text-xs"
              onClick={() => void handleSwitchToSsh()}
              disabled={switchToSshMutation.isPending}
              data-testid="git-auth-switch-ssh"
            >
              {switchToSshMutation.isPending ? (
                <Loader2 className="h-3.5 w-3.5 animate-spin" />
              ) : (
                <GitBranch className="h-3.5 w-3.5" />
              )}
              Use SSH
            </Button>
          )}
          {canSetupGithubHttps && (
            <Button
              type="button"
              variant="secondary"
              size="sm"
              className="h-8 gap-2 px-3 text-xs"
              onClick={() => void handleSetupGhGitAuth()}
              disabled={setupGhGitAuthMutation.isPending}
              data-testid="git-auth-setup-gh"
            >
              {setupGhGitAuthMutation.isPending ? (
                <Loader2 className="h-3.5 w-3.5 animate-spin" />
              ) : (
                <KeyRound className="h-3.5 w-3.5" />
              )}
              Setup HTTPS
            </Button>
          )}
          {canCopyGithubHttpsSetup && (
            <Button
              type="button"
              variant="secondary"
              size="sm"
              className="h-8 gap-2 px-3 text-xs"
              onClick={() => void handleCopyGithubHttpsSetup()}
              data-testid="git-auth-copy-gh-login"
            >
              <Copy className="h-3.5 w-3.5" />
              Copy HTTPS setup
            </Button>
          )}
        </div>
      )}

      <ConfirmationDialog {...confirmationDialogProps} />
    </div>
  );
}
