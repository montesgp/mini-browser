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

impl Rect {
    /// Whether the point `(x, y)` lies inside the rectangle (its right and bottom edges are
    /// outside).
    pub fn contains(&self, x: i32, y: i32) -> bool {
        (self.x..self.x + self.width).contains(&x) && (self.y..self.y + self.height).contains(&y)
    }
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

/// Bounds of a floating box `box_height` pixels tall in the bottom-right corner of the page
/// `area`, `margin` pixels from its right and bottom sides, at most `max_width` wide and keeping
/// `margin` free on its left. A box taller than the area keeps its top `margin` below the area's
/// top, so its header stays visible. It stays at least one pixel wide however narrow the area.
pub fn corner_box(area: Rect, max_width: i32, box_height: i32, margin: i32) -> Rect {
    let width = max_width.min(area.width - 2 * margin).max(1);
    let bottom = area.y + area.height - margin;
    Rect {
        x: area.x + area.width - margin - width,
        y: (bottom - box_height).max(area.y + margin),
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

/// Whether a left click at `point` closes the open settings box at `box_bounds`: a click on the
/// frame (outside the `page` area) or on the shown top `bar` does, a click inside the box never
/// does. Clicks on the page close the box by taking the keyboard focus.
pub fn click_closes_box(
    point: (i32, i32),
    box_bounds: Rect,
    page: Rect,
    bar: Option<Rect>,
) -> bool {
    let (x, y) = point;
    let on_bar = bar.is_some_and(|bar| bar.contains(x, y));
    !box_bounds.contains(x, y) && (on_bar || !page.contains(x, y))
}

/// The screen bounds of a window opened next to the window at `source` (screen coordinates):
/// `offset` pixels right and down with the same size, kept inside the screen's `work_area` and
/// shrunk to it when larger (a maximized source).
pub fn offset_window(source: Rect, offset: i32, work_area: Rect) -> Rect {
    let width = source.width.min(work_area.width);
    let height = source.height.min(work_area.height);
    let place = |start: i32, area_start: i32, area_size: i32, size: i32| {
        (start + offset).clamp(area_start, area_start + area_size - size)
    };
    Rect {
        x: place(source.x, work_area.x, work_area.width, width),
        y: place(source.y, work_area.y, work_area.height, height),
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

/// Where a window dragged by its tab goes when the pointer releases it at a screen's edge, like
/// Windows' Aero Snap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Snap {
    /// The left half of the screen's work area.
    LeftHalf,
    /// The right half of the screen's work area.
    RightHalf,
    /// The whole work area: the window maximizes.
    Maximize,
}

/// How close, in pixels, the pointer gets to a screen's left, right or top edge for a window
/// released there to snap. The pointer stops at an outer edge; at an edge shared with another
/// screen it passes on, so the band is a few pixels wide.
pub const SNAP_EDGE: i32 = 4;

/// Where a window released with the pointer at `point` snaps on the screen whose bounds are
/// `screen` (both in screen coordinates): within [`SNAP_EDGE`] pixels of its left or right edge
/// it fills that half, of its top edge it maximizes, elsewhere (or off the screen) it stays.
/// In a top corner, the side wins.
pub fn snap_target(point: (i32, i32), screen: Rect) -> Option<Snap> {
    let (x, y) = point;
    if !screen.contains(x, y) {
        return None;
    }
    if x < screen.x + SNAP_EDGE {
        Some(Snap::LeftHalf)
    } else if x >= screen.x + screen.width - SNAP_EDGE {
        Some(Snap::RightHalf)
    } else if y < screen.y + SNAP_EDGE {
        Some(Snap::Maximize)
    } else {
        None
    }
}

/// The bounds a window snapped to a half of the screen whose work area is `work_area` takes:
/// that half, the whole height; the right half takes the odd pixel. `None` for
/// [`Snap::Maximize`], which maximizes the window instead.
pub fn snap_bounds(snap: Snap, work_area: Rect) -> Option<Rect> {
    let left = work_area.width / 2;
    let (x, width) = match snap {
        Snap::LeftHalf => (work_area.x, left),
        Snap::RightHalf => (work_area.x + left, work_area.width - left),
        Snap::Maximize => return None,
    };
    Some(Rect {
        x,
        width,
        ..work_area
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: i32, y: i32, width: i32, height: i32) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn a_rect_contains_its_inside_but_not_its_far_edges() {
        let r = rect(10, 20, 30, 40);
        assert!(r.contains(10, 20));
        assert!(r.contains(39, 59));
        assert!(!r.contains(40, 30));
        assert!(!r.contains(20, 60));
        assert!(!r.contains(9, 30));
        assert!(!r.contains(20, 19));
        assert!(!rect(0, 0, 0, 10).contains(0, 5));
    }

    #[test]
    fn clicks_on_the_frame_or_the_bar_close_the_settings_box() {
        let page = rect(8, 0, 1264, 792);
        let settings = rect(976, 600, 280, 176);
        let bar = Some(rect(8, 8, 1264, 28));
        // The frame: left side, bottom side, right side.
        assert!(click_closes_box((3, 400), settings, page, bar));
        assert!(click_closes_box((600, 795), settings, page, None));
        assert!(click_closes_box((1275, 700), settings, page, bar));
        // The shown bar, which lies over the page area.
        assert!(click_closes_box((900, 20), settings, page, bar));
        // The page itself closes it through the focus instead.
        assert!(!click_closes_box((600, 300), settings, page, bar));
        assert!(!click_closes_box((900, 20), settings, page, None));
        // Inside the box never closes it, even where it reaches past the page.
        assert!(!click_closes_box((1000, 700), settings, page, bar));
        let wide = rect(0, 600, 1280, 176);
        assert!(!click_closes_box((3, 700), wide, page, bar));
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
    fn a_corner_box_sits_in_the_bottom_right_corner_of_the_page_area() {
        // A restored 1280 x 800 window with an 8 px frame.
        assert_eq!(
            corner_box(rect(8, 8, 1264, 784), 280, 200, 16),
            rect(976, 576, 280, 200)
        );
        // A maximized window without a frame.
        assert_eq!(
            corner_box(rect(0, 0, 1920, 1032), 280, 200, 16),
            rect(1624, 816, 280, 200)
        );
    }

    #[test]
    fn a_corner_box_narrows_to_keep_its_margins_in_narrow_page_areas() {
        assert_eq!(
            corner_box(rect(8, 8, 200, 584), 280, 200, 16),
            rect(24, 376, 168, 200)
        );
        assert_eq!(corner_box(rect(0, 0, 10, 10), 280, 200, 16).width, 1);
    }

    #[test]
    fn a_corner_box_taller_than_the_page_area_keeps_its_top_inside_it() {
        assert_eq!(
            corner_box(rect(0, 40, 800, 100), 280, 200, 16),
            rect(504, 56, 280, 200)
        );
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

    #[test]
    fn a_window_opens_offset_from_its_source_with_the_same_size() {
        let work = Rect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1040,
        };
        let source = Rect {
            x: 100,
            y: 50,
            width: 1280,
            height: 800,
        };
        assert_eq!(
            offset_window(source, 32, work),
            Rect {
                x: 132,
                y: 82,
                ..source
            }
        );
    }

    #[test]
    fn an_offset_window_stays_inside_the_work_area() {
        let work = Rect {
            x: -1920,
            y: 40,
            width: 1920,
            height: 1000,
        };
        // Near the bottom-right corner: pushed back so it ends at the work area's edges.
        let near_corner = Rect {
            x: -1300,
            y: 220,
            width: 1280,
            height: 800,
        };
        assert_eq!(
            offset_window(near_corner, 32, work),
            Rect {
                x: -1280,
                y: 240,
                ..near_corner
            }
        );
        // Larger than the work area (a maximized source): shrunk to it, at its origin.
        let maximized = Rect {
            x: -1928,
            y: 32,
            width: 1936,
            height: 1016,
        };
        assert_eq!(offset_window(maximized, 32, work), work);
        // Left of and above the work area: moved inside.
        let outside = Rect {
            x: -2500,
            y: 0,
            width: 800,
            height: 600,
        };
        assert_eq!(
            offset_window(outside, 32, work),
            Rect {
                x: -1920,
                y: 40,
                ..outside
            }
        );
    }

    /// A 1920 × 1080 screen right of the primary one, its taskbar taking the bottom 48 pixels.
    fn side_screen() -> (Rect, Rect) {
        (rect(1080, -231, 1920, 1080), rect(1080, -231, 1920, 1032))
    }

    #[test]
    fn a_window_released_at_a_side_edge_snaps_to_that_half() {
        let (screen, _) = side_screen();
        for x in [1080, 1080 + SNAP_EDGE - 1] {
            assert_eq!(snap_target((x, 300), screen), Some(Snap::LeftHalf), "x {x}");
        }
        for x in [2999, 3000 - SNAP_EDGE] {
            assert_eq!(
                snap_target((x, 300), screen),
                Some(Snap::RightHalf),
                "x {x}"
            );
        }
        // The bottom edge does nothing, as in Windows.
        assert_eq!(snap_target((2000, 848), screen), None);
    }

    #[test]
    fn a_window_released_at_the_top_edge_maximizes() {
        let (screen, _) = side_screen();
        for y in [-231, -231 + SNAP_EDGE - 1] {
            assert_eq!(
                snap_target((2000, y), screen),
                Some(Snap::Maximize),
                "y {y}"
            );
        }
        // In a top corner, the side wins.
        assert_eq!(snap_target((1080, -231), screen), Some(Snap::LeftHalf));
        assert_eq!(snap_target((2999, -231), screen), Some(Snap::RightHalf));
    }

    #[test]
    fn a_window_released_away_from_the_edges_or_off_the_screen_stays() {
        let (screen, _) = side_screen();
        assert_eq!(snap_target((2000, 300), screen), None);
        assert_eq!(snap_target((1080 + SNAP_EDGE, 300), screen), None);
        assert_eq!(snap_target((3000 - SNAP_EDGE - 1, 300), screen), None);
        assert_eq!(snap_target((2000, -231 + SNAP_EDGE), screen), None);
        // Points on another screen are that screen's business.
        assert_eq!(snap_target((1079, 300), screen), None);
        assert_eq!(snap_target((3000, 300), screen), None);
        assert_eq!(snap_target((2000, -232), screen), None);
    }

    #[test]
    fn a_snapped_window_fills_half_of_the_work_area() {
        let (_, work) = side_screen();
        assert_eq!(
            snap_bounds(Snap::LeftHalf, work),
            Some(rect(1080, -231, 960, 1032))
        );
        assert_eq!(
            snap_bounds(Snap::RightHalf, work),
            Some(rect(2040, -231, 960, 1032))
        );
        // An odd width: the right half takes the extra pixel, so the halves fill it.
        let odd = rect(0, 0, 1081, 1872);
        assert_eq!(
            snap_bounds(Snap::LeftHalf, odd),
            Some(rect(0, 0, 540, 1872))
        );
        assert_eq!(
            snap_bounds(Snap::RightHalf, odd),
            Some(rect(540, 0, 541, 1872))
        );
        assert_eq!(snap_bounds(Snap::Maximize, work), None);
    }
}
