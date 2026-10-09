//! Shared output conventions; no Git execution or repository policy.
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use std::io::{self, IsTerminal, Write};
use std::time::{Duration, Instant};

pub(crate) fn safe_text(text: &str) -> String {
    text.chars()
        .flat_map(|c| {
            if c.is_control() && c != '\n' && c != '\t' {
                format!("\\u{{{:x}}}", c as u32).chars().collect::<Vec<_>>()
            } else {
                vec![c]
            }
        })
        .collect()
}
pub(crate) fn json(result: &serde_json::Value) -> u8 {
    let mut out = io::stdout().lock();
    u8::from(
        serde_json::to_writer(&mut out, result).is_err()
            || writeln!(out).is_err()
            || out.flush().is_err(),
    )
}
pub(crate) fn duration(duration: Duration) -> String {
    let s = duration.as_secs();
    if s >= 3600 {
        format!("{}h {}m {}s", s / 3600, s % 3600 / 60, s % 60)
    } else if s >= 60 {
        format!("{}m {}s", s / 60, s % 60)
    } else {
        format!("{s}s")
    }
}
/// Selection has no known total yet, but must be visible before Git probes finish.
pub(crate) struct Discovery {
    bar: ProgressBar,
    last: Instant,
    start: Instant,
}
impl Discovery {
    pub fn new(kind: &str) -> Result<Self, String> {
        writeln!(io::stderr(), "Discovering {kind}…").map_err(|e| e.to_string())?;
        let interactive =
            io::stderr().is_terminal() && std::env::var_os("TERM").is_none_or(|v| v != "dumb");
        let bar = ProgressBar::with_draw_target(
            None,
            if interactive {
                ProgressDrawTarget::stderr()
            } else {
                ProgressDrawTarget::hidden()
            },
        );
        bar.set_style(
            ProgressStyle::with_template("  Discovery in progress · elapsed {human_elapsed}")
                .expect("static discovery template")
                .with_key(
                    "human_elapsed",
                    |state: &indicatif::ProgressState, writer: &mut dyn std::fmt::Write| {
                        let _ = write!(writer, "{}", duration(state.elapsed()));
                    },
                ),
        );
        bar.enable_steady_tick(Duration::from_millis(100));
        Ok(Self {
            bar,
            last: Instant::now(),
            start: Instant::now(),
        })
    }
    pub fn update(&mut self) -> Result<(), String> {
        if self.bar.is_hidden() && self.last.elapsed() >= Duration::from_secs(15) {
            writeln!(
                io::stderr(),
                "  Discovery in progress · elapsed {}",
                duration(self.start.elapsed())
            )
            .map_err(|e| e.to_string())?;
            self.last = Instant::now();
        }
        Ok(())
    }
}
impl Drop for Discovery {
    fn drop(&mut self) {
        self.bar.finish_and_clear();
    }
}

