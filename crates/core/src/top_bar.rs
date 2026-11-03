//! When the top bar shows. Pinned, always. In auto mode, while the pointer is at the top edge of
//! the window or over the shown bar, for a moment after it leaves, and for a moment after the
//! tabs change. The caller passes the current time, so the clock is under its control.

use std::time::{Duration, Instant};

use crate::settings::TopBarMode;

/// How long the bar stays after the pointer leaves it.
pub const HIDE_DELAY: Duration = Duration::from_millis(500);
/// How long the bar shows after the tabs change.
pub const FLASH: Duration = Duration::from_millis(1500);
/// The smallest band at the top of the window that reveals the bar, for thin padding frames.
pub const REVEAL_MIN: i32 = 6;

/// Where the pointer is, relative to the top bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pointer {
    /// Outside the window, below the bar's area, or unknown.
    Away,
    /// At the top edge of the window, which reveals the bar.
    Edge,
    /// Over the bar's area below the top edge, which keeps a shown bar but does not reveal it.
    Bar,
}

/// Where `point` (window coordinates; `None` when the pointer is not over the window) is in a
/// window `width` pixels wide whose padding frame is `padding` pixels and whose bar, right
/// below the frame's top side, is `bar_height` pixels tall. The top edge is the frame's top
/// side, at least [`REVEAL_MIN`] pixels. When the screen cuts off the top `cut` pixels of the
/// window (a maximized window on Windows), the top edge is measured from the screen's edge.
pub fn pointer_zone(
    point: Option<(i32, i32)>,
    width: i32,
    cut: i32,
    padding: i32,
    bar_height: i32,
) -> Pointer {
    let Some((_, y)) = point.filter(|&(x, y)| (0..width).contains(&x) && y >= 0) else {
        return Pointer::Away;
    };
    if y < padding.max(cut.max(0) + REVEAL_MIN) {
        Pointer::Edge
    } else if y < padding + bar_height {
        Pointer::Bar
    } else {
        Pointer::Away
    }
}

/// Whether the auto-mode bar is shown; see the module documentation.
#[derive(Debug, Default)]
pub struct TopBar {
    /// Until when the bar shows without the pointer on it.
    shown_until: Option<Instant>,
}

impl TopBar {
    /// Shows the bar for [`FLASH`] from `now`, unless it already shows longer.
    pub fn flash(&mut self, now: Instant) {
        self.show_until(now + FLASH);
    }

    /// Whether the bar shows at `now` in `mode` with the pointer at `pointer`.
    pub fn update(&mut self, mode: TopBarMode, pointer: Pointer, now: Instant) -> bool {
        if mode == TopBarMode::Pinned {
            return true;
        }
        let hovered = match pointer {
            Pointer::Edge => true,
            Pointer::Bar => self.is_shown(now),
            Pointer::Away => false,
        };
        if hovered {
            self.show_until(now + HIDE_DELAY);
        }
        self.is_shown(now)
    }

    fn is_shown(&self, now: Instant) -> bool {
        self.shown_until.is_some_and(|until| now < until)
    }

