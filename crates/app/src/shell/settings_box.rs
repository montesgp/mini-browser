//! The settings box: a small dark overlay to change the padding frame, when the top bar shows
//! and whether the app checks for updates, with a notice when a newer release is out, and a thin
//! edge in the address box's border color so it stands out on dark pages.
//! Every window has its own box; the settings are the app's, so every change applies at once to
//! every window and is saved to the settings file.

use std::{cell::RefCell, path::PathBuf};

use cef::*;
use mini_core::{
    layout,
    settings::{self, PADDING_MAX, PADDING_MIN, Settings, TopBarMode},
};

use super::{
    address_box::{self, BORDER_COLOR, BOX_COLOR, TEXT_COLOR},
    home, tabs, top_bar, update,
    window::{self, DarkPanelDelegate},
    windows::{self, WindowId},
};

const WIDTH: i32 = 280;
/// Space between the box and the bottom-right corner of the page area.
const MARGIN: i32 = 16;
const INSET: i32 = 14;
const EDGE: i32 = 1;
const DIM_TEXT_COLOR: u32 = 0xFF8C_8C8C;
/// The `×` at rest: brighter than the dim labels, so it is easy to find, and below the text
/// color it takes on hover.
const CLOSE_TEXT_COLOR: u32 = 0xFFB4_B4B4;
/// A `−` / `+` button whose direction is at its bound.
const BOUND_BUTTON_COLOR: u32 = 0xFF4A_4A4A;
const WARNING_TEXT_COLOR: u32 = 0xFFD8_A657;
/// Shown while the last change could not be saved to the settings file.
const NOT_SAVED: &str = "Not saved: applies until restart";
const FONT: &str = "Segoe UI, Helvetica, Arial, 14px";
const TITLE_FONT: &str = "Segoe UI, Helvetica, Arial, Bold 15px";
/// The `−` and `+` glyphs; at this size their strokes look equally heavy.
const STEP_FONT: &str = "Segoe UI, Helvetica, Arial, 20px";
/// The `×` uses the `−`/`+` buttons' size: a smaller label button has no room left for its
/// text inside the button's insets, and Views draws nothing.
const CLOSE_FONT: &str = "Segoe UI, Helvetica, Arial, 18px";
const CLOSE_SIZE: i32 = BUTTON_SIZE;
const VALUE_WIDTH: i32 = 56;
const BUTTON_SIZE: i32 = 28;
const MODE_WIDTH: i32 = 64;
/// Background of the selected top bar mode.
const SELECTED_COLOR: u32 = 0xFF38_3838;
/// The Download button's text, in the home page's accent blue.
const ACCENT_TEXT_COLOR: u32 = 0xFF5B_8DEF;
const DOWNLOAD_WIDTH: i32 = 96;

const VK_ESCAPE: i32 = 0x1B;

pub struct SettingsBox {
    window: Window,
    controller: OverlayController,
    panel: Panel,
    minus: LabelButton,
    value: LabelButton,
    plus: LabelButton,
    auto: LabelButton,
    pinned: LabelButton,
    updates_on: LabelButton,
    updates_off: LabelButton,
    close: LabelButton,
    not_saved: LabelButton,
    /// The update notice: its line of text and the row with the Download button.
    notice: Panel,
    notice_text: LabelButton,
    download: LabelButton,
}

/// The app's settings, shared by every window, and where they are saved.
struct Store {
    settings: Settings,
    path: PathBuf,
    /// Whether the last change could not be saved.
    unsaved: bool,
}

thread_local! {
    static STORE: RefCell<Option<Store>> = const { RefCell::new(None) };
}

/// Starts the app with `settings`; changes are saved to `path`. Call it before the first
/// window opens.
pub fn init(settings: Settings, path: PathBuf) {
    STORE.set(Some(Store {
        settings,
        path,
        unsaved: false,
    }));
}

/// Reads the box of window `win` with `f`; `None` before it is created or once it is gone.
fn with_box<R>(win: WindowId, f: impl FnOnce(&SettingsBox) -> R) -> Option<R> {
    let state = windows::get(win)?;
    let settings_box = state.settings_box.borrow();
    settings_box.as_ref().map(f)
}

