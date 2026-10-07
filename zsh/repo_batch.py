"""Bounded subprocess execution with resource locks, deadlines, and retry records."""

from concurrent.futures import ThreadPoolExecutor, as_completed
from dataclasses import dataclass
import fcntl
import os
from pathlib import Path
import random
import signal
import subprocess
import threading
import time


@dataclass(frozen=True)
class Job:
    name: str
    cwd: Path
    command: tuple
    resource: Path


@dataclass(frozen=True)
class Result:
    name: str
    code: int
    attempts: int
    seconds: float
    log: Path


class Supervisor:
    def __init__(self, jobs, attempts, timeout, delay, log_dir, retryable, env=None, progress=None):
        self.jobs = jobs
        self.attempts = attempts
        self.timeout = timeout
        self.delay = delay
        self.log_dir = Path(log_dir)
        self.retryable = retryable
        self.env = env
        self.progress = progress or (lambda job, attempt, message: None)
        self.cancelled = threading.Event()
        self.guard = threading.Lock()
        self.resources = {}
        self.next_index = 0

    @staticmethod
    def terminate(process):
        # A deadline is a hard stop for the complete worker process group.
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        process.wait()

    def cancel(self):
        self.cancelled.set()

    def run(self, index, job):
        started = time.monotonic()
        log = self.log_dir / f"{index + 1:04d}.log"
        code, attempts = 130, 0
        with self.guard:
            resource_guard = self.resources.setdefault(job.resource, threading.Lock())
        while not resource_guard.acquire(timeout=0.05):
            if self.cancelled.is_set():
                log.write_text("Cancelled before execution.\n")
                return Result(job.name, 130, 0, time.monotonic() - started, log)
        try:
            with log.open("w") as output:
                # Never unlink the lock: another invocation may hold its inode.
                with (job.resource / "repo-batch.lock").open("a") as lock:
                    try:
                        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
                    except BlockingIOError:
                        output.write("Repository is busy in another batch invocation.\n")
                        return Result(job.name, 75, 0, time.monotonic() - started, log)
                    while attempts < self.attempts and not self.cancelled.is_set():
                        attempts += 1
                        self.progress(job, attempts, "RUNNING")
                        output.write(f"Attempt {attempts}/{self.attempts}\n")
                        output.flush()
                        offset = output.tell()
                        process = subprocess.Popen(
                            job.command, cwd=job.cwd, env=self.env, stdin=subprocess.DEVNULL,
                            stdout=output, stderr=subprocess.STDOUT, start_new_session=True,
                            pass_fds=(lock.fileno(),),
                        )
                        deadline = time.monotonic() + self.timeout
                        try:
                            while process.poll() is None:
                                if self.cancelled.is_set() or time.monotonic() >= deadline:
                                    code = 130 if self.cancelled.is_set() else 124
                                    output.write("Cancelled.\n" if code == 130 else "Attempt timed out.\n")
                                    break
                                self.cancelled.wait(0.05)
                            else:
                                code = process.returncode
                        finally:
                            self.terminate(process)
                        output.flush()
                        with log.open(errors="replace") as recorded:
                            recorded.seek(offset)
                            attempt_log = recorded.read()
                        if code == 0 or code == 130 or attempts == self.attempts:
                            break
                        if code != 124 and not self.retryable(attempt_log):
                            break
                        delay = min(30, self.delay * 2 ** min(attempts - 1, 10))
                        delay += random.uniform(0, min(1, delay / 4))
                        output.write(f"Temporary failure; retrying in {delay:.1f}s.\n")
                        self.progress(job, attempts, f"RETRY in {delay:.1f}s")
                        output.flush()
                        if self.cancelled.wait(delay):
                            code = 130
                            break
                    if self.cancelled.is_set():
                        code = 130
        except Exception as error:
            with log.open("a") as output:
                output.write(f"Worker failure: {error}\n")
            code = 1
        finally:
            resource_guard.release()
        return Result(job.name, code, attempts, time.monotonic() - started, log)

    def batch(self, jobs, completed=lambda result: None):
        results = [None] * len(jobs)
        offset = self.next_index
        self.next_index += len(jobs)
        with ThreadPoolExecutor(max_workers=self.jobs) as executor:
            futures = {executor.submit(self.run, offset + i, job): i for i, job in enumerate(jobs)}
            try:
                for future in as_completed(futures):
                    result = future.result()
                    results[futures[future]] = result
                    completed(result)
            except BaseException:
                self.cancel()
                raise
        return results
