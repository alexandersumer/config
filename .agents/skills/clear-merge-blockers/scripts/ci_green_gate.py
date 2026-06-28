#!/usr/bin/env python3
"""Validate that a normalized CI/provider snapshot is fully green.

Input JSON shape:
{
  "head_sha": "abc123",
  "provider_snapshot_complete": true,
  "merge_sha": "def456",
  "scoped_to_head": true,
  "checks": [
    {"name": "Type Check", "state": "SUCCESSFUL", "sha": "abc123", "required": true}
  ],
  "pipelines": [...],
  "statuses": [...],
  "gates": [...]
}

Required fields: head_sha, provider_snapshot_complete, and at least one gate
list. Set provider_snapshot_complete only after enumerating the provider gate
set. Set scoped_to_head only when the provider query was scoped to head_sha.

The script exits 0 only when the snapshot explicitly says its provider gate list
is complete and every included current-sha gate is terminal green. All other
outcomes are intentionally nonzero so an agent cannot use an incomplete or stale
snapshot to claim hosted CI is green.
"""

from __future__ import annotations

import argparse
import json
import sys
from collections.abc import Iterable
from pathlib import Path
from typing import Any


GATE_LIST_KEYS = (
    "checks",
    "pipelines",
    "statuses",
    "gates",
    "required_checks",
    "required_gates",
    "builds",
)

NAME_KEYS = (
    "name",
    "key",
    "display_name",
    "description",
    "title",
    "uuid",
    "id",
    "build_number",
)

SHA_KEYS = (
    "sha",
    "commit",
    "commit_sha",
    "head_sha",
    "target_commit",
    "revision",
)

STATE_KEYS = (
    "state",
    "status",
    "conclusion",
    "result",
    "outcome",
    "lifecycle_state",
)

GREEN_TOKENS = {
    "success",
    "successful",
    "succeeded",
    "passed",
    "pass",
    "green",
    "ok",
}

RED_TOKENS = {
    "failed",
    "failure",
    "error",
    "errored",
    "timed_out",
    "timeout",
    "expired",
    "rejected",
    "unsuccessful",
}

WAITING_TOKENS = {
    "pending",
    "queued",
    "running",
    "in_progress",
    "inprogress",
    "progress",
    "waiting",
    "started",
    "building",
    "created",
    "new",
    "notrun",
    "not_run",
    "missing",
    "skipped",
    "skip",
    "canceled",
    "cancelled",
    "stopped",
}


def load_json(path: str) -> Any:
    if path == "-":
        return json.load(sys.stdin)
    with Path(path).open(encoding="utf-8") as handle:
        return json.load(handle)


def as_bool(value: Any) -> bool | None:
    if isinstance(value, bool):
        return value
    if isinstance(value, str):
        lowered = value.strip().lower()
        if lowered in {"true", "yes", "1", "required"}:
            return True
        if lowered in {"false", "no", "0", "optional"}:
            return False
    return None


def as_strict_bool(value: Any) -> bool | None:
    if isinstance(value, bool):
        return value
    if isinstance(value, str):
        lowered = value.strip().lower()
        if lowered in {"true", "yes", "1"}:
            return True
        if lowered in {"false", "no", "0"}:
            return False
    return None


def first_present(data: dict[str, Any], keys: Iterable[str]) -> Any:
    for key in keys:
        if key in data and data[key] not in (None, ""):
            return data[key]
    return None


def short_sha(value: Any) -> str | None:
    if value is None:
        return None
    text = str(value).strip()
    if not text:
        return None
    return text


def sha_matches(gate_sha: str | None, accepted_shas: set[str]) -> bool:
    if gate_sha is None:
        return False
    for accepted in accepted_shas:
        if gate_sha == accepted:
            return True
        shortest = min(len(gate_sha), len(accepted))
        if shortest >= 7 and (gate_sha.startswith(accepted) or accepted.startswith(gate_sha)):
            return True
    return False


def text_tokens(value: Any) -> list[str]:
    if value is None:
        return []
    if isinstance(value, str):
        return [value]
    if isinstance(value, (int, float, bool)):
        return [str(value)]
    if isinstance(value, list):
        tokens: list[str] = []
        for item in value:
            tokens.extend(text_tokens(item))
        return tokens
    if isinstance(value, dict):
        tokens = []
        for key in STATE_KEYS + ("name",):
            if key in value:
                tokens.extend(text_tokens(value[key]))
        return tokens
    return [str(value)]


def normalize_token(value: str) -> str:
    return value.strip().lower().replace("-", "_").replace(" ", "_")


def gate_state_text(gate: dict[str, Any]) -> str:
    tokens: list[str] = []
    for key in STATE_KEYS:
        if key in gate:
            tokens.extend(text_tokens(gate[key]))
    return " ".join(str(token) for token in tokens if str(token).strip())


def classify_state(gate: dict[str, Any]) -> tuple[str, str]:
    raw = gate_state_text(gate)
    tokens = {normalize_token(token) for token in raw.replace("/", " ").split() if token.strip()}

    if tokens & RED_TOKENS:
        return "red", raw or "missing state"
    if tokens & WAITING_TOKENS:
        return "waiting", raw or "missing state"
    if tokens & GREEN_TOKENS:
        return "green", raw or "missing state"
    return "unknown", raw or "missing state"


def gate_name(gate: dict[str, Any], index: int) -> str:
    value = first_present(gate, NAME_KEYS)
    return str(value) if value is not None else f"gate[{index}]"


def gate_sha(gate: dict[str, Any]) -> str | None:
    value = first_present(gate, SHA_KEYS)
    if isinstance(value, dict):
        value = first_present(value, SHA_KEYS + ("hash",))
    return short_sha(value)