/// Creates the (hidden) box as an overlay of `window` (window `win`).
pub fn create(win: WindowId, window: &Window) {
    let Some(panel) = panel_create(Some(&mut DarkPanelDelegate::new(BOX_COLOR, 0))) else {
        return;
    };
    panel.set_to_box_layout(Some(&BoxLayoutSettings {
        inside_border_insets: Insets {
            top: INSET,
            left: INSET,
            bottom: INSET,
            right: INSET,
        },
        between_child_spacing: 10,
        cross_axis_alignment: AxisAlignment::STRETCH,
        ..Default::default()
    }));
    let (
        Some(title),
        Some(close),
        Some(label),
        Some(minus),
        Some(value),
        Some(plus),
        Some(bar_label),
        Some(auto),
        Some(pinned),
        Some(updates_label),
        Some(updates_on),
        Some(updates_off),
        Some(notice_text),
        Some(download_spacer),
        Some(download),
        Some(not_saved),
        Some(unsaved),
    ) = (
        text(win, "Settings", TITLE_FONT, TEXT_COLOR),
        close_button(win),
        text(win, "Padding", FONT, DIM_TEXT_COLOR),
        step_button(win, "\u{2212}", -1),
        text(win, "", FONT, TEXT_COLOR),
        step_button(win, "+", 1),
        text(win, "Top bar", FONT, DIM_TEXT_COLOR),
        mode_button(win, TopBarMode::Auto),
        mode_button(win, TopBarMode::Pinned),
        text(win, "Update check", FONT, DIM_TEXT_COLOR),
        choice_button(win, "On", Choice::Updates(true), MODE_WIDTH),
        choice_button(win, "Off", Choice::Updates(false), MODE_WIDTH),
        text(win, "", FONT, TEXT_COLOR),
        text(win, "", FONT, TEXT_COLOR),
        download_button(win),
        text(win, NOT_SAVED, FONT, WARNING_TEXT_COLOR),
        STORE.with_borrow(|store| store.as_ref().map(|store| store.unsaved)),
    )
    else {
        return;
    };
    not_saved.set_visible(unsaved.into());
    value.set_horizontal_alignment(HorizontalAlignment::CENTER);
    value.set_minimum_size(Some(&Size {
        width: VALUE_WIDTH,
        height: BUTTON_SIZE,
    }));

    for (label, controls) in [
        (&title, [&close].as_slice()),
        (&label, [&minus, &value, &plus].as_slice()),
        (&bar_label, [&auto, &pinned].as_slice()),
        (&updates_label, [&updates_on, &updates_off].as_slice()),
    ] {
        let Some(row) = row(label, controls) else {
            return;
        };
        panel.add_child_view(Some(&mut View::from(&row)));
    }
    let Some(notice) = notice(&notice_text, &download_spacer, &download) else {
        return;
    };
    panel.add_child_view(Some(&mut View::from(&notice)));
    panel.add_child_view(Some(&mut View::from(&not_saved)));
    let Some(panel) = edged(&panel) else {
        return;
    };

    let Some(controller) =
        window.add_overlay_view(Some(&mut View::from(&panel)), DockingMode::CUSTOM, 1)
    else {
        return;
    };
    let Some(state) = windows::get(win) else {
        return;
    };
    state.settings_box.replace(Some(SettingsBox {
        window: window.clone(),
        controller,
        panel,
        minus,
        value,
        plus,
        auto,
        pinned,
        updates_on,
        updates_off,
        close,
        not_saved,
        notice,
        notice_text,
        download,
    }));
    show_padding(win);
    show_top_bar(win);
    show_update_check(win);
    show_update(win);
}

/// The (hidden) update notice: `text` on its own line, then `download` on the right of a row
/// that starts with the empty `spacer`; the line is too long to share a row with the button.
fn notice(text: &LabelButton, spacer: &LabelButton, download: &LabelButton) -> Option<Panel> {
    let notice = panel_create(Some(&mut DarkPanelDelegate::new(BOX_COLOR, 0)))?;
    notice.set_to_box_layout(Some(&BoxLayoutSettings {
        between_child_spacing: 6,
        cross_axis_alignment: AxisAlignment::STRETCH,
        ..Default::default()
    }));
    notice.add_child_view(Some(&mut View::from(text)));
    let row = row(spacer, &[download])?;
    notice.add_child_view(Some(&mut View::from(&row)));
    notice.set_visible(0);
    Some(notice)
}

/// A row with `label` on the left and `controls` on the right.
fn row(label: &LabelButton, controls: &[&LabelButton]) -> Option<Panel> {
    let row = panel_create(Some(&mut DarkPanelDelegate::new(BOX_COLOR, 0)))?;
    let layout = row.set_to_box_layout(Some(&BoxLayoutSettings {
        horizontal: 1,
        between_child_spacing: 6,
        cross_axis_alignment: AxisAlignment::CENTER,
        ..Default::default()
    }))?;
    let mut label_view = View::from(label);
    row.add_child_view(Some(&mut label_view));
    for control in controls {
        row.add_child_view(Some(&mut View::from(*control)));
    }
    layout.set_flex_for_view(Some(&mut label_view), 1);
    Some(row)
}

