#!/usr/bin/env python3
"""Regression tests for ci_green_gate.py.

These tests cover provider snapshots that have previously tempted agents to
claim progress or stop while CI was not green.
"""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any


HELPER = Path(__file__).with_name("ci_green_gate.py")
HEAD_SHA = "a3acf0c8de408fe954baf92b0be91f0eea3c9f33"
MERGE_QUEUE_SHA = "b4bd50d6b26838874f8ee8d6cdde81f15e4bda3a"


def gate(
    state: Any,
    *,
    name: str = "CI Build",
    sha: str = HEAD_SHA,
    **fields: Any,
) -> dict[str, Any]:
    return {"name": name, "state": state, "sha": sha, **fields}


def run_helper(snapshot: dict[str, Any]) -> tuple[int, dict[str, Any]]:
    with tempfile.NamedTemporaryFile("w", encoding="utf-8", suffix=".json") as handle:
        json.dump(snapshot, handle)
        handle.flush()
        result = subprocess.run(
            [sys.executable, str(HELPER), handle.name],
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )

    try:
        report = json.loads(result.stdout)
    except json.JSONDecodeError as exc:
        raise AssertionError(
            f"helper did not emit JSON\nstdout={result.stdout}\nstderr={result.stderr}"
        ) from exc

    return result.returncode, report


def assert_case(
    name: str,
    snapshot: dict[str, Any],
    expected_code: int,
    expected_state: str,
    expected_green: bool,
) -> None:
    code, report = run_helper(snapshot)
    assert code == expected_code, f"{name}: expected exit {expected_code}, got {code}: {report}"
    assert report.get("terminal_provider_state") == expected_state, (
        f"{name}: expected state {expected_state}, got {report}"
    )
    assert report.get("green") is expected_green, f"{name}: expected green={expected_green}: {report}"


