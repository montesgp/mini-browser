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
pub fn strip_layout(
    available: i32,
    tabs: usize,
    active: usize,
    min_width: i32,
    max_width: i32,
    spacing: i32,
    indicator_width: i32,
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
    let mut fitting = fit(available);
    let mut available = available;
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

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: i32 = 80;
    const MAX: i32 = 220;
    const GAP: i32 = 1;

    const INDICATOR: i32 = 36;

    fn layout(available: i32, tabs: usize, active: usize) -> StripLayout {
        strip_layout(available, tabs, active, MIN, MAX, GAP, 0)
    }

    fn with_indicator(available: i32, tabs: usize, active: usize) -> StripLayout {
        strip_layout(available, tabs, active, MIN, MAX, GAP, INDICATOR)
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
    fn no_tabs_show_nothing() {
        assert_eq!(layout(944, 0, 0), shown(0, 0, MAX));
    }
}
