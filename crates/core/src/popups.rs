//! Rate limit for popups: pages open popups (`window.open`, `target=_blank`) as tabs, so a
//! page must not be able to flood the window with them.

use std::time::{Duration, Instant};

/// Popups one opener may open within [`POPUP_WINDOW`].
pub const POPUP_LIMIT: usize = 3;
/// The sliding time window of [`POPUP_LIMIT`].
pub const POPUP_WINDOW: Duration = Duration::from_secs(2);

/// Allows at most `limit` popups per opener within any `window` of time. The caller passes the
/// current time, so the clock is under its control.
#[derive(Debug)]
pub struct PopupThrottle<K> {
    limit: usize,
    window: Duration,
    /// Times of the popups each opener opened within the last `window`.
    recent: Vec<(K, Vec<Instant>)>,
}

impl<K> Default for PopupThrottle<K> {
    fn default() -> Self {
        Self::new(POPUP_LIMIT, POPUP_WINDOW)
    }
}

impl<K> PopupThrottle<K> {
    pub fn new(limit: usize, window: Duration) -> Self {
        Self {
            limit,
            window,
            recent: Vec::new(),
        }
    }
}

impl<K: PartialEq> PopupThrottle<K> {
    /// Whether `opener` may open a popup at `now`; an allowed popup counts toward the limit, a
    /// refused one does not.
    pub fn allow(&mut self, opener: K, now: Instant) -> bool {
        let index = match self.recent.iter().position(|(known, _)| *known == opener) {
            Some(index) => index,
            None => {
                self.recent.push((opener, Vec::new()));
                self.recent.len() - 1
            }
        };
        let times = &mut self.recent[index].1;
        let window = self.window;
        times.retain(|time| now.saturating_duration_since(*time) < window);
        let allowed = times.len() < self.limit;
        if allowed {
            times.push(now);
        }
        allowed
    }

    /// Forgets `opener`, for example once its browser has closed.
    pub fn forget(&mut self, opener: &K) {
        self.recent.retain(|(known, _)| known != opener);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    #[test]
    fn the_default_allows_three_popups_per_two_seconds() {
        let start = Instant::now();
        let mut throttle = PopupThrottle::default();
        assert!(throttle.allow(1, start));
        assert!(throttle.allow(1, start + ms(10)));
        assert!(throttle.allow(1, start + ms(20)));
        assert!(!throttle.allow(1, start + ms(30)));
        assert!(!throttle.allow(1, start + ms(1999)));
        assert!(throttle.allow(1, start + ms(2000)));
    }

    #[test]
    fn the_window_slides_with_each_popup() {
        let start = Instant::now();
        let mut throttle = PopupThrottle::new(2, ms(1000));
        assert!(throttle.allow(1, start));
        assert!(throttle.allow(1, start + ms(600)));
        assert!(!throttle.allow(1, start + ms(999)));
        // The first popup leaves the window; the second one is still in it.
        assert!(throttle.allow(1, start + ms(1000)));
        assert!(!throttle.allow(1, start + ms(1599)));
        assert!(throttle.allow(1, start + ms(1600)));
    }

    #[test]
    fn refused_popups_do_not_extend_the_wait() {
        let start = Instant::now();
        let mut throttle = PopupThrottle::new(1, ms(1000));
        assert!(throttle.allow(1, start));
        for millis in (100..1000).step_by(100) {
            assert!(!throttle.allow(1, start + ms(millis)), "{millis} ms");
        }
        assert!(throttle.allow(1, start + ms(1000)));
    }

    #[test]
    fn each_opener_has_its_own_limit() {
        let start = Instant::now();
        let mut throttle = PopupThrottle::new(1, ms(1000));
        assert!(throttle.allow(1, start));
        assert!(!throttle.allow(1, start));
        assert!(throttle.allow(2, start));
        assert!(!throttle.allow(2, start));
    }

    #[test]
    fn a_forgotten_opener_starts_over_and_others_keep_their_count() {
        let start = Instant::now();
        let mut throttle = PopupThrottle::new(1, ms(1000));
        assert!(throttle.allow(1, start));
        assert!(throttle.allow(2, start));
        throttle.forget(&1);
        assert!(throttle.allow(1, start + ms(1)));
        assert!(!throttle.allow(2, start + ms(1)));
        throttle.forget(&3);
    }

    #[test]
    fn a_zero_limit_refuses_every_popup() {
        let mut throttle = PopupThrottle::new(0, ms(1000));
        assert!(!throttle.allow(1, Instant::now()));
    }
}
