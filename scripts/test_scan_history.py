#!/usr/bin/env python3
"""Unit tests for scripts/scan_history.py.

Run with:
    python3 -m unittest discover -s scripts -p 'test_scan_history.py'
or:
    python3 scripts/test_scan_history.py
"""
import sys
import types
import unittest
from argparse import Namespace
from pathlib import Path
from unittest.mock import MagicMock, patch

# Ensure scripts/ is importable
sys.path.insert(0, str(Path(__file__).parent))

import scan_history as sh


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _make_args(**kwargs) -> Namespace:
    defaults = dict(
        output="reports/test-report.md",
        branches="all",
        since=None,
        max_commits=0,
        names="",
        dangling=False,
        max_dangling=500,
    )
    defaults.update(kwargs)
    return Namespace(**defaults)


def _patterns(extra_names: list[str] | None = None):
    return sh.build_patterns(extra_names or [])


# ---------------------------------------------------------------------------
# Pattern tests
# ---------------------------------------------------------------------------

class TestInternalURLPatterns(unittest.TestCase):
    def setUp(self):
        self.pats = _patterns()["Internal URLs"]

    def _match(self, text: str) -> bool:
        return any(p.search(text) for p in self.pats)

    def test_localhost_port(self):
        self.assertTrue(self._match("http://localhost:3847/api"))

    def test_127_ip_port(self):
        self.assertTrue(self._match("http://127.0.0.1:5173"))

    def test_staging_domain(self):
        self.assertTrue(self._match("https://staging.example.com"))

    def test_internal_domain(self):
        self.assertTrue(self._match("https://internal.company.io"))

    def test_dot_local(self):
        self.assertTrue(self._match("myserver.local"))

    def test_no_false_positive_plain_localhost(self):
        # localhost without port — should NOT match localhost:port pattern
        self.assertFalse(self._match("connect to localhost without port"))

    def test_no_false_positive_random_domain(self):
        self.assertFalse(self._match("https://www.example.com"))

    def test_no_false_positive_partial_internal_domain(self):
        self.assertFalse(self._match("internal.c"))

    def test_known_local_config_name_ignored(self):
        self.assertTrue(self._match("settings.local"))
        self.assertTrue(sh.should_ignore_internal_url_match("settings.local", "settings.local"))


class TestProprietaryCommentPatterns(unittest.TestCase):
    def setUp(self):
        self.pats = _patterns()["Proprietary Comments"]

    def _match(self, text: str) -> bool:
        return any(p.search(text) for p in self.pats)

    def test_todo_wp(self):
        self.assertTrue(self._match("// TODO(WP2): implement this"))

    def test_todo_phase(self):
        self.assertTrue(self._match("// TODO(Phase 3): refactor"))

    def test_todo_d(self):
        self.assertTrue(self._match("# TODO(D1): fix later"))

    def test_todo_rc(self):
        self.assertTrue(self._match("// TODO(RC1): polish"))

    def test_fixme_internal(self):
        self.assertTrue(self._match("// FIXME: internal workaround"))

    def test_no_plain_todo(self):
        # Plain TODO without a phase/wp marker should NOT match TODO(WP...) pattern
        self.assertFalse(self._match("// TODO: fix this"))

    def test_no_false_positive_internal_status(self):
        self.assertFalse(self._match("TargetColumn::Todo => InternalStatus::Ready"))


