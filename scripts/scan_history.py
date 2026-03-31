#!/usr/bin/env python3
"""Git history sensitive content scanner for open-source release preparation.

Scans the full git history (blobs + commit messages) for non-secret sensitive
content and outputs a categorized Markdown report.

Usage:
    python3 scripts/scan_history.py [options]

Options:
    --output PATH       Output report path (default: reports/history-scan-report.md)
    --branches LIST     Comma-separated branch names (default: all branches via --all)
    --since DATE        Only scan commits after this date (e.g. 2023-01-01)
    --max-commits N     Limit commits scanned (useful for testing)
    --names LIST        Comma-separated literal substrings for internal name scanning
    --dangling          Also scan unreachable commits via git fsck
    --max-dangling N    Limit dangling commits scanned (default: 500)
"""

from __future__ import annotations

import argparse
import hashlib
import os
import re
import subprocess
import sys
from collections import defaultdict
from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path
from typing import Iterator

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

BINARY_EXTENSIONS = frozenset({
    ".png", ".jpg", ".jpeg", ".gif", ".bmp", ".tiff", ".ico", ".webp",
    ".svg",  # can contain scripts but skip for now
    ".db", ".sqlite", ".sqlite3",
    ".wasm",
    ".zip", ".tar", ".gz", ".bz2", ".xz", ".7z", ".rar",
    ".pdf", ".doc", ".docx", ".xls", ".xlsx", ".ppt", ".pptx",
    ".exe", ".dll", ".so", ".dylib", ".a", ".lib",
    ".mp3", ".mp4", ".wav", ".avi", ".mov", ".mkv",
    ".ttf", ".otf", ".woff", ".woff2", ".eot",
    ".lock",  # skip Cargo.lock / package-lock.json large binary-like files
})

# Paths that indicate a finding is likely benign (test fixtures, example configs)
BENIGN_PATH_PATTERNS = [
    re.compile(r"(?:^|/)tests?/"),
    re.compile(r"(?:^|/)test_"),
    re.compile(r"\.test\.[a-z]+$"),
    re.compile(r"(?:^|/)fixtures?/"),
    re.compile(r"\.env\.example$"),
    re.compile(r"(?:^|/)CLAUDE\.md$"),
    re.compile(r"\.toml$"),
    re.compile(r"vite\.config\.[a-z]+$"),
    re.compile(r"tauri\.conf\.[a-z]+$"),
    re.compile(r"(?:^|/)specs?/"),
    re.compile(r"(?:^|/)docs?/"),
    re.compile(r"(?:^|/)examples?/"),
]

# Category names (order matters for report)
CATEGORIES = [
    "Internal URLs",
    "Proprietary Comments",
    "Customer/Personal Data",
    "Secrets & Credentials",
    "License Issues",
    "Infrastructure Config",
    "Internal References",
    "Commit Messages",
]


# ---------------------------------------------------------------------------
# Data structures
# ---------------------------------------------------------------------------

@dataclass
class Finding:
    category: str
    sha: str
    date: str
    author: str
    file_path: str
    line_content: str
    match_text: str
    likely_benign: bool = False

    def dedup_key(self) -> str:
        content = f"{self.file_path}\x00{self.match_text.strip()}"
        return hashlib.sha256(content.encode("utf-8", errors="replace")).hexdigest()


@dataclass
class ScanResult:
    findings: list[Finding] = field(default_factory=list)
    commits_scanned: int = 0
    dangling_scanned: int = 0
    dangling_skipped: list[str] = field(default_factory=list)
    had_encoding_replacements: bool = False
    git_exit_code: int = 0


# ---------------------------------------------------------------------------
# Pattern engine
# ---------------------------------------------------------------------------

