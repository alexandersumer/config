"""Line-oriented workspace progress and immediate failures."""

import os
import shutil
import sys
import threading
import textwrap
import time


def color(text, kind, stream=None):
    stream = stream if stream is not None else sys.stdout
    if not stream.isatty() or os.environ.get("NO_COLOR") or os.environ.get("TERM") == "dumb":
        return text
    code = {"green": 32, "red": 31, "yellow": 33}[kind]
    return f"\033[{code}m{text}\033[0m"


def duration(seconds):
    if seconds < 60:
        return f"{seconds:.1f}s"
    minutes, seconds = divmod(int(seconds), 60)
    hours, minutes = divmod(minutes, 60)
    return f"{hours}h {minutes}m {seconds}s" if hours else f"{minutes}m {seconds}s"


def quantity(count, singular, plural=None):
    return f"{count} {singular if count == 1 else plural or singular + 's'}"


class BatchOutput:
    def __init__(self, names, name, failure, stream=None, interval=15, on_error=None):
        self.names = names
        self.name = name
        self.failure = failure
        self.stream = stream if stream is not None else sys.stdout
        self.interval = interval
        self.on_error = on_error or (lambda: None)
        self.error = None
        self.guard = threading.Lock()
        self.stopped = threading.Event()
        self.results = {}
        self.active = {}
        self.started = self.last_update = time.monotonic()
        self.last_count = 0
        self.reported_waits = set()

    def write(self, line):
        print(line, file=self.stream, flush=True)

    def status(self):
        failed = sum(r.code not in (0, 130) for r in self.results.values())
        cancelled = sum(r.code == 130 for r in self.results.values())
        counts = [f"{len(self.results)}/{len(self.names)} complete"]
        if failed:
            counts.append(f"{failed} failed")
        if cancelled:
            counts.append(f"{cancelled} cancelled")
        counts.append("elapsed " + duration(time.monotonic() - self.started))
        return "  " + ", ".join(counts)

    def progress(self, job, attempt, message):
        with self.guard:
            self.active.setdefault(job.name, time.monotonic())

    def completed(self, result):
        with self.guard:
            if self.error is not None:
                raise self.error
            self.active.pop(result.name, None)
            self.results[result.name] = result
            if result.code not in (0, 130):
                self.write("  " + color("failed:", "red", self.stream) + " " + self.name(result.name))
                for line in textwrap.wrap(self.failure(result), max(30, shutil.get_terminal_size().columns - 10), break_long_words=False, break_on_hyphens=False):
                    self.write("          " + line)

    def heartbeat(self):
        try:
            self.update_progress()
        except Exception as error:
            self.error = error
            self.on_error()

    def update_progress(self):
        while not self.stopped.wait(min(1, self.interval)):
            with self.guard:
                now = time.monotonic()
                if now - self.last_update >= self.interval:
                    if len(self.results) != self.last_count:
                        self.write(self.status())
                        self.last_count = len(self.results)
                    elif self.active:
                        name = min(self.active, key=self.active.get)
                        if name not in self.reported_waits:
                            self.write(f"  Waiting for {self.name(name)} ({duration(now - self.active[name])}).")
                            self.reported_waits.add(name)
                    self.last_update = now

    def __enter__(self):
        self.thread = threading.Thread(target=self.heartbeat, daemon=True)
        self.thread.start()
        return self

    def __exit__(self, *error):
        self.stopped.set()
        self.thread.join()
        if error[0] is None:
            if self.error is not None:
                raise self.error
            failed = any(r.code not in (0, 130) for r in self.results.values())
            cancelled = any(r.code == 130 for r in self.results.values())
            kind = "red" if failed else "yellow" if cancelled else "green"
            self.write(color(self.status(), kind, self.stream))
        self.stream.flush()