/// Wraps `content` in an [`EDGE`] wide frame in the border color.
fn edged(content: &Panel) -> Option<Panel> {
    let edge = panel_create(Some(&mut DarkPanelDelegate::new(BORDER_COLOR, 0)))?;
    edge.set_to_box_layout(Some(&BoxLayoutSettings {
        inside_border_insets: Insets {
            top: EDGE,
            left: EDGE,
            bottom: EDGE,
            right: EDGE,
        },
        cross_axis_alignment: AxisAlignment::STRETCH,
        ..Default::default()
    }));
    edge.add_child_view(Some(&mut View::from(content)));
    Some(edge)
}

/// A non-interactive line of text. Views has no label in the CEF API, so it is a disabled
/// label button.
fn text(win: WindowId, text: &str, font: &str, color: u32) -> Option<LabelButton> {
    let button = label_button_create(
        Some(&mut SettingsButtonDelegate::new(win, Choice::None)),
        Some(&CefString::from(text)),
    )?;
    button.set_font_list(Some(&CefString::from(font)));
    button.set_text_color(ButtonState::DISABLED, color);
    button.set_enabled(0);
    button.set_focusable(0);
    button.set_ink_drop_enabled(0);
    Some(button)
}

/// A `−` or `+` button that moves the padding by `steps`.
fn step_button(win: WindowId, label: &str, steps: i32) -> Option<LabelButton> {
    let button = label_button_create(
        Some(&mut SettingsButtonDelegate::new(
            win,
            Choice::Padding(steps),
        )),
        Some(&CefString::from(label)),
    )?;
    button.set_font_list(Some(&CefString::from(STEP_FONT)));
    button.set_enabled_text_colors(TEXT_COLOR);
    button.set_horizontal_alignment(HorizontalAlignment::CENTER);
    let size = Size {
        width: BUTTON_SIZE,
        height: BUTTON_SIZE,
    };
    button.set_minimum_size(Some(&size));
    button.set_maximum_size(Some(&size));
    button.set_focusable(1);
    button.set_accessible_name(Some(&CefString::from(if steps < 0 {
        "Decrease padding"
    } else {
        "Increase padding"
    })));
    Some(button)
}

/// The `×` in the header that closes the box.
fn close_button(win: WindowId) -> Option<LabelButton> {
    let button = label_button_create(
        Some(&mut SettingsButtonDelegate::new(win, Choice::Close)),
        Some(&CefString::from("\u{00D7}")),
    )?;
    button.set_font_list(Some(&CefString::from(CLOSE_FONT)));
    button.set_enabled_text_colors(CLOSE_TEXT_COLOR);
    button.set_text_color(ButtonState::HOVERED, TEXT_COLOR);
    button.set_text_color(ButtonState::PRESSED, TEXT_COLOR);
    button.set_horizontal_alignment(HorizontalAlignment::CENTER);
    let size = Size {
        width: CLOSE_SIZE,
        height: CLOSE_SIZE,
    };
    button.set_minimum_size(Some(&size));
    button.set_maximum_size(Some(&size));
    button.set_focusable(1);
    button.set_accessible_name(Some(&CefString::from("Close settings")));
    Some(button)
}

/// An `Auto` or `Pinned` button that sets when the top bar shows.
fn mode_button(win: WindowId, mode: TopBarMode) -> Option<LabelButton> {
    choice_button(win, mode_name(mode), Choice::TopBar(mode), MODE_WIDTH)
}

/// A `width` pixels wide button labeled `label` that makes `choice`.
fn choice_button(win: WindowId, label: &str, choice: Choice, width: i32) -> Option<LabelButton> {
    let button = label_button_create(
        Some(&mut SettingsButtonDelegate::new(win, choice)),
        Some(&CefString::from(label)),
    )?;
    button.set_font_list(Some(&CefString::from(FONT)));
    button.set_horizontal_alignment(HorizontalAlignment::CENTER);
    let size = Size {
        width,
        height: BUTTON_SIZE,
    };
    button.set_minimum_size(Some(&size));
    button.set_maximum_size(Some(&size));
    button.set_focusable(1);
    Some(button)
}

