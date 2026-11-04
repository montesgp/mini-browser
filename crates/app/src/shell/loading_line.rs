//! The loading line: a thin animated line in the top side of the padding frame while the active
//! tab loads. The frame's top side is a band of the window's layout, so it keeps dragging the
//! window and the line never covers the page.

use std::{
    cell::{Cell, RefCell},
    time::Instant,
};

use cef::*;
use mini_core::layout;

use super::window::{BACKGROUND_COLOR, DarkPanelDelegate};

/// Muted blue that stands out on the near-black frame without glaring.
const LINE_COLOR: u32 = 0xFF5B_8DEF;
const LINE_HEIGHT: i32 = 2;
/// One sweep of the moving segment across the line.
const PERIOD_MS: u64 = 1200;
/// About 60 frames a second.
const FRAME_MS: i64 = 16;

struct LoadingLine {
    band: Panel,
    line: Panel,
    started: Instant,
    loading: bool,
    /// Whether a frame is scheduled.
    ticking: bool,
}

thread_local! {
    static BAND_HEIGHT: Cell<i32> = const { Cell::new(0) };
    static LOADING_LINE: RefCell<Option<LoadingLine>> = const { RefCell::new(None) };
}

/// Creates the band that forms the top side of the frame, `padding` pixels tall, with the
/// (hidden) line in it.
pub fn create(padding: u32) -> Option<Panel> {
    BAND_HEIGHT.set(i32::try_from(padding).unwrap_or(0));
    let band = panel_create(Some(&mut BandDelegate::new()))?;
    // CEF lays out a 0 px view at its default size, so a band with no height is hidden instead
    // (a maximized window has no frame).
    band.set_visible((padding > 0).into());
    let line = panel_create(Some(&mut DarkPanelDelegate::new(LINE_COLOR, LINE_HEIGHT)))?;
    line.set_visible(0);
    band.add_child_view(Some(&mut View::from(&line)));
    LOADING_LINE.set(Some(LoadingLine {
        band: band.clone(),
        line,
        started: Instant::now(),
        loading: false,
        ticking: false,
    }));
    Some(band)
}

pub fn destroy() {
    LOADING_LINE.take();
}

/// Changes the band's height to a new padding; the window lays itself out afterwards.
pub fn set_height(padding: u32) {
    BAND_HEIGHT.set(i32::try_from(padding).unwrap_or(0));
    if let Some(band) = LOADING_LINE.with_borrow(|state| state.as_ref().map(|s| s.band.clone())) {
        band.set_visible((padding > 0).into());
        band.invalidate_layout();
    }
}

/// Shows and animates the line while `loading`, hides it otherwise.
pub fn set_loading(loading: bool) {
    let Some((line, start_ticking)) = LOADING_LINE.with_borrow_mut(|state| {
        let state = state.as_mut()?;
        state.loading = loading;
        let start_ticking = loading && !state.ticking;
        if start_ticking {
            state.started = Instant::now();
            state.ticking = true;
        }
        Some((state.line.clone(), start_ticking))
    }) else {
        return;
    };
    if !loading {
        line.set_visible(0);
        return;
    }
    if start_ticking {
        tick();
    }
}

/// Moves the segment to its place for the current time and schedules the next frame, until
/// the line is hidden.
fn tick() {
    let Some((band, line, elapsed)) = LOADING_LINE.with_borrow(|state| {
        state.as_ref().map(|state| {
            let elapsed = u64::try_from(state.started.elapsed().as_millis()).unwrap_or(0);
            (state.band.clone(), state.line.clone(), elapsed)
        })
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
    let mut task = Tick::new();
    post_delayed_task(ThreadId::UI, Some(&mut task), FRAME_MS);
}

wrap_task! {
    struct Tick;

    impl Task {
        fn execute(&self) {
            // Stop once loading ended; `set_loading(false)` already hid the line.
            let ticking = LOADING_LINE.with_borrow_mut(|state| {
                state.as_mut().is_some_and(|state| {
                    state.ticking = state.loading;
                    state.ticking
                })
            });
            if ticking {
                tick();
            }
        }
    }
}

// The band: no layout (the line is placed by hand), the frame's color and the padding's height.
wrap_panel_delegate! {
    struct BandDelegate {}

    impl ViewDelegate {
        fn preferred_size(&self, _view: Option<&mut View>) -> Size {
            // A zero side makes CEF fall back to the layout's size; see `DarkPanelDelegate`.
            Size {
                width: 1,
                height: BAND_HEIGHT.get(),
            }
        }

        fn height_for_width(&self, _view: Option<&mut View>, _width: i32) -> i32 {
            BAND_HEIGHT.get()
        }

        fn on_theme_changed(&self, view: Option<&mut View>) {
            if let Some(view) = view {
                view.set_background_color(BACKGROUND_COLOR);
            }
        }
    }

    impl PanelDelegate {}
}
