"""Plain, ordered batch results with occasional progress during long waits."""

import sys
import shutil
import threading
import textwrap
import time


def duration(seconds):
    if seconds < 60:
        return f"{seconds:.1f}s"
    minutes, seconds = divmod(int(seconds), 60)
    hours, minutes = divmod(minutes, 60)
    return f"{hours}h {minutes}m {seconds}s" if hours else f"{minutes}m {seconds}s"


class BatchOutput:
    def __init__(self, names, name, failure, stream=None, interval=15):
        self.names = names
        self.name = name
        self.failure = failure
        self.stream = stream if stream is not None else sys.stdout
        self.interval = interval
        self.guard = threading.Lock()
        self.stopped = threading.Event()
        self.results = {}
        self.active = set()
        self.next_row = 0
        self.last_output = time.monotonic()

    def write(self, line):
        print(line, file=self.stream, flush=True)
        self.last_output = time.monotonic()

    def progress(self, job, attempt, message):
        with self.guard:
            self.active.add(job.name)

    def completed(self, result):
        with self.guard:
            self.active.discard(result.name)
            self.results[result.name] = result
            while self.next_row < len(self.names):
                result = self.results.get(self.names[self.next_row])
                if result is None:
                    break
                self.next_row += 1
                label = "ok" if result.code == 0 else "cancelled" if result.code == 130 else "failed"
                prefix = f"  {self.next_row:>{len(str(len(self.names)))}}/{len(self.names)}  {label:<9}  "
                attempts = f" ({result.attempts} attempts)" if result.attempts > 1 else ""
                self.write(prefix + self.name(result.name) + attempts)
                if result.code not in (0, 130):
                    indent = " " * len(prefix)
                    width = max(30, shutil.get_terminal_size().columns - len(indent))
                    for line in textwrap.wrap(self.failure(result), width, break_long_words=False, break_on_hyphens=False):
                        self.write(indent + line)
                    self.write(indent + f"Log: {result.log.name}")

    def heartbeat(self):
        while not self.stopped.wait(min(1, self.interval)):
            with self.guard:
                if self.active and time.monotonic() - self.last_output >= self.interval:
                    names = ", ".join(self.name(n) for n in self.names if n in self.active)
                    self.write(f"  Progress: {len(self.results)}/{len(self.names)} complete. Waiting for: {names}")

    def __enter__(self):
        self.thread = threading.Thread(target=self.heartbeat, daemon=True)
        self.thread.start()
        return self

    def __exit__(self, *error):
        self.stopped.set()
        self.thread.join()
