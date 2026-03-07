/**
 * Shared types for ralphx-external-mcp
 */
/** Permission bitmask constants */
export const Permission = {
    READ: 1,
    WRITE: 2,
    ADMIN: 4,
};
export function hasPermission(permissions, flag) {
    return (permissions & flag) !== 0;
}
//# sourceMappingURL=types.js.map