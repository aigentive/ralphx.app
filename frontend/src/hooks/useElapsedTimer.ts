import { useEffect, useState } from "react";

/**
 * Tracks elapsed time, incrementing every second from an initial value.
 *
 * @param initialSeconds - Starting elapsed seconds (null means timer is inactive)
 * @param entityId - ID of the entity being tracked; changing this resets the timer
 * @returns Current elapsed seconds (increments live), or null if inactive
 */
export function useElapsedTimer(
  initialSeconds: number | null,
  entityId: string,
): number | null {
  const [elapsedTime, setElapsedTime] = useState(initialSeconds);

  useEffect(() => {
    if (initialSeconds === null) return;

    setElapsedTime(initialSeconds);

    const interval = setInterval(() => {
      setElapsedTime((prev) => (prev !== null ? prev + 1 : null));
    }, 1000);

    return () => clearInterval(interval);
  }, [initialSeconds, entityId]);

  return elapsedTime;
}
