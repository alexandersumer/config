#!/usr/bin/env python3
"""Check a PR title and description against the branch-description format.

Usage:
  check_pr_description.py --title "<title>" --body-file <path> [--max-words N]

The body is the opening paragraph, then optional labeled sections in a fixed
order, then optional Conventional Commit footers. Exit 0 means the form is
valid. Exit 1 prints one problem per line. Exit 2 means the input could not be
read. The check covers form only; accuracy and clarity stay with the writer.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

DEFAULT_MAX_WORDS = 200
MAX_OPENING_WORDS = 50
MAX_LIST_ITEMS = 6
MAX_ITEM_WORDS = 25
MAX_TABLE_ROWS = 6
MAX_CODE_LINES = 12

LABELS = ("Why", "Changes", "Evidence", "Rollout", "Review", "Follow-up")

SUBJECT = re.compile(
    r"^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)"
    r"(\([a-z0-9-]+\))?!?: [a-z].{0,70}[^.]$"
)
TITLE_TAGS = re.compile(r"^(\[[a-z0-9-]+\] )*")
LABEL = re.compile(r"^\*\*([^*]+?):\*\*(?:\s+(.*))?$", re.S)
BOLD_LEAD = re.compile(r"^\*\*[^*]+\*\*")
LIST_ITEM = re.compile(r"^([-*+]|\d+[.)]) +")
NESTED_ITEM = re.compile(r"^\s+([-*+]|\d+[.)]) +")
HEADING = re.compile(r"^(#{1,6} |=+$|-{3,}$)")
TRAILER = re.compile(
    r"^(BREAKING[ -]CHANGE|Refs|Fixes|Closes|Resolves|Related|Issue|Jira|[A-Z][a-z]*(-[A-Za-z]+)+)"
    r"(: \S| #\S)"
)
ATTRIBUTION = re.compile(r"Generated with \[|^Co-Authored-By: ", re.I)
CODE_SPAN = re.compile(r"`[^`]*`")
URL = re.compile(r"https?://\S+")

WORK_LOG = (
    (re.compile(r"\bI\b(?!/)"), "first person 'I': describe the change, not your session"),
    (re.compile(r"\bthis (pr|pull request)\b", re.I), "'this PR': state what changes directly"),
    (re.compile(r"\b\d[\d,]* (unit |integration |e2e )?tests?\b( pass| ran| run|,)", re.I),
     "test counts: name the test that fails without the change instead"),
    (re.compile(r"^(update|edit|edited)( \d+| \([^)]*\))?:", re.I | re.M),
     "update log: rewrite the whole description instead of appending"),
    (re.compile(r"\b(not committed|scratch check|left out on purpose)\b", re.I),
     "session notes: keep investigation details out of the description"),
)


def words(text: str) -> int:
    text = URL.sub("url", text)
    return sum(1 for token in text.split() if re.search(r"[\w\d]", token))


def subject_of(title: str) -> str:
    return TITLE_TAGS.sub("", title.strip(), count=1)


def check_title(title: str) -> list[str]:
    if not title.strip():
        return ["title: empty"]
    if not SUBJECT.match(subject_of(title)):
        return [
            "title: after optional [tag] prefixes, must be a Conventional Commit subject "
            "matching type(scope)!: lowercase description, at most 72 characters, no trailing period"
        ]
    return []


def split_footers(lines: list[str]) -> tuple[list[str], list[str]]:
    """Strip attribution lines and the trailing footer block."""
    lines = [line for line in lines if not ATTRIBUTION.search(line)]
    while lines and not lines[-1].strip():
        lines.pop()
    end = len(lines)
    while end and TRAILER.match(lines[end - 1].strip()):
        end -= 1
    if end < len(lines) and (end == 0 or not lines[end - 1].strip()):
        footers = lines[end:]
        lines = lines[:end]
    else:
        footers = []
    return lines, footers


def blocks(lines: list[str]) -> list[list[str]]:
    """Split into blank-line separated blocks, keeping fenced code intact."""
    result: list[list[str]] = []
    current: list[str] = []
    in_fence = False
    for line in lines:
        if line.strip().startswith("```"):
            in_fence = not in_fence
            current.append(line)
            continue
        if not line.strip() and not in_fence:
            if current:
                result.append(current)
                current = []
            continue
        current.append(line)
    if current:
        result.append(current)
    return result


def split_labels(all_blocks: list[list[str]]) -> list[list[str]]:
    """Separate a label line from a list or table written directly under it."""
    result: list[list[str]] = []
    for block in all_blocks:
        rest = block[1:]
        if LABEL.match(block[0]) and rest and (LIST_ITEM.match(rest[0]) or rest[0].lstrip().startswith("|")):
            result.extend([block[:1], rest])
        else:
            result.append(block)
    return result


def kind(block: list[str]) -> str:
    first = block[0]
    if first.strip().startswith("```"):
        return "code"
    if first.lstrip().startswith("|"):
        return "table"
    if LIST_ITEM.match(first):
        return "list"
    return "text"


def check_body(body: str, max_words: int) -> list[str]:
    problems: list[str] = []
    content, _ = split_footers(body.splitlines())
    if not "".join(content).strip():
        return ["body: empty; start with one or two sentences stating what now happens"]

    text_outside_code: list[str] = []
    fence_count = 0
    table_count = 0
    seen: list[str] = []
    all_blocks = split_labels(blocks(content))

    for index, block in enumerate(all_blocks):
        block_kind = kind(block)
        joined = "\n".join(block)

        if block_kind == "code":
            fence_count += 1
            if len(block) - 2 > MAX_CODE_LINES:
                problems.append(f"code block: {len(block) - 2} lines; keep it under {MAX_CODE_LINES + 1}")
            continue
        text_outside_code.append(CODE_SPAN.sub("code", joined))

        for line in block:
            if HEADING.match(line):
                problems.append(f"heading {line.strip()!r}: use the fixed bold labels, not headings")
            if NESTED_ITEM.match(line):
                problems.append(f"nested list item {line.strip()!r}: keep lists flat")
            if re.search(r"<(details|summary|br|h\d)\b", line, re.I):
                problems.append("html: no collapsed sections or html markup")

        if HEADING.match(block[0]):
            continue

        if index == 0:
            if block_kind != "text" or LABEL.match(joined) or BOLD_LEAD.match(joined):
                problems.append("opening: start with a plain paragraph stating what now happens, before any label")
            elif words(joined) > MAX_OPENING_WORDS:
                problems.append(f"opening: {words(joined)} words; keep it to one or two sentences under {MAX_OPENING_WORDS + 1} words")
            continue

        follows_label = bool(LABEL.match("\n".join(all_blocks[index - 1])))
        next_kind = kind(all_blocks[index + 1]) if index + 1 < len(all_blocks) else ""

        if block_kind == "table":
            table_count += 1
            rows = [line for line in block if not re.match(r"^\s*\|[\s:|-]+\|\s*$", line)]
            if len(rows) - 1 > MAX_TABLE_ROWS:
                problems.append(f"table: {len(rows) - 1} rows; keep it to {MAX_TABLE_ROWS}")
            if not follows_label or seen[-1:] != ["Evidence"]:
                problems.append("table: allowed only directly under **Evidence:**")
            continue

        if block_kind == "list":
            items = [line for line in block if LIST_ITEM.match(line)]
            if len(items) > MAX_LIST_ITEMS:
                problems.append(f"list: {len(items)} items; keep it to {MAX_LIST_ITEMS} or split the PR")
            for item in items:
                if BOLD_LEAD.match(LIST_ITEM.sub("", item)):
                    problems.append(f"list item {item.strip()[:40]!r}: no bold lead-ins inside lists")
                if words(item) > MAX_ITEM_WORDS + 1:
                    problems.append(f"list item {item.strip()[:40]!r}: over {MAX_ITEM_WORDS} words")
            if not follows_label:
                problems.append("list: must sit directly under a label such as **Changes:**")
            continue

        match = LABEL.match(joined)
        if match:
            label = match.group(1)
            if label not in LABELS:
                problems.append(f"label {label!r}: use only {', '.join(LABELS)}")
                continue
            if label in seen:
                problems.append(f"label {label!r}: appears more than once")
            elif seen and LABELS.index(label) < LABELS.index(seen[-1]):
                problems.append(f"label {label!r}: out of order; use {' > '.join(LABELS)}")
            seen.append(label)
            rest = (match.group(2) or "").strip()
            if label == "Changes" and (rest or next_kind != "list"):
                problems.append("label 'Changes': put the changes in a list on the lines below it")
            elif not rest and next_kind not in ("list", "table"):
                problems.append(f"label {label!r}: write its text on the same line")
            continue

        if BOLD_LEAD.match(joined):
            problems.append(f"bold lead {joined.split(chr(10))[0][:40]!r}: use only {', '.join(LABELS)} with a colon")
            continue
        problems.append(
            f"paragraph {joined[:40]!r}: after the opening, every paragraph starts with a label; "
            "fold this text into the labeled paragraph it belongs to"
        )

    if fence_count > 1:
        problems.append(f"code blocks: {fence_count}; keep at most one")
    if table_count > 1:
        problems.append(f"tables: {table_count}; keep at most one")

    prose = "\n".join(text_outside_code)
    inline_bold = re.sub(r"(^|\n)\*\*[^*]+?:\*\*", "", prose)
    if "**" in inline_bold:
        problems.append("bold: use bold only for the section labels")
    for pattern, message in WORK_LOG:
        if pattern.search(prose):
            problems.append(message)

    count = words("\n".join(content))
    if count > max_words:
        problems.append(f"length: {count} words; keep the body under {max_words + 1} excluding footers, or split the PR")
    return problems


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="Check a PR title and description.")
    parser.add_argument("--title", required=True)
    parser.add_argument("--body-file", required=True, type=Path)
    parser.add_argument("--max-words", type=int, default=DEFAULT_MAX_WORDS)
    args = parser.parse_args(argv)
    try:
        body = args.body_file.read_text(encoding="utf-8")
    except OSError as err:
        print(f"cannot read body file: {err}", file=sys.stderr)
        return 2

    problems = check_title(args.title) + check_body(body, args.max_words)
    if re.match(r"^\w+(\([a-z0-9-]+\))?!:", subject_of(args.title)) and not re.search(r"^BREAKING[ -]CHANGE: ", body, re.M):
        problems.append("footer: a '!' subject needs a BREAKING CHANGE: footer")
    if problems:
        for problem in problems:
            print(problem)
        return 1
    content, _ = split_footers(body.splitlines())
    print(f"ok: {words(chr(10).join(content))} words")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
