export function isProviderRole(role: string | null | undefined): boolean {
  return role !== "user" && role !== "system" && role != null;
}
