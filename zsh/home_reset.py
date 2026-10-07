"""Home workspace discovery and reset policy; execution lives in repo_batch."""

import argparse
from bisect import bisect_left
import json
import os
from pathlib import Path
import re
import shlex
import signal
import subprocess
import sys
import tempfile
import threading
import time

from repo_batch import Job, Supervisor


PRUNE = {"node_modules", ".venv", "venv", "target", "dist", "build", ".next", ".turbo", ".gradle", ".git"}
HOME_PRUNE = {"Library", ".Trash", ".cache", "Downloads", "Applications", "Desktop", "Documents", "Movies", "Music", "Pictures", "Public", ".npm", ".pyenv", ".sdkman"}


def git(path, *args):
    return subprocess.check_output(["git", "-C", str(path), *args], stderr=subprocess.PIPE, timeout=15)


def discover(root, recursive, nested):
    candidates = []
    if recursive:
        def failed(error):
            raise error
        for directory, dirs, files in os.walk(root, onerror=failed):
            dirs.sort()
            if ".git" in dirs or ".git" in files:
                candidates.append(Path(directory))
                if not nested:
                    dirs[:] = []
                    continue
            dirs[:] = [d for d in dirs if d not in PRUNE and
                       not (Path(directory) == Path.home() and d in HOME_PRUNE)]
    else:
        candidates = sorted(p for p in root.iterdir() if p.is_dir())
    found = []
    seen = set()
    for candidate in candidates:
        # A normal folder inside a repository is not itself a repository.
        if not (candidate / ".git").exists():
            continue
        top = Path(os.fsdecode(git(candidate, "rev-parse", "--show-toplevel"))[:-1]).resolve()
        common = Path(os.fsdecode(git(candidate, "rev-parse", "--git-common-dir"))[:-1])
        common = (candidate / common).resolve()
        if top not in seen:
            seen.add(top)
            found.append((top, common))
    return found


def retryable(log):
    if re.search(r"Permission denied|Access denied|Forbidden|Authentication failed|could not read Username|Host key verification failed|refusing|would be overwritten|Git operation in progress|cannot lock|Repository is busy", log, re.I):
        return False
    return bool(re.search(r"Could not resolve|Connection (?:timed out|reset|refused|closed)|Operation timed out|early EOF|unexpected disconnect|remote end hung up|TLS connection was non-properly terminated|(?:HTTP/[\d.]+\s+|returned error:?\s*)(?:5\d\d|429)\b|Failed to connect|Couldn't connect|kex_exchange_identification", log, re.I))


def check_tree(target):
    cwd = Path.cwd()
    if git(cwd, "status", "--porcelain=v1", "--untracked-files=no"):
        raise ValueError("Tracked changes or a dirty submodule: refusing reset.")
    git_dir = Path(os.fsdecode(git(cwd, "rev-parse", "--git-dir"))[:-1])
    for name in ("MERGE_HEAD", "CHERRY_PICK_HEAD", "REVERT_HEAD", "rebase-merge", "rebase-apply", "BISECT_LOG"):
        if (git_dir / name).exists():
            raise ValueError(f"Git operation in progress ({name}): refusing reset.")
    tracked = set(git(cwd, "ls-tree", "-r", "--name-only", "-z", target).split(b"\0")) - {b""}
    ordered = sorted(tracked)
    others = git(cwd, "ls-files", "--others", "--directory", "-z").split(b"\0")
    for entry in others:
        if not entry:
            continue
        path = entry.rstrip(b"/")
        parts = path.split(b"/")
        prefixes = {b"/".join(parts[:i]) for i in range(1, len(parts) + 1)}
        prefix = path + b"/"
        i = bisect_left(ordered, prefix)
        has_descendant = i < len(ordered) and ordered[i].startswith(prefix)
        if prefixes & tracked or has_descendant:
            raise ValueError(f"Untracked or ignored path would be overwritten: {os.fsdecode(entry)!r}")


