import { useEffect, useState } from "react";

const NAV_COMPACT_QUERY = "(min-width: 1280px)";

export function useNavCompactBreakpoint(): { isNavCompact: boolean } {
  const [isNavCompact, setIsNavCompact] = useState(() => {
    if (typeof window === "undefined") return false;
    return !window.matchMedia(NAV_COMPACT_QUERY).matches;
  });

  useEffect(() => {
    if (typeof window === "undefined") return;
    const mediaQuery = window.matchMedia(NAV_COMPACT_QUERY);

    const handleChange = (event: MediaQueryListEvent) => {
      setIsNavCompact(!event.matches);
    };

    if (mediaQuery.addEventListener) {
      mediaQuery.addEventListener("change", handleChange);
    } else {
      mediaQuery.addListener(handleChange);
    }

    setIsNavCompact(!mediaQuery.matches);

    return () => {
      if (mediaQuery.removeEventListener) {
        mediaQuery.removeEventListener("change", handleChange);
      } else {
        mediaQuery.removeListener(handleChange);
      }
    };
  }, []);

  return { isNavCompact };
}