/// The notice's Download button, which opens the download page in the default browser.
fn download_button(win: WindowId) -> Option<LabelButton> {
    let button = choice_button(win, "Download", Choice::Download, DOWNLOAD_WIDTH)?;
    button.set_background_color(SELECTED_COLOR);
    button.set_enabled_text_colors(ACCENT_TEXT_COLOR);
    button.set_accessible_name(Some(&CefString::from(
        "Download the new version (opens the download page in your browser)",
    )));
    Some(button)
}

/// Marks `button` as the selected choice of its row or not, and names it `name` with that state.
fn show_selected(button: &LabelButton, selected: bool, name: &str) {
    let (background, text_color) = if selected {
        (SELECTED_COLOR, TEXT_COLOR)
    } else {
        (BOX_COLOR, DIM_TEXT_COLOR)
    };
    button.set_background_color(background);
    button.set_enabled_text_colors(text_color);
    let state = if selected { "selected" } else { "not selected" };
    button.set_accessible_name(Some(&CefString::from(format!("{name} ({state})").as_str())));
}

fn mode_name(mode: TopBarMode) -> &'static str {
    match mode {
        TopBarMode::Auto => "Auto",
        TopBarMode::Pinned => "Pinned",
    }
}

/// Highlights the button of the current top bar mode in the box of window `win`.
fn show_top_bar(win: WindowId) {
    let (Some(settings), Some(buttons)) = (
        settings(),
        with_box(win, |settings_box| {
            [
                (TopBarMode::Auto, settings_box.auto.clone()),
                (TopBarMode::Pinned, settings_box.pinned.clone()),
            ]
        }),
    ) else {
        return;
    };
    let mode = settings.top_bar;
    for (button_mode, button) in buttons {
        let name = format!("Top bar: {}", mode_name(button_mode));
        show_selected(&button, button_mode == mode, &name);
    }
}

/// Highlights `On` or `Off` for the update check in the box of window `win`.
fn show_update_check(win: WindowId) {
    let (Some(settings), Some(buttons)) = (
        settings(),
        with_box(win, |settings_box| {
            [
                (true, "On", settings_box.updates_on.clone()),
                (false, "Off", settings_box.updates_off.clone()),
            ]
        }),
    ) else {
        return;
    };
    for (enabled, label, button) in buttons {
        let name = format!("Update check: {label}");
        show_selected(&button, enabled == settings.check_for_updates, &name);
    }
}

/// Shows the notice of the available update in the box of window `win`, or hides it.
fn show_update(win: WindowId) {
    let Some((notice, notice_text)) = with_box(win, |settings_box| {
        (
            settings_box.notice.clone(),
            settings_box.notice_text.clone(),
        )
    }) else {
        return;
    };
    if let Some(version) = update::available() {
        let line = format!("mini-browser {version} is available");
        notice_text.set_text(Some(&CefString::from(line.as_str())));
        notice.set_visible(1);
    } else {
        notice.set_visible(0);
    }
    // The notice changes the box's height.
    reposition(win);
}

/// Shows the available update, or hides its notice, in every box.
pub fn show_update_everywhere() {
    for win in windows::ids() {
        show_update(win);
    }
}

/// Shows the current padding and dims the button whose direction is at its bound. The button
/// stays enabled and focusable, so the keyboard focus never jumps to the other one (which
/// would make `Space` reverse direction); pressing it does nothing.
fn show_padding(win: WindowId) {
    let (Some(settings), Some((minus, value, plus, panel))) = (
        settings(),
        with_box(win, |settings_box| {
            (
                settings_box.minus.clone(),
                settings_box.value.clone(),
                settings_box.plus.clone(),
                settings_box.panel.clone(),
            )
        }),
    ) else {
        return;
    };
    let padding = settings.padding;
    value.set_text(Some(&CefString::from(format!("{padding} px").as_str())));
    for (button, steps, name) in [
        (&minus, -1, "Decrease padding"),
        (&plus, 1, "Increase padding"),
    ] {
        let can_move = settings.can_move_padding(steps);
        button.set_enabled_text_colors(if can_move {
            TEXT_COLOR
        } else {
            BOUND_BUTTON_COLOR
        });
        let limit = if steps < 0 { PADDING_MIN } else { PADDING_MAX };
        let accessible_name = if can_move {
            name.to_owned()
        } else {
            format!("{name} (at {limit} px)")
        };
        button.set_accessible_name(Some(&CefString::from(accessible_name.as_str())));
    }
    panel.layout();
}

