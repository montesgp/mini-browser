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
}
