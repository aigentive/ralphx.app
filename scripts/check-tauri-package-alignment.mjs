#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";

const repoRoot = process.argv[2] ? path.resolve(process.argv[2]) : process.cwd();

const frontendPackagePath = path.join(repoRoot, "frontend", "package.json");
const frontendLockPath = path.join(repoRoot, "frontend", "package-lock.json");
const cargoTomlPath = path.join(repoRoot, "src-tauri", "Cargo.toml");
const cargoLockPath = path.join(repoRoot, "src-tauri", "Cargo.lock");

const dependencyPairs = [
  ["@tauri-apps/api", "tauri"],
  ["@tauri-apps/plugin-dialog", "tauri-plugin-dialog"],
  ["@tauri-apps/plugin-fs", "tauri-plugin-fs"],
  ["@tauri-apps/plugin-global-shortcut", "tauri-plugin-global-shortcut"],
  ["@tauri-apps/plugin-opener", "tauri-plugin-opener"],
  ["@tauri-apps/plugin-process", "tauri-plugin-process"],
  ["@tauri-apps/plugin-updater", "tauri-plugin-updater"],
];

const cargoManifestOnlyPackages = ["tauri-build", "tauri-plugin-window-state"];
const errors = [];

function escapeRegex(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function ensureFileExists(filePath) {
  if (!fs.existsSync(filePath)) {
    errors.push(`Missing required file: ${path.relative(repoRoot, filePath)}`);
    return false;
  }

  return true;
}

function majorMinor(version) {
  const normalized = String(version).trim().replace(/^[~^]/, "");
  const match = normalized.match(/^(\d+)\.(\d+)/);
  return match ? `${match[1]}.${match[2]}` : null;
}

function requireMajorMinor(label, version) {
  const value = majorMinor(version);
  if (!value) {
    errors.push(`${label} must specify at least major.minor, found "${version}"`);
  }
  return value;
}

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

function readFrontendManifestVersions() {
  const packageJson = readJson(frontendPackagePath);
  const versions = new Map();

  for (const [frontendPackage] of dependencyPairs) {
    const version =
      packageJson.dependencies?.[frontendPackage] ??
      packageJson.devDependencies?.[frontendPackage] ??
      null;

    if (!version) {
      continue;
    }

    versions.set(frontendPackage, version);
    requireMajorMinor(`frontend/package.json -> ${frontendPackage}`, version);
  }

  return versions;
}

function readFrontendLockVersions() {
  const packageLock = readJson(frontendLockPath);
  const versions = new Map();

  for (const [frontendPackage] of dependencyPairs) {
    const version = packageLock?.packages?.[`node_modules/${frontendPackage}`]?.version ?? null;

    if (!version) {
      continue;
    }

    versions.set(frontendPackage, version);
    requireMajorMinor(`frontend/package-lock.json -> ${frontendPackage}`, version);
  }

  return versions;
}

function readCargoManifestVersions() {
  const cargoToml = fs.readFileSync(cargoTomlPath, "utf8");
  const versions = new Map();

  for (const cargoPackage of [...dependencyPairs.map(([, pkg]) => pkg), ...cargoManifestOnlyPackages]) {
    const inlineTableMatch = cargoToml.match(
      new RegExp(
        `^${escapeRegex(cargoPackage)}\\s*=\\s*\\{[^\\n]*version\\s*=\\s*"([^"]+)"`,
        "m"
      )
    );
    const stringMatch = cargoToml.match(
      new RegExp(`^${escapeRegex(cargoPackage)}\\s*=\\s*"([^"]+)"`, "m")
    );
    const version = inlineTableMatch?.[1] ?? stringMatch?.[1] ?? null;

    if (!version) {
      continue;
    }

    versions.set(cargoPackage, version);
    requireMajorMinor(`src-tauri/Cargo.toml -> ${cargoPackage}`, version);
  }

  return versions;
}

function readCargoLockVersions() {
  const cargoLock = fs.readFileSync(cargoLockPath, "utf8");
  const versions = new Map();

  for (const cargoPackage of [...dependencyPairs.map(([, pkg]) => pkg), ...cargoManifestOnlyPackages]) {
    const match = cargoLock.match(
      new RegExp(`name = "${escapeRegex(cargoPackage)}"\\nversion = "([^"]+)"`)
    );

    if (!match) {
      continue;
    }

    versions.set(cargoPackage, match[1]);
    requireMajorMinor(`src-tauri/Cargo.lock -> ${cargoPackage}`, match[1]);
  }

  return versions;
}

function compareVersionSurfaces(label, leftName, leftVersion, rightName, rightVersion) {
  const leftMajorMinor = majorMinor(leftVersion);
  const rightMajorMinor = majorMinor(rightVersion);

  if (!leftMajorMinor || !rightMajorMinor || leftMajorMinor === rightMajorMinor) {
    return;
  }

  errors.push(`${label}: ${leftName} (${leftVersion}) : ${rightName} (${rightVersion})`);
}

if (
  ensureFileExists(frontendPackagePath) &&
  ensureFileExists(frontendLockPath) &&
  ensureFileExists(cargoTomlPath) &&
  ensureFileExists(cargoLockPath)
) {
  const frontendManifestVersions = readFrontendManifestVersions();
  const frontendLockVersions = readFrontendLockVersions();
  const cargoManifestVersions = readCargoManifestVersions();
  const cargoLockVersions = readCargoLockVersions();

  for (const [frontendPackage, cargoPackage] of dependencyPairs) {
    const frontendManifestVersion = frontendManifestVersions.get(frontendPackage);
    const frontendLockVersion = frontendLockVersions.get(frontendPackage);
    const cargoManifestVersion = cargoManifestVersions.get(cargoPackage);
    const cargoLockVersion = cargoLockVersions.get(cargoPackage);

    if (frontendManifestVersion && frontendLockVersion) {
      compareVersionSurfaces(
        "frontend manifest/lock drift",
        frontendPackage,
        frontendManifestVersion,
        frontendPackage,
        frontendLockVersion
      );
    }

    if (cargoManifestVersion && cargoLockVersion) {
      compareVersionSurfaces(
        "Rust manifest/lock drift",
        cargoPackage,
        cargoManifestVersion,
        cargoPackage,
        cargoLockVersion
      );
    }

    const frontendEffectiveVersion = frontendLockVersion ?? frontendManifestVersion;
    const cargoEffectiveVersion = cargoLockVersion ?? cargoManifestVersion;

    if (frontendEffectiveVersion && cargoEffectiveVersion) {
      compareVersionSurfaces(
        "JS/Rust Tauri drift",
        cargoPackage,
        cargoEffectiveVersion,
        frontendPackage,
        frontendEffectiveVersion
      );
    }
  }

  for (const cargoPackage of cargoManifestOnlyPackages) {
    const cargoManifestVersion = cargoManifestVersions.get(cargoPackage);
    const cargoLockVersion = cargoLockVersions.get(cargoPackage);

    if (cargoManifestVersion && cargoLockVersion) {
      compareVersionSurfaces(
        "Rust manifest/lock drift",
        cargoPackage,
        cargoManifestVersion,
        cargoPackage,
        cargoLockVersion
      );
    }
  }
}

if (errors.length > 0) {
  console.log(errors.join("\n"));
  process.exit(1);
}