class TestCustomerDataPatterns(unittest.TestCase):
    def setUp(self):
        self.pats = _patterns()["Customer/Personal Data"]

    def _match(self, text: str) -> bool:
        return any(p.search(text) for p in self.pats)

    def test_real_email(self):
        self.assertTrue(self._match("contact: john.doe@acme.corp"))

    def test_company_email(self):
        self.assertTrue(self._match("email: ceo@ralphx.io"))

    def test_no_placeholder_email(self):
        self.assertFalse(self._match("test@example.com"))

    def test_no_test_email(self):
        self.assertFalse(self._match("user@test.com"))

    def test_no_git_remote_email(self):
        match = next((p.search("git@github.com:owner/repo.git") for p in self.pats if p.search("git@github.com:owner/repo.git")), None)
        self.assertIsNotNone(match)
        self.assertTrue(sh.should_ignore_customer_email(match.group(0)))

    def test_no_noreply_email(self):
        match = next((p.search("Co-Authored-By: Claude <noreply@anthropic.com>") for p in self.pats if p.search("Co-Authored-By: Claude <noreply@anthropic.com>")), None)
        self.assertIsNotNone(match)
        self.assertTrue(sh.should_ignore_customer_email(match.group(0)))


class TestSecretsPatterns(unittest.TestCase):
    def setUp(self):
        self.pats = _patterns()["Secrets & Credentials"]

    def _match(self, text: str) -> bool:
        return any(p.search(text) for p in self.pats)

    def test_anthropic_key(self):
        self.assertTrue(self._match("sk-ant-api03-AAAA1111BBBB2222CCCC3333DDDD4444"))

    def test_openrouter_key(self):
        self.assertTrue(self._match("sk-or-v1-1234567890abcdefghijklmnopqrstuv"))

    def test_ralphx_live_key(self):
        self.assertTrue(self._match("rxk_live_abc123def456"))

    def test_github_pat(self):
        self.assertTrue(self._match("token: ghp_" + "A" * 36))

    def test_password_assignment(self):
        self.assertTrue(self._match('password = "mysecretpass"'))

    def test_no_bearer_placeholder(self):
        # Bearer followed by template variable should not match
        self.assertFalse(self._match("Authorization: Bearer ${TOKEN}"))

    def test_no_placeholder_anthropic_token(self):
        self.assertFalse(self._match("ANTHROPIC_AUTH_TOKEN=your_token_here"))

    def test_secret_assignment(self):
        self.assertTrue(self._match('secret = "abc123xyz"'))


class TestLicensePatterns(unittest.TestCase):
    def setUp(self):
        self.pats = _patterns()["License Issues"]

    def _match(self, text: str) -> bool:
        return any(p.search(text) for p in self.pats)

    def test_copyright_symbol(self):
        self.assertTrue(self._match("Copyright © 2024 Acme Corp"))

    def test_all_rights_reserved(self):
        self.assertTrue(self._match("All rights reserved."))

    def test_proprietary(self):
        self.assertTrue(self._match("This is proprietary software."))

    def test_confidential(self):
        self.assertTrue(self._match("CONFIDENTIAL: do not share"))

    def test_no_license(self):
        self.assertFalse(self._match("Licensed under Apache-2.0"))


class TestInfrastructurePatterns(unittest.TestCase):
    def setUp(self):
        self.pats = _patterns()["Infrastructure Config"]

    def _match(self, text: str) -> bool:
        return any(p.search(text) for p in self.pats)

    def test_apple_certificate(self):
        self.assertTrue(self._match("APPLE_CERTIFICATE=base64data"))

    def test_tauri_signing(self):
        self.assertTrue(self._match("TAURI_SIGNING_PRIVATE_KEY=..."))

    def test_non_localhost_ip(self):
        self.assertTrue(self._match("server=192.168.1.100"))

    def test_postgres_url(self):
        self.assertTrue(self._match("postgres://user:pass@host/db"))

    def test_mongodb_url(self):
        self.assertTrue(self._match("mongodb://admin:secret@cluster.example.com:27017/mydb"))

    def test_no_loopback_ip(self):
        self.assertFalse(self._match("bind to 127.0.0.1"))

    def test_no_wildcard_ip(self):
        self.assertFalse(self._match("listen 0.0.0.0"))

    def test_no_svg_path_false_positive(self):
        self.assertFalse(self._match('d="M16.2 10a6.2 6.2 0 01-.1 1.2l2.1 1.6"'))


