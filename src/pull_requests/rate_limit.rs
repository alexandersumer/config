//! One cooldown and wait budget for all Bitbucket requests in an invocation.
use super::MAX_ATTEMPTS;
use crate::runtime;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime};

const WAIT_BUDGET: Duration = Duration::from_secs(60);

#[derive(Default)]
struct State {
    resume_at: Option<Instant>,
    reserved_wait: Duration,
    stop_reason: Option<String>,
}

#[derive(Default)]
pub(super) struct RateLimit(Mutex<State>);

impl RateLimit {
    pub(super) fn wait(&self) -> Result<bool, String> {
        let mut waited = false;
        loop {
            if runtime::cancelled() {
                return Err("Interrupted before Bitbucket request".into());
            }
            let remaining = {
                let state = self.0.lock().expect("rate-limit state");
                if let Some(reason) = &state.stop_reason {
                    return Err(reason.clone());
                }
                state
                    .resume_at
                    .map(|time| time.saturating_duration_since(Instant::now()))
            };
            match remaining {
                Some(wait) if !wait.is_zero() => {
                    waited = true;
                    std::thread::sleep(wait.min(Duration::from_millis(25)));
                }
                _ => return Ok(waited),
            }
        }
    }

    pub(super) fn limited(&self, header: &str, attempt: u32, human: bool) {
        let mut state = self.0.lock().expect("rate-limit state");
        if state.stop_reason.is_some() {
            return;
        }
        let delay = retry_after(header, SystemTime::now())
            .unwrap_or_else(|| Duration::from_secs(1 << attempt))
            .max(Duration::from_millis(250));
        let now = Instant::now();
        let previous = state.resume_at.unwrap_or(now).max(now);
        let Some(resume) = now.checked_add(delay) else {
            state.stop_reason =
                Some("Bitbucket rate limit requested a wait beyond the cooldown budget".into());
            return;
        };
        let added = resume.saturating_duration_since(previous);
        state.reserved_wait = state.reserved_wait.saturating_add(added);
        if state.reserved_wait > WAIT_BUDGET {
            state.stop_reason = Some(format!(
                "Bitbucket rate limit requested a {} cooldown beyond the remaining wait budget",
                crate::presentation::duration(delay)
            ));
        } else if attempt + 1 >= MAX_ATTEMPTS {
            state.stop_reason = Some("Bitbucket rate limit exhausted the retry budget".into());
        } else if !added.is_zero() {
            state.resume_at = Some(resume);
            if human {
                use std::io::Write;
                let _ = writeln!(
                    std::io::stderr().lock(),
                    "Bitbucket rate limit reached. Waiting {} before continuing.",
                    crate::presentation::duration(delay)
                );
            }
        }
    }

    pub(super) fn paused(&self) -> bool {
        self.0
            .lock()
            .expect("rate-limit state")
            .stop_reason
            .is_some()
    }

    pub(super) fn stop_reason(&self) -> Option<String> {
        self.0.lock().expect("rate-limit state").stop_reason.clone()
    }
}

fn retry_after(value: &str, now: SystemTime) -> Option<Duration> {
    let value = value.trim();
    if !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()) {
        // Overflow represents an unserviceably long provider delay.
        return Some(Duration::from_secs(value.parse().unwrap_or(u64::MAX)));
    }
    httpdate::parse_http_date(value)
        .ok()
        .map(|date| date.duration_since(now).unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_seconds_and_http_dates_without_shortening_large_delays() {
        let now = httpdate::parse_http_date("Sat, 21 Nov 2026 05:00:00 GMT").unwrap();
        assert_eq!(retry_after(" 12 ", now), Some(Duration::from_secs(12)));
        assert_eq!(
            retry_after("Sat, 21 Nov 2026 05:00:09 GMT", now),
            Some(Duration::from_secs(9))
        );
        assert_eq!(
            retry_after("Sat, 21 Nov 2026 04:59:00 GMT", now),
            Some(Duration::ZERO)
        );
        assert_eq!(
            retry_after("99999999999999999999999", now),
            Some(Duration::from_secs(u64::MAX))
        );
        assert_eq!(retry_after("invalid", now), None);
    }
}
