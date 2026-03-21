#!/usr/bin/env python3
from __future__ import annotations

import datetime as dt
import pathlib
import re
import sys


ROOT = pathlib.Path(__file__).resolve().parents[1]
MIGRATIONS_DIR = ROOT / "src-tauri" / "src" / "infrastructure" / "sqlite" / "migrations"


def fail(message: str) -> None:
    print(message, file=sys.stderr)
    raise SystemExit(1)


def slugify(value: str) -> str:
    slug = re.sub(r"[^a-z0-9]+", "_", value.strip().lower()).strip("_")
    if not slug:
        fail("description must contain at least one alphanumeric character")
    return slug


def main() -> None:
    if len(sys.argv) != 2:
        fail("usage: scripts/new_sqlite_migration.py <description>")

    slug = slugify(sys.argv[1])
    version = dt.datetime.now(dt.timezone.utc).strftime("%Y%m%d%H%M%S")
    base = f"v{version}_{slug}"
    migration_path = MIGRATIONS_DIR / f"{base}.rs"
    test_path = MIGRATIONS_DIR / f"{base}_tests.rs"

    if migration_path.exists() or test_path.exists():
        fail(f"migration files already exist for {base}")

    migration_path.write_text(
        "\n".join(
            [
                f"// Migration v{version}: {slug.replace('_', ' ')}",
                "",
                "use rusqlite::Connection;",
                "",
                "use crate::error::AppResult;",
                "",
                "pub fn migrate(conn: &Connection) -> AppResult<()> {",
                "    let _ = conn;",
                "    Ok(())",
                "}",
                "",
            ]
        )
    )
    test_path.write_text(
        "\n".join(
            [
                f"//! Tests for migration v{version}: {slug.replace('_', ' ')}",
                "",
                "use rusqlite::Connection;",
                "",
                f"use super::{base};",
                "",
                "fn setup_test_db() -> Connection {",
                '    Connection::open_in_memory().expect("Failed to create in-memory database")',
                "}",
                "",
                "#[test]",
                "fn test_migration_runs() {",
                "    let conn = setup_test_db();",
                f"    {base}::migrate(&conn).unwrap();",
                "}",
                "",
            ]
        )
    )

    print(f"Created {migration_path.relative_to(ROOT)}")
    print(f"Created {test_path.relative_to(ROOT)}")
    print("Next steps:")
    print(f"1. Add `mod {base};` and `#[cfg(test)] mod {base}_tests;` to migrations/mod.rs")
    print(
        f"2. Register migration version {version} in MIGRATIONS and set SCHEMA_VERSION to {version}"
    )
    print("3. Run `python3 scripts/validate_sqlite_migrations.py`")


if __name__ == "__main__":
    main()