def main() -> int:
    complete = {
        "head_sha": HEAD_SHA,
        "provider_snapshot_complete": True,
        "relevant_shas_complete": True,
        "scoped_to_head": False,
    }

    cases = [
        (
            "bitbucket_failed_pr_status_is_needs_local_fix",
            {
                **complete,
                "statuses": [
                    {
                        "name": "Pipeline - pullrequests: **",
                        "state": "FAILED",
                        "sha": HEAD_SHA,
                        "required": True,
                        "description": "1 / 22 tests failed",
                    }
                ],
            },
            1,
            "needs-local-fix",
            False,
        ),
        (
            "raw_bitbucket_pr_status_is_needs_local_fix",
            {
                "source": {
                    "commit": {
                        "hash": HEAD_SHA,
                    },
                },
                "_statuses": [
                    {
                        "name": "Pipeline - pullrequests: **",
                        "state": "FAILED",
                        "commit": {
                            "hash": HEAD_SHA,
                        },
                        "description": "1 / 22 tests failed",
                    }
                ],
            },
            1,
            "needs-local-fix",
            False,
        ),
        (
            "red_status_beats_running_pipeline",
            {
                **complete,
                "statuses": [
                    {
                        "name": "Pipeline - pullrequests: **",
                        "state": "FAILED",
                        "sha": HEAD_SHA,
                        "required": True,
                    }
                ],
                "pipelines": [
                    {
                        "name": "Pipeline - pullrequests: **",
                        "state": {"name": "IN_PROGRESS"},
                        "sha": HEAD_SHA,
                        "required": True,
                    }
                ],
            },
            1,
            "needs-local-fix",
            False,
        ),
        (
            "newer_green_status_supersedes_older_red_attempt",
            {
                **complete,
                "statuses": [
                    gate("FAILED", key="ci/build", updated_on="2026-07-15T00:00:00Z"),
                    gate("SUCCESSFUL", key="ci/build", updated_on="2026-07-15T00:05:00Z"),
                ],
            },
            0,
            "green",
            True,
        ),
        (
            "newer_red_status_supersedes_older_green_attempt",
            {
                **complete,
                "statuses": [
                    gate("SUCCESSFUL", key="ci/build", updated_on="2026-07-15T00:00:00Z"),
                    gate("FAILED", key="ci/build", updated_on="2026-07-15T00:05:00Z"),
                ],
            },
            1,
            "needs-local-fix",
            False,
        ),
        (
            "newer_build_number_supersedes_older_attempt",
            {
                **complete,
                "pipelines": [
                    gate("FAILED", name="Pull request pipeline", build_number=41),
                    gate("SUCCESSFUL", name="Pull request pipeline", build_number=42),
                ],
            },
            0,
            "green",
            True,
        ),
        (
            "build_number_beats_conflicting_timestamps",
            {
                **complete,
                "pipelines": [
                    gate(
                        "SUCCESSFUL",
                        name="Pull request pipeline",
                        build_number=41,
                        updated_on="2026-07-15T00:10:00Z",
                    ),
                    gate(
                        "FAILED",
                        name="Pull request pipeline",
                        build_number=42,
                        updated_on="2026-07-15T00:05:00Z",
                    ),
                ],
            },
            1,
            "needs-local-fix",
            False,
        ),
        (
            "ambiguous_attempt_order_keeps_red_evidence",
            {
                **complete,
                "statuses": [
                    gate("FAILED", key="ci/build"),
                    gate("SUCCESSFUL", key="ci/build"),
                ],
            },
            1,
            "needs-local-fix",
            False,
        ),
        (
            "different_timestamp_fields_keep_red_evidence",
            {
                **complete,
                "statuses": [
                    gate("FAILED", key="ci/build", updated_on="2026-07-15T00:05:00Z"),
                    gate("SUCCESSFUL", key="ci/build", created_on="2026-07-15T00:10:00Z"),
                ],
            },
            1,
            "needs-local-fix",
            False,
        ),
        (
            "newer_running_attempt_does_not_hide_older_red",
            {
                **complete,
                "statuses": [
                    gate("FAILED", key="ci/build", updated_on="2026-07-15T00:00:00Z"),
                    gate("IN_PROGRESS", key="ci/build", updated_on="2026-07-15T00:05:00Z"),
                ],
            },
            1,
            "needs-local-fix",
            False,
        ),
        (
            "green_then_running_does_not_resurrect_older_red",
            {
                **complete,
                "statuses": [
                    gate("FAILED", key="ci/build", updated_on="2026-07-15T00:00:00Z"),
                    gate("SUCCESSFUL", key="ci/build", updated_on="2026-07-15T00:05:00Z"),
                    gate("IN_PROGRESS", key="ci/build", updated_on="2026-07-15T00:10:00Z"),
                ],
            },
            1,
            "waiting",
            False,
        ),
        (
            "running_pipeline_is_waiting_not_green",
            {
                **complete,
                "pipelines": [
                    {
                        "name": "Pipeline - pullrequests: **",
                        "state": {"name": "IN_PROGRESS"},
                        "sha": HEAD_SHA,
                        "required": True,
                    }
                ],
            },
            1,
            "waiting",
            False,
        ),
        (
            "stopped_pipeline_is_waiting",
            {
                **complete,
                "pipelines": [
                    {
                        "name": "Pipeline - pullrequests: **",
                        "state": {"name": "STOPPED"},
                        "sha": HEAD_SHA,
                        "required": True,
                    }
                ],
            },
            1,
            "waiting",
            False,
        ),
        (
            "nested_bitbucket_failed_pipeline_is_needs_local_fix",
            {
                **complete,
                "pipelines": [
                    {
                        "name": "Pipeline - pullrequests: **",
                        "state": {
                            "name": "COMPLETED",
                            "result": {"name": "FAILED"},
                        },
                        "sha": HEAD_SHA,
                        "required": True,
                    }
                ],
            },
            1,
            "needs-local-fix",
            False,
        ),
        (
            "raw_bitbucket_pipeline_detail_is_needs_local_fix",
            {
                "uuid": "{e599c4e0-b4e6-4051-ab27-a1a5216cbcc6}",
                "build_number": 11980,
                "state": {
                    "name": "COMPLETED",
                    "result": {"name": "FAILED"},
                },
                "target": {
                    "commit": {
                        "hash": HEAD_SHA,
                    },
                    "source": "fix/stale-cache-state",
                    "destination": "main",
                    "pullrequest": {
                        "id": 2082,
                    },
                },
            },
            1,
            "needs-local-fix",
            False,
        ),
        (
            "successful_status_is_green",
            {
                **complete,
                "statuses": [
                    {
                        "name": "Pipeline - pullrequests: **",
                        "state": "SUCCESSFUL",
                        "sha": HEAD_SHA,
                        "required": True,
                    }
                ],
            },
            0,
            "green",
            True,
        ),
        (
            "declared_merge_queue_sha_requires_gate_evidence",
            {
                **complete,
                "merge_queue_sha": MERGE_QUEUE_SHA,
                "statuses": [gate("SUCCESSFUL")],
            },
            1,
            "waiting",
            False,
        ),
        (
            "undeclared_merge_queue_red_cannot_prove_green",
            {
                "head_sha": HEAD_SHA,
                "provider_snapshot_complete": True,
                "scoped_to_head": False,
                "statuses": [
                    gate("SUCCESSFUL", name="Source CI Build"),
                    gate("FAILED", name="Merge Queue CI Build", sha=MERGE_QUEUE_SHA),
                ],
            },
            2,
            "tooling-blocked",
            False,
        ),
        (
            "declared_merge_queue_red_is_needs_local_fix",
            {
                **complete,
                "merge_queue_sha": MERGE_QUEUE_SHA,
                "statuses": [
                    gate("SUCCESSFUL", name="Source CI Build"),
                    gate("FAILED", name="Merge Queue CI Build", sha=MERGE_QUEUE_SHA),
                ],
            },
            1,
            "needs-local-fix",
            False,
        ),
        (
            "prefix_equivalent_declared_sha_is_not_missing",
            {
                **complete,
                "merge_sha": HEAD_SHA[:12],
                "statuses": [gate("SUCCESSFUL")],
            },
            0,
            "green",
            True,
        ),
        (
            "head_and_merge_queue_gates_can_prove_green",
            {
                **complete,
                "merge_queue_sha": MERGE_QUEUE_SHA,
                "statuses": [
                    gate("SUCCESSFUL", name="Source CI Build"),
                    gate("SUCCESSFUL", name="Merge Queue CI Build", sha=MERGE_QUEUE_SHA),
                ],
            },
            0,
            "green",
            True,
        ),
        (
            "incomplete_snapshot_is_tooling_blocked",
            {
                "head_sha": HEAD_SHA,
                "statuses": [
                    {
                        "name": "Pipeline - pullrequests: **",
                        "state": "SUCCESSFUL",
                        "sha": HEAD_SHA,
                        "required": True,
                    }
                ],
            },
            2,
            "tooling-blocked",
            False,
        ),
    ]

    for name, snapshot, expected_code, expected_state, expected_green in cases:
        assert_case(name, snapshot, expected_code, expected_state, expected_green)
        print(f"ok {name}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