class TestInternalReferencesPatterns(unittest.TestCase):
    def setUp(self):
        self.pats = _patterns()["Internal References"]

    def _match(self, text: str) -> bool:
        return any(p.search(text) for p in self.pats)

    def test_jira_ref(self):
        self.assertTrue(self._match("See JIRA-1234 for context"))

    def test_founder_path(self):
        self.assertTrue(self._match("load ~/.ralphx/founder/profile.md"))

    def test_strategy_path(self):
        self.assertTrue(self._match("see ~/.ralphx/strategy/roadmap.md"))

    def test_no_false_positive(self):
        self.assertFalse(self._match("regular comment without refs"))

    def test_no_linear_false_positive(self):
        self.assertFalse(self._match("Linear interpolate between two RGB colors."))

    def test_no_slack_false_positive(self):
        self.assertFalse(self._match("Follow Slack-style thread layout."))


class TestCustomNamesPatterns(unittest.TestCase):
    def test_custom_name_match(self):
        pats = _patterns(["Alice Smith", "bob.jones"])["Internal References"]
        hits = [p for p in pats if p.search("contact Alice Smith for details")]
        self.assertTrue(len(hits) > 0)

    def test_custom_name_special_chars_escaped(self):
        # Name with dots and parens — must be re.escape'd, not treated as regex
        pats = _patterns(["bob.jones(dev)"])["Internal References"]
        # Should match literal "bob.jones(dev)" but not "bob_jones_dev"
        hits = [p for p in pats if p.search("bob.jones(dev)")]
        self.assertTrue(len(hits) > 0)
        misses = [p for p in pats if p.search("bob_jones_dev")]
        # re.escape turns . into \. so this should not match
        # (dot in regex means "any char" only if not escaped)
        # Verify the pattern was escaped properly:
        raw = pats[-1].pattern
        self.assertIn(r"bob\.jones\(dev\)", raw)

    def test_empty_names_no_crash(self):
        pats = _patterns([""])["Internal References"]
        self.assertIsInstance(pats, list)


# ---------------------------------------------------------------------------
# Parser tests
# ---------------------------------------------------------------------------

class TestStreamParser(unittest.TestCase):
    """Test stream_git_log by mocking subprocess output."""

    FIXTURE_DIFF = b"""\
commit abc1234567890123456789012345678901234567
author Jane Doe
date 2024-01-15T10:30:00+00:00

    Add feature X

diff --git a/src/app.ts b/src/app.ts
--- a/src/app.ts
+++ b/src/app.ts
@@ -1,3 +1,4 @@
 const x = 1;
+const secret = "sk-ant-api03-AAAA1111BBBB2222CCCC3333DDDD4444";
 const y = 2;
"""

    FIXTURE_BINARY = b"""\
commit def1234567890123456789012345678901234567
author John Smith
date 2024-01-16T11:00:00+00:00

    Add image

diff --git a/assets/logo.png b/assets/logo.png
Binary files /dev/null and b/assets/logo.png differ
"""

    FIXTURE_MERGE_COMMIT = b"""\
commit fed1234567890123456789012345678901234567
author Merge Bot
date 2024-01-17T12:00:00+00:00

    Merge branch 'feature' into main

diff --git a/src/config.ts b/src/config.ts
--- a/src/config.ts
+++ b/src/config.ts
@@ -1,2 +1,3 @@
 const port = 3000;
+const adminUrl = "https://admin.internal.company.com";
"""

    def _run_stream(self, fixture: bytes) -> list[tuple]:
        """Run stream_git_log against fixture bytes and collect yielded items."""
        raw_lines = [line + b"\n" for line in fixture.split(b"\n") if line]

        proc_mock = MagicMock()
        proc_mock.stdout = MagicMock()
        proc_mock.stdout.__iter__ = MagicMock(return_value=iter(raw_lines))
        proc_mock.stdout.close = MagicMock()
        proc_mock.returncode = 0
        proc_mock.wait = MagicMock()

        args = _make_args()
        state = sh.StreamState()

        with patch("subprocess.Popen", return_value=proc_mock):
            items = list(sh.stream_git_log(args, state))
        return items, state

    def test_normal_commit_added_line(self):
        items, state = self._run_stream(self.FIXTURE_DIFF)
        # Should have one diff line item + one commit message item
        diff_items = [i for i in items if i[3] == "src/app.ts"]
        self.assertEqual(len(diff_items), 1)
        self.assertIn("sk-ant-api03", diff_items[0][4])

    def test_commit_message_collected(self):
        items, state = self._run_stream(self.FIXTURE_DIFF)
        msg_items = [i for i in items if i[3] == "<commit-message>"]
        self.assertEqual(len(msg_items), 1)
        self.assertIn("Add feature X", msg_items[0][4])
        self.assertTrue(msg_items[0][5])  # is_commit_msg=True

    def test_binary_file_skipped(self):
        items, state = self._run_stream(self.FIXTURE_BINARY)
        # No diff content items from binary file
        diff_items = [i for i in items if "logo.png" in i[3]]
        self.assertEqual(len(diff_items), 0)

    def test_diff_header_lines_skipped(self):
        items, state = self._run_stream(self.FIXTURE_DIFF)
        # +++ / --- lines must not appear as content
        for item in items:
            self.assertFalse(item[4].startswith("+++"))
            self.assertFalse(item[4].startswith("---"))

    def test_merge_commit_content(self):
        items, state = self._run_stream(self.FIXTURE_MERGE_COMMIT)
        diff_items = [i for i in items if i[3] == "src/config.ts"]
        self.assertEqual(len(diff_items), 1)
        self.assertIn("admin.internal.company.com", diff_items[0][4])


