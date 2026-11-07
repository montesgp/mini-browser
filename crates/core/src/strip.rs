//! Tab strip geometry: how wide the tab labels are and which of them fit in the strip, and
//! where a dragged tab goes: along the strip, out of its window or into another window's strip.

use crate::layout::Rect;

/// The labels the strip shows: `count` tabs starting at index `first`, each `tab_width` wide,
/// and how many tabs are `hidden` because they do not fit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StripLayout {
    pub first: usize,
    pub count: usize,
    pub tab_width: i32,
    pub hidden: usize,
}

/// Lays out `tabs` labels in a strip `available` pixels wide with `spacing` pixels between
/// labels. Labels share the width evenly between `min_width` and `max_width`. When they do not
/// all fit at `min_width`, the strip shows as many as fit (at least one), stretched to fill it,
/// and the window of shown labels always contains `active`: the leftmost labels while the
/// active tab is among them, else the labels ending at the active one. When some tabs are
/// hidden, an indicator `indicator_width` pixels wide (plus `spacing`) follows the labels.
/// A new-tab button `button_width` pixels wide (plus `spacing`) always follows them.
#[allow(clippy::too_many_arguments)]
pub fn strip_layout(
    available: i32,
    tabs: usize,
    active: usize,
    min_width: i32,
    max_width: i32,
    spacing: i32,
    indicator_width: i32,
    button_width: i32,
) -> StripLayout {
    if tabs == 0 {
        return StripLayout {
            first: 0,
            count: 0,
            tab_width: max_width,
            hidden: 0,
        };
    }
    let fit = |width: i32| usize::try_from((width + spacing) / (min_width + spacing)).unwrap_or(0);
    let mut available = if button_width > 0 {
        available - button_width - spacing
    } else {
        available
    };
    let mut fitting = fit(available);
    if fitting < tabs && indicator_width > 0 {
        available -= indicator_width + spacing;
        fitting = fit(available);
    }
    let count = fitting.clamp(1, tabs);
    let gaps = spacing * (i32::try_from(count).unwrap_or(i32::MAX) - 1);
    let tab_width =
        ((available - gaps) / i32::try_from(count).unwrap_or(1)).clamp(min_width, max_width);
    let active = active.min(tabs - 1);
    let first = (active + 1).saturating_sub(count);
    StripLayout {
        first,
        count,
        tab_width,
        hidden: tabs - count,
    }
}

/// How far, in pixels along either axis, the pointer moves with the button down before a press
/// on a tab becomes a drag; a shorter move is still a click.
pub const DRAG_THRESHOLD: i32 = 5;

/// Whether a press moved by `(dx, dy)` became a drag.
pub fn beyond_drag_threshold(dx: i32, dy: i32) -> bool {
    dx.abs() > DRAG_THRESHOLD || dy.abs() > DRAG_THRESHOLD
}

/// Where the label of tab `index` starts in the strip, for labels laid out by `layout` with
/// `spacing` pixels between them; `None` when the tab is not shown.
pub fn tab_left(index: usize, layout: &StripLayout, spacing: i32) -> Option<i32> {
    let slot = index
        .checked_sub(layout.first)
        .filter(|slot| *slot < layout.count)?;
    Some(i32::try_from(slot).ok()? * (layout.tab_width + spacing))
}

/// The tab whose label lies under `x` in the strip; `None` between labels and past them.
pub fn tab_at(x: i32, layout: &StripLayout, spacing: i32) -> Option<usize> {
    let stride = layout.tab_width + spacing;
    if x < 0 || stride <= 0 || x % stride >= layout.tab_width {
        return None;
    }
    let slot = usize::try_from(x / stride).ok()?;
    (slot < layout.count).then_some(layout.first + slot)
}

/// Where a dragged tab goes: the shown position nearest to the dragged label's center, for the
/// pointer at `x` holding the label `grab` pixels from its left edge. Stays among the shown
/// tabs; `None` when no tab is shown.
pub fn drag_index(x: i32, grab: i32, layout: &StripLayout, spacing: i32) -> Option<usize> {
    let last = i32::try_from(layout.count.checked_sub(1)?).ok()?;
    let stride = (layout.tab_width + spacing).max(1);
    let center = x - grab + layout.tab_width / 2;
    let slot = center.div_euclid(stride).clamp(0, last);
    Some(layout.first + usize::try_from(slot).ok()?)
}

/// How far, in pixels, the pointer dragging a tab goes past the top bar (above, below or beyond
/// either end) before the tab tears off into a window of its own, as in Chrome.
pub const TEAR_OFF_DISTANCE: i32 = 40;