/// The current settings.
pub fn settings() -> Option<Settings> {
    STORE.with_borrow(|store| store.as_ref().map(|store| store.settings.clone()))
}

/// Moves the padding by `steps`, applies it to every window and saves it.
pub fn step(steps: i32) {
    let Some(settings) = change(|settings| settings.with_padding_steps(steps)) else {
        return;
    };
    for win in windows::ids() {
        window::set_padding(win, settings.padding);
        show_padding(win);
        // The warning line changes the box's height.
        reposition(win);
    }
}

/// Sets when the top bar shows, applies it to every window and saves it.
fn set_top_bar(mode: TopBarMode) {
    let Some(settings) = change(|settings| Settings {
        top_bar: mode,
        ..settings.clone()
    }) else {
        return;
    };
    for win in windows::ids() {
        top_bar::set_mode(win, settings.top_bar);
        show_top_bar(win);
        reposition(win);
    }
}

/// Turns the update check on or off and saves it; on checks right away, off hides any notice.
fn set_check_for_updates(enabled: bool) {
    let Some(settings) = change(|settings| Settings {
        check_for_updates: enabled,
        ..settings.clone()
    }) else {
        return;
    };
    for win in windows::ids() {
        show_update_check(win);
        reposition(win);
    }
    update::set_enabled(settings.check_for_updates);
}

/// Switches the top bar between auto and pinned and saves it (`Ctrl/Cmd+Shift+F`).
pub fn toggle_top_bar() {
    if let Some(settings) = settings() {
        set_top_bar(settings.top_bar.toggled());
    }
}

/// Replaces the settings with `new` of them and saves them; returns the new settings, or `None`
/// when nothing changed. A failed save is reported, shown in every box until a later save
/// succeeds, and the change still applies for this run.
fn change(new: impl FnOnce(&Settings) -> Settings) -> Option<Settings> {
    let (settings, path) = STORE.with_borrow_mut(|store| {
        let store = store.as_mut()?;
        let settings = new(&store.settings);
        if settings == store.settings {
            return None;
        }
        store.settings = settings.clone();
        Some((settings, store.path.clone()))
    })?;
    let saved = settings.save(&path);
    if let Err(error) = &saved {
        eprintln!(
            "minibrowser: cannot save settings to {}: {error}",
            path.display()
        );
    }
    STORE.with_borrow_mut(|store| {
        if let Some(store) = store.as_mut() {
            store.unsaved = saved.is_err();
        }
    });
    for win in windows::ids() {
        if let Some(not_saved) = with_box(win, |settings_box| settings_box.not_saved.clone()) {
            not_saved.set_visible(saved.is_err().into());
        }
    }
    // Open home pages show the settings in their commands card.
    home::refresh_state();
    Some(settings)
}

fn is_visible(win: WindowId) -> bool {
    with_box(win, |settings_box| settings_box.controller.clone())
        .is_some_and(|controller| controller.is_visible() == 1)
}

/// Opens the box of window `win` over the page, or closes it when it is open.
pub fn toggle(win: WindowId) {
    if is_visible(win) {
        hide(win);
        return;
    }
    let (Some(settings), Some((controller, minus, plus))) = (
        settings(),
        with_box(win, |settings_box| {
            (
                settings_box.controller.clone(),
                settings_box.minus.clone(),
                settings_box.plus.clone(),
            )
        }),
    ) else {
        return;
    };
    let at_max = !settings.can_move_padding(1);
    address_box::hide(win);
    // Theme changes reset button backgrounds, so show the selected choices again.
    show_top_bar(win);
    show_update_check(win);
    if let Some(download) = with_box(win, |settings_box| settings_box.download.clone()) {
        download.set_background_color(SELECTED_COLOR);
    }
    reposition(win);
    controller.set_visible(1);
    // Focus a button, so the window gets `Esc` and the shortcuts.
    let target = if at_max { minus } else { plus };
    target.request_focus();
}

/// Closes the box and gives the keyboard focus back to the page.
fn hide(win: WindowId) {
    if close(win) {
        tabs::focus_active_page(win);
    }
}

/// Closes the open box once the keyboard focus has left it for another view of the active
/// window, as a click on the page or on a tab moves it there. Clicks inside the box keep the
/// focus on its buttons; while the window is inactive the box stays open.
fn close_if_focus_left(win: WindowId) {
    let Some((window, buttons)) = with_box(win, |settings_box| {
        (
            settings_box.window.clone(),
            [
                settings_box.minus.clone(),
                settings_box.plus.clone(),
                settings_box.auto.clone(),
                settings_box.pinned.clone(),
                settings_box.updates_on.clone(),
                settings_box.updates_off.clone(),
                settings_box.download.clone(),
                settings_box.close.clone(),
            ],
        )
    }) else {
        return;
    };
    if !is_visible(win) || window.is_active() == 0 {
        return;
    }
    if buttons.iter().all(|button| button.has_focus() == 0) {
        close(win);
    }
}

