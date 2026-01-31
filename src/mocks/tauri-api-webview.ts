/**
 * Mock implementation of @tauri-apps/api/webview for web mode
 *
 * Provides mock webview functionality for features like file drop.
 */

import type { UnlistenFn, EventCallback, Options } from "./tauri-api-event";

/**
 * Position type
 */
export interface Position {
  x: number;
  y: number;
}

/**
 * Size type
 */
export interface Size {
  width: number;
  height: number;
}

/**
 * DragDropEvent types
 */
export type DragDropEvent =
  | { type: "over"; position: Position; paths: string[] }
  | { type: "drop"; position: Position; paths: string[] }
  | { type: "cancel" }
  | { type: "dragged"; paths: string[] };

/**
 * Mock Webview class
 */
export class Webview {
  /** The webview label */
  label: string;

  constructor(label: string) {
    this.label = label;
  }

  /**
   * Listen to an event on this webview
   */
  async listen<T>(
    event: string,
    _handler: EventCallback<T>,
    _options?: Options
  ): Promise<UnlistenFn> {
    console.debug(`[mock] Webview.listen("${event}") on ${this.label}`);
    return () => {
      console.debug(`[mock] Webview.unlisten("${event}") on ${this.label}`);
    };
  }

  /**
   * Listen to a one-time event on this webview
   */
  async once<T>(
    event: string,
    _handler: EventCallback<T>,
    _options?: Options
  ): Promise<UnlistenFn> {
    console.debug(`[mock] Webview.once("${event}") on ${this.label}`);
    return () => {};
  }

  /**
   * Emit an event to this webview
   */
  async emit(event: string, _payload?: unknown): Promise<void> {
    console.debug(`[mock] Webview.emit("${event}") to ${this.label}`);
  }

  /**
   * Listen for file drop events
   */
  async onDragDropEvent(
    _handler: EventCallback<DragDropEvent>
  ): Promise<UnlistenFn> {
    console.debug(`[mock] Webview.onDragDropEvent() on ${this.label}`);
    return () => {
      console.debug(`[mock] Webview.onDragDropEvent unlisten on ${this.label}`);
    };
  }

  /**
   * Get the webview position
   */
  async position(): Promise<Position> {
    return { x: 0, y: 0 };
  }

  /**
   * Get the webview size
   */
  async size(): Promise<Size> {
    return { width: window.innerWidth, height: window.innerHeight };
  }

  /**
   * Close the webview
   */
  async close(): Promise<void> {
    console.debug(`[mock] Webview.close() on ${this.label}`);
  }

  /**
   * Set webview focus
   */
  async setFocus(): Promise<void> {
    console.debug(`[mock] Webview.setFocus() on ${this.label}`);
  }

  /**
   * Hide the webview
   */
  async hide(): Promise<void> {
    console.debug(`[mock] Webview.hide() on ${this.label}`);
  }

  /**
   * Show the webview
   */
  async show(): Promise<void> {
    console.debug(`[mock] Webview.show() on ${this.label}`);
  }

  /**
   * Set the webview zoom level
   */
  async setZoom(scaleFactor: number): Promise<void> {
    console.debug(`[mock] Webview.setZoom(${scaleFactor}) on ${this.label}`);
  }

  /**
   * Print the webview
   */
  async print(): Promise<void> {
    console.debug(`[mock] Webview.print() on ${this.label}`);
  }

  /**
   * Clear all browsing data
   */
  async clearAllBrowsingData(): Promise<void> {
    console.debug(`[mock] Webview.clearAllBrowsingData() on ${this.label}`);
  }

  /**
   * Navigate to a URL
   */
  async navigate(url: string): Promise<void> {
    console.debug(`[mock] Webview.navigate(${url}) on ${this.label}`);
  }

  /**
   * Get all webviews
   */
  static async getAll(): Promise<Webview[]> {
    console.debug("[mock] Webview.getAll()");
    return [new Webview("main")];
  }

  /**
   * Get webview by label
   */
  static async getByLabel(label: string): Promise<Webview | null> {
    console.debug(`[mock] Webview.getByLabel(${label})`);
    return new Webview(label);
  }
}

// Singleton mock webview
const mockWebview = new Webview("main");

/**
 * Get the current webview instance
 */
export function getCurrentWebview(): Webview {
  console.debug("[mock] getCurrentWebview()");
  return mockWebview;
}

/**
 * Get all webviews
 */
export async function getAll(): Promise<Webview[]> {
  return Webview.getAll();
}