/// Whether a tab dragged with the pointer at `point` tears off its top bar at `bar` (both in
/// window coordinates): once the pointer is more than [`TEAR_OFF_DISTANCE`] pixels away from it.
pub fn tears_off(point: (i32, i32), bar: Rect) -> bool {
    let outside = |value: i32, start: i32, length: i32| {
        let last = start + length - 1;
        (start - value).max(value - last).max(0)
    };
    outside(point.0, bar.x, bar.width) > TEAR_OFF_DISTANCE
        || outside(point.1, bar.y, bar.height) > TEAR_OFF_DISTANCE
}

/// Where a tab dropped on a strip goes among its tabs, for the pointer at `x` (strip
/// coordinates) holding the dragged label `grab` pixels from its left edge: before the first
/// shown tab whose center is right of the dragged label's center, else after the shown tabs.
/// The dragged label is as wide as the strip's labels.
pub fn drop_index(x: i32, grab: i32, layout: &StripLayout, spacing: i32) -> usize {
    let width = layout.tab_width;
    let stride = width + spacing;
    let center = x - grab.clamp(0, (width - 1).max(0)) + width / 2;
    let before = (0..layout.count)
        .take_while(|slot| {
            i32::try_from(*slot).is_ok_and(|slot| slot * stride + width / 2 < center)
        })
        .count();
    layout.first + before
}