/// Closes the open box after a left click at `point` (window coordinates) on the frame or the
/// shown top bar, which take no keyboard focus; clicks inside the box keep it open.
#[cfg(target_os = "windows")]
pub fn close_on_click(win: WindowId, point: (i32, i32)) {
    let Some(controller) = with_box(win, |settings_box| settings_box.controller.clone()) else {
        return;
    };
    let Some(page) = window::page_area(win) else {
        return;
    };
    let core_rect = |rect: Rect| layout::Rect {
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: rect.height,
    };
    let bounds = core_rect(controller.bounds());
    let bar = top_bar::shown_bounds(win).map(core_rect);
    if controller.is_visible() == 1 && layout::click_closes_box(point, bounds, page, bar) {
        hide(win);
    }
}

wrap_task! {
    struct CloseIfFocusLeft {
        win: WindowId,
    }

    impl Task {
        fn execute(&self) {
            close_if_focus_left(self.win);
        }
    }
}

/// Closes the box of window `win` without moving the focus. Returns whether the box exists.
pub fn close(win: WindowId) -> bool {
    let controller = with_box(win, |settings_box| settings_box.controller.clone());
    if let Some(controller) = &controller {
        controller.set_visible(0);
    }
    controller.is_some()
}

/// While the box is open, `+` and `-` widen and narrow the frame. Returns whether the event
/// was used.
pub fn handle_step_keys(win: WindowId, event: Option<&KeyEvent>) -> bool {
    let Some(steps) = event
        .filter(|event| event.type_ == KeyEventType::RAWKEYDOWN)
        .and_then(|event| settings::padding_step_for_key(event.windows_key_code))
    else {
        return false;
    };
    if !is_visible(win) {
        return false;
    }
    step(steps);
    true
}

/// Closes the open box on `Esc`. Returns whether the event was used.
pub fn handle_escape(win: WindowId, event: Option<&KeyEvent>) -> bool {
    let is_escape = event.is_some_and(|event| {
        event.type_ == KeyEventType::RAWKEYDOWN && event.windows_key_code == VK_ESCAPE
    });
    if is_escape && is_visible(win) {
        hide(win);
        return true;
    }
    false
}

/// Puts the box in the bottom-right corner of the page area, out of the way of search fields at
/// the top or center of pages. Call it whenever window `win` is resized.
pub fn reposition(win: WindowId) {
    let Some((controller, panel)) = with_box(win, |settings_box| {
        (settings_box.controller.clone(), settings_box.panel.clone())
    }) else {
        return;
    };
    let Some(area) = window::page_area(win) else {
        return;
    };
    let height = panel.preferred_size().height;
    let bounds = layout::corner_box(area, WIDTH, height, MARGIN);
    controller.set_bounds(Some(&window::cef_rect(bounds)));
}

/// What a settings box button changes.
#[derive(Clone, Copy)]
enum Choice {
    /// A line of text.
    None,
    /// The padding change of a `−` / `+` button.
    Padding(i32),
    /// The top bar mode of an `Auto` / `Pinned` button.
    TopBar(TopBarMode),
    /// The update check of an `On` / `Off` button.
    Updates(bool),
    /// The notice's Download button.
    Download,
    /// The `×` that closes the box.
    Close,
}

wrap_button_delegate! {
    struct SettingsButtonDelegate {
        win: WindowId,
        choice: Choice,
    }

    impl ViewDelegate {
        fn on_blur(&self, _view: Option<&mut View>) {
            // The newly focused view is known once Views is done moving the focus.
            let mut task = CloseIfFocusLeft::new(self.win);
            post_task(ThreadId::UI, Some(&mut task));
        }
    }

    impl ButtonDelegate {
        fn on_button_pressed(&self, _button: Option<&mut Button>) {
            match self.choice {
                Choice::None => {}
                Choice::Padding(steps) => step(steps),
                Choice::TopBar(mode) => set_top_bar(mode),
                Choice::Updates(enabled) => set_check_for_updates(enabled),
                Choice::Download => update::open_download_page(),
                Choice::Close => hide(self.win),
            }
        }
    }
}
