import { useEffect, useState } from "react";

import {
  cancelDeferredFrameJob,
  scheduleDeferredFrameJob,
} from "./agentDeferredFrame";

export function useDeferredAgentHydration(key: string | null | undefined): boolean {
  const [isReady, setIsReady] = useState(false);

  useEffect(() => {
    setIsReady(false);
    if (!key) {
      return;
    }

    const job = scheduleDeferredFrameJob(() => setIsReady(true));
    return () => cancelDeferredFrameJob(job);
  }, [key]);

  return isReady;
}
