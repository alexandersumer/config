"""Reproducible local Git benchmark with controlled fetch latency (no network)."""

import json
from pathlib import Path
import statistics
import subprocess
import time

from test_home_reset import GitTests, ROOT

BASELINE_REVISION = "09d2df9fdfb8dbad6736fcd2b1b0c8152072ba17"


def main():
    fixture = GitTests()
    fixture.setUp()
    try:
        repos = [fixture.repo] + [fixture.clone(f"repo-{i:02d}") for i in range(11)]
        env = fixture.shim("delay", delay=.25)
        original = fixture.root / "original-git-functions.zsh"
        original.write_bytes(subprocess.check_output(["git", "show", f"{BASELINE_REVISION}:zsh/git-functions.zsh"], cwd=ROOT))
        samples = {"original": [], 1: [], 2: [], 4: []}
        for order in (("original", 1, 2, 4), (4, "original", 1, 2), (2, 4, "original", 1)):
            for workers in order:
                for repo in repos:
                    fixture.git(repo, "reset", "--hard", fixture.old)
                    for name in ("fetch-count", "fetch-done-count"):
                        (repo / ".git" / name).unlink(missing_ok=True)
                start = time.monotonic()
                if workers == "original":
                    result = subprocess.run(["zsh", "-f", "-c", 'source "$1"; home_reset_to_origin --root "$2" --retry-delay 0', "benchmark", str(original), str(fixture.workspace)], env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=60)
                    # Original success returns before its detached final fetch.
                    deadline = time.monotonic() + 20
                    while any(not (repo / ".git/fetch-done-count").exists() or (repo / ".git/fetch-done-count").read_text() != "3" for repo in repos):
                        if time.monotonic() > deadline:
                            raise RuntimeError("Original background fetches did not finish")
                        time.sleep(.02)
                else:
                    result = fixture.run_home("--jobs", str(workers), env=env)
                elapsed = time.monotonic() - start
                if result.returncode:
                    raise RuntimeError(result.stdout)
                for repo in repos:
                    assert fixture.git(repo, "rev-parse", "HEAD") == fixture.new
                samples[workers].append(elapsed)
                print(f"workers={workers}: {elapsed:.3f}s; all 12 HEADs verified", flush=True)
        medians = {workers: statistics.median(values) for workers, values in samples.items()}
        record = dict(repositories=12, fetch_delay_seconds=.25, samples=samples,
                      baseline_revision=BASELINE_REVISION,
                      median_seconds=medians, speedup_4_vs_1=medians[1] / medians[4],
                      speedup_4_vs_original=medians["original"] / medians[4])
        output = Path(__file__).with_name("home-reset-benchmark.json")
        output.write_text(json.dumps(record, indent=2) + "\n")
        print(json.dumps(record, indent=2))
    finally:
        fixture.tearDown()


if __name__ == "__main__":
    main()
