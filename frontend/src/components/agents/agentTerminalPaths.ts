export function compactTerminalPath(path: string): string {
  const trimmed = path.trim();
  if (!trimmed) {
    return path;
  }
  return trimmed
    .replace(/^\/Users\/[^/]+(?=\/|$)/, "~")
    .replace(/^\/home\/[^/]+(?=\/|$)/, "~");
}