class TestDanglingParser(unittest.TestCase):
    """Test dangling commit fsck parsing and scanning."""

    FSCK_OUTPUT = """\
Checking object directories: 100%
unreachable commit aaa0000000000000000000000000000000000001
unreachable blob bbb0000000000000000000000000000000000002
unreachable commit ccc0000000000000000000000000000000000003
"""

    def test_fsck_parsing(self):
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(
                stdout=self.FSCK_OUTPUT,
                returncode=0,
            )
            shas, truncated = sh.get_dangling_shas(max_dangling=10)
        self.assertEqual(len(shas), 2)
        self.assertIn("aaa0000000000000000000000000000000000001", shas)
        self.assertIn("ccc0000000000000000000000000000000000003", shas)
        self.assertEqual(truncated, 0)

    def test_fsck_truncation(self):
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(
                stdout=self.FSCK_OUTPUT,
                returncode=0,
            )
            shas, truncated = sh.get_dangling_shas(max_dangling=1)
        self.assertEqual(len(shas), 1)
        self.assertEqual(truncated, 1)

    def test_fsck_failure_returns_empty(self):
        with patch("subprocess.run", side_effect=Exception("git not found")):
            shas, truncated = sh.get_dangling_shas(max_dangling=500)
        self.assertEqual(shas, [])
        self.assertEqual(truncated, 0)

    def test_dangling_commit_scan(self):
        """stream_dangling_commit should yield findings from a fixture log."""
        fixture_log = (
            "commit zzz1234567890123456789012345678901234567\n"
            "author Test User\n"
            "date 2024-01-01T00:00:00+00:00\n"
            "\n"
            "    Dangling commit with secret\n"
            "\n"
            "diff --git a/file.txt b/file.txt\n"
            "--- a/file.txt\n"
            "+++ b/file.txt\n"
            "@@ -1 +1 @@\n"
            "+password = \"hunter2\"\n"
        )
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(
                stdout=fixture_log.encode(),
                returncode=0,
            )
            items = list(sh.stream_dangling_commit("zzz1234567890123456789012345678901234567"))
        diff_items = [i for i in items if i[3] == "file.txt"]
        self.assertEqual(len(diff_items), 1)
        self.assertIn('password = "hunter2"', diff_items[0][4])

    def test_dangling_error_recovery(self):
        """stream_dangling_commit should raise RuntimeError on failure."""
        with patch("subprocess.run", side_effect=Exception("sha corrupted")):
            with self.assertRaises(RuntimeError):
                list(sh.stream_dangling_commit("badc0ffee0"))


