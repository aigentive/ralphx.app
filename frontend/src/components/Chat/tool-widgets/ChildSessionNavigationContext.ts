import { createContext } from "react";

/** Provides navigation callback to nested tool call widgets without prop drilling through the widget registry. */
export const ChildSessionNavigationContext = createContext<(sessionId: string) => void>(
  () => { /* no-op default — widgets outside context tree are safe */ }
);
