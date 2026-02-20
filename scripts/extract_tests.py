#!/usr/bin/env python3
"""
Extract inline #[cfg(test)] mod blocks from Rust files to companion test files.

For each foo.rs:  creates foo_tests.rs + replaces inline block with:
  #[cfg(test)]
  #[path = "foo_tests.rs"]
  mod tests;

For each mod.rs: creates tests.rs + replaces inline block with:
  #[cfg(test)]
  mod tests;

Usage:
  python3 scripts/extract_tests.py [--dry-run] [--file path/to/file.rs]
"""

import argparse
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).parent.parent / "src-tauri"
CARGO_MANIFEST = ROOT / "Cargo.toml"

SKIP_DIRS = {
    "reconciliation",  # already extracted
    "transition_handler",  # already extracted
}

SKIP_FILES = {
    "execution_commands.rs",  # separate refactor
}


def strip_string_literals(content: str) -> str:
    """
    Replace string literal contents with spaces so braces inside them
    don't confuse the brace counter.

    Handles: r#"..."# r##"..."## "..." '...' // line comments /* block */
    """
    result = list(content)
    i = 0
    n = len(content)
    while i < n:
        # Raw strings: r#"..."# or r##"..."##
        if content[i] == 'r' and i + 1 < n and content[i + 1] == '#':
            j = i + 1
            hashes = 0
            while j < n and content[j] == '#':
                hashes += 1
                j += 1
            if j < n and content[j] == '"':
                # Found raw string opener r###..."###
                end_marker = '"' + '#' * hashes
                end = content.find(end_marker, j + 1)
                if end != -1:
                    # blank out the interior
                    for k in range(j + 1, end):
                        if result[k] not in '\n':
                            result[k] = ' '
                    i = end + len(end_marker)
                    continue
        # Regular strings: "..."
        if content[i] == '"':
            j = i + 1
            while j < n:
                if content[j] == '\\':
                    j += 2
                    continue
                if content[j] == '"':
                    break
                j += 1
            for k in range(i + 1, min(j, n)):
                if result[k] not in '\n':
                    result[k] = ' '
            i = j + 1
            continue
        # Char literals: '.'
        if content[i] == "'":
            j = i + 1
            if j < n and content[j] == '\\':
                j += 2
            else:
                j += 1
            if j < n and content[j] == "'":
                for k in range(i + 1, j):
                    result[k] = ' '
                i = j + 1
                continue
        # Line comments: //...
        if content[i] == '/' and i + 1 < n and content[i + 1] == '/':
            j = i + 2
            while j < n and content[j] != '\n':
                result[j] = ' '
                j += 1
            i = j
            continue
        # Block comments: /*...*/
        if content[i] == '/' and i + 1 < n and content[i + 1] == '*':
            end = content.find('*/', i + 2)
            if end != -1:
                for k in range(i + 2, end):
                    if result[k] not in '\n':
                        result[k] = ' '
                i = end + 2
                continue
        i += 1
    return ''.join(result)


def find_cfg_test_block(content: str) -> tuple[int, int, int] | None:
    """
    Finds the outermost #[cfg(test)] mod block.
    Returns (attr_line_start, open_brace_pos, close_brace_pos) as char offsets,
    or None if not found.

    Handles:
      #[cfg(test)]
      mod tests {
        ...
      }
    """
    # Find #[cfg(test)] followed (possibly after whitespace) by mod <name> {
    pattern = re.compile(
        r'#\[cfg\(test\)\]\s*(?:#\[[^\]]*\]\s*)*mod\s+\w+\s*\{',
        re.DOTALL
    )
    m = pattern.search(content)
    if not m:
        return None

    attr_start = m.start()
    open_brace = m.end() - 1  # position of '{'

    # Use string-stripped content for brace counting to avoid
    # counting braces inside raw strings, string literals, or comments
    stripped = strip_string_literals(content)

    # Count braces from open_brace to find matching close
    depth = 1
    pos = open_brace + 1
    while pos < len(stripped) and depth > 0:
        ch = stripped[pos]
        if ch == '{':
            depth += 1
        elif ch == '}':
            depth -= 1
        pos += 1

    if depth != 0:
        return None  # unbalanced braces — skip

    close_brace = pos - 1  # position of matching '}'
    return (attr_start, open_brace, close_brace)


def extract_mod_body(content: str, open_brace: int, close_brace: int) -> str:
    """Extract the body between { and }, stripping one level of indentation."""
    body = content[open_brace + 1:close_brace]
    # Strip leading/trailing blank lines
    lines = body.split('\n')
    # Remove common leading indentation (4 spaces or 1 tab)
    stripped = []
    for line in lines:
        if line.startswith('    '):
            stripped.append(line[4:])
        elif line.startswith('\t'):
            stripped.append(line[1:])
        else:
            stripped.append(line)
    return '\n'.join(stripped).strip('\n')


def get_mod_name(content: str, attr_start: int, open_brace: int) -> str:
    """Extract the mod name from the block."""
    chunk = content[attr_start:open_brace]
    m = re.search(r'mod\s+(\w+)', chunk)
    return m.group(1) if m else 'tests'


