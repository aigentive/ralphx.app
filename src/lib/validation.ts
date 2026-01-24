/**
 * Validation utilities for RalphX
 * This module requires strict TypeScript settings to compile correctly.
 */

/**
 * Safely accesses an array element with undefined handling.
 * Requires noUncheckedIndexedAccess to be enabled.
 */
export function safeArrayAccess<T>(arr: readonly T[], index: number): T | undefined {
  return arr[index];
}

/**
 * Type guard for non-null values.
 * Useful with strict null checks.
 */
export function isNotNull<T>(value: T | null | undefined): value is T {
  return value !== null && value !== undefined;
}

/**
 * Asserts a value is defined, throws if not.
 * Works with strictNullChecks.
 */
export function assertDefined<T>(value: T | null | undefined, message: string): asserts value is T {
  if (value === null || value === undefined) {
    throw new Error(message);
  }
}

/**
 * A function with explicit return type requirement.
 * Tests noImplicitReturns.
 */
export function getStatusLabel(status: string): string {
  switch (status) {
    case "backlog":
      return "Backlog";
    case "ready":
      return "Ready";
    case "executing":
      return "Executing";
    case "completed":
      return "Completed";
    default:
      return "Unknown";
  }
}

/**
 * Object property access helper.
 * Tests exactOptionalPropertyTypes.
 */
export interface StrictConfig {
  name: string;
  description?: string;
}

export function createConfig(name: string, description?: string): StrictConfig {
  const config: StrictConfig = { name };
  if (description !== undefined) {
    config.description = description;
  }
  return config;
}
