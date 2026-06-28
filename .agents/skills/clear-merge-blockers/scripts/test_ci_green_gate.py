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
