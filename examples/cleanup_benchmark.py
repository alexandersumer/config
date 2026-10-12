"""Benchmark complete cleanup on fresh disposable worktrees sharing a repository."""
import argparse
import json
import os
from pathlib import Path
import statistics
import subprocess
import tempfile
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--candidate", type=Path, required=True)
    parser.add_argument("--baseline", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--jobs", type=int, nargs="+", default=[1, 2, 4, 8])
    parser.add_argument("--worktrees", type=int, default=8)
    parser.add_argument("--packages", type=int, default=1200)
    parser.add_argument("--rounds", type=int, default=3)
    args = parser.parse_args()
    if min(args.worktrees, args.packages, args.rounds) < 1 or any(
        not 1 <= jobs <= 32 for jobs in args.jobs
    ):
        parser.error("positive fixture sizes and jobs between 1 and 32 are required")
    variants = [(f"jobs-{jobs}", args.candidate.resolve(), jobs) for jobs in args.jobs]
    if args.baseline:
        variants.insert(0, ("baseline", args.baseline.resolve(), None))
    for _, binary, _ in variants:
        if not binary.is_file():
            parser.error(f"binary does not exist: {binary}")
    env = dict(
        os.environ,
        GIT_CONFIG_GLOBAL="/dev/null",
        GIT_CONFIG_NOSYSTEM="1",
        GIT_AUTHOR_NAME="Benchmark",
        GIT_AUTHOR_EMAIL="bench@example.test",
        GIT_COMMITTER_NAME="Benchmark",
        GIT_COMMITTER_EMAIL="bench@example.test",
        TERM="dumb",
        NO_COLOR="1",
    )
    for name in ["GIT_DIR", "GIT_WORK_TREE", "GIT_COMMON_DIR", "GIT_INDEX_FILE", "GIT_CONFIG_COUNT"]:
        env.pop(name, None)

    def git(repo, *argv):
        return subprocess.run(
            ["git", "-C", str(repo), *argv], env=env, check=True,
            capture_output=True, text=True,
        ).stdout.strip()

    rows = []
    for repetition in range(args.rounds):
        offset = repetition % len(variants)
        for label, binary, jobs in variants[offset:] + variants[:offset]:
            with tempfile.TemporaryDirectory(prefix="workctl-cleanup-benchmark-") as directory:
                root = Path(directory)
                repo = root / "primary"
                repo.mkdir()
                git(repo, "init", "-b", "main")
                (repo / ".gitignore").write_text("node_modules/\n")
                (repo / "source").write_text("committed work\n")
                git(repo, "add", ".")
                git(repo, "commit", "-m", "fixture")
                primary_head = git(repo, "rev-parse", "HEAD")
                dirty = root / "active"
                git(repo, "worktree", "add", "--detach", str(dirty))
                (dirty / "precious").write_text("keep active work")
                trees = []
                for index in range(args.worktrees):
                    tree = root / f"linked-{index}"
                    git(repo, "worktree", "add", "--detach", str(tree))
                    git(tree, "commit", "--allow-empty", "-m", f"unpublished-{index}")
                    trees.append((tree, git(tree, "rev-parse", "HEAD")))
                    for package in range(args.packages):
                        dependency = tree / "node_modules" / f"package-{package}" / "lib"
                        dependency.mkdir(parents=True)
                        for filename in ["index.js", "index.d.ts", "map.json", "metadata.json"]:
                            (dependency / filename).write_text("generated dependency\n")
                command = [str(binary), "git", "worktree", "clean", "--apply", "--json", str(repo)]
                if jobs is not None:
                    command += ["--jobs", str(jobs)]
                started = time.monotonic()
                run = subprocess.run(command, env=env, capture_output=True, text=True, timeout=300)
                elapsed = time.monotonic() - started
                assert run.returncode == 0, run.stdout + run.stderr
                result = json.loads(run.stdout)
                assert result["summary"]["removed"] == args.worktrees, result
                assert result["summary"]["protected_worktrees"] == 1, result
                assert result["summary"]["errors"] == result["summary"]["blocked"] == 0, result
                for tree, head in trees:
                    assert not tree.exists()
                    assert git(repo, "rev-parse", f"refs/workctl/cleanup/{head}") == head
                    assert git(repo, "show", f"refs/workctl/cleanup/{head}:source") == "committed work"
                assert (dirty / "precious").read_text() == "keep active work"
                assert git(repo, "rev-parse", "HEAD") == primary_head
                assert git(repo, "status", "--porcelain") == ""
                assert git(repo, "worktree", "list", "--porcelain").count("worktree ") == 2
                row = dict(repetition=repetition + 1, variant=label, seconds=elapsed,
                           summary=result["summary"], verified=True)
                if jobs is not None:
                    row["phases"] = {
                        phase: sum(entry["timing"][phase] for entry in result["results"])
                        for phase in ["inspection_seconds", "recovery_seconds", "removal_seconds"]
                    }
                rows.append(row)
                medians = {
                    name: statistics.median(row["seconds"] for row in rows if row["variant"] == name)
                    for name, _, _ in variants
                    if any(row["variant"] == name for row in rows)
                }
                args.output.parent.mkdir(parents=True, exist_ok=True)
                args.output.write_text(json.dumps(dict(
                    worktrees=args.worktrees, packages_per_tree=args.packages,
                    files_per_package=4, rounds=args.rounds, results=rows, medians=medians,
                ), indent=2) + "\n")
                print(json.dumps(dict(repetition=repetition + 1, variant=label,
                                      seconds=elapsed, verified=True)), flush=True)
    print(json.dumps(dict(medians=medians)), flush=True)


if __name__ == "__main__":
    main()