def build_patterns(extra_names: list[str]) -> dict[str, list[re.Pattern]]:
    """Build compiled regex patterns for each scan category."""
    patterns: dict[str, list[re.Pattern]] = {}

    # 1. Internal URLs
    patterns["Internal URLs"] = [
        re.compile(r"localhost:\d{4,5}", re.IGNORECASE),
        re.compile(r"127\.0\.0\.1:\d{4,5}"),
        re.compile(r"\bstaging\.[a-z0-9]", re.IGNORECASE),
        re.compile(r"\binternal\.[a-z0-9]", re.IGNORECASE),
        re.compile(r"\badmin\.[a-z0-9]", re.IGNORECASE),
        re.compile(r"[a-z0-9.-]+\.local\b", re.IGNORECASE),
        # Known internal ports for RalphX
        re.compile(r":3847\b"),
        re.compile(r":3848\b"),
    ]

    # 2. Proprietary Comments
    patterns["Proprietary Comments"] = [
        re.compile(r"TODO\s*\(\s*(?:Phase\s*\d+|WP\d+|D\d+|RC\d+)\s*\)", re.IGNORECASE),
        re.compile(r"FIXME[^\n]*internal", re.IGNORECASE),
        re.compile(r"HACK[^\n]*proprietary", re.IGNORECASE),
        re.compile(r"TODO[^\n]*internal", re.IGNORECASE),
        re.compile(r"NOTE[^\n]*proprietary", re.IGNORECASE),
    ]

    # 3. Customer/Personal Data
    patterns["Customer/Personal Data"] = [
        # Email patterns — exclude common placeholder/test domains
        re.compile(
            r"\b[A-Za-z0-9._%+-]+"
            r"@"
            r"(?!example\.com|test\.com|foo\.com|bar\.com|localhost\b)"
            r"[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b"
        ),
    ]

    # 4. Secrets & Credentials
    patterns["Secrets & Credentials"] = [
        re.compile(r"\bsk-ant-[A-Za-z0-9_-]{20,}"),
        re.compile(r"\bsk-or-v1-[A-Za-z0-9_-]{20,}"),
        re.compile(r"\brxk_live_[A-Za-z0-9_-]{10,}"),
        re.compile(r"\bghp_[A-Za-z0-9]{36,}"),
        re.compile(r"\bgho_[A-Za-z0-9]{36,}"),
        re.compile(r"ANTHROPIC_AUTH_TOKEN\s*=\s*(?!your_|<|\"your|'your)[^\s\"']{10,}"),
        re.compile(r"ANTHROPIC_API_KEY\s*=\s*(?!your_|<|\"your|'your)[^\s\"']{10,}"),
        # Bearer tokens: long token not followed by placeholder indicators
        re.compile(r"Bearer\s+[A-Za-z0-9_\-\.]{20,}(?!\s*[}>]|\s*token\b)", re.IGNORECASE),
        # password assignments with actual values (not empty or placeholder)
        re.compile(r"password\s*[=:]\s*[\"'][^\"']{4,}[\"']", re.IGNORECASE),
        re.compile(r"secret\s*[=:]\s*[\"'][^\"']{4,}[\"']", re.IGNORECASE),
    ]

    # 5. License Issues
    patterns["License Issues"] = [
        re.compile(r"Copyright\s+©", re.IGNORECASE),
        re.compile(r"All\s+rights\s+reserved", re.IGNORECASE),
        re.compile(r"\bproprietary\b", re.IGNORECASE),
        re.compile(r"\bconfidential\b", re.IGNORECASE),
        re.compile(r"MIT License", re.IGNORECASE),
        re.compile(r"GPL\s+(?:v\d+|\d+\.\d+)", re.IGNORECASE),
        re.compile(r"DO NOT DISTRIBUTE", re.IGNORECASE),
    ]

    # 6. Infrastructure Config
    patterns["Infrastructure Config"] = [
        # .env file additions
        re.compile(r"^\+.*\.env\b"),
        re.compile(r"APPLE_CERTIFICATE", re.IGNORECASE),
        re.compile(r"TAURI_SIGNING", re.IGNORECASE),
        # Hardcoded non-loopback IPs
        re.compile(r"\b(?!127\.0\.0\.1|0\.0\.0\.0|255\.255\.255\.255)(?:(?:25[0-5]|2[0-4]\d|[01]?\d\d?)\.){3}(?:25[0-5]|2[0-4]\d|[01]?\d\d?)\b"),
        # Database connection strings
        re.compile(r"(?:postgres|mysql|mongodb|redis)://[^\s\"'<>]+", re.IGNORECASE),
        # Generic connection strings
        re.compile(r"(?:Server|Host)\s*=\s*[^\s;\"'<>]{5,};\s*(?:Database|Uid|User)", re.IGNORECASE),
    ]

    # 7. Internal References
    internal_patterns = [
        re.compile(r"\bJIRA-\d+\b"),
        re.compile(r"\bLinear\b"),
        re.compile(r"\bSlack\b"),
        re.compile(r"~/\.ralphx/founder/"),
        re.compile(r"~/\.ralphx/strategy/"),
        re.compile(r"founder-profile\.md"),
        re.compile(r"project-goal-card\.md"),
        re.compile(r"project-metrics\.md"),
    ]
    # Add custom --names patterns
    for name in extra_names:
        if name.strip():
            internal_patterns.append(re.compile(re.escape(name.strip()), re.IGNORECASE))
    patterns["Internal References"] = internal_patterns

    # 8. Commit Messages — same patterns applied to commit subject/body
    # Build a combined set from categories 1-7 for commit message scanning
    commit_patterns: list[re.Pattern] = []
    for cat in [
        "Internal URLs", "Proprietary Comments", "Customer/Personal Data",
        "Secrets & Credentials", "License Issues", "Infrastructure Config",
        "Internal References",
    ]:
        commit_patterns.extend(patterns[cat])
    patterns["Commit Messages"] = commit_patterns

    return patterns


