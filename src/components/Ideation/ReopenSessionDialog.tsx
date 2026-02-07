import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { AlertTriangle, Loader2 } from "lucide-react";

export type ReopenMode = "reopen" | "reset";

interface ReopenSessionDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  mode: ReopenMode;
  sessionTitle: string;
  taskCount: number;
  onConfirm: () => void;
  isLoading: boolean;
}

const CONTENT: Record<
  ReopenMode,
  {
    title: string;
    confirmLabel: string;
    loadingLabel: string;
    description: (title: string, count: number) => string;
  }
> = {
  reopen: {
    title: "Reopen Session",
    confirmLabel: "Reopen",
    loadingLabel: "Reopening...",
    description: (title, count) =>
      `This will reopen "${title}" and delete ${count === 1 ? "1 task" : `all ${count} tasks`} created from it. Running agents will be stopped and git branches will be cleaned up. The session will return to Active so you can edit proposals.`,
  },
  reset: {
    title: "Reset & Re-accept",
    confirmLabel: "Reset & Re-accept",
    loadingLabel: "Resetting...",
    description: (title, count) =>
      `This will delete ${count === 1 ? "1 existing task" : `all ${count} existing tasks`} from "${title}", clean up git resources, then immediately re-apply all proposals as fresh tasks. Running agents will be stopped.`,
  },
};

export function ReopenSessionDialog({
  open,
  onOpenChange,
  mode,
  sessionTitle,
  taskCount,
  onConfirm,
  isLoading,
}: ReopenSessionDialogProps) {
  const content = CONTENT[mode];

  const handleOpenChange = (next: boolean) => {
    if (!next && isLoading) return;
    onOpenChange(next);
  };

  return (
    <AlertDialog open={open} onOpenChange={handleOpenChange}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle className="flex items-center gap-2">
            <AlertTriangle
              className="h-5 w-5 shrink-0"
              style={{ color: "var(--status-warning)" }}
            />
            {content.title}
          </AlertDialogTitle>
          <AlertDialogDescription>
            {content.description(sessionTitle, taskCount)}
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel disabled={isLoading}>Cancel</AlertDialogCancel>
          <AlertDialogAction
            onClick={(e) => {
              e.preventDefault();
              onConfirm();
            }}
            disabled={isLoading}
            className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
          >
            {isLoading ? (
              <>
                <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                {content.loadingLabel}
              </>
            ) : (
              content.confirmLabel
            )}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
