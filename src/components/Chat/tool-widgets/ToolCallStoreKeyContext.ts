import { createContext } from "react";

/** Provides the Zustand store key to nested tool call widgets without prop drilling through the widget registry. */
export const ToolCallStoreKeyContext = createContext<string | null>(null);