def is_likely_benign(file_path: str) -> bool:
    for pat in BENIGN_PATH_PATTERNS:
        if pat.search(file_path):
            return True
    return False


def is_binary_extension(file_path: str) -> bool:
    ext = Path(file_path).suffix.lower()
    return ext in BINARY_EXTENSIONS


# ---------------------------------------------------------------------------
# Git streaming parser
# ---------------------------------------------------------------------------

@dataclass
class CommitContext:
    sha: str = ""
    date: str = ""
    author: str = ""
    message_lines: list[str] = field(default_factory=list)
    in_message: bool = False


@dataclass
class StreamState:
    """Mutable state populated by stream_git_log after exhaustion."""
    exit_code: int = 0
    had_encoding_replacements: bool = False
    commits_in_stream: int = 0  # unique commit SHAs seen in stream


def stream_git_log(
    args: argparse.Namespace,
    state: StreamState,
) -> Iterator[tuple[str, str, str, str, str, bool]]:
    """Stream git log output and yield (sha, date, author, file_path, line, is_commit_msg).

    Populates `state` with exit_code and had_encoding_replacements after the generator
    is exhausted (or the caller breaks out of the loop).
    """
    cmd = [
        "git", "log",
        "-p", "-m",
        "--date-order",
        "--all",
        "--format=commit %H%nauthor %an%ndate %ad%n",
        "--date=iso-strict",
    ]
    if args.since:
        cmd.extend(["--since", args.since])
    if args.branches and args.branches != "all":
        # Override --all with specific branches
        cmd = [c for c in cmd if c != "--all"]
        cmd.extend(args.branches.split(","))
    if args.max_commits:
        cmd.extend(["--max-count", str(args.max_commits)])

    proc = subprocess.Popen(
        cmd,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )

    ctx = CommitContext()
    current_file: str = ""
    in_diff = False
    skip_binary_file = False
    _seen_shas: set[str] = set()

    assert proc.stdout is not None
    try:
        for raw_line in proc.stdout:
            # Decode with replacement for non-UTF-8 content
            line = raw_line.decode("utf-8", errors="replace")
            if "\ufffd" in line:
                state.had_encoding_replacements = True

            stripped = line.rstrip("\n")

            # --- Commit header lines ---
            if stripped.startswith("commit ") and len(stripped) == 47:
                # Flush commit message findings before switching context
                if ctx.sha and ctx.message_lines:
                    msg_text = " ".join(ctx.message_lines)
                    yield (ctx.sha, ctx.date, ctx.author, "<commit-message>", msg_text, True)
                new_sha = stripped[7:]
                ctx = CommitContext(sha=new_sha)
                if new_sha not in _seen_shas:
                    _seen_shas.add(new_sha)
                    state.commits_in_stream += 1
                in_diff = False
                current_file = ""
                skip_binary_file = False
                ctx.in_message = True
                continue

            if not ctx.sha:
                continue

            if stripped.startswith("author "):
                ctx.author = stripped[7:]
                continue

            if stripped.startswith("date "):
                ctx.date = stripped[5:25]  # ISO date portion
                continue

            # --- Diff headers ---
            if stripped.startswith("diff --git "):
                ctx.in_message = False
                in_diff = True
                # Extract file path from diff header: diff --git a/path b/path
                m = re.match(r"^diff --git a/.+ b/(.+)$", stripped)
                current_file = m.group(1) if m else ""
                skip_binary_file = is_binary_extension(current_file)
                continue

            if stripped.startswith("Binary files"):
                skip_binary_file = True
                continue

            # Skip +++ / --- diff header lines
            if in_diff and re.match(r"^(\+\+\+|---)\s", stripped):
                continue

            # Collect commit message lines (before first diff)
            if ctx.in_message and not in_diff:
                if stripped and not stripped.startswith("Merge:") and not stripped.startswith("Author:"):
                    ctx.message_lines.append(stripped)
                continue

            # --- Added diff lines only ---
            if in_diff and stripped.startswith("+") and not stripped.startswith("+++"):
                if skip_binary_file:
                    continue
                content_line = stripped[1:]  # strip leading +
                yield (ctx.sha, ctx.date, ctx.author, current_file, content_line, False)

        # Flush final commit message
        if ctx.sha and ctx.message_lines:
            msg_text = " ".join(ctx.message_lines)
            yield (ctx.sha, ctx.date, ctx.author, "<commit-message>", msg_text, True)

    finally:
        if hasattr(proc.stdout, "close"):
            proc.stdout.close()
        proc.wait()
        state.exit_code = proc.returncode


