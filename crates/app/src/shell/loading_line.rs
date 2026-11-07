//! The loading line: a thin animated line in the top side of the padding frame while the active
//! tab loads. The frame's top side is a band of the window's layout, so it keeps dragging the
//! window and the line never covers the page.

use std::time::Instant;

use cef::*;
use mini_core::layout;

use super::{
    window::{BACKGROUND_COLOR, DarkPanelDelegate},
    windows::{self, WindowId},
};

/// Muted blue that stands out on the near-black frame without glaring.
const LINE_COLOR: u32 = 0xFF5B_8DEF;
const LINE_HEIGHT: i32 = 2;
/// One sweep of the moving segment across the line.
const PERIOD_MS: u64 = 1200;
/// About 60 frames a second.
const FRAME_MS: i64 = 16;

pub struct LoadingLine {
    band: Panel,
    line: Panel,
    started: Instant,
    loading: bool,
    /// Whether a frame is scheduled.
    ticking: bool,
}

/// Runs `f` on the loading line of window `win`; `None` before it is created or once it is
/// gone.
fn with_line<R>(win: WindowId, f: impl FnOnce(&mut LoadingLine) -> R) -> Option<R> {
    let state = windows::get(win)?;
    let mut line = state.loading_line.borrow_mut();
    line.as_mut().map(f)
}

/// The band's height in window `win`.
fn band_height(win: WindowId) -> i32 {
    windows::get(win).map_or(0, |state| state.band_height.get())
}

fn set_band_height(win: WindowId, padding: u32) {
    if let Some(state) = windows::get(win) {
        state.band_height.set(i32::try_from(padding).unwrap_or(0));
    }
}

/// Creates the band that forms the top side of the frame of window `win`, `padding` pixels
/// tall, with the (hidden) line in it.
pub fn create(win: WindowId, padding: u32) -> Option<Panel> {
    set_band_height(win, padding);
    let band = panel_create(Some(&mut BandDelegate::new(win)))?;
    // CEF lays out a 0 px view at its default size, so a band with no height is hidden instead
    // (a maximized window has no frame).
    band.set_visible((padding > 0).into());
    let line = panel_create(Some(&mut DarkPanelDelegate::new(LINE_COLOR, LINE_HEIGHT)))?;
    line.set_visible(0);
    band.add_child_view(Some(&mut View::from(&line)));
    windows::get(win)?.loading_line.replace(Some(LoadingLine {
        band: band.clone(),
        line,
        started: Instant::now(),
        loading: false,
        ticking: false,
    }));
    Some(band)
}

/// Changes the band's height to a new padding; window `win` lays itself out afterwards.
pub fn set_height(win: WindowId, padding: u32) {
    set_band_height(win, padding);
    if let Some(band) = with_line(win, |state| state.band.clone()) {
        band.set_visible((padding > 0).into());
        band.invalidate_layout();
    }
}

/// Shows and animates the line of window `win` while `loading`, hides it otherwise.
pub fn set_loading(win: WindowId, loading: bool) {
    let Some((line, start_ticking)) = with_line(win, |state| {
        state.loading = loading;
        let start_ticking = loading && !state.ticking;
        if start_ticking {
            state.started = Instant::now();
            state.ticking = true;
        }
        (state.line.clone(), start_ticking)
    }) else {
        return;
    };
    if !loading {
        line.set_visible(0);
        return;
    }
    if start_ticking {
        tick(win);
    }
}

/// Moves the segment to its place for the current time and schedules the next frame, until
/// the line is hidden.
fn tick(win: WindowId) {
    let Some((band, line, elapsed)) = with_line(win, |state| {
        let elapsed = u64::try_from(state.started.elapsed().as_millis()).unwrap_or(0);
        (state.band.clone(), state.line.clone(), elapsed)
    }) else {
        return;
    };
    let bounds = band.bounds();
    let (x, width) = layout::loading_segment(bounds.width, elapsed, PERIOD_MS);
    line.set_bounds(Some(&Rect {
        x,
        y: (bounds.height - LINE_HEIGHT).max(0) / 2,
        width,
        height: LINE_HEIGHT,
    }));
    line.set_visible((width > 0).into());
    let mut task = Tick::new(win);
    post_delayed_task(ThreadId::UI, Some(&mut task), FRAME_MS);
}

wrap_task! {
    struct Tick {
        win: WindowId,
    }

    impl Task {
        fn execute(&self) {
            // Stop once loading ended; `set_loading(false)` already hid the line.
            let ticking = with_line(self.win, |state| {
                state.ticking = state.loading;
                state.ticking
            });
            if ticking == Some(true) {
                tick(self.win);
            }
        }
    }
}

// The band: no layout (the line is placed by hand), the frame's color and the padding's height.
wrap_panel_delegate! {
    struct BandDelegate {
        win: WindowId,
    }

    impl ViewDelegate {
        fn preferred_size(&self, _view: Option<&mut View>) -> Size {
            // A zero side makes CEF fall back to the layout's size; see `DarkPanelDelegate`.
            Size {
                width: 1,
                height: band_height(self.win),
            }
        }

        fn height_for_width(&self, _view: Option<&mut View>, _width: i32) -> i32 {
            band_height(self.win)
        }

        fn on_theme_changed(&self, view: Option<&mut View>) {
            if let Some(view) = view {
                view.set_background_color(BACKGROUND_COLOR);
            }
        }
    }

    impl PanelDelegate {}
}
