//! Window geometry: the padding frame that drags the window and the placement of the floating
//! boxes. Coordinates are window pixels (density independent), origin at the top left.

/// A rectangle in window coordinates.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// The top, bottom, left and right sides of a frame `padding` pixels wide inside a window of
/// `width` × `height`. The left and right sides sit between the top and bottom ones. Sides that
/// do not fit (padding wider than half the window) come out empty or negative.
pub fn frame_rects(width: i32, height: i32, padding: i32) -> [Rect; 4] {
    let inner_height = height - 2 * padding;
    [
        Rect {
            x: 0,
            y: 0,
            width,
            height: padding,
        },
        Rect {
            x: 0,
            y: height - padding,
            width,
            height: padding,
        },
        Rect {
            x: 0,
            y: padding,
            width: padding,
            height: inner_height,
        },
        Rect {
            x: width - padding,
            y: padding,
            width: padding,
            height: inner_height,
        },
    ]
}

/// The width of the padding frame: the saved `padding` while the window is restored. Maximized,
/// the window has no frame of its own; only the `overhang`, the part of the window that reaches
/// past the screen's work area (0 when it fits), is inset so the page fills the work area.
pub fn frame_padding(padding: u32, maximized: bool, overhang: i32) -> u32 {
    if maximized {
        u32::try_from(overhang).unwrap_or(0)
    } else {
        padding
    }
}

/// Where a window's views start on the screen. A restored window's views start at its bounds;
/// a maximized frameless window on Windows is larger than its screen, so its views are measured
/// from a child view (`child`, when it could be converted to screen coordinates).
pub fn view_origin(maximized: bool, bounds: (i32, i32), child: Option<(i32, i32)>) -> (i32, i32) {
    match child {
        Some(child) if maximized => child,
        _ => bounds,
    }
}

/// Bounds of a floating box `box_height` pixels tall: centered horizontally a fifth of the way
/// down a window of `window_width` × `window_height`, at most `max_width` wide and keeping
/// `side_margin` free on each side. It stays at least one pixel wide however narrow the window.
pub fn floating_box(
    window_width: i32,
    window_height: i32,
    max_width: i32,
    side_margin: i32,
    box_height: i32,
) -> Rect {
    let width = max_width.min(window_width - 2 * side_margin).max(1);
    Rect {
        x: (window_width - width) / 2,
        y: window_height / 5,
        width,
        height: box_height,
    }
}

/// Bounds of a `width` × `height` box centered in a window of `window_width` × `window_height`.
pub fn centered_box(window_width: i32, window_height: i32, width: i32, height: i32) -> Rect {
    Rect {
        x: (window_width - width) / 2,
        y: (window_height - height) / 2,
        width,
        height,
    }
}

/// The visible part `(x, width)` of the loading line's moving segment in a line `width` pixels
/// wide, `elapsed_ms` into the animation. The segment, a third of the line, sweeps from just
/// left of the line to just right of it once every `period_ms`, then starts over.
pub fn loading_segment(width: i32, elapsed_ms: u64, period_ms: u64) -> (i32, i32) {
    if width <= 0 || period_ms == 0 {
        return (0, 0);
    }
    let segment = i64::from(width / 3);
    let travel = i64::from(width) + segment;
    let progress = i64::try_from(elapsed_ms % period_ms).unwrap_or(0);
    let period = i64::try_from(period_ms).unwrap_or(i64::MAX);
    let start = -segment + travel * progress / period;
    let left = start.clamp(0, i64::from(width));
    let right = (start + segment).clamp(0, i64::from(width));
    // Both ends lie within `0..=width`, so they fit in `i32`.
    (left as i32, (right - left) as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_restored_window_starts_at_its_bounds() {
        assert_eq!(view_origin(false, (0, 536), Some((-8, 0))), (0, 536));
        assert_eq!(view_origin(false, (120, 40), None), (120, 40));
        assert_eq!(view_origin(false, (0, 500), Some((-8, 0))), (0, 500));
    }

    #[test]
    fn a_maximized_window_starts_at_its_child_view() {
        assert_eq!(view_origin(true, (-8, -8), Some((0, 0))), (0, 0));
        assert_eq!(view_origin(true, (-8, -8), None), (-8, -8));
    }

    fn rect(x: i32, y: i32, width: i32, height: i32) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn the_frame_covers_each_edge_without_overlap() {
        assert_eq!(
            frame_rects(1280, 800, 8),
            [
                rect(0, 0, 1280, 8),
                rect(0, 792, 1280, 8),
                rect(0, 8, 8, 784),
                rect(1272, 8, 8, 784),
            ]
        );
    }

    #[test]
    fn a_zero_padding_frame_is_empty() {
        assert!(
            frame_rects(1280, 800, 0)
                .iter()
                .all(|side| side.width == 0 || side.height == 0)
        );
    }

    #[test]
    fn a_frame_wider_than_the_window_leaves_no_positive_side_area() {
        let [top, bottom, left, right] = frame_rects(10, 10, 6);
        assert_eq!(left.height, -2);
        assert_eq!(right.height, -2);
        assert_eq!((top.width, bottom.width), (10, 10));
    }

    #[test]
    fn a_restored_window_keeps_its_saved_frame() {
        assert_eq!(frame_padding(8, false, 0), 8);
        assert_eq!(frame_padding(32, false, 8), 32);
    }

    #[test]
    fn a_maximized_window_has_no_frame_beyond_its_overhang() {
        assert_eq!(frame_padding(20, true, 0), 0);
        assert_eq!(frame_padding(20, true, 8), 8);
        assert_eq!(frame_padding(4, true, 8), 8);
        assert_eq!(frame_padding(20, true, -3), 0);
    }

    #[test]
    fn a_floating_box_is_centered_a_fifth_down_at_its_max_width() {
        assert_eq!(
            floating_box(1280, 800, 640, 32, 48),
            rect(320, 160, 640, 48)
        );
    }

    #[test]
    fn a_floating_box_keeps_its_side_margins_in_narrow_windows() {
        assert_eq!(floating_box(500, 400, 640, 32, 48), rect(32, 80, 436, 48));
        assert_eq!(floating_box(200, 400, 280, 0, 90), rect(0, 80, 200, 90));
    }

    #[test]
    fn a_floating_box_stays_one_pixel_wide_in_tiny_windows() {
        assert_eq!(floating_box(40, 10, 640, 32, 48), rect(19, 2, 1, 48));
        assert_eq!(floating_box(0, 0, 640, 32, 48).width, 1);
    }

    #[test]
    fn a_centered_box_sits_in_the_middle_of_the_window() {
        assert_eq!(centered_box(1280, 800, 300, 24), rect(490, 388, 300, 24));
        assert_eq!(centered_box(200, 100, 300, 24), rect(-50, 38, 300, 24));
    }

    #[test]
    fn the_loading_segment_sweeps_across_the_line_and_repeats() {
        assert_eq!(loading_segment(900, 0, 1200), (0, 0));
        assert_eq!(loading_segment(900, 100, 1200), (0, 100));
        assert_eq!(loading_segment(900, 300, 1200), (0, 300));
        assert_eq!(loading_segment(900, 600, 1200), (300, 300));
        assert_eq!(loading_segment(900, 1100, 1200), (800, 100));
        assert_eq!(loading_segment(900, 1800, 1200), (300, 300));
    }

    #[test]
    fn the_loading_segment_of_an_empty_line_is_empty() {
        assert_eq!(loading_segment(0, 600, 1200), (0, 0));
        assert_eq!(loading_segment(900, 600, 0), (0, 0));
    }
}