def count_reachable_commits() -> int:
    """Run git rev-list --all --count for cross-check."""
    try:
        result = subprocess.run(
            ["git", "rev-list", "--all", "--count"],
            capture_output=True, text=True, timeout=60
        )
        return int(result.stdout.strip())
    except Exception:
        return -1


# ---------------------------------------------------------------------------
# Dangling commit scanning
# ---------------------------------------------------------------------------

def get_dangling_shas(max_dangling: int) -> tuple[list[str], int]:
    """Return (shas_to_scan, count_truncated)."""
    try:
        result = subprocess.run(
            ["git", "fsck", "--unreachable", "--no-reflogs"],
            capture_output=True,
            text=True,
            timeout=120,
        )
        shas = []
        for line in result.stdout.splitlines():
            if line.startswith("unreachable commit "):
                shas.append(line.split()[-1])
        truncated = max(0, len(shas) - max_dangling)
        return shas[:max_dangling], truncated
    except Exception as e:
        print(f"[WARNING] git fsck failed: {e}", file=sys.stderr)
        return [], 0


def stream_dangling_commit(sha: str) -> Iterator[tuple[str, str, str, str, str, bool]]:
    """Scan a single dangling commit via git log -p -1 <sha>."""
    try:
        result = subprocess.run(
            ["git", "log", "-p", "-1", "--format=commit %H%nauthor %an%ndate %ad%n",
             "--date=iso-strict", sha],
            capture_output=True,
            timeout=30,
        )
        lines = result.stdout.decode("utf-8", errors="replace").splitlines()
        ctx = CommitContext()
        current_file = ""
        in_diff = False
        skip_binary = False
        for line in lines:
            if line.startswith("commit ") and len(line) == 47:
                ctx = CommitContext(sha=line[7:], in_message=True)
                continue
            if line.startswith("author "):
                ctx.author = line[7:]
                continue
            if line.startswith("date "):
                ctx.date = line[5:25]
                continue
            if line.startswith("diff --git "):
                ctx.in_message = False
                in_diff = True
                m = re.match(r"^diff --git a/.+ b/(.+)$", line)
                current_file = m.group(1) if m else ""
                skip_binary = is_binary_extension(current_file)
                continue
            if line.startswith("Binary files"):
                skip_binary = True
                continue
            if in_diff and re.match(r"^(\+\+\+|---)\s", line):
                continue
            if ctx.in_message and not in_diff:
                if line.strip():
                    ctx.message_lines.append(line.strip())
                continue
            if in_diff and line.startswith("+") and not line.startswith("+++"):
                if not skip_binary:
                    yield (ctx.sha, ctx.date, ctx.author, current_file, line[1:], False)
        if ctx.sha and ctx.message_lines:
            yield (ctx.sha, ctx.date, ctx.author, "<commit-message>",
                   " ".join(ctx.message_lines), True)
    except Exception as e:
        raise RuntimeError(f"Failed to scan dangling commit {sha}: {e}") from e


