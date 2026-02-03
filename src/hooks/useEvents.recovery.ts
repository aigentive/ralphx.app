/**
 * Recovery prompt event hook - handles backend recovery prompts
 */

import { useEffect } from "react";
import { useEventBus } from "@/providers/EventProvider";
import { RecoveryPromptEventSchema } from "@/types/events";
import { useUiStore } from "@/stores/uiStore";

export function useRecoveryPromptEvents() {
  const bus = useEventBus();
  const setRecoveryPrompt = useUiStore((s) => s.setRecoveryPrompt);

  useEffect(() => {
    return bus.subscribe<unknown>("recovery:prompt", (payload) => {
      const parsed = RecoveryPromptEventSchema.safeParse(payload);
      if (!parsed.success) {
        return;
      }
      setRecoveryPrompt(parsed.data);
    });
  }, [bus, setRecoveryPrompt]);
}
