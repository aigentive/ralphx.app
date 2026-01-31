/**
 * Event Bus Abstraction
 *
 * Provides a unified interface for event subscription that works in both:
 * - Tauri mode: Uses real Tauri listen() from @tauri-apps/api/event
 * - Web mode: Uses in-memory event emitter for browser testing
 *
 * This abstraction allows the app to run without Tauri for visual testing
 * and Playwright automation.
 */

import { listen, emit, type UnlistenFn, type Event } from "@tauri-apps/api/event";
import { isTauriMode } from "./tauri-detection";

/**
 * Unsubscribe function returned by subscribe()
 */
export type Unsubscribe = () => void;

/**
 * Event handler function
 */
export type EventHandler<T = unknown> = (payload: T) => void;

/**
 * Event bus interface for subscribing to and emitting events
 */
export interface EventBus {
  /**
   * Subscribe to an event
   * @param event - Event name to listen for
   * @param handler - Callback function receiving the event payload
   * @returns Unsubscribe function to stop listening
   */
  subscribe<T = unknown>(event: string, handler: EventHandler<T>): Unsubscribe;

  /**
   * Emit an event (primarily for testing/mock mode)
   * @param event - Event name to emit
   * @param payload - Event payload data
   */
  emit<T = unknown>(event: string, payload: T): void;
}

/**
 * Tauri Event Bus - Uses real Tauri listen() API
 *
 * Wraps the Tauri event system for use in the native app.
 */
export class TauriEventBus implements EventBus {
  private unlisteners: Map<string, Set<Promise<UnlistenFn>>> = new Map();

  subscribe<T = unknown>(event: string, handler: EventHandler<T>): Unsubscribe {
    // Create the listener promise
    const unlistenPromise = listen<T>(event, (e: Event<T>) => {
      handler(e.payload);
    });

    // Track for cleanup
    if (!this.unlisteners.has(event)) {
      this.unlisteners.set(event, new Set());
    }
    this.unlisteners.get(event)!.add(unlistenPromise);

    // Return unsubscribe function
    return () => {
      unlistenPromise.then((fn) => fn());
      this.unlisteners.get(event)?.delete(unlistenPromise);
    };
  }

  emit<T = unknown>(event: string, payload: T): void {
    // Use Tauri's emit for IPC if needed
    emit(event, payload).catch((err) => {
      console.warn(`[TauriEventBus] Failed to emit ${event}:`, err);
    });
  }
}

/**
 * Mock Event Bus - In-memory event emitter for browser mode
 *
 * Used when running without Tauri (e.g., Playwright tests, dev:web mode).
 * Provides the same interface but events stay in-browser.
 */
export class MockEventBus implements EventBus {
  private listeners: Map<string, Set<EventHandler<unknown>>> = new Map();

  subscribe<T = unknown>(event: string, handler: EventHandler<T>): Unsubscribe {
    if (!this.listeners.has(event)) {
      this.listeners.set(event, new Set());
    }
    // Cast needed because Map stores EventHandler<unknown>
    const typedHandler = handler as EventHandler<unknown>;
    this.listeners.get(event)!.add(typedHandler);

    // Return unsubscribe function
    return () => {
      this.listeners.get(event)?.delete(typedHandler);
    };
  }

  emit<T = unknown>(event: string, payload: T): void {
    const handlers = this.listeners.get(event);
    if (handlers) {
      handlers.forEach((handler) => {
        try {
          handler(payload);
        } catch (err) {
          console.error(`[MockEventBus] Error in handler for ${event}:`, err);
        }
      });
    }
  }

  /**
   * Clear all listeners (useful for test cleanup)
   */
  clear(): void {
    this.listeners.clear();
  }

  /**
   * Get listener count for an event (useful for debugging/testing)
   */
  getListenerCount(event: string): number {
    return this.listeners.get(event)?.size ?? 0;
  }
}

/**
 * Create the appropriate event bus based on environment
 *
 * @returns TauriEventBus in Tauri mode, MockEventBus in browser mode
 */
export function createEventBus(): EventBus {
  if (isTauriMode()) {
    return new TauriEventBus();
  }
  return new MockEventBus();
}