# ---------------------------------------------------------------------------
# Scanner core
# ---------------------------------------------------------------------------

def scan(args: argparse.Namespace) -> ScanResult:
    result = ScanResult()
    patterns = build_patterns(args.names.split(",") if args.names else [])
    seen: set[str] = set()  # deduplication set
    commits_seen: set[str] = set()

    def process_line(sha: str, date: str, author: str, file_path: str,
                     content: str, is_commit_msg: bool) -> None:
        # Determine which categories to check
        if is_commit_msg:
            categories_to_check = ["Commit Messages"]
        else:
            categories_to_check = [c for c in CATEGORIES if c != "Commit Messages"]

        benign = is_likely_benign(file_path) if not is_commit_msg else False

        for category in categories_to_check:
            for pat in patterns[category]:
                m = pat.search(content)
                if m:
                    finding = Finding(
                        category=category,
                        sha=sha,
                        date=date,
                        author=author,
                        file_path=file_path,
                        line_content=content.strip(),
                        match_text=m.group(0),
                        likely_benign=benign,
                    )
                    key = finding.dedup_key()
                    if key not in seen:
                        seen.add(key)
                        result.findings.append(finding)
                    # Only first matching pattern per category per line
                    break

    # --- Main scan ---
    stream_state = StreamState()
    for item in stream_git_log(args, stream_state):
        sha, date, author, file_path, content, is_commit_msg = item
        commits_seen.add(sha)
        process_line(sha, date, author, file_path, content, is_commit_msg)

    # Use stream-level count so commits with no yielded content are still counted
    result.commits_scanned = stream_state.commits_in_stream
    result.git_exit_code = stream_state.exit_code
    result.had_encoding_replacements = stream_state.had_encoding_replacements

    # --- Dangling scan ---
    if args.dangling:
        shas, truncated = get_dangling_shas(args.max_dangling)
        if truncated:
            print(f"[INFO] Dangling commit scan truncated to {args.max_dangling} (skipped {truncated})",
                  file=sys.stderr)

        for sha in shas:
            try:
                for item in stream_dangling_commit(sha):
                    sha2, date, author, file_path, content, is_commit_msg = item
                    if sha2 not in commits_seen:
                        commits_seen.add(sha2)
                        result.dangling_scanned += 1
                    process_line(sha2, date, author, file_path, content, is_commit_msg)
            except RuntimeError as e:
                print(f"[WARNING] {e}", file=sys.stderr)
                result.dangling_skipped.append(sha)

    return result


# ---------------------------------------------------------------------------
# Markdown report generator
# ---------------------------------------------------------------------------

