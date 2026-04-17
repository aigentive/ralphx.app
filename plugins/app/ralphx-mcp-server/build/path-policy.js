import os from "node:os";
import path from "node:path";
export function expandHome(value) {
    if (!value.startsWith("~"))
        return value;
    return path.join(os.homedir(), value.slice(1));
}
export function normalizePathLike(value) {
    return path.resolve(expandHome(value));
}
export function isWithin(root, candidate) {
    const relative = path.relative(root, candidate);
    return relative === "" || (!relative.startsWith("..") && !path.isAbsolute(relative));
}
export function getPrimaryFilesystemRoot() {
    return normalizePathLike(process.env.RALPHX_WORKING_DIRECTORY ?? process.env.PWD ?? process.cwd());
}
export function getAllowedFilesystemRoots() {
    const roots = new Set();
    roots.add(getPrimaryFilesystemRoot());
    const pwd = process.env.PWD;
    if (pwd) {
        roots.add(normalizePathLike(pwd));
    }
    roots.add(normalizePathLike(process.cwd()));
    return [...roots];
}
export function resolveScopedFilesystemPath(inputPath, basePath) {
    const baseRoot = normalizePathLike(basePath ?? getPrimaryFilesystemRoot());
    const resolved = path.isAbsolute(inputPath) || inputPath.startsWith("~")
        ? normalizePathLike(inputPath)
        : normalizePathLike(path.join(baseRoot, inputPath));
    const allowedRoots = getAllowedFilesystemRoots();
    if (!allowedRoots.some((root) => isWithin(root, resolved))) {
        throw new Error(`Path "${inputPath}" resolves outside the allowed filesystem roots.`);
    }
    return resolved;
}
//# sourceMappingURL=path-policy.js.map