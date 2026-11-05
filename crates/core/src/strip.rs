//! Tab strip geometry: how wide the tab labels are and which of them fit in the strip.

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

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: i32 = 80;
    const MAX: i32 = 220;
    const GAP: i32 = 1;

    const INDICATOR: i32 = 36;

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

    #[test]
    fn no_tabs_show_nothing() {
        assert_eq!(layout(944, 0, 0), shown(0, 0, MAX));
    }
}
