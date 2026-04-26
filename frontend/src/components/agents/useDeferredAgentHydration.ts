import { useEffect, useState } from "react";

export function useDeferredAgentHydration(key: string | null | undefined): boolean {
  const [isReady, setIsReady] = useState(false);

  useEffect(() => {
    setIsReady(false);
    if (!key) {
      return;
    }

    const frame = window.requestAnimationFrame(() => setIsReady(true));
    return () => window.cancelAnimationFrame(frame);
  }, [key]);

  return isReady;
}