def extract_gates(snapshot: Any) -> list[dict[str, Any]]:
    if isinstance(snapshot, list):
        return [item for item in snapshot if isinstance(item, dict)]
    if not isinstance(snapshot, dict):
        return []

    gates: list[dict[str, Any]] = []
    for key in GATE_LIST_KEYS:
        value = snapshot.get(key)
        if isinstance(value, list):
            gates.extend(item for item in value if isinstance(item, dict))
        elif isinstance(value, dict):
            nested = value.get("values")
            if isinstance(nested, list):
                gates.extend(item for item in nested if isinstance(item, dict))

    values = snapshot.get("values")
    if not gates and isinstance(values, list):
        gates.extend(item for item in values if isinstance(item, dict))

    return gates


def latest_sha(snapshot: dict[str, Any]) -> str | None:
    for key in ("head_sha", "source_sha", "latest_sha", "sha", "commit_sha"):
        value = short_sha(snapshot.get(key))
        if value:
            return value
    return None


def accepted_shas(snapshot: dict[str, Any], head_sha: str) -> set[str]:
    shas = {head_sha}
    for key in ("merge_sha", "synthetic_sha", "merge_queue_sha"):
        value = short_sha(snapshot.get(key))
        if value:
            shas.add(value)
    return shas


def snapshot_is_complete(snapshot: dict[str, Any]) -> bool:
    return as_strict_bool(snapshot.get("provider_snapshot_complete")) is True


def validate(snapshot: Any) -> tuple[int, dict[str, Any]]:
    if not isinstance(snapshot, dict):
        return 2, {
            "green": False,
            "terminal_provider_state": "tooling-blocked",
            "reason": "snapshot must be a JSON object",
        }

    head_sha = latest_sha(snapshot)
    if not head_sha:
        return 2, {
            "green": False,
            "terminal_provider_state": "tooling-blocked",
            "reason": "snapshot missing head_sha/source_sha/latest_sha",
        }

    if not snapshot_is_complete(snapshot):
        return 2, {
            "green": False,
            "terminal_provider_state": "tooling-blocked",
            "head_sha": head_sha,
            "reason": "snapshot must set provider_snapshot_complete=true after enumerating provider gates",
        }

    scoped_to_head = as_strict_bool(snapshot.get("scoped_to_head")) is True
    gates = extract_gates(snapshot)
    if not gates:
        return 2, {
            "green": False,
            "terminal_provider_state": "tooling-blocked",
            "head_sha": head_sha,
            "reason": "snapshot contains no checks, pipelines, statuses, or gates",
        }

    accepted = accepted_shas(snapshot, head_sha)
    blockers: list[dict[str, Any]] = []
    green_gates: list[dict[str, Any]] = []
    stale_gates: list[dict[str, Any]] = []

    for index, gate in enumerate(gates):
        required = as_bool(gate.get("required"))
        name = gate_name(gate, index)
        sha = gate_sha(gate)

        if sha is not None and not sha_matches(sha, accepted):
            stale_gates.append({"name": name, "sha": sha, "required": required, "reason": "not latest sha"})
            continue

        if sha is None and not scoped_to_head:
            blockers.append({
                "name": name,
                "sha": None,
                "required": required,
                "classification": "unknown",
                "state": gate_state_text(gate) or "missing state",
                "reason": "gate has no sha and snapshot is not scoped_to_head",
            })
            continue

        classification, raw_state = classify_state(gate)
        entry = {
            "name": name,
            "sha": sha or head_sha,
            "required": required,
            "classification": classification,
            "state": raw_state,
        }

        if classification == "green":
            green_gates.append(entry)
        else:
            if classification == "red":
                entry["reason"] = "gate is red"
            elif classification == "waiting":
                entry["reason"] = "gate is not terminal success"
            else:
                entry["reason"] = "gate state is unknown"
            blockers.append(entry)

    if not green_gates and not blockers:
        blockers.append({
            "name": "current-sha gate set",
            "sha": head_sha,
            "classification": "unknown",
            "state": "no current gates",
            "reason": "only stale gates were present",
        })

    if blockers:
        classifications = {blocker["classification"] for blocker in blockers}
        if "red" in classifications:
            terminal = "needs-local-fix"
        elif classifications == {"waiting"}:
            terminal = "waiting"
        else:
            terminal = "tooling-blocked"
        return 1, {
            "green": False,
            "terminal_provider_state": terminal,
            "head_sha": head_sha,
            "accepted_shas": sorted(accepted),
            "counts": {
                "green": len(green_gates),
                "blockers": len(blockers),
                "stale": len(stale_gates),
            },
            "blockers": blockers,
            "green_gates": green_gates,
            "stale_gates": stale_gates,
        }

    return 0, {
        "green": True,
        "terminal_provider_state": "green",
        "head_sha": head_sha,
        "accepted_shas": sorted(accepted),
        "counts": {
            "green": len(green_gates),
            "blockers": 0,
            "stale": len(stale_gates),
        },
        "green_gates": green_gates,
        "stale_gates": stale_gates,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("snapshot", help="Path to normalized JSON snapshot, or '-' for stdin")
    args = parser.parse_args()

    try:
        snapshot = load_json(args.snapshot)
        code, report = validate(snapshot)
    except json.JSONDecodeError as exc:
        code, report = 2, {
            "green": False,
            "terminal_provider_state": "tooling-blocked",
            "reason": f"invalid JSON: {exc}",
        }
    except OSError as exc:
        code, report = 2, {
            "green": False,
            "terminal_provider_state": "tooling-blocked",
            "reason": str(exc),
        }

    print(json.dumps(report, indent=2, sort_keys=True))
    return code


if __name__ == "__main__":
    raise SystemExit(main())
