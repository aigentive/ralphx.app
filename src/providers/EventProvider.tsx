/**
 * EventProvider - Global event listener setup and event bus context
 *
 * This component:
 * 1. Provides an EventBus instance via React context (for hook migration in Task 4)
 * 2. Sets up global Tauri event listeners for the application
 *
 * The EventBus abstraction allows switching between:
 * - TauriEventBus: Real Tauri events in native mode
 * - MockEventBus: In-memory events for browser testing
 *
 * It should wrap the main App content to ensure events are captured
 * throughout the application lifecycle.
 */

import { createContext, useContext, useMemo, type ReactNode } from "react";
import {
  useTaskEvents,
  useSupervisorAlerts,
  useReviewEvents,
  useFileChangeEvents,
  useAgentEvents,
  useProposalEvents,
  useStepEvents,
  useExecutionErrorEvents,
} from "@/hooks/useEvents";
import { useIdeationEvents } from "@/hooks/useIdeationEvents";
import { usePlanArtifactEvents } from "@/hooks/useEvents.planArtifact";
import { createEventBus, type EventBus } from "@/lib/event-bus";

/**
 * Context for the event bus instance
 */
const EventBusContext = createContext<EventBus | null>(null);

/**
 * Hook to access the event bus from context
 *
 * @returns The EventBus instance (TauriEventBus or MockEventBus)
 * @throws Error if used outside of EventProvider
 *
 * @example
 * ```tsx
 * function MyComponent() {
 *   const bus = useEventBus();
 *
 *   useEffect(() => {
 *     return bus.subscribe('my:event', (payload) => {
 *       console.log('Received:', payload);
 *     });
 *   }, [bus]);
 * }
 * ```
 */
export function useEventBus(): EventBus {
  const bus = useContext(EventBusContext);
  if (!bus) {
    throw new Error("useEventBus must be used within an EventProvider");
  }
  return bus;
}

interface EventProviderProps {
  children: ReactNode;
}

/**
 * Global event provider component
 *
 * Sets up all global event listeners:
 * - Task events (created, updated, deleted, status_changed)
 * - Proposal events (created, updated, deleted)
 * - Step events (created, updated, deleted, reordered)
 * - Supervisor alerts
 * - Review events (placeholder for Phase 9)
 * - File change events (placeholder)
 * - Ideation events (session title updates from session-namer agent)
 *
 * @example
 * ```tsx
 * function App() {
 *   return (
 *     <QueryClientProvider client={queryClient}>
 *       <EventProvider>
 *         <Router>
 *           <Routes />
 *         </Router>
 *       </EventProvider>
 *     </QueryClientProvider>
 *   );
 * }
 * ```
 */
export function EventProvider({ children }: EventProviderProps) {
  // Create event bus once based on environment (Tauri or browser mode)
  const eventBus = useMemo(() => createEventBus(), []);

  // Set up global event listeners
  // Note: These still use the direct Tauri listen() API.
  // Task 4 will migrate them to use the eventBus via useEventBus() hook.
  useTaskEvents();
  useProposalEvents(); // Listen to proposal events for Ideation view
  useStepEvents(); // Listen to step events for task execution progress
  useSupervisorAlerts();
  useReviewEvents();
  useFileChangeEvents();
  useAgentEvents(); // Listen to agent:message events for Activity view
  useExecutionErrorEvents(); // Handle agent execution errors and unstick UI
  useIdeationEvents(); // Listen to ideation events (session title updates)
  usePlanArtifactEvents(); // Listen to plan artifact events for real-time updates

  return (
    <EventBusContext.Provider value={eventBus}>
      {children}
    </EventBusContext.Provider>
  );
}