def process_file(rs_file: Path, dry_run: bool = False) -> bool:
    """
    Extract test block from rs_file. Returns True if extraction was done.
    """
    content = rs_file.read_text(encoding='utf-8')

    result = find_cfg_test_block(content)
    if result is None:
        return False

    attr_start, open_brace, close_brace = result
    mod_name = get_mod_name(content, attr_start, open_brace)
    body = extract_mod_body(content, open_brace, close_brace)

    # Determine companion file path
    is_mod_rs = rs_file.name == 'mod.rs'

    if is_mod_rs:
        # mod.rs → companion is tests.rs in same directory
        companion = rs_file.parent / 'tests.rs'
        mod_decl = f'#[cfg(test)]\nmod {mod_name};\n'
    else:
        # foo.rs → companion is foo_tests.rs in same directory
        stem = rs_file.stem
        companion = rs_file.parent / f'{stem}_tests.rs'
        mod_decl = f'#[cfg(test)]\n#[path = "{stem}_tests.rs"]\nmod {mod_name};\n'

    # Check if companion already exists
    if companion.exists():
        print(f'  SKIP {rs_file.relative_to(ROOT)} — companion already exists')
        return False

    # Build new source content: replace block with mod declaration
    # Include any trailing newline after close_brace
    end = close_brace + 1
    if end < len(content) and content[end] == '\n':
        end += 1

    new_content = content[:attr_start] + mod_decl + content[end:]

    # Build companion content
    companion_content = body + '\n'

    if dry_run:
        print(f'  DRY-RUN {rs_file.relative_to(ROOT)}')
        print(f'    → companion: {companion.relative_to(ROOT)}')
        print(f'    → body lines: {len(body.splitlines())}')
        return True

    # Write files
    rs_file.write_text(new_content, encoding='utf-8')
    companion.write_text(companion_content, encoding='utf-8')
    print(f'  ✓ {rs_file.relative_to(ROOT)} → {companion.name} ({len(body.splitlines())} lines)')
    return True


def cargo_check() -> bool:
    """Run cargo check and return True if it passes."""
    result = subprocess.run(
        ['cargo', 'check', '--manifest-path', str(CARGO_MANIFEST)],
        capture_output=True, text=True, timeout=120
    )
    if result.returncode != 0:
        print('  CARGO CHECK FAILED:')
        print(result.stderr[-2000:])
    return result.returncode == 0


def git_commit(rs_file: Path, companion: Path) -> None:
    cwd = ROOT.parent
    rel = rs_file.relative_to(cwd)
    rel_companion = companion.relative_to(cwd)
    msg = (
        f'refactor: extract inline tests from {rel}\n\n'
        f'Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>'
    )
    subprocess.run(['git', 'add', str(rel), str(rel_companion)], cwd=cwd, check=True)
    subprocess.run(['git', 'commit', '-m', msg], cwd=cwd, check=True)


def should_skip(rs_file: Path) -> bool:
    parts = set(rs_file.parts)
    if rs_file.name in SKIP_FILES:
        return True
    for part in rs_file.parts:
        if part in SKIP_DIRS:
            return True
    # Skip files that are already companion test files
    if rs_file.stem.endswith('_tests') or rs_file.name == 'tests.rs':
        return True
    return False


def collect_files(src_dir: Path) -> list[Path]:
    files = []
    for rs_file in sorted(src_dir.rglob('*.rs')):
        if should_skip(rs_file):
            continue
        content = rs_file.read_text(encoding='utf-8')
        if '#[cfg(test)]' in content:
            files.append(rs_file)
    return files


def main():
    parser = argparse.ArgumentParser(description='Extract inline Rust test modules to companion files')
    parser.add_argument('--dry-run', action='store_true', help='Preview only, no file changes')
    parser.add_argument('--file', help='Process a single file only')
    parser.add_argument('--no-commit', action='store_true', help='Skip git commits')
    parser.add_argument('--no-check', action='store_true', help='Skip cargo check (faster, use with caution)')
    args = parser.parse_args()

    src_dir = ROOT / 'src'

    if args.file:
        files = [Path(args.file).resolve()]
    else:
        print(f'Scanning {src_dir} for inline test blocks...')
        files = collect_files(src_dir)
        print(f'Found {len(files)} files with inline tests\n')

    skipped = []
    processed = []
    failed = []

    for rs_file in files:
        is_mod_rs = rs_file.name == 'mod.rs'
        stem = rs_file.stem
        companion = (
            rs_file.parent / 'tests.rs'
            if is_mod_rs
            else rs_file.parent / f'{stem}_tests.rs'
        )

        did_extract = process_file(rs_file, dry_run=args.dry_run)
        if not did_extract:
            skipped.append(rs_file)
            continue

        if args.dry_run:
            processed.append(rs_file)
            continue

        # Cargo check after each file
        if not args.no_check:
            if not cargo_check():
                print(f'  REVERTING {rs_file.name}')
                cwd = ROOT.parent
                subprocess.run(['git', 'checkout', '--', str(rs_file.relative_to(cwd))], cwd=cwd)
                if companion.exists():
                    companion.unlink()
                failed.append(rs_file)
                continue

        # Commit
        if not args.no_commit:
            try:
                git_commit(rs_file, companion)
            except subprocess.CalledProcessError as e:
                print(f'  COMMIT FAILED: {e}')
                failed.append(rs_file)
                continue

        processed.append(rs_file)

    print(f'\n{"DRY-RUN " if args.dry_run else ""}Summary:')
    print(f'  Processed: {len(processed)}')
    print(f'  Skipped:   {len(skipped)}')
    print(f'  Failed:    {len(failed)}')

    if failed:
        print('\nFailed files:')
        for f in failed:
            print(f'  {f.relative_to(ROOT)}')
        sys.exit(1)


if __name__ == '__main__':
    main()
