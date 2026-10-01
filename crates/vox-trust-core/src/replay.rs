//! Verifier-side helpers: replay detection and failure rate limiting (draft spec, section 6).
//!
//! These are policy helpers, not cryptography. They take the clock as an argument so they
//! are deterministic and testable.
//!
//! Counters are 16 bits and wrap, so comparisons use serial-number arithmetic (RFC 1982):
//! a counter is "newer" than the last accepted one if it is ahead by 1 to 32767 steps.

use std::collections::{HashMap, VecDeque};

/// Result of a replay check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayVerdict {
    /// First seal seen for this key, or a counter newer than the last accepted one.
    Fresh,
    /// The counter equals or is behind the last accepted one for this key.
    Replay,
    /// The coarse time is outside the allowed clock skew.
    Stale,
}

/// Tracks the last accepted counter per key.
#[derive(Debug, Clone, Default)]
pub struct ReplayGuard {
    skew_minutes: u16,
    last: HashMap<u32, u16>,
}

/// `true` if `a` is strictly after `b` in 16-bit serial-number arithmetic.
pub fn serial_after(a: u16, b: u16) -> bool {
    let diff = a.wrapping_sub(b);
    diff != 0 && diff < 0x8000
}

/// Absolute distance between two 16-bit minute stamps on the wrap-around circle.
pub fn minute_distance(a: u16, b: u16) -> u16 {
    let d = a.wrapping_sub(b);
    d.min(d.wrapping_neg())
}

impl ReplayGuard {
    /// A guard that tolerates `skew_minutes` of clock difference when checking time.
    pub fn new(skew_minutes: u16) -> Self {
        ReplayGuard {
            skew_minutes,
            last: HashMap::new(),
        }
    }

    /// Checks a seal and, if it is fresh, records it.
    ///
    /// Pass `now_minutes = None` for recorded audio, where an old seal is expected and only
    /// the counter is checked. For live audio pass the verifier's clock in minutes, modulo 2^16.
    ///
    /// **Call this only after the seal's authenticity has been verified.** Recording the
    /// counter of an unauthenticated seal would let an attacker lock out the real signer.
    pub fn check(
        &mut self,
        key_id: u32,
        counter: u16,
        time: u16,
        now_minutes: Option<u16>,
    ) -> ReplayVerdict {
        if let Some(now) = now_minutes {
            if minute_distance(time, now) > self.skew_minutes {
                return ReplayVerdict::Stale;
            }
        }
        match self.last.get(&key_id) {
            Some(&last) if !serial_after(counter, last) => ReplayVerdict::Replay,
            _ => {
                self.last.insert(key_id, counter);
                ReplayVerdict::Fresh
            }
        }
    }
}

/// Limits how many verification failures are tolerated within a time window.
///
/// A 32-bit tag can be guessed by brute force against an online verifier. Refusing to keep
/// answering after repeated failures makes that impractical.
#[derive(Debug, Clone)]
pub struct FailureLimiter {
    max_failures: u32,
    window_secs: u64,
    failures: VecDeque<u64>,
}

impl FailureLimiter {
    /// Allow at most `max_failures` failures within any `window_secs` seconds.
    pub fn new(max_failures: u32, window_secs: u64) -> Self {
        FailureLimiter {
            max_failures,
            window_secs,
            failures: VecDeque::new(),
        }
    }

    fn expire(&mut self, now: u64) {
        while let Some(&oldest) = self.failures.front() {
            if now.saturating_sub(oldest) >= self.window_secs {
                self.failures.pop_front();
            } else {
                break;
            }
        }
    }

    /// Whether another verification attempt is currently allowed.
    pub fn allow(&mut self, now: u64) -> bool {
        self.expire(now);
        (self.failures.len() as u64) < u64::from(self.max_failures)
    }

    /// Records a failed verification at time `now` (seconds).
    pub fn record_failure(&mut self, now: u64) {
        self.expire(now);
        self.failures.push_back(now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serial_arithmetic_handles_wrap_around() {
        assert!(serial_after(2, 1));
        assert!(!serial_after(1, 1));
        assert!(!serial_after(1, 2));
        assert!(serial_after(0, 65535)); // wrapped forward by one
        assert!(!serial_after(65535, 0));
        assert!(serial_after(32768 + 5, 6)); // 32767 steps ahead
        assert!(!serial_after(32768 + 6, 6)); // 32768 steps: ambiguous, treated as not after
    }

    #[test]
    fn minute_distance_wraps() {
        assert_eq!(minute_distance(10, 12), 2);
        assert_eq!(minute_distance(0, 65535), 1);
        assert_eq!(minute_distance(65535, 1), 2);
        assert_eq!(minute_distance(5, 5), 0);
    }

    #[test]
    fn counters_must_move_forward_per_key() {
        let mut g = ReplayGuard::new(5);
        assert_eq!(g.check(1, 10, 0, None), ReplayVerdict::Fresh);
        assert_eq!(g.check(1, 10, 0, None), ReplayVerdict::Replay);
        assert_eq!(g.check(1, 9, 0, None), ReplayVerdict::Replay);
        assert_eq!(g.check(1, 11, 0, None), ReplayVerdict::Fresh);
        // other keys are independent
        assert_eq!(g.check(2, 3, 0, None), ReplayVerdict::Fresh);
        // wrap-around still counts as forward
        assert_eq!(g.check(3, 65535, 0, None), ReplayVerdict::Fresh);
        assert_eq!(g.check(3, 0, 0, None), ReplayVerdict::Fresh);
        assert_eq!(g.check(3, 65535, 0, None), ReplayVerdict::Replay);
    }

    #[test]
    fn live_audio_rejects_stale_time_without_recording_the_counter() {
        let mut g = ReplayGuard::new(2);
        assert_eq!(g.check(1, 5, 100, Some(101)), ReplayVerdict::Fresh);
        // a stale seal is rejected and does not advance the counter
        assert_eq!(g.check(1, 6, 50, Some(101)), ReplayVerdict::Stale);
        assert_eq!(g.check(1, 6, 101, Some(101)), ReplayVerdict::Fresh);
        // the minute clock wraps
        assert_eq!(g.check(2, 1, 65535, Some(1)), ReplayVerdict::Fresh);
    }

    #[test]
    fn limiter_blocks_after_too_many_failures_and_recovers() {
        let mut l = FailureLimiter::new(3, 60);
        assert!(l.allow(0));
        for t in [0, 1, 2] {
            assert!(l.allow(t));
            l.record_failure(t);
        }
        assert!(!l.allow(3));
        assert!(!l.allow(59));
        assert!(l.allow(61)); // the failure at t=0 and t=1 aged out, one remains
        assert!(l.allow(70));
    }

    #[test]
    fn limiter_with_zero_budget_always_refuses() {
        let mut l = FailureLimiter::new(0, 60);
        assert!(!l.allow(0));
    }
}