# ---------------------------------------------------------------------------
# Deduplication test
# ---------------------------------------------------------------------------

class TestDeduplication(unittest.TestCase):
    def test_same_content_same_path_deduplicated(self):
        f1 = sh.Finding(
            category="Secrets & Credentials",
            sha="abc123", date="2024-01-01", author="Alice",
            file_path="src/config.ts",
            line_content='const sk = "sk-ant-api03-XXXX"',
            match_text="sk-ant-api03-XXXX",
        )
        f2 = sh.Finding(
            category="Secrets & Credentials",
            sha="def456", date="2024-01-02", author="Bob",
            file_path="src/config.ts",
            line_content='const sk = "sk-ant-api03-XXXX"',
            match_text="sk-ant-api03-XXXX",
        )
        self.assertEqual(f1.dedup_key(), f2.dedup_key())

    def test_different_paths_not_deduplicated(self):
        f1 = sh.Finding(
            category="Secrets & Credentials",
            sha="abc123", date="2024-01-01", author="Alice",
            file_path="src/config.ts",
            line_content="password = secret",
            match_text="password = secret",
        )
        f2 = sh.Finding(
            category="Secrets & Credentials",
            sha="abc123", date="2024-01-01", author="Alice",
            file_path="src/other.ts",
            line_content="password = secret",
            match_text="password = secret",
        )
        self.assertNotEqual(f1.dedup_key(), f2.dedup_key())

    def test_scan_deduplicates(self):
        """Full scan should deduplicate the same finding appearing twice."""
        # Build a fixture with same content in two commits
        fixture_content = [
            ("sha0000000000000000000000000000000000001", "2024-01-01", "Alice",
             "src/config.ts", 'sk-ant-api03-AAAA1111BBBB2222CCCC3333DDDD4444', False),
            ("sha0000000000000000000000000000000000002", "2024-01-02", "Alice",
             "src/config.ts", 'sk-ant-api03-AAAA1111BBBB2222CCCC3333DDDD4444', False),
        ]
        args = _make_args()
        state = sh.StreamState()
        with patch.object(sh, "stream_git_log", return_value=iter(fixture_content)):
            result = sh.scan(args)

        # Should only appear once despite two commits
        secret_findings = [f for f in result.findings if f.category == "Secrets & Credentials"]
        self.assertEqual(len(secret_findings), 1)

    def test_skips_generated_build_artifacts(self):
        fixture_content = [
            ("sha0000000000000000000000000000000000001", "2024-01-01", "Alice",
             "pkg/build/index.js", 'const url = "http://127.0.0.1:3847";', False),
        ]
        args = _make_args()
        with patch.object(sh, "stream_git_log", return_value=iter(fixture_content)):
            with patch.object(sh, "stream_author_metadata", return_value=iter(())):
                result = sh.scan(args)

        self.assertEqual(result.findings, [])

    def test_skips_scanner_self_regex_matches(self):
        fixture_content = [
            ("sha0000000000000000000000000000000000001", "2024-01-01", "Alice",
             "scripts/scan_history.py", 're.compile(r"APPLE_CERTIFICATE", re.IGNORECASE),', False),
        ]
        args = _make_args()
        with patch.object(sh, "stream_git_log", return_value=iter(fixture_content)):
            with patch.object(sh, "stream_author_metadata", return_value=iter(())):
                result = sh.scan(args)

        self.assertEqual(result.findings, [])


