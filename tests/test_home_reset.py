import concurrent.futures
import io
import json
import os
from pathlib import Path
import shutil
import shlex
import signal
import subprocess
import sys
import tempfile
import threading
import time
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "zsh"))
from repo_batch import Job, Result, Supervisor
from batch_output import BatchOutput, color
from home_reset import discover, failure_reason, retryable


def command(cwd, *args, env=None):
    return subprocess.check_output(args, cwd=cwd, env=env, stderr=subprocess.STDOUT).decode().strip()


class BatchTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.logs = self.root / "logs"
        self.logs.mkdir()

    def tearDown(self):
        self.temp.cleanup()

    def supervisor(self, jobs=4, attempts=3, timeout=3):
        return Supervisor(jobs, attempts, timeout, 0, self.logs, retryable)

    def job(self, name, script, resource=None):
        resource = resource or self.root / name
        resource.mkdir(exist_ok=True)
        return Job(name, self.root, (sys.executable, "-c", script), resource)

    def test_bounded_overlap_and_order(self):
        trace = self.root / "trace"
        script = "import os,time; f=os.open(%r,os.O_CREAT|os.O_APPEND|os.O_WRONLY,0o600); os.write(f,('start '+str(time.monotonic())+'\\n').encode()); time.sleep(.25); os.write(f,('end '+str(time.monotonic())+'\\n').encode())" % str(trace)
        jobs = [self.job(str(i), script) for i in range(12)]
        results = self.supervisor().batch(jobs)
        self.assertEqual([r.name for r in results], [j.name for j in jobs])
        self.assertTrue(all(r.code == 0 for r in results))
        active, peak = 0, 0
        for kind, stamp in sorted((line.split() for line in trace.read_text().splitlines()), key=lambda pair: float(pair[1])):
            active += 1 if kind == "start" else -1
            peak = max(peak, active)
            self.assertLessEqual(active, 4)
        self.assertEqual(active, 0)
        self.assertGreaterEqual(peak, 2)

    def test_shared_resource_serializes(self):
        resource = self.root / "shared"
        jobs = [self.job(str(i), "import time; time.sleep(.15)", resource) for i in range(4)]
        start = time.monotonic()
        self.assertTrue(all(r.code == 0 for r in self.supervisor().batch(jobs)))
        self.assertGreater(time.monotonic() - start, .6)

    def test_cross_invocation_lock(self):
        resource = self.root / "shared"
        marker = self.root / "running"
        job = self.job("first", f"from pathlib import Path; import time; Path({str(marker)!r}).touch(); time.sleep(.6)", resource)
        first, second = self.supervisor(), self.supervisor()
        with concurrent.futures.ThreadPoolExecutor() as pool:
            future = pool.submit(first.batch, [job])
            until = time.monotonic() + 2
            while not marker.exists() and time.monotonic() < until:
                time.sleep(.01)
            result = second.batch([job])[0]
            self.assertEqual(result.code, 75)
            self.assertEqual(result.attempts, 0)
            self.assertEqual(future.result()[0].code, 0)

    def test_transient_recovery_and_attempt_logs(self):
        count = self.root / "count"
        job = self.job("retry", f"from pathlib import Path; import sys; p=Path({str(count)!r}); n=int(p.read_text())+1 if p.exists() else 1; p.write_text(str(n)); print('Connection reset' if n<3 else 'success'); sys.exit(n<3)")
        result = self.supervisor().batch([job])[0]
        self.assertEqual((result.code, result.attempts), (0, 3))
        self.assertEqual(result.log.read_text().count("Connection reset"), 2)

    def test_permanent_failures_do_not_retry(self):
        for message in ("Permission denied (publickey)", "Authentication failed", "dirty worktree", "cannot lock ref"):
            job = self.job("permanent", f"import sys; print({message!r}); sys.exit(1)")
            result = self.supervisor().batch([job])[0]
            self.assertEqual((result.code, result.attempts), (1, 1))

    def test_attempt_limit(self):
        job = self.job("exhausted", "import sys; print('Connection reset'); sys.exit(1)")
        result = self.supervisor().batch([job])[0]
        self.assertEqual((result.code, result.attempts), (1, 3))

    def test_timeout_reaps_grandchildren(self):
        marker = self.root / "escaped"
        job = self.job("hang", f"import subprocess,time,sys; subprocess.Popen([sys.executable,'-c',\"import time; from pathlib import Path; time.sleep(1); Path({str(marker)!r}).touch()\"]); time.sleep(20)")
        result = self.supervisor(attempts=1, timeout=.2).batch([job])[0]
        self.assertEqual(result.code, 124)
        time.sleep(1.1)
        self.assertFalse(marker.exists())

    def test_cancel_running_and_queued(self):
        supervisor = self.supervisor(jobs=2)
        jobs = [self.job(str(i), "import time; time.sleep(10)") for i in range(6)]
        timer = threading.Timer(.2, supervisor.cancel)
        timer.start()
        start = time.monotonic()
        results = supervisor.batch(jobs)
        timer.join()
        self.assertTrue(all(r.code == 130 for r in results))
        self.assertLess(time.monotonic() - start, 3)
        self.assertTrue(all(r.log.exists() for r in results))

    def test_cancel_interrupts_retry_backoff(self):
        supervisor = self.supervisor()
        supervisor.delay = 30
        job = self.job("backoff", "import sys; print('Connection reset'); sys.exit(1)")
        timer = threading.Timer(.2, supervisor.cancel)
        timer.start()
        start = time.monotonic()
        result = supervisor.batch([job])[0]
        timer.join()
        self.assertEqual(result.code, 130, result.log.read_text())
        self.assertLess(time.monotonic() - start, 3)

    def test_cancellation_before_spawn_never_starts_worker(self):
        supervisor = self.supervisor()
        supervisor.progress = lambda *args: supervisor.cancel()
        job = self.job('cancel-before-spawn', 'raise RuntimeError("must not execute")')
        with patch('repo_batch.subprocess.Popen', wraps=subprocess.Popen) as spawn:
            result = supervisor.batch([job])[0]
        spawn.assert_not_called()
        self.assertEqual((result.code, result.attempts), (130, 0))

    def test_cancellation_waits_for_inflight_spawn_before_returning(self):
        supervisor = self.supervisor()
        job = self.job('inflight-spawn', 'import time; time.sleep(20)')
        entered, resume, cancelling, finished = (threading.Event() for _ in range(4))
        real_spawn = subprocess.Popen
        def launch(*args, **kwargs):
            entered.set()
            if not resume.wait(2):
                raise RuntimeError('spawn fixture was not released')
            return real_spawn(*args, **kwargs)
        def cancel():
            cancelling.set()
            supervisor.cancel()
            finished.set()
        with patch('repo_batch.subprocess.Popen', side_effect=launch):
            with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
                running = pool.submit(supervisor.batch, [job])
                try:
                    self.assertTrue(entered.wait(2))
                    cancellation = pool.submit(cancel)
                    self.assertTrue(cancelling.wait(2))
                    self.assertFalse(finished.wait(.05))
                finally:
                    resume.set()
                    supervisor.cancel()
                cancellation.result(timeout=2)
                result = running.result(timeout=3)[0]
        self.assertEqual(result.code, 130, result.log.read_text())

    def test_sigterm_resistant_worker_is_killed(self):
        job = self.job("resistant", "import signal,time; signal.signal(signal.SIGTERM,signal.SIG_IGN); time.sleep(20)")
        start = time.monotonic()
        result = self.supervisor(attempts=1, timeout=.2).batch([job])[0]
        self.assertEqual(result.code, 124)
        self.assertLess(time.monotonic() - start, 3)

    def test_deadline_does_not_signal_a_group_retired_by_sigterm(self):
        real_signal = os.killpg
        retired = set()
        def raced_signal(group, sig):
            if sig == signal.SIGTERM:
                retired.add(group)
            if sig == signal.SIGKILL and group in retired:
                raise PermissionError("group retired after SIGTERM")
            return real_signal(group, sig)
        job = self.job("retirement", "import time; time.sleep(20)")
        with patch("repo_batch.os.killpg", raced_signal):
            result = self.supervisor(attempts=1, timeout=.2).batch([job])[0]
        self.assertEqual(result.code, 124, result.log.read_text())

    def test_genuine_signal_permission_failure_remains_a_failure(self):
        real_signal = os.killpg
        job = self.job("permission", "import time; time.sleep(20)")
        def denied(group, sig):
            # Stop our fixture for cleanup, then simulate a real permission error.
            real_signal(group, signal.SIGKILL)
            raise PermissionError("cannot signal live worker group")
        with patch("repo_batch.os.killpg", denied):
            result = self.supervisor(attempts=1, timeout=.2).batch([job])[0]
        self.assertEqual(result.code, 1)
        self.assertIn("cannot signal live worker group", result.log.read_text())

    def test_reporting_failure_cancels_remaining_workers(self):
        supervisor = self.supervisor(jobs=2)
        jobs = [self.job("fast", "pass"), self.job("slow", "import time; time.sleep(20)")]
        def broken_output(result):
            raise BrokenPipeError("closed output")
        start = time.monotonic()
        with self.assertRaises(BrokenPipeError):
            supervisor.batch(jobs, broken_output)
        self.assertTrue(supervisor.cancelled.is_set())
        self.assertLess(time.monotonic() - start, 3)

    def test_heartbeat_output_failure_cancels_running_workers(self):
        class BrokenStream(io.StringIO):
            def write(self, value):
                raise BrokenPipeError('closed output')
        supervisor = self.supervisor(jobs=2)
        jobs = [self.job(str(i), 'import time; time.sleep(20)') for i in range(4)]
        start = time.monotonic()
        with self.assertRaises(BrokenPipeError):
            with BatchOutput([job.name for job in jobs], str, str, BrokenStream(), interval=.03, on_error=supervisor.cancel) as output:
                supervisor.progress = output.progress
                supervisor.batch(jobs, output.completed)
        self.assertTrue(supervisor.cancelled.is_set())
        self.assertLess(time.monotonic() - start, 3)
        self.assertEqual(len(list(self.logs.glob('*.log'))), 4)

    def test_retry_classifier(self):
        for log in ("HTTP/2 429", "error: requested URL returned error: 503", "early EOF", "Could not resolve hostname"):
            self.assertTrue(retryable(log), log)
        for log in ("HTTP/2 401", "Authentication failed: Connection reset", "dirty files", "cannot lock ref", "Error: would be overwritten: 503", "error: refusing reset; Connection reset"):
            self.assertFalse(retryable(log), log)

    def test_internal_git_probe_timeout_has_retryable_exit(self):
        script = f"""import runpy,subprocess,sys
sys.path.insert(0, {str(ROOT / 'zsh')!r})
def timeout(*args, **kwargs):
    raise subprocess.TimeoutExpired(args[0], 15)
subprocess.check_output = timeout
sys.argv = ['home_reset.py', '--remote-branch', 'origin']
runpy.run_path({str(ROOT / 'zsh/home_reset.py')!r}, run_name='__main__')
"""
        result = subprocess.run([sys.executable, "-c", script], stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        self.assertEqual(result.returncode, 124, result.stdout)
        self.assertIn("Operation timed out", result.stdout)

    def test_worker_spawn_failure_is_reported(self):
        job = self.job("missing", "")
        job = Job(job.name, job.cwd, ("/does/not/exist",), job.resource)
        result = self.supervisor().batch([job])[0]
        self.assertEqual(result.code, 1)
        self.assertIn("Worker failure", result.log.read_text())

    def test_non_utf8_process_output(self):
        job = self.job("bytes", "import os; os.write(1, b'non-utf8: \\xff\\n')")
        result = self.supervisor().batch([job])[0]
        self.assertEqual(result.code, 0)
        self.assertIn("non-utf8", result.log.read_text(errors="replace"))

    def test_multiple_batches_keep_distinct_logs(self):
        supervisor = self.supervisor()
        first = supervisor.batch([self.job("a", "print('first workspace')")])[0]
        second = supervisor.batch([self.job("b", "print('second workspace')")])[0]
        self.assertNotEqual(first.log, second.log)
        self.assertIn("first workspace", first.log.read_text())
        self.assertIn("second workspace", second.log.read_text())

    def test_stress_mixed_failures(self):
        jobs = []
        for i in range(80):
            state = self.root / f"attempt-{i}"
            script = f"from pathlib import Path; import sys,time; p=Path({str(state)!r}); n=int(p.read_text())+1 if p.exists() else 1; p.write_text(str(n)); time.sleep(.01); mode={i % 3}; failed=mode==1 and n==1 or mode==2; print('Connection reset' if mode==1 and failed else 'permanent failure' if failed else 'OK'); sys.exit(bool(failed))"
            jobs.append(self.job(str(i), script, self.root / f"resource-{i % 9}"))
        results = self.supervisor().batch(jobs)
        self.assertEqual(len(results), 80)
        self.assertEqual(len({r.log for r in results}), 80)
        for i, result in enumerate(results):
            self.assertEqual(result.code, int(i % 3 == 2))
            self.assertEqual(result.attempts, 2 if i % 3 == 1 else 1)


class OutputTests(unittest.TestCase):
    def test_failure_is_immediate_while_earlier_job_is_pending(self):
        stream = io.StringIO()
        with BatchOutput(['first', 'second'], str, lambda r: 'Access denied.', stream) as output:
            output.completed(Result('second', 1, 1, 0, Path('0002.log')))
            self.assertIn('Failed: second', stream.getvalue())
            self.assertIn('Access denied.', stream.getvalue())
            output.completed(Result('first', 0, 3, 0, Path('0001.log')))
        text = stream.getvalue()
        self.assertIn('2/2 complete, 1 failed', text)
        self.assertNotIn('first', text)
        self.assertIn('Log: 0002.log', text)

    def test_failure_block_wraps_at_narrow_terminal_width(self):
        stream = io.StringIO()
        reason = 'Fetch failed: repository unavailable or access denied. Reset was not performed.'
        with patch('batch_output.shutil.get_terminal_size', return_value=os.terminal_size((40, 24))):
            with BatchOutput(['repo'], str, lambda r: reason, stream) as output:
                output.completed(Result('repo', 128, 1, 0, Path('0001.log')))
        text = stream.getvalue()
        block = text.split('  Failed: repo\n', 1)[1].split('\n\n', 1)[0]
        self.assertTrue(all(len(line) <= 40 for line in block.splitlines()))
        self.assertIn('    Log: 0001.log', block)
        self.assertIn('Reset was not performed.', ' '.join(line.strip() for line in block.splitlines()))
        self.assertNotIn('\r', text)

    def test_pipeline_does_not_repeat_unchanged_progress(self):
        stream = io.StringIO()
        with BatchOutput(['first', 'second'], str, str, stream, interval=.03) as output:
            output.progress(Job('second', Path('.'), (), Path('.')), 1, 'RUNNING')
            output.completed(Result('first', 0, 1, 0, Path('0001.log')))
            deadline = time.monotonic() + 2
            while 'Waiting for second' not in stream.getvalue() and time.monotonic() < deadline:
                time.sleep(.01)
            time.sleep(.1)
            self.assertEqual(stream.getvalue().count('1/2 complete'), 1)
            self.assertEqual(stream.getvalue().count('Waiting for second'), 1)
            output.completed(Result('second', 0, 1, 0, Path('0002.log')))
        self.assertIn('2/2 complete', stream.getvalue())
        before = stream.getvalue()
        time.sleep(.05)
        self.assertEqual(stream.getvalue(), before)

    def test_terminal_uses_complete_lines_and_keeps_failure(self):
        class Terminal(io.StringIO):
            def isatty(self):
                return True
        stream = Terminal()
        with patch.dict(os.environ, {'NO_COLOR': '1'}):
            with BatchOutput(['first', 'second'], str, lambda r: 'Access denied.', stream) as output:
                output.completed(Result('second', 1, 1, 0, Path('0002.log')))
                self.assertTrue(stream.getvalue().endswith('\n'))
                output.completed(Result('first', 0, 1, 0, Path('0001.log')))
        text = stream.getvalue()
        self.assertIn('Failed: second', text)
        self.assertIn('2/2 complete, 1 failed', text)
        self.assertIn('\n  Failed: second\n    Access denied.\n    Log: 0002.log\n\n', text)
        self.assertNotIn('\r', text)
        self.assertNotIn('\x1b', text)

    def test_terminal_heartbeat_is_throttled_and_newline_terminated(self):
        class Terminal(io.StringIO):
            def isatty(self):
                return True
        stream = Terminal()
        with patch.dict(os.environ, {'NO_COLOR': '1'}):
            with BatchOutput(['first', 'second'], str, str, stream, interval=.03) as output:
                output.progress(Job('second', Path('.'), (), Path('.')), 1, 'RUNNING')
                output.completed(Result('first', 0, 1, 0, Path('0001.log')))
                deadline = time.monotonic() + 2
                while 'Waiting for second' not in stream.getvalue() and time.monotonic() < deadline:
                    time.sleep(.01)
                time.sleep(.1)
                text = stream.getvalue()
                self.assertEqual(text.count('1/2 complete'), 1)
                self.assertEqual(text.count('Waiting for second'), 1)
                self.assertTrue(text.endswith('\n'))
                self.assertNotIn('\r', text)
                output.completed(Result('second', 0, 1, 0, Path('0002.log')))
        self.assertIn('2/2 complete', stream.getvalue())
        self.assertNotIn('\r', stream.getvalue())

    def test_color_respects_terminal_no_color_and_dumb(self):
        class Terminal(io.StringIO):
            def isatty(self):
                return True
        stream = Terminal()
        with patch.dict(os.environ, {'TERM': 'xterm', 'NO_COLOR': ''}):
            self.assertEqual(color('failed', 'red', stream), '\x1b[31mfailed\x1b[0m')
            self.assertEqual(color('failed', 'red', io.StringIO()), 'failed')
        for env in ({'NO_COLOR': '1', 'TERM': 'xterm'}, {'NO_COLOR': '', 'TERM': 'dumb'}):
            with patch.dict(os.environ, env):
                self.assertEqual(color('failed', 'red', stream), 'failed')

    def test_bitbucket_reason_does_not_guess_between_access_and_missing_repo(self):
        with tempfile.TemporaryDirectory() as root:
            log = Path(root) / '0001.log'
            log.write_text('Attempt 1/3\nYou may not have access to this repository or it no longer exists in this workspace.\nfatal: Could not read from remote repository.\nerror: fetch failed (exit 128); reset was not performed\n')
            result = Result('repo', 128, 1, 0, log)
            self.assertIn('repository unavailable or access denied', failure_reason(result))
            self.assertFalse(retryable(log.read_text()))

    def test_failure_reason_uses_last_attempt_and_strips_terminal_color(self):
        with tempfile.TemporaryDirectory() as root:
            log = Path(root) / '0001.log'
            log.write_text('Attempt 1/2\nConnection reset\nAttempt 2/2\n\033[31mPermission denied (publickey)\033[0m\nerror: fetch failed (exit 1); reset was not performed\n')
            self.assertEqual(failure_reason(Result('repo', 1, 2, 0, log)), 'Permission denied (publickey)')

    def test_case_conflict_reason_explains_failed_fetch_and_preserved_checkout(self):
        with tempfile.TemporaryDirectory() as root:
            log = Path(root) / '0001.log'
            log.write_text("Attempt 1/1\nerror: You're on a case-insensitive filesystem, and the remote you are\ntrying to fetch from has references that only differ in casing.\nerror: fetch failed (exit 1); reset was not performed\n")
            result = Result('repo', 1, 1, 0, log)
            self.assertEqual(failure_reason(result), 'Remote refs differ only by case on this case-insensitive filesystem. Fetch failed; reset was not performed.')
            self.assertFalse(retryable(log.read_text()))


class GitTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.workspace = self.root / "workspace"
        self.workspace.mkdir()
        self.seed = self.root / "seed"
        self.seed.mkdir()
        self.env = dict(os.environ, GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL=os.devnull, GIT_AUTHOR_NAME="Test", GIT_AUTHOR_EMAIL="test@example.com", GIT_COMMITTER_NAME="Test", GIT_COMMITTER_EMAIL="test@example.com")
        self.git(self.seed, "init", "-b", "main")
        (self.seed / "file").write_text("initial\n")
        self.git(self.seed, "add", ".")
        self.git(self.seed, "commit", "-m", "initial")
        self.origin = self.root / "origin.git"
        self.git(self.root, "clone", "--bare", str(self.seed), str(self.origin))
        self.git(self.seed, "remote", "add", "origin", str(self.origin))
        self.repo = self.clone("a repo")
        self.old = self.git(self.repo, "rev-parse", "HEAD")
        (self.seed / "file").write_text("updated\n")
        self.git(self.seed, "commit", "-am", "update")
        self.git(self.seed, "push", "origin", "main")
        self.new = self.git(self.seed, "rev-parse", "HEAD")

    def tearDown(self):
        self.temp.cleanup()

    def git(self, path, *args):
        return command(path, "git", *args, env=self.env)

    def clone(self, name):
        path = self.workspace / name
        self.git(self.workspace, "clone", str(self.origin), str(path))
        return path

    def run_home(self, *args, env=None):
        return subprocess.run(["zsh", "-f", "-c", 'source "$1"; shift; home_reset_to_origin "$@"', "test", str(ROOT / "zsh/git-functions.zsh"), "--root", str(self.workspace), "--retry-delay", "0", *args], env=env or self.env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=30)

    def run_helper(self, cwd, function, *args, env=None):
        return subprocess.run(["zsh", "-f", "-c", 'source "$1"; shift; "$@"', "test", str(ROOT / "zsh/git-functions.zsh"), function, *args], cwd=cwd, env=env or self.env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=30)

    def shim(self, mode, delay=.25):
        directory = self.root / "bin"
        directory.mkdir(exist_ok=True)
        real_git = shutil.which("git")
        (directory / "git").write_text(f'''#!{sys.executable}
import os,sys,time
from pathlib import Path
if 'fetch' in sys.argv[1:]:
    p=Path('.git/fetch-count')
    n=int(p.read_text())+1 if p.exists() else 1
    p.write_text(str(n))
    mode=os.environ.get('FETCH_MODE')
    if mode=='transient' and n<3:
        print('Connection reset',file=sys.stderr); sys.exit(1)
    if mode=='auth':
        print('Permission denied (publickey)',file=sys.stderr); sys.exit(1)
    if mode=='hang':
        Path(os.environ['START_MARKER']).write_text(str(os.getpid())); time.sleep(20)
    if mode=='delay':
        import subprocess
        time.sleep({delay})
        result=subprocess.run([{real_git!r},*sys.argv[1:]])
        done=Path('.git/fetch-done-count')
        done.write_text(str(int(done.read_text())+1 if done.exists() else 1))
        sys.exit(result.returncode)
os.execv({real_git!r},[{real_git!r},*sys.argv[1:]])
''')
        (directory / "git").chmod(0o755)
        return dict(self.env, PATH=str(directory) + os.pathsep + os.environ["PATH"], FETCH_MODE=mode, START_MARKER=str(self.root / "started"))

    def test_real_fetch_recovery_and_permanent_failure(self):
        result = self.run_home(env=self.shim("transient"))
        self.assertEqual(result.returncode, 0, result.stdout)
        self.assertEqual((self.repo / ".git/fetch-count").read_text(), "3")
        self.assertEqual(self.git(self.repo, "rev-parse", "HEAD"), self.new)
        self.assertIn("1 recovered", result.stdout)
        self.git(self.repo, "reset", "--hard", self.old)
        (self.repo / ".git/fetch-count").unlink()
        result = self.run_home(env=self.shim("auth"))
        self.assertEqual(result.returncode, 1, result.stdout)
        self.assertEqual((self.repo / ".git/fetch-count").read_text(), "1")
        self.assertEqual(self.git(self.repo, "rev-parse", "HEAD"), self.old)

    def test_shared_helper_access_failure_never_cleans_locks_or_reports_ok(self):
        ai_lock = self.repo / ".git/refs/ai-working-log.lock"
        remote_lock = self.repo / ".git/refs/remotes/origin/main.lock"
        ai_lock.write_text("active AI writer")
        remote_lock.write_text("active remote writer")
        result = self.run_helper(self.repo, "reset_to_origin", "--single", "--sync", env=self.shim("auth"))
        self.assertEqual(result.returncode, 1, result.stdout)
        self.assertEqual((self.repo / ".git/fetch-count").read_text(), "1")
        self.assertEqual(ai_lock.read_text(), "active AI writer")
        self.assertEqual(remote_lock.read_text(), "active remote writer")
        self.assertEqual(self.git(self.repo, "rev-parse", "HEAD"), self.old)
        self.assertIn("Permission denied", result.stdout)
        self.assertNotIn("continuing", result.stdout)

    def test_shared_full_fetch_failure_is_not_masked(self):
        lock = self.repo / ".git/refs/remotes/origin/main.lock"
        lock.write_text("active lock")
        result = self.run_helper(self.repo, "reset_to_origin", "--single", "--sync")
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertEqual(self.git(self.repo, "rev-parse", "HEAD"), self.old)
        self.assertEqual(lock.read_text(), "active lock")
        self.assertIn("cannot lock ref", result.stdout)
        self.assertNotIn("resetting to", result.stdout)

    def test_branch_pruning_preserves_canonical_and_linked_worktrees(self):
        self.git(self.repo, "branch", "feature")
        self.git(self.repo, "branch", "unused")
        linked = self.root / "linked"
        self.git(self.repo, "worktree", "add", str(linked), "feature")
        (self.repo / "canonical-notes").write_text("preserve canonical")
        (linked / "linked-notes").write_text("preserve linked")
        result = self.run_helper(linked, "prune_branch", "--force", "main", "unused")
        self.assertEqual(result.returncode, 0, result.stdout)
        self.assertEqual((self.repo / "canonical-notes").read_text(), "preserve canonical")
        self.assertEqual((linked / "linked-notes").read_text(), "preserve linked")
        self.assertEqual(self.git(self.repo, "rev-parse", "main"), self.old)
        missing = subprocess.run(["git", "-C", str(self.repo), "show-ref", "--verify", "refs/heads/unused"], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        self.assertNotEqual(missing.returncode, 0)
        result = self.run_helper(self.repo, "prune_all_except_origin", "main")
        self.assertEqual(result.returncode, 0, result.stdout)
        self.assertEqual(self.git(linked, "branch", "--show-current"), "feature")
        self.assertIn("keeping branch feature", result.stdout)

    def test_branch_case_conflict_is_reported_without_ref_repair(self):
        # Packed refs can represent both spellings even on a case-insensitive FS.
        self.git(self.origin, "pack-refs", "--all", "--prune")
        packed = self.origin / "packed-refs"
        refs = {line.split()[1]: line.split()[0] for line in packed.read_text().splitlines() if line and not line.startswith("#")}
        refs["refs/heads/topic/child"] = self.old
        refs["refs/heads/Topic/child"] = self.new
        packed.write_text("# pack-refs with: peeled fully-peeled sorted\n" + "".join(f"{refs[ref]} {ref}\n" for ref in sorted(refs)))
        listing = self.git(self.repo, "ls-remote", "origin")
        self.assertIn("refs/heads/topic/child", listing)
        self.assertIn("refs/heads/Topic/child", listing)
        case_probe = self.root / "Case-Probe"
        case_probe.touch()
        insensitive = (self.root / "case-probe").exists()
        result = self.run_helper(self.repo, "reset_to_origin", "--single", "--sync")
        if insensitive:
            self.assertNotEqual(result.returncode, 0, result.stdout)
            self.assertRegex(result.stdout, "case-insensitive|cannot lock ref")
            self.assertNotIn("resetting to", result.stdout)
            self.assertEqual(self.git(self.repo, "rev-parse", "HEAD"), self.old)
        else:
            self.assertEqual(result.returncode, 0, result.stdout)
            self.assertEqual(self.git(self.repo, "rev-parse", "refs/remotes/origin/topic/child"), self.old)
            self.assertEqual(self.git(self.repo, "rev-parse", "refs/remotes/origin/Topic/child"), self.new)
        self.assertNotIn("cleaning up", result.stdout)
        self.assertNotIn("continuing", result.stdout)

    def test_real_fetch_timeout(self):
        result = self.run_home("--verbose", "--timeout", "1", "--retries", "2", env=self.shim("hang"))
        self.assertEqual(result.returncode, 1, result.stdout)
        # The deadline includes startup/preflight; a loaded attempt can expire
        # before reaching fetch. Verify the actual attempt budget and final code.
        self.assertIn("Attempt 1/2", result.stdout)
        self.assertIn("Attempt 2/2", result.stdout)
        logs = Path(result.stdout.split('Logs: ', 1)[1].strip())
        record = json.loads((logs / 'results.json').read_text())[0]
        self.assertEqual((record['attempts'], record['code']), (2, 124))
        self.assertIn("exit=124", result.stdout)
        self.assertEqual(self.git(self.repo, "rev-parse", "HEAD"), self.old)

    def test_supervised_slow_safety_scan_uses_attempt_deadline(self):
        directory = self.root / 'slow-scan-bin'
        directory.mkdir()
        real_git = shutil.which('git')
        (directory / 'git').write_text('#!/bin/sh\ncase "$*" in\n  *"ls-files --others"*) sleep .15 ;;\nesac\nexec ' + shlex.quote(real_git) + ' "$@"\n')
        (directory / 'git').chmod(0o755)
        script = f'''import runpy,subprocess,sys
sys.path.insert(0, {str(ROOT / 'zsh')!r})
real_check_output = subprocess.check_output
def shorter_probe_budget(*args, **kwargs):
    if kwargs.get('timeout') is not None:
        kwargs['timeout'] = .1
    return real_check_output(*args, **kwargs)
subprocess.check_output = shorter_probe_budget
sys.argv = ['home_reset.py', '--check-tree', 'origin/main']
runpy.run_path({str(ROOT / 'zsh/home_reset.py')!r}, run_name='__main__')
'''
        env = dict(self.env, PATH=str(directory) + os.pathsep + os.environ['PATH'])
        for supervised, code in [('0', 124), ('1', 0)]:
            with self.subTest(supervised=supervised):
                result = subprocess.run([sys.executable, '-c', script], cwd=self.repo, env=dict(env, HOME_RESET_SUPERVISED=supervised), text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=5)
                self.assertEqual(result.returncode, code, result.stdout)
        self.assertEqual(self.git(self.repo, 'rev-parse', 'HEAD'), self.old)

    def test_public_command_sigint_reaps_fetch(self):
        env = self.shim("hang")
        marker = self.root / "started"
        process = subprocess.Popen([sys.executable, str(ROOT / "zsh/home_reset.py"), "--root", str(self.workspace), "--jobs", "2"], env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        try:
            until = time.monotonic() + 5
            while not marker.exists() and time.monotonic() < until:
                time.sleep(.02)
            self.assertTrue(marker.exists())
            process.send_signal(signal.SIGINT)
            output, _ = process.communicate(timeout=5)
            self.assertEqual(process.returncode, 130, output)
            self.assertIn("cancelled", output)
            with self.assertRaises(ProcessLookupError):
                os.kill(int(marker.read_text()), 0)
            self.assertEqual(self.git(self.repo, "rev-parse", "HEAD"), self.old)
        finally:
            if process.poll() is None:
                process.kill()
                process.communicate()

    def test_ignored_directory_file_and_directory_collisions(self):
        (self.seed / "directory").mkdir()
        (self.seed / "directory/remote").write_text("remote")
        (self.seed / "flat").write_text("remote")
        self.git(self.seed, "add", ".")
        self.git(self.seed, "commit", "-m", "directories")
        self.git(self.seed, "push", "origin", "main")
        (self.repo / "directory").write_text("untracked file")
        result = self.run_home()
        self.assertEqual(result.returncode, 1, result.stdout)
        self.assertEqual((self.repo / "directory").read_text(), "untracked file")
        (self.repo / "directory").unlink()
        (self.repo / "flat").mkdir()
        (self.repo / "flat/local").write_text("untracked directory")
        result = self.run_home()
        self.assertEqual(result.returncode, 1, result.stdout)
        self.assertEqual((self.repo / "flat/local").read_text(), "untracked directory")

    def test_duplicate_roots_do_not_duplicate_execution(self):
        env = dict(self.env, HOME_RESET_TO_ORIGIN_ROOTS=shlex.quote(str(self.workspace)) + " " + shlex.quote(str(self.workspace)))
        result = subprocess.run([sys.executable, str(ROOT / "zsh/home_reset.py"), "--list"], env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        self.assertEqual(result.returncode, 0, result.stdout)
        self.assertEqual(len(result.stdout.splitlines()), 1)

    def test_linked_worktrees_are_excluded_before_fetch_and_preserved(self):
        self.git(self.repo, 'branch', 'feature')
        linked = self.workspace / '0 linked worktree'
        self.git(self.repo, 'worktree', 'add', str(linked), 'feature')
        (linked / 'file').write_text('unpublished linked work\n')
        self.git(linked, 'commit', '-am', 'unfinished linked work')
        linked_head = self.git(linked, 'rev-parse', 'HEAD')
        (linked / 'file').write_text('precious linked edits\n')
        (linked / 'untracked').write_text('precious untracked work\n')
        linked_status = self.git(linked, 'status', '--porcelain=v1')
        excluded = set()
        found = discover(self.workspace, True, False, excluded)
        self.assertEqual([path for path, common in found], [self.repo.resolve()])
        self.assertEqual(excluded, {linked.resolve()})
        preview = self.run_home('--list')
        self.assertEqual(preview.returncode, 0, preview.stdout)
        self.assertNotIn('0 linked worktree', preview.stdout)
        result = self.run_home(env=self.shim('delay', delay=0))
        self.assertEqual(result.returncode, 0, result.stdout)
        self.assertIn('Skipped 1 linked worktree', result.stdout)
        self.assertIn('1/1 complete', result.stdout)
        self.assertEqual(self.git(self.repo, 'rev-parse', 'HEAD'), self.new)
        self.assertEqual((self.repo / '.git/fetch-count').read_text(), '1')
        self.assertEqual(self.git(linked, 'rev-parse', 'HEAD'), linked_head)
        self.assertEqual(self.git(linked, 'status', '--porcelain=v1'), linked_status)
        self.assertEqual((linked / 'file').read_text(), 'precious linked edits\n')
        self.assertEqual((linked / 'untracked').read_text(), 'precious untracked work\n')
        logs = Path(result.stdout.split('Logs: ', 1)[1].strip())
        self.assertEqual(json.loads((logs / 'excluded-worktrees.json').read_text()), [str(linked.resolve())])
        only_linked = self.run_home('--root', str(linked))
        self.assertEqual(only_linked.returncode, 0, only_linked.stdout)
        self.assertIn('No repositories to reset', only_linked.stdout)
        self.assertEqual((linked / 'file').read_text(), 'precious linked edits\n')

    def test_separate_git_directory_is_not_mistaken_for_linked_worktree(self):
        separate = self.workspace / 'separate'
        separate.mkdir()
        metadata = self.root / 'separate-metadata'
        self.git(separate, 'init', '-b', 'main', '--separate-git-dir', str(metadata))
        (separate / 'file').write_text('normal primary checkout\n')
        self.git(separate, 'add', '.')
        self.git(separate, 'commit', '-m', 'initial')
        excluded = set()
        found = discover(self.workspace, True, False, excluded)
        self.assertIn((separate.resolve(), metadata.resolve()), found)
        self.assertFalse(excluded)

    def test_discovery_groups_symlinks_and_preview_match(self):
        group = self.workspace / "group"
        group.mkdir()
        grouped = group / "repo"
        self.git(group, "clone", str(self.origin), str(grouped))
        (self.workspace / "alias").symlink_to(self.repo, target_is_directory=True)
        (group / "loop").symlink_to(self.workspace, target_is_directory=True)
        preview = self.run_home("--list")
        self.assertEqual(preview.returncode, 0, preview.stdout)
        self.assertEqual(len(preview.stdout.splitlines()), 2)
        result = self.run_home()
        self.assertEqual(result.returncode, 0, result.stdout)
        self.assertIn("2 succeeded", result.stdout)

    def test_submodule_worktree_is_not_recursively_reset(self):
        self.git(self.seed, "-c", "protocol.file.allow=always", "submodule", "add", str(self.origin), "module")
        self.git(self.seed, "commit", "-am", "add submodule")
        self.git(self.seed, "push", "origin", "main")
        self.git(self.repo, "fetch", "origin")
        self.git(self.repo, "reset", "--hard", "origin/main")
        self.git(self.repo, "-c", "protocol.file.allow=always", "submodule", "update", "--init")
        module = self.repo / "module"
        before = self.git(module, "rev-parse", "HEAD")
        (module / "ignored-data").write_text("precious")
        module_git = Path(self.git(module, "rev-parse", "--absolute-git-dir"))
        (module_git / "info/exclude").write_text("ignored-data\n")
        self.git(self.repo, "config", "submodule.recurse", "true")
        result = self.run_home()
        self.assertEqual(result.returncode, 0, result.stdout)
        self.assertEqual(self.git(module, "rev-parse", "HEAD"), before)
        self.assertEqual((module / "ignored-data").read_text(), "precious")

    def test_renamed_default_branch_and_explicit_remote_branch(self):
        self.git(self.seed, "switch", "-c", "trunk")
        self.git(self.seed, "push", "origin", "trunk")
        self.git(self.origin, "symbolic-ref", "HEAD", "refs/heads/trunk")
        self.git(self.seed, "push", "origin", "--delete", "main")
        result = self.run_home()
        self.assertEqual(result.returncode, 0, result.stdout)
        self.assertEqual(self.git(self.repo, "branch", "--show-current"), "trunk")
        self.assertEqual(self.git(self.repo, "rev-parse", "HEAD"), self.new)
        self.assertEqual(self.git(self.repo, "rev-parse", "main"), self.old)
        self.git(self.seed, "switch", "-c", "release/topic")
        self.git(self.seed, "push", "origin", "release/topic")
        self.git(self.repo, "remote", "add", "upstream", str(self.origin))
        result = self.run_home("--", "upstream", "release/topic")
        self.assertEqual(result.returncode, 0, result.stdout)
        self.assertEqual(self.git(self.repo, "branch", "--show-current"), "release/topic")

    def test_corrupt_discovery_fails_before_any_reset(self):
        invalid = self.workspace / "zz-invalid"
        invalid.mkdir()
        (invalid / ".git").write_text("invalid git directory")
        result = self.run_home()
        self.assertEqual(result.returncode, 1, result.stdout)
        self.assertEqual(self.git(self.repo, "rev-parse", "HEAD"), self.old)

    def test_post_checkout_hook_changes_are_preserved(self):
        hook = self.repo / ".git/hooks/post-checkout"
        hook.write_text("#!/bin/sh\nprintf 'hook changes' > file\n")
        hook.chmod(0o755)
        result = self.run_home()
        self.assertEqual(result.returncode, 1, result.stdout)
        self.assertEqual((self.repo / "file").read_text(), "hook changes")
        self.assertEqual(self.git(self.repo, "rev-parse", "HEAD"), self.old)

    def test_real_reset_preserves_branches_worktrees_and_backup(self):
        self.git(self.repo, "branch", "feature")
        linked = self.root / "linked"
        self.git(self.repo, "worktree", "add", str(linked), "feature")
        (linked / "uncommitted").write_text("keep")
        result = self.run_home()
        self.assertEqual(result.returncode, 0, result.stdout)
        self.assertEqual(self.git(self.repo, "rev-parse", "HEAD"), self.new)
        self.assertEqual(self.git(self.repo, "rev-parse", "feature"), self.old)
        self.assertEqual((linked / "uncommitted").read_text(), "keep")
        backups = self.git(self.repo, "for-each-ref", "--format=%(objectname)", "refs/home-reset-backups")
        self.assertIn(self.old, backups)
        self.assertIn("1 succeeded", result.stdout)
        self.assertNotIn("\x1b", result.stdout)

    def test_dirty_repo_fails_while_others_finish(self):
        other = self.clone("other")
        (self.repo / "file").write_text("local changes")
        result = self.run_home()
        self.assertEqual(result.returncode, 1)
        self.assertEqual((self.repo / "file").read_text(), "local changes")
        self.assertEqual(self.git(self.repo, "rev-parse", "HEAD"), self.old)
        self.assertEqual(self.git(other, "rev-parse", "HEAD"), self.new)
        self.assertIn("1 succeeded, 1 failed", result.stdout)

    def test_default_output_is_short_and_full_logs_remain_available(self):
        result = self.run_home()
        self.assertEqual(result.returncode, 0, result.stdout)
        self.assertIn('1/1 complete', result.stdout)
        self.assertNotIn('a repo', result.stdout)
        self.assertNotIn('RUNNING', result.stdout)
        self.assertNotIn('Attempt 1/3', result.stdout)
        self.assertNotIn(str(self.repo), result.stdout)
        self.assertNotIn('HEAD is now', result.stdout)
        directory = Path(result.stdout.split('Logs: ', 1)[1].strip())
        self.assertIn('HEAD is now', (directory / '0001.log').read_text())
        self.assertEqual(json.loads((directory / 'results.json').read_text())[0]['code'], 0)
        verbose = self.run_home('--verbose')
        self.assertEqual(verbose.returncode, 0, verbose.stdout)
        self.assertIn('Attempt 1/3', verbose.stdout)
        self.assertIn('HEAD is now', verbose.stdout)

    def test_untracked_and_ignored_collisions_refused(self):
        for ignored in (False, True):
            with self.subTest(ignored=ignored):
                self.git(self.repo, "reset", "--hard", self.old)
                (self.seed / "collision").write_text("remote")
                self.git(self.seed, "add", "collision")
                if self.git(self.seed, "status", "--porcelain"):
                    self.git(self.seed, "commit", "-m", "collision")
                    self.git(self.seed, "push", "origin", "main")
                (self.repo / "collision").write_text("precious")
                if ignored:
                    (self.repo / ".git/info/exclude").write_text("collision\n")
                result = self.run_home()
                self.assertEqual(result.returncode, 1, result.stdout)
                self.assertEqual((self.repo / "collision").read_text(), "precious")
                self.assertEqual(self.git(self.repo, "rev-parse", "HEAD"), self.old)
                self.assertIn("would be overwritten", result.stdout)

    def test_noncolliding_untracked_and_ignored_preserved(self):
        (self.repo / "notes").write_text("keep")
        (self.repo / "build").mkdir()
        (self.repo / "build/data").write_text("keep ignored")
        (self.repo / ".git/info/exclude").write_text("build/\n")
        result = self.run_home()
        self.assertEqual(result.returncode, 0, result.stdout)
        self.assertEqual((self.repo / "notes").read_text(), "keep")
        self.assertEqual((self.repo / "build/data").read_text(), "keep ignored")

    def test_switch_refuses_ignored_collision_with_local_branch(self):
        (self.repo / "old-only").write_text("local branch file")
        self.git(self.repo, "add", "old-only")
        self.git(self.repo, "commit", "-m", "local only")
        tip = self.git(self.repo, "rev-parse", "HEAD")
        self.git(self.repo, "switch", "-c", "feature", self.old)
        (self.repo / ".git/info/exclude").write_text("old-only\n")
        (self.repo / "old-only").write_text("precious ignored file")
        result = self.run_home()
        self.assertEqual(result.returncode, 1, result.stdout)
        self.assertEqual((self.repo / "old-only").read_text(), "precious ignored file")
        self.assertEqual(self.git(self.repo, "rev-parse", "main"), tip)
        self.assertEqual(self.git(self.repo, "branch", "--show-current"), "feature")

    def test_git_operation_and_existing_lock_preserved(self):
        (self.repo / ".git/MERGE_HEAD").write_text(self.old)
        result = self.run_home()
        self.assertEqual(result.returncode, 1, result.stdout)
        self.assertEqual(self.git(self.repo, "rev-parse", "HEAD"), self.old)
        (self.repo / ".git/MERGE_HEAD").unlink()
        (self.seed / "file").write_text("another update\n")
        self.git(self.seed, "commit", "-am", "another update")
        self.git(self.seed, "push", "origin", "main")
        lock = self.repo / ".git/refs/remotes/origin/main.lock"
        lock.write_text("active lock")
        result = self.run_home()
        self.assertEqual(result.returncode, 1, result.stdout)
        self.assertEqual(lock.read_text(), "active lock")
        self.assertEqual(self.git(self.repo, "rev-parse", "HEAD"), self.old)

    def test_linked_default_branch_refusal(self):
        self.git(self.repo, "switch", "-c", "feature")
        linked = self.root / "linked"
        self.git(self.repo, "worktree", "add", str(linked), "main")
        result = self.run_home()
        self.assertEqual(result.returncode, 1, result.stdout)
        self.assertEqual(self.git(self.repo, "branch", "--show-current"), "feature")
        self.assertEqual(self.git(linked, "rev-parse", "HEAD"), self.old)

    def test_discovery_nested_spaces_newlines_and_duplicate_roots(self):
        odd = self.clone("z\n odd")
        nested = self.repo / "nested"
        self.git(self.repo, "clone", str(self.origin), str(nested))
        self.assertEqual(len(discover(self.workspace, True, False)), 2)
        self.assertEqual(len(discover(self.workspace, True, True)), 3)
        result = self.run_home("--list")
        self.assertEqual(result.returncode, 0, result.stdout)
        self.assertIn("z\\x0a", result.stdout)
        self.assertEqual(self.git(self.repo, "rev-parse", "HEAD"), self.old)

    def test_invalid_options_do_not_mutate(self):
        for args in (("--jobs", "0"), ("--jobs", "-1"), ("--timeout", "0"), ("--retries", "bad"), ("--", "--prune"), ("--all-home",)):
            result = self.run_home(*args)
            self.assertEqual(result.returncode, 2, result.stdout)
            self.assertEqual(self.git(self.repo, "rev-parse", "HEAD"), self.old)


if __name__ == "__main__":
    unittest.main(verbosity=2)