/// Where the pointer sits, in window coordinates, in a window whose only tab it drags: on the
/// tab's label, `grab` pixels from its left edge (within its `tab_width`), halfway down the
/// strip `strip_height` pixels tall below a frame `padding` pixels wide.
pub fn torn_tab_offset(grab: i32, tab_width: i32, padding: i32, strip_height: i32) -> (i32, i32) {
    (
        padding + grab.clamp(0, (tab_width - 1).max(0)),
        padding + strip_height / 2,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: i32 = 80;
    const MAX: i32 = 220;
    const GAP: i32 = 1;

    const INDICATOR: i32 = 36;

    #[test]
    fn a_drop_outside_the_tabs_lands_at_an_end() {
        let strip = layout(1000, 3, 0);
        assert_eq!(drop_index(-500, 0, &strip, GAP), strip.first);
        assert_eq!(drop_index(5000, 0, &strip, GAP), strip.first + strip.count);
    }

    fn layout(available: i32, tabs: usize, active: usize) -> StripLayout {
        strip_layout(available, tabs, active, MIN, MAX, GAP, 0, 0)
    }

    fn with_indicator(available: i32, tabs: usize, active: usize) -> StripLayout {
        strip_layout(available, tabs, active, MIN, MAX, GAP, INDICATOR, 0)
    }

    const BUTTON: i32 = 28;

    fn with_button(available: i32, tabs: usize, active: usize) -> StripLayout {
        strip_layout(available, tabs, active, MIN, MAX, GAP, INDICATOR, BUTTON)
    }

    fn shown(first: usize, count: usize, tab_width: i32) -> StripLayout {
        StripLayout {
            first,
            count,
            tab_width,
            hidden: 0,
        }
    }

    #[test]
    fn few_tabs_get_the_maximum_width() {
        assert_eq!(layout(1264, 3, 0), shown(0, 3, MAX));
    }

    #[test]
    fn many_tabs_shrink_evenly_to_fill_the_strip() {
        // 8 tabs in 944 px: (944 - 7) / 8 = 117.
        assert_eq!(layout(944, 8, 7), shown(0, 8, 117));
    }

    #[test]
    fn labels_never_go_below_the_minimum_width() {
        // 20 tabs at 80 px need 1619 px; 944 px fits (944 + 1) / 81 = 11 labels.
        let strip = layout(944, 20, 0);
        assert_eq!((strip.first, strip.count), (0, 11));
        assert!(strip.tab_width >= MIN);
        let used = strip.tab_width * 11 + GAP * 10;
        assert!(used <= 944, "labels overflow: {used} px");
    }

    #[test]
    fn the_active_tab_is_always_shown() {
        for active in 0..20 {
            let strip = layout(944, 20, active);
            assert!(
                (strip.first..strip.first + strip.count).contains(&active),
                "active {active} hidden by {strip:?}"
            );
            assert!(strip.first + strip.count <= 20);
        }
    }

    #[test]
    fn a_far_active_tab_is_the_last_label_shown() {
        assert_eq!(layout(944, 20, 15).first, 5);
        assert_eq!(layout(944, 20, 19).first, 9);
        assert_eq!(layout(944, 20, 10).first, 0);
    }

    #[test]
    fn a_tiny_strip_still_shows_the_active_tab() {
        assert_eq!(
            layout(30, 5, 3),
            StripLayout {
                hidden: 4,
                ..shown(3, 1, MIN)
            }
        );
        assert_eq!(layout(0, 1, 0), shown(0, 1, MIN));
    }

    #[test]
    fn tabs_that_do_not_fit_are_counted_as_hidden() {
        assert_eq!(layout(944, 20, 0).hidden, 9);
        assert_eq!(layout(30, 5, 3).hidden, 4);
        assert_eq!(layout(944, 8, 7).hidden, 0);
    }

    #[test]
    fn the_indicator_takes_room_only_when_tabs_are_hidden() {
        // All 8 fit: no indicator, the same layout as without one.
        assert_eq!(with_indicator(944, 8, 7), layout(944, 8, 7));
        // 20 tabs: the indicator and its gap leave 907 px, (907 + 1) / 81 = 11 labels.
        let strip = with_indicator(944, 20, 0);
        assert_eq!((strip.first, strip.count, strip.hidden), (0, 11, 9));
        let used = strip.tab_width * 11 + GAP * 10 + GAP + INDICATOR;
        assert!(used <= 944, "labels and indicator overflow: {used} px");
        // 12 tabs fit 980 px at once, so they need no indicator.
        assert_eq!(with_indicator(980, 12, 0), layout(980, 12, 0));
        assert_eq!(with_indicator(980, 12, 0).count, 12);
        // 20 tabs: 12 labels fit 980 px, but the indicator leaves room for 11.
        assert_eq!(layout(980, 20, 0).count, 12);
        let strip = with_indicator(980, 20, 0);
        assert_eq!((strip.count, strip.hidden), (11, 9));
        assert!(strip.tab_width * 11 + GAP * 11 + INDICATOR <= 980);
    }

    #[test]
    fn the_indicator_never_hides_the_active_tab() {
        for active in 0..20 {
            let strip = with_indicator(400, 20, active);
            assert!((strip.first..strip.first + strip.count).contains(&active));
            assert_eq!(strip.hidden, 20 - strip.count);
        }
        assert_eq!(
            with_indicator(30, 5, 3),
            StripLayout {
                hidden: 4,
                ..shown(3, 1, MIN)
            }
        );
    }

    #[test]
    fn the_new_tab_button_always_takes_room() {
        // Few tabs keep the maximum width.
        assert_eq!(with_button(1264, 3, 0), shown(0, 3, MAX));
        // 8 tabs: the button and its gap leave 915 px, (915 - 7) / 8 = 113.
        assert_eq!(with_button(944, 8, 7), shown(0, 8, 113));
        // 12 tabs fit 980 px alone, but not next to the button: 11 labels and the count.
        assert_eq!(layout(980, 12, 0).count, 12);
        let strip = with_button(980, 12, 0);
        assert_eq!((strip.count, strip.hidden), (11, 1));
        assert!(strip.tab_width * 11 + GAP * 10 + GAP + INDICATOR + GAP + BUTTON <= 980);
    }

    #[test]
    fn the_new_tab_button_never_hides_the_active_tab() {
        for active in 0..20 {
            let strip = with_button(400, 20, active);
            assert!((strip.first..strip.first + strip.count).contains(&active));
            assert_eq!(strip.hidden, 20 - strip.count);
            let count = i32::try_from(strip.count).unwrap();
            let used = strip.tab_width * count + GAP * count + INDICATOR + GAP + BUTTON;
            assert!(
                used <= 400 || strip.count == 1,
                "{strip:?} overflows: {used} px"
            );
        }
        assert_eq!(
            with_button(30, 5, 3),
            StripLayout {
                hidden: 4,
                ..shown(3, 1, MIN)
            }
        );
    }

    /// Tabs 5..9 shown, 100 px wide with 1 px gaps: tab 5 at 0..100, tab 6 at 101..201, ...
    fn scrolled() -> StripLayout {
        StripLayout {
            first: 5,
            count: 4,
            tab_width: 100,
            hidden: 3,
        }
    }

    #[test]
    fn a_small_move_is_still_a_click() {
        assert!(!beyond_drag_threshold(0, 0));
        assert!(!beyond_drag_threshold(DRAG_THRESHOLD, -DRAG_THRESHOLD));
        assert!(beyond_drag_threshold(DRAG_THRESHOLD + 1, 0));
        assert!(beyond_drag_threshold(-DRAG_THRESHOLD - 1, 0));
        assert!(beyond_drag_threshold(0, DRAG_THRESHOLD + 1));
    }

    #[test]
    fn shown_tabs_start_one_label_and_gap_apart() {
        let strip = scrolled();
        assert_eq!(tab_left(5, &strip, GAP), Some(0));
        assert_eq!(tab_left(7, &strip, GAP), Some(202));
        assert_eq!(tab_left(8, &strip, GAP), Some(303));
        assert_eq!(tab_left(4, &strip, GAP), None);
        assert_eq!(tab_left(9, &strip, GAP), None);
    }

    #[test]
    fn the_tab_under_the_pointer_is_found_by_its_label() {
        let strip = scrolled();
        assert_eq!(tab_at(0, &strip, GAP), Some(5));
        assert_eq!(tab_at(99, &strip, GAP), Some(5));
        assert_eq!(tab_at(100, &strip, GAP), None, "the gap");
        assert_eq!(tab_at(101, &strip, GAP), Some(6));
        assert_eq!(tab_at(402, &strip, GAP), Some(8));
        assert_eq!(tab_at(403, &strip, GAP), None, "the + button's gap");
        assert_eq!(tab_at(-1, &strip, GAP), None);
        assert_eq!(tab_at(10, &layout(944, 0, 0), GAP), None);
    }

    #[test]
    fn a_dragged_tab_goes_where_its_center_is() {
        let strip = scrolled();
        // Held at its middle: the pointer is the label's center.
        assert_eq!(drag_index(50, 50, &strip, GAP), Some(5));
        assert_eq!(drag_index(100, 50, &strip, GAP), Some(5));
        assert_eq!(drag_index(101, 50, &strip, GAP), Some(6));
        // Held near its left edge: it moves once its center (40 px right of the pointer)
        // reaches the next slot.
        assert_eq!(drag_index(60, 10, &strip, GAP), Some(5));
        assert_eq!(drag_index(61, 10, &strip, GAP), Some(6));
        // It stays among the shown tabs.
        assert_eq!(drag_index(-300, 50, &strip, GAP), Some(5));
        assert_eq!(drag_index(2000, 50, &strip, GAP), Some(8));
        assert_eq!(drag_index(10, 50, &layout(944, 0, 0), GAP), None);
    }

    fn bar() -> Rect {
        // An 8 px frame: the bar 28 px tall right below it, across a 1280 px window.
        Rect {
            x: 8,
            y: 8,
            width: 1264,
            height: 28,
        }
    }

    #[test]
    fn a_tab_tears_off_only_past_the_distance_from_the_bar() {
        // On the bar and up to the distance away from it, the tab stays in the strip.
        for point in [
            (100, 8),
            (100, 35),
            (100, 8 - 40),
            (100, 35 + 40),
            (8 - 40, 20),
        ] {
            assert!(!tears_off(point, bar()), "{point:?}");
        }
        assert!(!tears_off((1271 + 40, 20), bar()));
        // One pixel further, above, below or beyond either end, it tears off.
        for point in [(100, 8 - 41), (100, 35 + 41), (8 - 41, 20), (1271 + 41, 20)] {
            assert!(tears_off(point, bar()), "{point:?}");
        }
    }

    #[test]
    fn a_dropped_tab_goes_before_the_first_tab_whose_center_is_past_its_own() {
        let strip = scrolled();
        // Centers of the shown tabs 5..9: 50, 151, 252, 353. Held at its middle, the dragged
        // label's center is the pointer.
        assert_eq!(drop_index(10, 50, &strip, GAP), 5);
        assert_eq!(drop_index(50, 50, &strip, GAP), 5);
        assert_eq!(drop_index(51, 50, &strip, GAP), 6);
        assert_eq!(drop_index(252, 50, &strip, GAP), 7);
        assert_eq!(drop_index(253, 50, &strip, GAP), 8);
        // Past the last center, after the shown tabs.
        assert_eq!(drop_index(354, 50, &strip, GAP), 9);
        assert_eq!(drop_index(2000, 50, &strip, GAP), 9);
        assert_eq!(drop_index(-300, 50, &strip, GAP), 5);
        // Held near its left edge: its center is 40 px right of the pointer.
        assert_eq!(drop_index(10, 10, &strip, GAP), 5);
        assert_eq!(drop_index(11, 10, &strip, GAP), 6);
        // A grab wider than the strip's labels counts from the label's right edge.
        assert_eq!(
            drop_index(10, 500, &strip, GAP),
            drop_index(10, 99, &strip, GAP)
        );
        // A strip without tabs takes it as its first.
        assert_eq!(drop_index(10, 50, &layout(944, 0, 0), GAP), 0);
    }

    #[test]
    fn a_torn_off_tab_sits_under_the_pointer_where_it_was_held() {
        // Held 30 px into its label: the pointer is 30 px into the new window's only tab,
        // halfway down the strip below the frame.
        assert_eq!(torn_tab_offset(30, 220, 8, 28), (38, 22));
        assert_eq!(torn_tab_offset(30, 220, 0, 28), (30, 14));
        // A grab past the new label's width stays on it.
        assert_eq!(torn_tab_offset(400, 220, 8, 28), (8 + 219, 22));
        assert_eq!(torn_tab_offset(-5, 220, 8, 28), (8, 22));
    }

    #[test]
    fn no_tabs_show_nothing() {
        assert_eq!(layout(944, 0, 0), shown(0, 0, MAX));
    }
}