def backup_branch(target, branch):
    cwd = Path.cwd()
    # Make overwritten default-branch commits recoverable without relying on reflogs.
    local = "refs/heads/" + branch
    old = git(cwd, "for-each-ref", "--format=%(objectname)", local).strip()
    if not old:
        return
    check_tree(local)
    new = git(cwd, "rev-parse", target).strip()
    if old != new:
        backup = f"refs/home-reset-backups/{time.time_ns()}/{local[len('refs/heads/') :]}"
        git(cwd, "update-ref", backup, os.fsdecode(old))
        print(f"Saved previous branch tip: {backup}", flush=True)


def display(value, multiline=False):
    # Paths and Git logs may contain terminal control characters.
    value = str(value)
    if multiline:
        value = re.sub(r"\x1b\[[0-?]*[ -/]*[@-~]", "", value)
    return "".join(c if (multiline and c == "\n") or c.isprintable() else f"\\x{ord(c):02x}" for c in value)


def positive(value):
    n = int(value)
    if n < 1:
        raise argparse.ArgumentTypeError("must be a positive integer")
    return n


def nonnegative(value):
    n = int(value)
    if not 0 <= n <= 3600:
        raise argparse.ArgumentTypeError("must be between 0 and 3600")
    return n


def main(argv=None):
    argv = sys.argv[1:] if argv is None else argv
    if argv[:1] == ["--retryable"]:
        return 0 if retryable(Path(argv[1]).read_text(errors="replace")) else 1
    if argv[:1] == ["--remote-branch"]:
        for line in git(Path.cwd(), "ls-remote", "--symref", argv[1], "HEAD").splitlines():
            if line.startswith(b"ref: refs/heads/") and line.endswith(b"\tHEAD"):
                print(os.fsdecode(line[len(b"ref: refs/heads/"):-len(b"\tHEAD")]))
                return 0
        raise ValueError("Remote HEAD does not identify a default branch.")
    if argv[:1] in (["--check-tree"], ["--prepare-tree"]):
        check_tree(argv[1])
        if argv[0] == "--prepare-tree":
            backup_branch(argv[1], argv[2])
        return 0
    parser = argparse.ArgumentParser(description="Reset workspace repositories with supervised parallel workers. Local branches and worktrees are preserved; overwritten default-branch tips are backed up.")
    parser.add_argument("root", nargs="?")
    parser.add_argument("--root", dest="named_root")
    parser.add_argument("--jobs", type=positive, default=4)
    parser.add_argument("--retries", type=nonnegative, default=3, help="total attempts per repository, 0 treated as 1 (maximum 10; legacy meaning)")
    parser.add_argument("--retry-delay", type=nonnegative, default=2)
    parser.add_argument("--timeout", type=positive, default=300, help="seconds per attempt")
    parser.add_argument("--all-home", action="store_true")
    parser.add_argument("--include-nested", action="store_true")
    parser.add_argument("--list", action="store_true")
    if "--" in argv:
        split = argv.index("--")
        options, forwarded = argv[:split], argv[split + 1:]
    else:
        options, forwarded = argv, []
    args = parser.parse_args(options)
    if args.jobs > 32 or args.retries > 10:
        parser.error("at most 32 workers and 10 total attempts are supported")
    if args.root and args.named_root:
        parser.error("specify only one root path")
    if args.all_home and (args.root or args.named_root):
        parser.error("--all-home cannot be combined with a root path")
    if any(a.startswith("--") and a not in ("--sync", "--no-prune") for a in forwarded) or len([a for a in forwarded if not a.startswith("--")]) > 2:
        parser.error("forward only --sync, --no-prune, and optional remote/branch")
    root_arg = args.named_root or args.root
    home = Path.home()
    if root_arg:
        roots = [Path(root_arg).expanduser().resolve()]
    elif args.all_home:
        roots = [home]
    elif os.environ.get("HOME_RESET_TO_ORIGIN_ROOTS"):
        roots = [Path(p).expanduser().resolve() for p in shlex.split(os.environ["HOME_RESET_TO_ORIGIN_ROOTS"])]
    else:
        roots = [home / p for p in ("atlassian", "oss", "src", "stable") if (home / p).is_dir()]
    discovered, seen = [], set()
    for root in roots:
        if not root.is_dir():
            parser.error(f"not a directory: {root}")
        entries = []
        for path, common in discover(root, True, args.include_nested):
            if path not in seen:
                seen.add(path)
                entries.append((path, common))
        discovered.append((root, entries))
    if args.list:
        for _, entries in discovered:
            for path, _ in entries:
                print(display(path))
        return 0
    total = sum(len(entries) for _, entries in discovered)
    if not total:
        print("No repositories found.")
        return 0
    log_dir = Path(tempfile.mkdtemp(prefix="home-reset-"))
    source = Path(__file__).with_name("git-functions.zsh")
    env = dict(os.environ, HOME_RESET_SUPERVISED="1", GIT_TERMINAL_PROMPT="0", GCM_INTERACTIVE="Never", NO_COLOR="1", TERM="dumb")
    output_guard = threading.Lock()
    def progress(job, attempt, message):
        with output_guard:
            print(f"{message}: {display(job.name)} | attempt {attempt}/{max(1, args.retries)}", flush=True)
    supervisor = Supervisor(args.jobs, max(1, args.retries), args.timeout, args.retry_delay, log_dir, retryable, env, progress)
    def stop(signum, frame):
        supervisor.cancel()
    previous = {s: signal.signal(s, stop) for s in (signal.SIGINT, signal.SIGTERM)}
    all_results = []
    started = time.monotonic()
    print(f"Resetting {total} repositories; workers={args.jobs}, attempts={max(1, args.retries)}, deadline={args.timeout}s/attempt.", flush=True)
    print(f"Branches/worktrees preserved. Logs: {log_dir}", flush=True)
    try:
        for root, entries in discovered:
            print(f"\nWorkspace: {display(root)} ({len(entries)} repositories)", flush=True)
            jobs = [Job(str(path), path, ("zsh", "-f", "-c", 'source "$1"; shift; _reset_to_remote_default_single "$@" --sync --no-prune', "home-reset", str(source), *forwarded), common) for path, common in entries]
            done = 0
            def completed(result):
                nonlocal done
                done += 1
                label = "OK" if result.code == 0 else "CANCELLED" if result.code == 130 else "FAILED"
                with output_guard:
                    print(f"[{done}/{len(jobs)} finished] {label}: {display(result.name)} ({result.attempts} attempts, {result.seconds:.1f}s)", flush=True)
            results = supervisor.batch(jobs, completed)
            all_results.extend(results)
            for result in results:
                print(f"\n--- {display(result.name)} | exit={result.code} | log={result.log} ---")
                print(display(result.log.read_text(errors="replace"), multiline=True), end="", flush=True)
        record = [dict(name=r.name, code=r.code, attempts=r.attempts, seconds=r.seconds, log=str(r.log)) for r in all_results]
        (log_dir / "results.json").write_text(json.dumps(record, indent=2))
        ok = sum(r.code == 0 for r in all_results)
        recovered = sum(r.code == 0 and r.attempts > 1 for r in all_results)
        print(f"\nSummary: {len(all_results)} total | {ok} OK ({recovered} recovered) | {len(all_results) - ok} failed/cancelled | {time.monotonic() - started:.1f}s")
        print(f"Full logs and results: {log_dir}")
        return 130 if supervisor.cancelled.is_set() else int(ok != len(all_results))
    finally:
        for s, handler in previous.items():
            signal.signal(s, handler)


if __name__ == "__main__":
    try:
        sys.exit(main())
    except KeyboardInterrupt:
        sys.exit(130)
    except subprocess.TimeoutExpired as error:
        print(f"Operation timed out: {display(error)}", file=sys.stderr)
        sys.exit(124)
    except (ValueError, OSError, subprocess.SubprocessError) as error:
        detail = os.fsdecode(error.stderr or b"") if isinstance(error, subprocess.CalledProcessError) else ""
        print(f"Error: {display(error)} {display(detail)}", file=sys.stderr)
        sys.exit(1)