pub(crate) struct Progress {
    bar: ProgressBar,
    last: Instant,
    start: Instant,
    total: usize,
}
impl Progress {
    pub fn new(total: usize) -> Self {
        let interactive =
            io::stderr().is_terminal() && std::env::var_os("TERM").is_none_or(|v| v != "dumb");
        let bar = ProgressBar::with_draw_target(
            Some(total as u64),
            if interactive {
                ProgressDrawTarget::stderr()
            } else {
                ProgressDrawTarget::hidden()
            },
        );
        Self::with_bar(total, bar)
    }
    fn with_bar(total: usize, bar: ProgressBar) -> Self {
        bar.set_style(
            ProgressStyle::with_template(
                "  {pos}/{len} complete · {msg} active · elapsed {human_elapsed}",
            )
            .expect("static progress template")
            .with_key(
                "human_elapsed",
                |state: &indicatif::ProgressState, writer: &mut dyn std::fmt::Write| {
                    let _ = write!(writer, "{}", duration(state.elapsed()));
                },
            ),
        );
        bar.enable_steady_tick(Duration::from_millis(100));
        Self {
            bar,
            last: Instant::now(),
            start: Instant::now(),
            total,
        }
    }
    pub fn update(&mut self, complete: usize, active: usize) -> Result<(), String> {
        let text = format!(
            "{complete}/{} complete · {active} active · elapsed {}",
            self.total,
            duration(self.start.elapsed())
        );
        self.bar.set_message(active.to_string());
        self.bar.set_position(complete as u64);
        if self.bar.is_hidden() && self.last.elapsed() >= Duration::from_secs(15) {
            writeln!(io::stderr(), "  {text}").map_err(|e| e.to_string())?;
            self.last = Instant::now();
        }
        Ok(())
    }
    pub fn message(&self, text: &str) -> Result<(), String> {
        if self.bar.is_hidden() {
            writeln!(io::stderr(), "{text}").map_err(|e| e.to_string())
        } else {
            let mut result = Ok(());
            self.bar
                .suspend(|| result = writeln!(io::stderr(), "{text}").map_err(|e| e.to_string()));
            result
        }
    }
    pub fn result(&self, text: &str) -> Result<(), String> {
        let mut result = Ok(());
        self.bar.suspend(|| {
            result = writeln!(io::stdout(), "{text}")
                .and_then(|_| io::stdout().flush())
                .map_err(|e| e.to_string());
        });
        result
    }
    pub fn finish(&self) {
        self.bar.finish_and_clear();
    }
}
impl Drop for Progress {
    fn drop(&mut self) {
        self.finish();
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn machine_paths_preserve_invalid_utf8_bytes() {
        use std::os::unix::ffi::OsStringExt;
        let p = std::path::PathBuf::from(std::ffi::OsString::from_vec(b"/repo-\xff".to_vec()));
        let result = serde_json::json!({"path":paths([&p])[0],"path_bytes":path_bytes(&p)});
        let wire = serde_json::to_vec(&result).unwrap();
        let decoded: serde_json::Value = serde_json::from_slice(&wire).unwrap();
        assert_eq!(decoded["path_bytes"][6], 255);
    }
    #[test]
    fn durations_are_readable() {
        assert_eq!(duration(Duration::from_secs(257)), "4m 17s");
    }
    #[test]
    fn interactive_progress_wraps_and_clears_at_narrow_and_normal_widths() {
        for width in [28, 80] {
            let term = indicatif::InMemoryTerm::new(12, width);
            let bar = ProgressBar::with_draw_target(
                Some(143),
                ProgressDrawTarget::term_like(Box::new(term.clone())),
            );
            let mut progress = Progress::with_bar(143, bar);
            progress.update(42, 8).unwrap();
            progress.bar.force_draw();
            let screen = term.contents().replace('\n', "");
            assert!(screen.contains("42/143 complete"), "{screen}");
            assert!(screen.contains("8 active"), "{screen}");
            progress.finish();
            assert!(term.contents().trim().is_empty());
        }
    }
    #[test]
    fn redirected_progress_is_periodic_and_reports_active_work() {
        let bar = ProgressBar::with_draw_target(Some(3), ProgressDrawTarget::hidden());
        let mut p = Progress::with_bar(3, bar);
        p.last = Instant::now() - Duration::from_secs(16);
        p.update(1, 2).unwrap();
        assert!(p.last.elapsed() < Duration::from_secs(1));
    }
}

pub(crate) fn paths<'a>(paths: impl IntoIterator<Item = &'a std::path::PathBuf>) -> Vec<String> {
    paths
        .into_iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect()
}
pub(crate) fn path_bytes(path: &std::path::Path) -> Vec<u8> {
    path.as_os_str().as_encoded_bytes().to_vec()
}

pub(crate) fn status(text: &str, failed: bool, stderr: bool) -> String {
    let terminal = if stderr {
        io::stderr().is_terminal()
    } else {
        io::stdout().is_terminal()
    };
    let enabled = terminal
        && std::env::var_os("NO_COLOR").is_none()
        && std::env::var_os("TERM").is_none_or(|v| v != "dumb")
        && std::env::var_os("CLICOLOR").is_none_or(|v| v != "0");
    let style = if failed {
        console::Style::new().red()
    } else {
        console::Style::new().green()
    };
    style.force_styling(enabled).apply_to(text).to_string()
}
