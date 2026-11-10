#!/usr/bin/env python3
"""Regression tests for check_pr_description.py.

Failing cases mirror real descriptions that buried reviewers: section headings,
work logs, unlabeled prose, nested bullets, and bodies far past the limit. The
SKILL.md examples must pass, so the documented format and the check stay in sync.
"""

from __future__ import annotations

import re
import subprocess
import sys
import tempfile
from pathlib import Path

HELPER = Path(__file__).with_name("check_pr_description.py")
SKILL = Path(__file__).resolve().parent.parent / "SKILL.md"
TITLE = "fix(uploads): resume interrupted uploads from the last saved chunk"

GOOD = """\
Interrupted uploads now resume from the last saved chunk instead of restarting.

**Why:** Large uploads over mobile networks restart after any drop, so some never finish.

**Changes:**
- Save the confirmed chunk offset after each chunk.
- Resume from the saved offset when the client reconnects within 24 hours.

**Evidence:**

| File size | Retries before | after |
| --- | --- | --- |
| 2 GB | 7 | 1 |

**Rollout:** Behind `upload-resume`; turning it off restores full restarts.

**Review:** Is 24 hours the right resume window?

**Follow-up:** Expire abandoned offsets in a nightly job.

Refs: UP-142

🤖 Generated with [Claude Code](https://claude.com/claude-code)
"""


def run(title: str, body: str, *args: str) -> tuple[int, str]:
    with tempfile.NamedTemporaryFile("w", encoding="utf-8", suffix=".md") as handle:
        handle.write(body)
        handle.flush()
        result = subprocess.run(
            [sys.executable, str(HELPER), "--title", title, "--body-file", handle.name, *args],
            check=False,
            capture_output=True,
            text=True,
        )
    return result.returncode, result.stdout + result.stderr


def expect_pass(name: str, body: str, title: str = TITLE, *args: str) -> None:
    code, output = run(title, body, *args)
    assert code == 0, f"{name}: expected pass, got exit {code}:\n{output}"
    print(f"ok {name}")


def expect_fail(name: str, body: str, needle: str, title: str = TITLE, *args: str) -> None:
    code, output = run(title, body, *args)
    assert code == 1, f"{name}: expected exit 1, got {code}:\n{output}"
    assert needle in output, f"{name}: expected {needle!r} in:\n{output}"
    print(f"ok {name}")


def skill_examples() -> list[tuple[str, str]]:
    text = SKILL.read_text(encoding="utf-8")
    section = text.split("\n## Examples\n", 1)[1]
    examples = []
    for block in re.findall(r"```markdown\n(.*?)```", section, re.S):
        title, _, body = block.partition("\n\n")
        examples.append((title.strip(), body))
    assert len(examples) >= 2, "SKILL.md must keep at least two checked examples"
    return examples


def main() -> int:
    expect_pass("full_format", GOOD)
    expect_pass("opening_only", "Fix the broken install link in the README.\n")
    expect_pass("repo_title_tags", GOOD, "[low-risk] [no-soak] " + TITLE)
    expect_pass(
        "breaking_change_footer",
        "Rename the `--out` flag to `--output`.\n\nBREAKING CHANGE: scripts passing `--out` must use `--output`.\n",
        "feat(cli)!: rename the out flag to output",
    )
    expect_pass("io_is_not_first_person", "Batch file I/O so large exports stop timing out.\n")
    for index, (title, body) in enumerate(skill_examples()):
        expect_pass(f"skill_example_{index}", body, title)

    expect_fail("empty_body", "\n🤖 Generated with [Claude Code](https://claude.com/claude-code)\n", "body: empty")
    expect_fail("capitalized_title", GOOD, "title:", "fix(uploads): Resume uploads")
    expect_fail("title_trailing_period", GOOD, "title:", "fix(uploads): resume uploads.")
    expect_fail("title_without_type", GOOD, "title:", "Resume interrupted uploads")
    expect_fail("bang_without_footer", GOOD, "BREAKING CHANGE", "feat(uploads)!: resume uploads")
    expect_fail("markdown_heading", GOOD.replace("**Why:**", "## Why\n\n**Why:**"), "heading")
    expect_fail("unknown_label", GOOD.replace("**Rollout:**", "**Risk:**"), "label 'Risk'")
    expect_fail("period_label", GOOD.replace("**Rollout:**", "**Rollout.**"), "bold lead")
    expect_fail("labels_out_of_order", GOOD.replace("**Why:**", "**Review:** Is this right?\n\n**Why:**"), "out of order")
    expect_fail("duplicate_label", GOOD.replace("**Review:**", "**Why:** Again.\n\n**Review:**"), "more than once")
    expect_fail("unlabeled_paragraph", GOOD.replace("**Rollout:**", "Also tidy the logger.\n\n**Rollout:**"), "starts with a label")
    expect_fail(
        "footer_lookalike_is_still_checked",
        "Resume interrupted uploads.\n\nRollout: no flags. Change 1 takes effect on deploy.\n",
        "starts with a label",
    )
    expect_fail("opening_label", "**Why:** Uploads restart.\n", "opening:")
    expect_fail("long_opening", "Resume uploads " + "and keep going " * 20 + "now.\n", "opening:")
    expect_fail("nested_list", GOOD.replace("chunk.\n", "chunk.\n  - Also the header.\n", 1), "nested list")
    expect_fail("bold_list_lead", GOOD.replace("- Save", "- **Offsets.** Save"), "bold lead-ins")
    expect_fail("long_list_item", GOOD.replace("- Save", "- Save " + "words " * 30), "over 25 words")
    expect_fail("too_many_items", GOOD.replace("- Save", "- One.\n" * 6 + "- Save"), "items")
    expect_fail("list_without_label", GOOD.replace("**Rollout:**", "- Stray item.\n\n**Rollout:**"), "directly under a label")
    expect_fail("changes_inline", GOOD.replace("**Changes:**\n", "**Changes:** Save offsets.\n\n"), "label 'Changes'")
    expect_fail("table_outside_evidence", GOOD.replace("**Evidence:**\n", "| a |\n| --- |\n| b |\n\n**Evidence:**\n"), "table")
    expect_fail("inline_bold", GOOD.replace("so some never", "so **some** never"), "bold:")
    expect_fail("html_details", GOOD.replace("**Rollout:**", "<details>\n\n**Rollout:**"), "html")
    expect_fail("first_person", GOOD.replace("Large uploads", "I found large uploads"), "first person")
    expect_fail("this_pr", "This PR resumes interrupted uploads.\n", "'this PR'")
    expect_fail("test_counts", GOOD.replace("Behind", "153 tests pass. Behind"), "test counts")
    expect_fail("update_log", GOOD.replace("Refs:", "Update (Nov 3): fixed nits.\n\nRefs:"), "update log")
    expect_fail("session_notes", GOOD.replace("Behind", "A scratch check confirmed it. Behind"), "session notes")
    code_block = "```\n" + "line\n" * 13 + "```\n"
    expect_fail("long_code_block", GOOD.replace("**Rollout:**", code_block + "\n**Rollout:**"), "code block")
    long_body = GOOD.replace("Behind", "word " * 200 + "Behind")
    expect_fail("over_word_limit", long_body, "length:")
    expect_pass("raised_word_limit", long_body, TITLE, "--max-words", "400")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
