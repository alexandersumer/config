#!/usr/bin/env python3
"""Check a PR title and description against the branch-description format.

Usage:
  check_pr_description.py --title "<title>" --body-file <path> [--max-words N]

Exit 0 prints "ok". Exit 1 prints one problem per line. Exit 2 means the body
file could not be read. Only structure and length are checked; accuracy and
clarity stay with the writer.
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
LABELS = ("Why", "Changes", "Evidence", "Rollout", "Review", "Follow-up")

SUBJECT = re.compile(
    r"^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)"
    r"(\([a-z0-9-]+\))?(!?): [a-z].{0,70}[^.]$"
)
TITLE_TAGS = re.compile(r"^(\[[a-z0-9-]+\] )*")
LABEL = re.compile(r"^\*\*([^*]+?):\*\*(.*)$", re.S)
BOLD = re.compile(r"\*\*|__")
LIST_ITEM = re.compile(r"^([-*+]|\d+[.)]) ")
NESTED_ITEM = re.compile(r"^\s+([-*+]|\d+[.)]) ")
HEADING = re.compile(r"^(#{1,6} |=+\s*$|-{3,}\s*$|\*{3,}\s*$)")
HTML = re.compile(r"<(details|summary|br|h\d|div|table)\b", re.I)
FOOTER = re.compile(
    r"^(BREAKING[ -]CHANGE|Refs|Fixes|Closes|Resolves|[A-Z][a-z]*(-[A-Za-z]+)+)(: | #)\S"
)
ATTRIBUTION = re.compile(r"Generated with \[|^Co-Authored-By: ", re.I)
CODE_SPAN = re.compile(r"`[^`\n]*`")


def word_count(text: str) -> int:
    return sum(1 for token in text.split() if re.search(r"\w", token))


def without_footers(body: str) -> tuple[list[str], list[str]]:
    """Return the body lines and the trailing footer lines, minus attribution."""
    lines = [line for line in body.splitlines() if not ATTRIBUTION.search(line)]
    while lines and not lines[-1].strip():
        lines.pop()
    start = len(lines)
    while start and FOOTER.match(lines[start - 1]):
        start -= 1
    if start == len(lines) or (start and lines[start - 1].strip()):
        return lines, []
    return lines[:start], lines[start:]


def blocks(lines: list[str]) -> list[list[str]]:
    """Split on blank lines, keep fenced code whole, and separate a label line
    from the list or table written directly under it."""
    result: list[list[str]] = [[]]
    in_fence = False
    for line in lines:
        if line.lstrip().startswith("```"):
            in_fence = not in_fence
        if not line.strip() and not in_fence:
            result.append([])
        elif (
            not in_fence
            and len(result[-1]) == 1
            and LABEL.match(result[-1][0])
            and (LIST_ITEM.match(line) or line.lstrip().startswith("|"))
        ):
            result.append([line])
        else:
            result[-1].append(line)
    return [block for block in result if block]


def kind(block: list[str]) -> str:
    first = block[0].lstrip()
    if first.startswith("```"):
        return "code"
    if first.startswith("|"):
        return "table"
    if LIST_ITEM.match(block[0]):
        return "list"
    if LABEL.match(block[0]):
        return "label"
    return "text"


def check_title(title: str) -> list[str]:
    if not SUBJECT.match(TITLE_TAGS.sub("", title.strip(), count=1)):
        return [
            "title: after optional [tag] prefixes, use a Conventional Commit subject: "
            "type(scope)!: lowercase description, at most 72 characters, no trailing period"
        ]
    return []


def check_block(block: list[str], previous: str, following: str, seen: list[str]) -> list[str]:
    block_kind = kind(block)
    text = "\n".join(block)
    if block_kind == "code":
        return []

    problems = []
    prose = CODE_SPAN.sub("", text)
    for line in block:
        if HEADING.match(line):
            problems.append(f"heading {line.strip()[:40]!r}: use the section labels instead")
        if NESTED_ITEM.match(line):
            problems.append(f"nested item {line.strip()[:40]!r}: keep lists flat")
        if HTML.search(line):
            problems.append(f"html {line.strip()[:40]!r}: use plain Markdown")
    if problems:
        return problems

    match = LABEL.match(text)
    if match:
        label, rest = match.groups()
        if BOLD.search(rest):
            problems.append(f"label {label!r}: no bold inside the section text")
        if label not in LABELS:
            return problems + [f"label {label!r}: use only {', '.join(LABELS)}"]
        if label in seen:
            problems.append(f"label {label!r}: use each label once")
        elif seen and LABELS.index(label) < LABELS.index(seen[-1]):
            problems.append(f"label {label!r}: keep the order {', '.join(LABELS)}")
        seen.append(label)
        if label == "Changes" and (rest.strip() or following != "list"):
            problems.append("label 'Changes': put the changes in a list directly below it")
        elif label != "Changes" and not rest.strip() and following not in ("list", "table"):
            problems.append(f"label {label!r}: write its text on the same line")
        return problems

    if BOLD.search(prose):
        problems.append(f"bold in {block[0].strip()[:40]!r}: use bold only for section labels")
    if block_kind == "text":
        problems.append(f"paragraph {block[0].strip()[:40]!r}: start it with a section label")
    elif previous != "label":
        problems.append(f"{block_kind} {block[0].strip()[:40]!r}: put it directly under a section label")
    elif block_kind == "table" and seen[-1] != "Evidence":
        problems.append("table: allowed only under **Evidence:**")
    elif block_kind == "list":
        items = [line for line in block if LIST_ITEM.match(line)]
        if len(items) > MAX_LIST_ITEMS:
            problems.append(f"list: {len(items)} items; use at most {MAX_LIST_ITEMS} or split the PR")
        for item in items:
            if word_count(LIST_ITEM.sub("", item)) > MAX_ITEM_WORDS:
                problems.append(f"item {item.strip()[:40]!r}: over {MAX_ITEM_WORDS} words")
    return problems


def check(title: str, body: str, max_words: int = DEFAULT_MAX_WORDS) -> list[str]:
    problems = check_title(title)
    content, footers = without_footers(body)
    parts = blocks(content)
    if not parts:
        return problems + ["body: empty; start with one or two sentences saying what now happens"]

    opening = parts[0]
    if kind(opening) != "text" or BOLD.search(opening[0]) or HEADING.match(opening[0]):
        problems.append("opening: start with a plain paragraph saying what now happens")
    elif word_count("\n".join(opening)) > MAX_OPENING_WORDS:
        problems.append(f"opening: {word_count(' '.join(opening))} words; use at most {MAX_OPENING_WORDS}")

    seen: list[str] = []
    kinds = [kind(part) for part in parts] + [""]
    for index in range(1, len(parts)):
        problems += check_block(parts[index], kinds[index - 1], kinds[index + 1], seen)

    total = word_count("\n".join(content))
    if total > max_words:
        problems.append(f"length: {total} words; use at most {max_words}, excluding footers, or split the PR")

    breaking = SUBJECT.match(TITLE_TAGS.sub("", title.strip(), count=1))
    has_footer = any(re.match(r"BREAKING[ -]CHANGE: ", line) for line in footers)
    if breaking and breaking.group(3) and not has_footer:
        problems.append("footer: a '!' title needs a BREAKING CHANGE: footer")
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

    problems = check(args.title, body, args.max_words)
    for problem in problems:
        print(problem)
    if problems:
        return 1
    print(f"ok: {word_count(chr(10).join(without_footers(body)[0]))} words")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
