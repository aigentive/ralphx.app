#!/usr/bin/env python3
from __future__ import annotations

import pathlib
import re
import sys


ROOT = pathlib.Path(__file__).resolve().parents[1]
MOD_RS = ROOT / "src-tauri" / "src" / "infrastructure" / "sqlite" / "migrations" / "mod.rs"
MIGRATIONS_DIR = MOD_RS.parent
LEGACY_MAX_VERSION = 81
TIMESTAMP_VERSION_RE = re.compile(r"^20\d{12}$")


def fail(message: str) -> None:
    print(f"migration validation failed: {message}", file=sys.stderr)
    raise SystemExit(1)


def main() -> None:
    text = MOD_RS.read_text()

    schema_match = re.search(r"pub const SCHEMA_VERSION: i64 = (\d+);", text)
    if not schema_match:
        fail("could not parse SCHEMA_VERSION from migrations/mod.rs")
    schema_version = int(schema_match.group(1))

    declared_modules = set(
        re.findall(r"^mod (v\d+[a-zA-Z0-9_]*)\s*;", text, flags=re.MULTILINE)
    )

    entry_pattern = re.compile(
        r"Migration \{\s*"
        r"version: (\d+),\s*"
        r'name: "([^"]+)",\s*'
        r"migrate: ([a-zA-Z0-9_]+)::migrate,\s*"
        r"\}",
        flags=re.MULTILINE,
    )
    entries = [
        (int(version), name, module)
        for version, name, module in entry_pattern.findall(text)
    ]

    if not entries:
        fail("no migration entries found in migrations/mod.rs")

    versions = [version for version, _, _ in entries]
    if versions != sorted(versions):
        fail("migration versions are not strictly ordered ascending in MIGRATIONS")
    if len(versions) != len(set(versions)):
        fail("duplicate migration versions detected in MIGRATIONS")
    if schema_version != versions[-1]:
        fail(
            f"SCHEMA_VERSION {schema_version} does not match latest migration version {versions[-1]}"
        )

    modules = [module for _, _, module in entries]
    if len(modules) != len(set(modules)):
        fail("duplicate migration modules detected in MIGRATIONS")

    for version, _name, module in entries:
        if module not in declared_modules:
            fail(f"migration module `{module}` is used in MIGRATIONS but not declared")

        file_path = MIGRATIONS_DIR / f"{module}.rs"
        if not file_path.exists():
            fail(f"migration file missing for module `{module}`")

        file_match = re.match(r"^v(\d+)_", module)
        if file_match is None:
            fail(f"migration module `{module}` must start with `v<version>_`")

        file_version = int(file_match.group(1))
        if file_version != version:
            fail(
                f"migration file/module `{module}` encodes version {file_version}, "
                f"but MIGRATIONS registers version {version}"
            )

        if version > LEGACY_MAX_VERSION and not TIMESTAMP_VERSION_RE.match(str(version)):
            fail(
                f"migration version {version} must use a UTC timestamp id "
                f"(YYYYMMDDHHMMSS) once above legacy max {LEGACY_MAX_VERSION}"
            )

    print(
        f"validated {len(entries)} sqlite migrations; latest version {schema_version}"
    )


if __name__ == "__main__":
    main()