def generate_report(result: ScanResult, args: argparse.Namespace) -> str:
    lines: list[str] = []
    now = datetime.utcnow().strftime("%Y-%m-%d %H:%M UTC")

    lines.append("# Git History Sensitive Content Scan Report")
    lines.append(f"\n**Generated:** {now}  ")
    lines.append(f"**Commits scanned:** {result.commits_scanned}  ")
    if args.dangling:
        lines.append(f"**Dangling commits scanned:** {result.dangling_scanned}  ")

    reachable = count_reachable_commits()
    if reachable >= 0 and not args.max_commits:
        status = "✅" if result.commits_scanned == reachable else "⚠️"
        lines.append(f"**Reachable commits (git rev-list):** {reachable} {status}  ")

    lines.append(f"**Total findings:** {len(result.findings)}  ")

    if result.git_exit_code != 0:
        lines.append(f"\n> ⚠️ **WARNING:** `git log` exited with code {result.git_exit_code}. "
                     "Scan may be incomplete.")

    if result.had_encoding_replacements:
        lines.append("\n> ⚠️ **NOTE:** Non-UTF-8 content encountered. Replacement characters (U+FFFD) "
                     "were inserted. Some content may be garbled.")

    # Per-category summary table
    lines.append("\n## Summary by Category\n")
    lines.append("| Category | Findings |")
    lines.append("|----------|----------|")
    cat_counts: dict[str, int] = defaultdict(int)
    for f in result.findings:
        cat_counts[f.category] += 1
    for cat in CATEGORIES:
        count = cat_counts.get(cat, 0)
        lines.append(f"| {cat} | {count} |")

    # Top files by finding count
    file_counts: dict[str, int] = defaultdict(int)
    for f in result.findings:
        file_counts[f.file_path] += 1
    top_files = sorted(file_counts.items(), key=lambda x: -x[1])[:10]
    if top_files:
        lines.append("\n## Top Files by Finding Count\n")
        lines.append("| File | Findings |")
        lines.append("|------|----------|")
        for fp, cnt in top_files:
            lines.append(f"| `{fp}` | {cnt} |")

    # Per-category sections
    findings_by_cat: dict[str, list[Finding]] = defaultdict(list)
    for f in result.findings:
        findings_by_cat[f.category].append(f)

    for cat in CATEGORIES:
        findings = findings_by_cat.get(cat, [])
        lines.append(f"\n## {cat}\n")
        if not findings:
            lines.append("_No findings._")
            continue
        lines.append(f"**{len(findings)} finding(s)**\n")
        for i, f in enumerate(findings, 1):
            benign_tag = " `[likely benign]`" if f.likely_benign else ""
            lines.append(f"### Finding {i}{benign_tag}\n")
            lines.append(f"- **SHA:** `{f.sha}`")
            lines.append(f"- **Date:** {f.date}")
            lines.append(f"- **Author:** {f.author}")
            lines.append(f"- **File:** `{f.file_path}`")
            lines.append(f"- **Match:** `{f.match_text}`")
            lines.append(f"- **Content:**")
            lines.append(f"  ```")
            # Escape triple backticks inside content to avoid breaking fences
            safe_content = f.line_content.replace("```", "` ` `")
            lines.append(f"  {safe_content}")
            lines.append(f"  ```")
            lines.append("")

    # Footer
    lines.append("\n---\n")
    if result.dangling_skipped:
        lines.append(f"**Skipped dangling commits ({len(result.dangling_skipped)}):** "
                     + ", ".join(f"`{s}`" for s in result.dangling_skipped[:20]))
        if len(result.dangling_skipped) > 20:
            lines.append(f"... and {len(result.dangling_skipped) - 20} more")
        lines.append("")

    lines.append(f"_Report generated by `scripts/scan_history.py` on {now}_")
    return "\n".join(lines)


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------

def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Scan full git history for sensitive content.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__,
    )
    parser.add_argument(
        "--output", default="reports/history-scan-report.md",
        help="Output report path (default: reports/history-scan-report.md)"
    )
    parser.add_argument(
        "--branches", default="all",
        help="Comma-separated branch names, or 'all' (default: all)"
    )
    parser.add_argument(
        "--since",
        help="Only scan commits after this date (e.g. 2023-01-01)"
    )
    parser.add_argument(
        "--max-commits", type=int, default=0, dest="max_commits",
        help="Limit number of commits scanned (0 = no limit)"
    )
    parser.add_argument(
        "--names", default="",
        help="Comma-separated literal substrings for internal name scanning (Category 7)"
    )
    parser.add_argument(
        "--dangling", action="store_true",
        help="Also scan unreachable commits via git fsck (slower)"
    )
    parser.add_argument(
        "--max-dangling", type=int, default=500, dest="max_dangling",
        help="Limit dangling commits scanned (default: 500)"
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)

    print(f"[INFO] Starting git history scan...", file=sys.stderr)
    result = scan(args)
    print(f"[INFO] Scanned {result.commits_scanned} commits, found {len(result.findings)} findings.",
          file=sys.stderr)

    report = generate_report(result, args)

    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(report, encoding="utf-8")
    print(f"[INFO] Report written to: {output_path}", file=sys.stderr)

    return 0


if __name__ == "__main__":
    sys.exit(main())
