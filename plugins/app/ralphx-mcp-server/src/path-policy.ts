import os from "node:os";
import path from "node:path";

export function expandHome(value: string): string {
  if (!value.startsWith("~")) return value;
  return path.join(os.homedir(), value.slice(1));
}

export function normalizePathLike(value: string): string {
  return path.resolve(expandHome(value));
}

export function isWithin(root: string, candidate: string): boolean {
  const relative = path.relative(root, candidate);
  return relative === "" || (!relative.startsWith("..") && !path.isAbsolute(relative));
}

export function getPrimaryFilesystemRoot(): string {
  return normalizePathLike(process.cwd());
}

export function getAllowedFilesystemRoots(): string[] {
  return [getPrimaryFilesystemRoot()];
}

export function resolveScopedFilesystemPath(inputPath: string, basePath?: string): string {
  const baseRoot = normalizePathLike(basePath ?? getPrimaryFilesystemRoot());
  const resolved = path.isAbsolute(inputPath) || inputPath.startsWith("~")
    ? normalizePathLike(inputPath)
    : normalizePathLike(path.join(baseRoot, inputPath));

  const allowedRoots = getAllowedFilesystemRoots();
  if (!allowedRoots.some((root) => isWithin(root, resolved))) {
    throw new Error(
      `Path "${inputPath}" resolves outside the allowed filesystem roots.`
    );
  }

  return resolved;
}
