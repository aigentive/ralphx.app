/* eslint-disable react-refresh/only-export-components */
import { useState, useRef, useCallback, useMemo } from "react";
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

interface ConfirmOptions {
  title: string;
  description: string;
  confirmText?: string;
  cancelText?: string;
  variant?: "default" | "destructive";
}

interface ConfirmationDialogProps {
  isOpen: boolean;
  options: ConfirmOptions | null;
  onConfirm: () => void;
  onCancel: () => void;
}

/**
 * Standalone dialog component - stable reference, won't cause parent re-renders
 */
function ConfirmationDialogComponent({
  isOpen,
  options,
  onConfirm,
  onCancel,
}: ConfirmationDialogProps) {
  if (!options) return null;

  return (
    <AlertDialog open={isOpen} onOpenChange={(open) => !open && onCancel()}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>{options.title}</AlertDialogTitle>
          <AlertDialogDescription>{options.description}</AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel onClick={onCancel}>
            {options.cancelText ?? "Cancel"}
          </AlertDialogCancel>
          <AlertDialogAction
            onClick={onConfirm}
            variant={options.variant ?? "default"}
          >
            {options.confirmText ?? "Confirm"}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}

interface UseConfirmationReturn {
  confirm: (options: ConfirmOptions) => Promise<boolean>;
  confirmationDialogProps: ConfirmationDialogProps;
  ConfirmationDialog: typeof ConfirmationDialogComponent;
}

/**
 * Hook for showing confirmation dialogs with async/await pattern.
 *
 * Usage:
 * ```tsx
 * const { confirm, confirmationDialogProps, ConfirmationDialog } = useConfirmation();
 *
 * // In your component:
 * <ConfirmationDialog {...confirmationDialogProps} />
 *
 * // To show dialog:
 * const confirmed = await confirm({ title: "Delete?", description: "..." });
 * ```
 */
export function useConfirmation(): UseConfirmationReturn {
  const [isOpen, setIsOpen] = useState(false);
  const [options, setOptions] = useState<ConfirmOptions | null>(null);
  const resolveRef = useRef<((value: boolean) => void) | null>(null);

  const confirm = useCallback((opts: ConfirmOptions): Promise<boolean> => {
    setOptions(opts);
    setIsOpen(true);
    return new Promise((resolve) => {
      resolveRef.current = resolve;
    });
  }, []);

  const onConfirm = useCallback(() => {
    setIsOpen(false);
    setOptions(null);
    resolveRef.current?.(true);
    resolveRef.current = null;
  }, []);

  const onCancel = useCallback(() => {
    setIsOpen(false);
    setOptions(null);
    resolveRef.current?.(false);
    resolveRef.current = null;
  }, []);

  const confirmationDialogProps = useMemo(
    () => ({ isOpen, options, onConfirm, onCancel }),
    [isOpen, options, onConfirm, onCancel]
  );

  return {
    confirm,
    confirmationDialogProps,
    ConfirmationDialog: ConfirmationDialogComponent,
  };
}