    fn show_until(&mut self, until: Instant) {
        self.shown_until = Some(self.shown_until.map_or(until, |current| current.max(until)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    #[test]
    fn the_top_edge_is_the_frames_top_side() {
        assert_eq!(pointer_zone(Some((0, 0)), 1280, 0, 8, 28), Pointer::Edge);
        assert_eq!(pointer_zone(Some((640, 7)), 1280, 0, 8, 28), Pointer::Edge);
        assert_eq!(pointer_zone(Some((1279, 7)), 1280, 0, 8, 28), Pointer::Edge);
        assert_eq!(pointer_zone(Some((640, 8)), 1280, 0, 8, 28), Pointer::Bar);
    }

    #[test]
    fn a_thin_frame_still_has_a_reachable_top_edge() {
        assert_eq!(pointer_zone(Some((640, 5)), 1280, 0, 4, 28), Pointer::Edge);
        assert_eq!(pointer_zone(Some((640, 6)), 1280, 0, 4, 28), Pointer::Bar);
    }

    #[test]
    fn the_bar_area_ends_below_the_bar() {
        assert_eq!(pointer_zone(Some((640, 35)), 1280, 0, 8, 28), Pointer::Bar);
        assert_eq!(pointer_zone(Some((640, 36)), 1280, 0, 8, 28), Pointer::Away);
        assert_eq!(
            pointer_zone(Some((640, 500)), 1280, 0, 8, 28),
            Pointer::Away
        );
    }

    #[test]
    fn a_window_cut_off_by_the_screen_reveals_from_the_screens_edge() {
        // Maximized on Windows: the top 8 px are above the screen, so the pointer can only
        // reach y = 8.
        assert_eq!(pointer_zone(Some((640, 8)), 1280, 8, 8, 28), Pointer::Edge);
        assert_eq!(pointer_zone(Some((640, 13)), 1280, 8, 8, 28), Pointer::Edge);
        assert_eq!(pointer_zone(Some((640, 14)), 1280, 8, 8, 28), Pointer::Bar);
        assert_eq!(pointer_zone(Some((640, 35)), 1280, 8, 8, 28), Pointer::Bar);
        assert_eq!(pointer_zone(Some((640, 36)), 1280, 8, 8, 28), Pointer::Away);
        // A wide frame keeps its whole visible top side as the edge.
        assert_eq!(
            pointer_zone(Some((640, 19)), 1280, 8, 20, 28),
            Pointer::Edge
        );
        assert_eq!(pointer_zone(Some((640, 20)), 1280, 8, 20, 28), Pointer::Bar);
    }

    #[test]
    fn a_pointer_outside_the_window_is_away() {
        assert_eq!(pointer_zone(None, 1280, 0, 8, 28), Pointer::Away);
        assert_eq!(pointer_zone(Some((640, -1)), 1280, 0, 8, 28), Pointer::Away);
        assert_eq!(pointer_zone(Some((-1, 0)), 1280, 0, 8, 28), Pointer::Away);
        assert_eq!(pointer_zone(Some((1280, 0)), 1280, 0, 8, 28), Pointer::Away);
    }

    #[test]
    fn a_pinned_bar_always_shows() {
        let now = Instant::now();
        let mut bar = TopBar::default();
        for pointer in [Pointer::Away, Pointer::Edge, Pointer::Bar] {
            assert!(bar.update(TopBarMode::Pinned, pointer, now), "{pointer:?}");
        }
    }

    #[test]
    fn an_auto_bar_starts_hidden() {
        let now = Instant::now();
        let mut bar = TopBar::default();
        assert!(!bar.update(TopBarMode::Auto, Pointer::Away, now));
    }

    #[test]
    fn the_top_edge_reveals_the_bar_and_it_hides_after_the_pointer_leaves() {
        let start = Instant::now();
        let mut bar = TopBar::default();
        assert!(bar.update(TopBarMode::Auto, Pointer::Edge, start));
        assert!(bar.update(TopBarMode::Auto, Pointer::Away, start + ms(100)));
        assert!(bar.update(TopBarMode::Auto, Pointer::Away, start + ms(499)));
        assert!(!bar.update(TopBarMode::Auto, Pointer::Away, start + ms(500)));
    }

    #[test]
    fn the_pointer_over_the_shown_bar_keeps_it() {
        let start = Instant::now();
        let mut bar = TopBar::default();
        assert!(bar.update(TopBarMode::Auto, Pointer::Edge, start));
        for step in 1..=20 {
            let now = start + ms(step * 400);
            assert!(bar.update(TopBarMode::Auto, Pointer::Bar, now), "{step}");
        }
        let left = start + ms(8000);
        assert!(bar.update(TopBarMode::Auto, Pointer::Away, left + ms(400)));
        assert!(!bar.update(TopBarMode::Auto, Pointer::Away, left + ms(500)));
    }

    #[test]
    fn the_bar_area_does_not_reveal_a_hidden_bar() {
        let now = Instant::now();
        let mut bar = TopBar::default();
        assert!(!bar.update(TopBarMode::Auto, Pointer::Bar, now));
        assert!(!bar.update(TopBarMode::Auto, Pointer::Bar, now + ms(100)));
    }

    #[test]
    fn a_flash_shows_the_bar_for_a_moment() {
        let start = Instant::now();
        let mut bar = TopBar::default();
        bar.flash(start);
        assert!(bar.update(TopBarMode::Auto, Pointer::Away, start));
        assert!(bar.update(TopBarMode::Auto, Pointer::Away, start + ms(1499)));
        assert!(!bar.update(TopBarMode::Auto, Pointer::Away, start + ms(1500)));
    }

    #[test]
    fn a_new_flash_restarts_the_moment() {
        let start = Instant::now();
        let mut bar = TopBar::default();
        bar.flash(start);
        bar.flash(start + ms(1000));
        assert!(bar.update(TopBarMode::Auto, Pointer::Away, start + ms(2400)));
        assert!(!bar.update(TopBarMode::Auto, Pointer::Away, start + ms(2500)));
    }

    #[test]
    fn hovering_during_a_flash_keeps_the_bar_past_it() {
        let start = Instant::now();
        let mut bar = TopBar::default();
        bar.flash(start);
        assert!(bar.update(TopBarMode::Auto, Pointer::Bar, start + ms(1400)));
        assert!(bar.update(TopBarMode::Auto, Pointer::Bar, start + ms(1800)));
        assert!(bar.update(TopBarMode::Auto, Pointer::Away, start + ms(2299)));
        assert!(!bar.update(TopBarMode::Auto, Pointer::Away, start + ms(2300)));
    }

    #[test]
    fn a_short_hover_does_not_cut_a_flash_short() {
        let start = Instant::now();
        let mut bar = TopBar::default();
        bar.flash(start);
        assert!(bar.update(TopBarMode::Auto, Pointer::Edge, start + ms(100)));
        assert!(bar.update(TopBarMode::Auto, Pointer::Away, start + ms(1499)));
        assert!(!bar.update(TopBarMode::Auto, Pointer::Away, start + ms(1500)));
    }

    #[test]
    fn switching_from_pinned_to_auto_hides_the_bar_unless_hovered() {
        let now = Instant::now();
        let mut bar = TopBar::default();
        assert!(bar.update(TopBarMode::Pinned, Pointer::Away, now));
        assert!(!bar.update(TopBarMode::Auto, Pointer::Away, now + ms(1)));
        assert!(bar.update(TopBarMode::Auto, Pointer::Edge, now + ms(2)));
    }
}