class TestAuthorMetadata(unittest.TestCase):
    def test_personal_email_flagged(self):
        args = _make_args()
        with patch.object(sh, "stream_git_log", return_value=iter(())):
            with patch.object(sh, "stream_author_metadata", return_value=iter([
                ("abc1234567890123456789012345678901234567", "2024-01-01", "Alice", "alice@example.org"),
            ])):
                result = sh.scan(args)

        findings = [f for f in result.findings if f.category == "Author Metadata"]
        self.assertEqual(len(findings), 1)
        self.assertEqual(findings[0].match_text, "alice@example.org")

    def test_github_noreply_not_flagged(self):
        args = _make_args()
        with patch.object(sh, "stream_git_log", return_value=iter(())):
            with patch.object(sh, "stream_author_metadata", return_value=iter([
                ("abc1234567890123456789012345678901234567", "2024-01-01", "Alice", "12345+alice@users.noreply.github.com"),
            ])):
                result = sh.scan(args)

        findings = [f for f in result.findings if f.category == "Author Metadata"]
        self.assertEqual(findings, [])

    def test_invalid_author_email_flagged(self):
        args = _make_args()
        with patch.object(sh, "stream_git_log", return_value=iter(())):
            with patch.object(sh, "stream_author_metadata", return_value=iter([
                ("abc1234567890123456789012345678901234567", "2024-01-01", "Opus", "Opus 4.6"),
            ])):
                result = sh.scan(args)

        findings = [f for f in result.findings if f.category == "Author Metadata"]
        self.assertEqual(len(findings), 1)
        self.assertEqual(findings[0].match_text, "Opus 4.6")


# ---------------------------------------------------------------------------
# Path annotation test
# ---------------------------------------------------------------------------

class TestPathAnnotation(unittest.TestCase):
    def test_test_path_annotated(self):
        self.assertTrue(sh.is_likely_benign("tests/fixtures/sample.ts"))
        self.assertTrue(sh.is_likely_benign("src/test_helpers.py"))
        self.assertTrue(sh.is_likely_benign("components/Button.test.tsx"))

    def test_env_example_annotated(self):
        self.assertTrue(sh.is_likely_benign(".env.example"))

    def test_claude_md_annotated(self):
        self.assertTrue(sh.is_likely_benign("CLAUDE.md"))

    def test_toml_annotated(self):
        self.assertTrue(sh.is_likely_benign("Cargo.toml"))

    def test_rust_and_js_test_conventions_annotated(self):
        self.assertTrue(sh.is_likely_benign("src-tauri/src/utils/secret_redactor_tests.rs"))
        self.assertTrue(sh.is_likely_benign("ralphx-plugin/ralphx-mcp-server/src/__tests__/redact.test.ts"))

    def test_generated_and_mock_paths_skipped(self):
        self.assertTrue(sh.should_skip_file("scripts/test_scan_history.py"))
        self.assertTrue(sh.should_skip_file("ralphx-plugin/ralphx-external-mcp/build/index.js"))
        self.assertTrue(
            sh.should_skip_file(
                "screenshots/features/2026-02-07_phase85-visual-verification_mock-check.md"
            )
        )

    def test_production_code_not_annotated(self):
        self.assertFalse(sh.is_likely_benign("src/services/auth.ts"))
        self.assertFalse(sh.is_likely_benign("src-tauri/src/commands.rs"))
        self.assertFalse(sh.should_skip_file("src/services/auth.ts"))


# ---------------------------------------------------------------------------
# Report validity test
# ---------------------------------------------------------------------------

