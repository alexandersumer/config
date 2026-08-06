#!/usr/bin/env python3
"""Validate that a normalized CI/provider snapshot is fully green.

Input JSON shape:
{
  "head_sha": "abc123",
  "provider_snapshot_complete": true,
  "relevant_shas_complete": true,
  "merge_sha": "def456",
  "scoped_to_head": true,
  "checks": [
    {"name": "Type Check", "state": "SUCCESSFUL", "sha": "abc123", "required": true}
  ],
  "pipelines": [...],
  "statuses": [...],
  "gates": [...]
}

Required for green: head_sha, provider_snapshot_complete,
relevant_shas_complete, and at least one gate list. Set
provider_snapshot_complete only after enumerating the provider gate set. Set
relevant_shas_complete only after declaring every applicable merge, synthetic,
or merge-queue SHA. Set scoped_to_head only when the provider query was scoped
to head_sha. Each declared alternate SHA also requires gate evidence.
An incomplete snapshot can still prove "not green" when it contains a
current-sha red gate.

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
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


GATE_LIST_KEYS = (
    "checks",
    "pipelines",
    "statuses",
    "gates",
    "_checks",
    "_pipelines",
    "_statuses",
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

ORDER_TIMESTAMP_KEYS = (
    "updated_on",
    "completed_on",
    "created_on",
    "updated_at",
    "completed_at",
    "created_at",
)

ORDER_NUMBER_KEYS = (
    "build_number",
    "run_number",
    "attempt_number",
    "attempt",
)


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
    direct = short_sha(value)
    if direct:
        return direct

    target = gate.get("target")
    if isinstance(target, dict):
        commit = target.get("commit")
        if isinstance(commit, dict):
            value = first_present(commit, SHA_KEYS + ("hash",))
            direct = short_sha(value)
            if direct:
                return direct

    return None


def extract_gates(snapshot: Any) -> list[tuple[str, dict[str, Any]]]:
    if isinstance(snapshot, list):
        return [("root", item) for item in snapshot if isinstance(item, dict)]
    if not isinstance(snapshot, dict):
        return []

    gates: list[tuple[str, dict[str, Any]]] = []
    if (
        isinstance(snapshot.get("state"), (str, dict))
        and ("build_number" in snapshot or "uuid" in snapshot)
        and isinstance(snapshot.get("target"), dict)
    ):
        gates.append(("root_pipeline", snapshot))

    for key in GATE_LIST_KEYS:
        value = snapshot.get(key)
        if isinstance(value, list):
            gates.extend((key, item) for item in value if isinstance(item, dict))
        elif isinstance(value, dict):
            nested = value.get("values")
            if isinstance(nested, list):
                gates.extend((key, item) for item in nested if isinstance(item, dict))

    values = snapshot.get("values")
    if not gates and isinstance(values, list):
        gates.extend(("values", item) for item in values if isinstance(item, dict))

    return gates


def gate_identity(gate: dict[str, Any]) -> str | None:
    key = short_sha(gate.get("key"))
    if key:
        return f"key:{key}"

    name = first_present(gate, ("name", "display_name", "description", "title"))
    text = short_sha(name)
    return f"name:{text}" if text else None


def parse_timestamp(value: Any) -> float | None:
    if not isinstance(value, str) or not value.strip():
        return None
    text = value.strip()
    if text.endswith("Z"):
        text = f"{text[:-1]}+00:00"
    try:
        parsed = datetime.fromisoformat(text)
    except ValueError:
        return None
    if parsed.tzinfo is None:
        parsed = parsed.replace(tzinfo=timezone.utc)
    return parsed.timestamp()


def parse_attempt_number(value: Any) -> int | None:
    if isinstance(value, bool):
        return None
    if isinstance(value, int):
        return value
    if isinstance(value, str):
        try:
            return int(value.strip())
        except ValueError:
            return None
    return None


def gate_order(gate: dict[str, Any]) -> tuple[str, float] | None:
    for key in ORDER_NUMBER_KEYS:
        parsed = parse_attempt_number(gate.get(key))
        if parsed is not None:
            return key, float(parsed)
    for key in ORDER_TIMESTAMP_KEYS:
        parsed = parse_timestamp(gate.get(key))
        if parsed is not None:
            return key, parsed
    return None


def canonical_sha(gate_sha_value: str, accepted_shas: set[str]) -> str:
    for accepted in accepted_shas:
        if sha_matches(gate_sha_value, {accepted}):
            return accepted
    return gate_sha_value


def collapse_superseded_gates(
    records: list[tuple[str, dict[str, Any]]],
    head_sha: str,
    accepted_shas: set[str],
    scoped_to_head: bool,
) -> tuple[list[tuple[str, dict[str, Any]]], list[dict[str, Any]]]:
    groups: dict[tuple[str, str, str], list[int]] = {}
    for index, (surface, gate) in enumerate(records):
        identity = gate_identity(gate)
        sha = gate_sha(gate)
        if sha is None and scoped_to_head:
            sha = head_sha
        if identity is None or sha is None:
            continue
        groups.setdefault((surface, identity, canonical_sha(sha, accepted_shas)), []).append(index)

    superseded_indexes: set[int] = set()
    superseded: list[dict[str, Any]] = []
    for (surface, identity, sha), indexes in groups.items():
        if len(indexes) < 2:
            continue
        ordered = [(index, gate_order(records[index][1])) for index in indexes]
        if any(order is None for _, order in ordered):
            continue
        order_fields = {order[0] for _, order in ordered if order is not None}
        if len(order_fields) != 1:
            continue
        terminal_values = [
            order[1]
            for index, order in ordered
            if order is not None and classify_state(records[index][1])[0] in {"green", "red"}
        ]
        if not terminal_values:
            continue
        latest_terminal_value = max(terminal_values)
        for index, order in ordered:
            if order is None or order[1] >= latest_terminal_value:
                continue
            gate = records[index][1]
            superseded_indexes.add(index)
            superseded.append({
                "name": gate_name(gate, index),
                "surface": surface,
                "identity": identity,
                "sha": sha,
                "state": gate_state_text(gate) or "missing state",
                "order_field": order[0],
                "order_value": order[1],
                "reason": "superseded by a newer attempt on the same provider surface",
            })

    active = [record for index, record in enumerate(records) if index not in superseded_indexes]
    return active, superseded


def latest_sha(snapshot: dict[str, Any]) -> str | None:
    for key in ("head_sha", "source_sha", "latest_sha", "sha", "commit_sha"):
        value = short_sha(snapshot.get(key))
        if value:
            return value

    source = snapshot.get("source")
    if isinstance(source, dict):
        commit = source.get("commit")
        if isinstance(commit, dict):
            value = short_sha(first_present(commit, ("hash",) + SHA_KEYS))
            if value:
                return value

    target = snapshot.get("target")
    if isinstance(target, dict):
        commit = target.get("commit")
        if isinstance(commit, dict):
            value = short_sha(first_present(commit, ("hash",) + SHA_KEYS))
            if value:
                return value

    return None


def accepted_shas(snapshot: dict[str, Any], head_sha: str) -> set[str]:
    candidates = [head_sha]
    for key in ("merge_sha", "synthetic_sha", "merge_queue_sha"):
        value = short_sha(snapshot.get(key))
        if value:
            candidates.append(value)

    shas: set[str] = set()
    for value in sorted(candidates, key=lambda candidate: (-len(candidate), candidate)):
        if not any(sha_matches(value, {accepted}) for accepted in shas):
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

    snapshot_complete = snapshot_is_complete(snapshot)
    relevant_shas_complete = as_strict_bool(snapshot.get("relevant_shas_complete")) is True
    scoped_to_head = as_strict_bool(snapshot.get("scoped_to_head")) is True
    gate_records = extract_gates(snapshot)
    if not gate_records:
        return 2, {
            "green": False,
            "terminal_provider_state": "tooling-blocked",
            "head_sha": head_sha,
            "reason": "snapshot contains no checks, pipelines, statuses, or gates",
        }

    accepted = accepted_shas(snapshot, head_sha)
    gate_records, superseded_gates = collapse_superseded_gates(
        gate_records,
        head_sha,
        accepted,
        scoped_to_head,
    )
    blockers: list[dict[str, Any]] = []
    green_gates: list[dict[str, Any]] = []
    stale_gates: list[dict[str, Any]] = []
    covered_shas: set[str] = set()

    for index, (surface, gate) in enumerate(gate_records):
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

        covered_shas.add(canonical_sha(sha, accepted) if sha is not None else head_sha)

        classification, raw_state = classify_state(gate)
        entry = {
            "name": name,
            "surface": surface,
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

    for missing_sha in sorted(accepted - covered_shas):
        blockers.append({
            "name": "required SHA gate set",
            "sha": missing_sha,
            "classification": "waiting",
            "state": "no gates",
            "reason": "snapshot contains no gate for this declared SHA",
        })

    if not green_gates and not blockers:
        blockers.append({
            "name": "current-sha gate set",
            "sha": head_sha,
            "classification": "unknown",
            "state": "no current gates",
            "reason": "only stale gates were present",
        })

    if not snapshot_complete or not relevant_shas_complete:
        missing_completeness = []
        if not snapshot_complete:
            missing_completeness.append("provider_snapshot_complete=true")
        if not relevant_shas_complete:
            missing_completeness.append("relevant_shas_complete=true")
        missing_text = " and ".join(missing_completeness)
        red_blockers = [blocker for blocker in blockers if blocker["classification"] == "red"]
        if red_blockers:
            return 1, {
                "green": False,
                "terminal_provider_state": "needs-local-fix",
                "head_sha": head_sha,
                "accepted_shas": sorted(accepted),
                "reason": (
                    f"snapshot missing {missing_text}, "
                    "but current-sha red gates prove CI is not green"
                ),
                "counts": {
                    "green": len(green_gates),
                    "blockers": len(blockers),
                    "stale": len(stale_gates),
                    "superseded": len(superseded_gates),
                },
                "blockers": blockers,
                "green_gates": green_gates,
                "stale_gates": stale_gates,
                "superseded_gates": superseded_gates,
            }

        return 2, {
            "green": False,
            "terminal_provider_state": "tooling-blocked",
            "head_sha": head_sha,
            "accepted_shas": sorted(accepted),
            "reason": f"snapshot must set {missing_text} before proving green",
            "counts": {
                "green": len(green_gates),
                "blockers": len(blockers),
                "stale": len(stale_gates),
                "superseded": len(superseded_gates),
            },
            "blockers": blockers,
            "green_gates": green_gates,
            "stale_gates": stale_gates,
            "superseded_gates": superseded_gates,
        }

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
                "superseded": len(superseded_gates),
            },
            "blockers": blockers,
            "green_gates": green_gates,
            "stale_gates": stale_gates,
            "superseded_gates": superseded_gates,
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
            "superseded": len(superseded_gates),
        },
        "green_gates": green_gates,
        "stale_gates": stale_gates,
        "superseded_gates": superseded_gates,
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
