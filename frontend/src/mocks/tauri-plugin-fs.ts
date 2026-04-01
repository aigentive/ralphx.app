/**
 * Mock implementation of @tauri-apps/plugin-fs for web mode
 *
 * File system operations cannot work in browser mode.
 * Returns empty/error responses to prevent runtime crashes.
 */

/**
 * Mock readTextFile - throws error since we can't read files in browser
 */
export async function readTextFile(_path: string): Promise<string> {
  console.debug("[mock] fs.readTextFile called - throwing error");
  throw new Error("[web mode] File system access not available in browser");
}

/**
 * Mock readFile - throws error since we can't read files in browser
 */
export async function readFile(_path: string): Promise<Uint8Array> {
  console.debug("[mock] fs.readFile called - throwing error");
  throw new Error("[web mode] File system access not available in browser");
}

/**
 * Mock writeTextFile - no-op
 */
export async function writeTextFile(
  _path: string,
  _contents: string
): Promise<void> {
  console.debug("[mock] fs.writeTextFile called - no-op");
}

/**
 * Mock writeFile - no-op
 */
export async function writeFile(
  _path: string,
  _contents: Uint8Array
): Promise<void> {
  console.debug("[mock] fs.writeFile called - no-op");
}

/**
 * Mock exists - returns false
 */
export async function exists(_path: string): Promise<boolean> {
  console.debug("[mock] fs.exists called - returning false");
  return false;
}

/**
 * Mock mkdir - no-op
 */
export async function mkdir(
  _path: string,
  _options?: { recursive?: boolean }
): Promise<void> {
  console.debug("[mock] fs.mkdir called - no-op");
}

/**
 * Mock readDir - returns empty array
 */
export async function readDir(_path: string): Promise<{ name: string; path: string }[]> {
  console.debug("[mock] fs.readDir called - returning empty array");
  return [];
}

/**
 * Mock remove - no-op
 */
export async function remove(
  _path: string,
  _options?: { recursive?: boolean }
): Promise<void> {
  console.debug("[mock] fs.remove called - no-op");
}

/**
 * Mock rename - no-op
 */
export async function rename(_oldPath: string, _newPath: string): Promise<void> {
  console.debug("[mock] fs.rename called - no-op");
}

/**
 * Mock copyFile - no-op
 */
export async function copyFile(_source: string, _destination: string): Promise<void> {
  console.debug("[mock] fs.copyFile called - no-op");
}