class TestReportGeneration(unittest.TestCase):
    def _make_findings(self) -> list[sh.Finding]:
        return [
            sh.Finding(
                category="Secrets & Credentials",
                sha="abc1234567890123456789012345678901234567",
                date="2024-01-01T00:00:00",
                author="Alice",
                file_path="src/config.ts",
                line_content='const key = "sk-ant-api03-XXXX"',
                match_text="sk-ant-api03-XXXX",
            ),
            sh.Finding(
                category="Internal URLs",
                sha="def1234567890123456789012345678901234567",
                date="2024-01-02T00:00:00",
                author="Bob",
                file_path="tests/setup.ts",
                line_content="const url = 'http://localhost:3847'",
                match_text="localhost:3847",
                likely_benign=True,
            ),
        ]

    def test_report_has_valid_headers(self):
        args = _make_args()
        result = sh.ScanResult(findings=self._make_findings(), commits_scanned=10)
        with patch.object(sh, "count_reachable_commits", return_value=-1):
            report = sh.generate_report(result, args)

        self.assertIn("# Git History Sensitive Content Scan Report", report)
        self.assertIn("## Summary by Category", report)
        self.assertIn("## Secrets & Credentials", report)
        self.assertIn("## Internal URLs", report)
        self.assertIn("## Author Metadata", report)

    def test_report_balanced_code_fences(self):
        args = _make_args()
        result = sh.ScanResult(findings=self._make_findings(), commits_scanned=10)
        with patch.object(sh, "count_reachable_commits", return_value=-1):
            report = sh.generate_report(result, args)

        # Count opening and closing ``` fences — must be balanced (even number)
        fence_count = report.count("```")
        self.assertEqual(fence_count % 2, 0, f"Unbalanced code fences: {fence_count}")

    def test_report_likely_benign_tag(self):
        args = _make_args()
        result = sh.ScanResult(findings=self._make_findings(), commits_scanned=10)
        with patch.object(sh, "count_reachable_commits", return_value=-1):
            report = sh.generate_report(result, args)

        self.assertIn("[likely benign]", report)

    def test_report_empty_findings(self):
        args = _make_args()
        result = sh.ScanResult(findings=[], commits_scanned=5)
        with patch.object(sh, "count_reachable_commits", return_value=-1):
            report = sh.generate_report(result, args)

        self.assertIn("_No findings._", report)
        self.assertIn("**Total findings:** 0", report)

    def test_report_git_exit_code_warning(self):
        args = _make_args()
        result = sh.ScanResult(findings=[], commits_scanned=0, git_exit_code=128)
        with patch.object(sh, "count_reachable_commits", return_value=-1):
            report = sh.generate_report(result, args)

        self.assertIn("WARNING", report)
        self.assertIn("128", report)

    def test_report_encoding_warning(self):
        args = _make_args()
        result = sh.ScanResult(findings=[], commits_scanned=0, had_encoding_replacements=True)
        with patch.object(sh, "count_reachable_commits", return_value=-1):
            report = sh.generate_report(result, args)

        self.assertIn("Non-UTF-8", report)

    def test_report_dangling_skipped_footer(self):
        args = _make_args(dangling=True)
        result = sh.ScanResult(
            findings=[],
            commits_scanned=0,
            dangling_scanned=2,
            dangling_skipped=["abc123", "def456"],
        )
        with patch.object(sh, "count_reachable_commits", return_value=-1):
            report = sh.generate_report(result, args)

        self.assertIn("Skipped dangling commits", report)
        self.assertIn("abc123", report)

    def test_report_notes_history_change(self):
        args = _make_args()
        result = sh.ScanResult(
            findings=[],
            commits_scanned=10,
            reachable_commits_before=10,
            reachable_commits_after=11,
        )
        report = sh.generate_report(result, args)

        self.assertIn("Git history changed while the scan was running", report)


# ---------------------------------------------------------------------------
# Binary extension test
# ---------------------------------------------------------------------------

class TestBinaryDetection(unittest.TestCase):
    def test_png_is_binary(self):
        self.assertTrue(sh.is_binary_extension("image.png"))

    def test_db_is_binary(self):
        self.assertTrue(sh.is_binary_extension("data.db"))

    def test_wasm_is_binary(self):
        self.assertTrue(sh.is_binary_extension("module.wasm"))

    def test_ts_not_binary(self):
        self.assertFalse(sh.is_binary_extension("src/index.ts"))

    def test_rs_not_binary(self):
        self.assertFalse(sh.is_binary_extension("main.rs"))


if __name__ == "__main__":
    unittest.main(verbosity=2)
