//! The tab strip: the top bar's row of tabs. Clicking a title activates its tab and hides the
//! auto-mode bar; the `×` of the active tab and of the hovered tab closes that tab; dragging the
//! rest of the strip moves the window. Tabs share the strip's width evenly and long titles end in
//! an ellipsis; when there are more tabs than fit, the strip shows the ones around the active tab
//! followed by how many are hidden (`+3`). A `+` button right after the last shown tab opens a
//! new tab at the end. Views reports no middle clicks or drags to button delegates
//! (`on_button_pressed` has no event flags, and buttons only react to the left button), so on
//! Windows `mouse_hook` closes a tab on a middle click, reorders tabs dragged along the strip,
//! hit-testing the labels with [`tab_at`], and moves tabs dragged out of it or onto it.

use cef::*;
#[cfg(target_os = "windows")]
use mini_core::strip;
use mini_core::{
    strip::{StripLayout, strip_layout},
    tabs::TabId,
};

use super::{
    tabs, top_bar,
    window::{self, DarkPanelDelegate},
    windows::{self, WindowId},
};

pub const STRIP_HEIGHT: i32 = 28;
pub const STRIP_COLOR: u32 = 0xFF18_1818;
const TAB_COLOR: u32 = 0xFF26_2626;
const ACTIVE_TAB_COLOR: u32 = 0xFF38_3838;
const TAB_TEXT_COLOR: u32 = 0xFF8C_8C8C;
const ACTIVE_TAB_TEXT_COLOR: u32 = 0xFFF5_F5F5;
const TAB_MIN_WIDTH: i32 = 72;
const TAB_MAX_WIDTH: i32 = 220;
const TAB_SPACING: i32 = 1;
/// Width of a tab's close button.
const CLOSE_WIDTH: i32 = 28;
const CLOSE_HOVER_COLOR: u32 = 0xFF4C_4C4C;
const CLOSE_GLYPH: &str = "\u{00D7}";
const CLOSE_FONT: &str = "Segoe UI, Helvetica, Arial, 15px";
/// Width of the hidden-tab count that follows the labels.
const HIDDEN_COUNT_WIDTH: i32 = 36;
/// Width of the new-tab button that follows the labels.
const NEW_TAB_WIDTH: i32 = 28;
const NEW_TAB_GLYPH: &str = "+";
const NEW_TAB_FONT: &str = "Segoe UI, Helvetica, Arial, 17px";
const NEW_TAB_HOVER_COLOR: u32 = 0xFF2E_2E2E;
const FONT: &str = "Segoe UI, Helvetica, Arial, 13px";

/// One shown tab: a panel holding its title and its close button.
#[derive(Clone)]
struct TabViews {
    id: TabId,
    active: bool,
    width: i32,
    panel: Panel,
    title: LabelButton,
    close: LabelButton,
}

/// A window's strip.
#[derive(Default)]
pub struct Strip {
    panel: Option<Panel>,
    /// The views of each shown tab, in strip order.
    buttons: Vec<TabViews>,
    /// The layout the labels were built for, and for how many tabs.
    shown: Option<(StripLayout, usize)>,
}

/// Runs `f` on the strip of window `win`; `None` once the window is gone.
fn with_strip<R>(win: WindowId, f: impl FnOnce(&mut Strip) -> R) -> Option<R> {
    let state = windows::get(win)?;
    let mut strip = state.strip.borrow_mut();
    Some(f(&mut strip))
}

/// The layout the labels of window `win` were built for, and for how many tabs.
fn shown(win: WindowId) -> Option<(StripLayout, usize)> {
    with_strip(win, |strip| strip.shown).flatten()
}

/// Creates the strip panel of window `win`.
pub fn create(win: WindowId) -> Option<Panel> {
    let panel = panel_create(Some(&mut DarkPanelDelegate::new(STRIP_COLOR, STRIP_HEIGHT)))?;
    panel.set_to_box_layout(Some(&BoxLayoutSettings {
        horizontal: 1,
        between_child_spacing: TAB_SPACING,
        cross_axis_alignment: AxisAlignment::STRETCH,
        ..Default::default()
    }));
    with_strip(win, |strip| strip.panel = Some(panel.clone()))?;
    Some(panel)
}

pub fn panel(win: WindowId) -> Option<Panel> {
    with_strip(win, |strip| strip.panel.clone()).flatten()
}

/// Where the labels, the new-tab button and the hidden-tab count end, in strip coordinates.
pub fn labels_end(win: WindowId) -> i32 {
    let Some(panel) = panel(win) else {
        return 0;
    };
    let count = i32::try_from(panel.child_view_count()).unwrap_or(0);
    (count > 0)
        .then(|| panel.child_view_at(count - 1))
        .flatten()
        .map_or(0, |last| last.bounds().x + last.bounds().width)
}

/// The shown tab whose label lies under `x` (strip coordinates), and where its label starts.
#[cfg(target_os = "windows")]
pub fn tab_at(win: WindowId, x: i32) -> Option<(TabId, i32)> {
    let (layout, _) = shown(win)?;
    let index = strip::tab_at(x, &layout, TAB_SPACING)?;
    let id = with_strip(win, |strip| {
        strip
            .buttons
            .get(index - layout.first)
            .map(|views| views.id)
    })
    .flatten()?;
    Some((id, strip::tab_left(index, &layout, TAB_SPACING)?))
}

/// The position among the tabs of a tab dragged with the pointer at `x` (strip coordinates),
/// held `grab` pixels from its label's left edge.
#[cfg(target_os = "windows")]
pub fn drag_index(win: WindowId, x: i32, grab: i32) -> Option<usize> {
    let (layout, _) = shown(win)?;
    strip::drag_index(x, grab, &layout, TAB_SPACING)
}

/// Where a tab dropped on the strip of window `win` goes among its tabs, for the pointer at `x`
/// (strip coordinates) holding the label `grab` pixels from its left edge.
#[cfg(target_os = "windows")]
pub fn drop_index(win: WindowId, x: i32, grab: i32) -> Option<usize> {
    let (layout, _) = shown(win)?;
    Some(strip::drop_index(x, grab, &layout, TAB_SPACING))
}

/// Where the pointer holding a tab `grab` pixels from its label's left edge sits in a window
/// with a frame `padding` pixels wide whose only tab it is, in window coordinates.
#[cfg(target_os = "windows")]
pub fn only_tab_grab(grab: i32, padding: i32) -> (i32, i32) {
    // A lone tab has the widest label, unless the window is narrower than that.
    strip::torn_tab_offset(grab, TAB_MAX_WIDTH, padding, STRIP_HEIGHT)
}

/// The label layout for `entries` in the strip's current width.
fn layout_for(panel: &Panel, entries: &[tabs::StripEntry]) -> StripLayout {
    let active = entries.iter().position(|entry| entry.active).unwrap_or(0);
    strip_layout(
        panel.bounds().width,
        entries.len(),
        active,
        TAB_MIN_WIDTH,
        TAB_MAX_WIDTH,
        TAB_SPACING,
        HIDDEN_COUNT_WIDTH,
        NEW_TAB_WIDTH,
    )
}

/// Rebuilds the labels when the strip's width no longer suits them. Call it whenever the
/// window is laid out.
pub fn fit(win: WindowId) {
    let Some(panel) = panel(win) else {
        return;
    };
    let entries = tabs::strip_entries(win);
    if shown(win) != Some((layout_for(&panel, &entries), entries.len())) {
        refresh(win);
    }
}

/// Rebuilds the strip of window `win` from its tab model.
pub fn refresh(win: WindowId) {
    let Some(panel) = panel(win) else {
        return;
    };
    panel.remove_all_child_views();
    let entries = tabs::strip_entries(win);
    let layout = layout_for(&panel, &entries);
    with_strip(win, |strip| strip.shown = Some((layout, entries.len())));
    let mut buttons = Vec::new();
    for entry in entries.iter().skip(layout.first).take(layout.count) {
        let Some(tab) = tab_views(win, entry, layout.tab_width) else {
            continue;
        };
        panel.add_child_view(Some(&mut View::from(&tab.panel)));
        buttons.push(tab);
    }
    add_new_tab_button(win, &panel);
    if layout.hidden > 0 {
        add_hidden_count(&panel, layout.hidden);
    }
    // The previous views are released after the borrow ends: releasing them calls into CEF.
    let previous = with_strip(win, |strip| std::mem::replace(&mut strip.buttons, buttons));
    drop(previous);
    panel.layout();
    window::update_drag_regions(win);
}

/// Shows a new title on the label of `tab`, without rebuilding the strip.
pub fn retitle(win: WindowId, tab: TabId, title: &str) {
    let Some(panel) = panel(win) else {
        return;
    };
    let Some(button) = views_of(win, tab).map(|views| views.title) else {
        return;
    };
    set_title(&button, title);
    panel.layout();
    window::update_drag_regions(win);
}

/// The views of `tab` while the strip of window `win` shows it.
fn views_of(win: WindowId, tab: TabId) -> Option<TabViews> {
    with_strip(win, |strip| {
        strip.buttons.iter().find(|views| views.id == tab).cloned()
    })
    .flatten()
}

/// The panel of a tab `width` pixels wide, with its title and close button.
fn tab_views(win: WindowId, entry: &tabs::StripEntry, width: i32) -> Option<TabViews> {
    let (background, text_color) = colors(entry.active);
    let panel = panel_create(Some(&mut DarkPanelDelegate::new(background, 0)))?;
    panel.set_to_box_layout(Some(&BoxLayoutSettings {
        horizontal: 1,
        cross_axis_alignment: AxisAlignment::STRETCH,
        ..Default::default()
    }));
    panel.set_background_color(background);
    let mut delegate = TabButtonDelegate::new(win, entry.id, entry.active);
    let text = CefString::from(entry.title.as_str());
    let title = label_button_create(Some(&mut delegate), Some(&text))?;
    style_button(&title, FONT, background, text_color);
    set_title(&title, &entry.title);
    let mut delegate = CloseButtonDelegate::new(win, entry.id, entry.active);
    let close = label_button_create(Some(&mut delegate), Some(&CefString::from(CLOSE_GLYPH)))?;
    style_button(&close, CLOSE_FONT, background, text_color);
    let size = Size {
        width: CLOSE_WIDTH,
        height: STRIP_HEIGHT,
    };
    close.set_minimum_size(Some(&size));
    close.set_maximum_size(Some(&size));
    close.set_tooltip_text(Some(&CefString::from("Close tab")));
    close.set_accessible_name(Some(&CefString::from("Close tab")));
    panel.add_child_view(Some(&mut View::from(&title)));
    panel.add_child_view(Some(&mut View::from(&close)));
    let tab = TabViews {
        id: entry.id,
        active: entry.active,
        width,
        panel,
        title,
        close,
    };
    show_close(&tab, entry.active);
    Some(tab)
}

fn style_button(button: &LabelButton, font: &str, background: u32, text_color: u32) {
    button.set_font_list(Some(&CefString::from(font)));
    button.set_enabled_text_colors(text_color);
    button.set_background_color(background);
    button.set_ink_drop_enabled(0);
    button.set_focusable(0);
    button.set_horizontal_alignment(HorizontalAlignment::CENTER);
}

/// Shows or hides the close button of `tab`. The title takes the rest of the tab's width, so
/// every tab keeps the same width and a longer title is elided with an ellipsis.
fn show_close(tab: &TabViews, shown: bool) {
    let close = if shown { CLOSE_WIDTH } else { 0 };
    let size = Size {
        width: (tab.width - close).max(1),
        height: STRIP_HEIGHT,
    };
    tab.title.set_minimum_size(Some(&size));
    tab.title.set_maximum_size(Some(&size));
    tab.close.set_visible(shown.into());
    tab.panel.layout();
}

/// Shows the close button of `tab` while it is active or the pointer is on its title or button.
fn update_close(win: WindowId, tab: TabId) {
    let Some(views) = views_of(win, tab) else {
        return;
    };
    let shown = views.active || lit(&views.title) || lit(&views.close);
    if (views.close.is_visible() == 1) != shown {
        show_close(&views, shown);
    }
}

fn lit(button: &LabelButton) -> bool {
    lit_state(button.state())
}

fn lit_state(state: ButtonState) -> bool {
    state == ButtonState::HOVERED || state == ButtonState::PRESSED
}

/// Checks the close button of `tab` after the current event: moving from the title to its close
/// button leaves the title before it enters the button.
fn update_close_later(win: WindowId, tab: TabId) {
    let mut task = UpdateClose::new(win, tab);
    post_task(ThreadId::UI, Some(&mut task));
}

/// Adds the `+` button that opens a new tab at the end, like `Ctrl/Cmd+T`.
fn add_new_tab_button(win: WindowId, panel: &Panel) {
    let Some(button) = label_button_create(
        Some(&mut NewTabDelegate::new(win)),
        Some(&CefString::from(NEW_TAB_GLYPH)),
    ) else {
        return;
    };
    style_button(&button, NEW_TAB_FONT, STRIP_COLOR, TAB_TEXT_COLOR);
    let size = Size {
        width: NEW_TAB_WIDTH,
        height: STRIP_HEIGHT,
    };
    button.set_minimum_size(Some(&size));
    button.set_maximum_size(Some(&size));
    button.set_tooltip_text(Some(&CefString::from("New tab")));
    button.set_accessible_name(Some(&CefString::from("New tab")));
    panel.add_child_view(Some(&mut View::from(&button)));
}

/// Adds the `+N` count of tabs that do not fit, as plain text.
fn add_hidden_count(panel: &Panel, hidden: usize) {
    let text = CefString::from(format!("+{hidden}").as_str());
    let Some(count) = label_button_create(Some(&mut HiddenCountDelegate::new()), Some(&text))
    else {
        return;
    };
    // Views has no label in the CEF API: a disabled, unfocusable button is plain text.
    count.set_font_list(Some(&CefString::from(FONT)));
    count.set_text_color(ButtonState::DISABLED, TAB_TEXT_COLOR);
    count.set_enabled(0);
    count.set_focusable(0);
    count.set_ink_drop_enabled(0);
    count.set_horizontal_alignment(HorizontalAlignment::CENTER);
    let size = Size {
        width: HIDDEN_COUNT_WIDTH,
        height: STRIP_HEIGHT,
    };
    count.set_minimum_size(Some(&size));
    count.set_maximum_size(Some(&size));
    let more = if hidden == 1 { "tab" } else { "tabs" };
    let name = format!("{hidden} more {more}; Ctrl+Tab to reach them");
    count.set_tooltip_text(Some(&CefString::from(name.as_str())));
    count.set_accessible_name(Some(&CefString::from(name.as_str())));
    panel.add_child_view(Some(&mut View::from(&count)));
}

fn set_title(button: &LabelButton, title: &str) {
    button.set_text(Some(&CefString::from(title)));
    button.set_tooltip_text(Some(&CefString::from(title)));
}

/// Background and text color of a tab label.
fn colors(active: bool) -> (u32, u32) {
    if active {
        (ACTIVE_TAB_COLOR, ACTIVE_TAB_TEXT_COLOR)
    } else {
        (TAB_COLOR, TAB_TEXT_COLOR)
    }
}

wrap_button_delegate! {
    struct TabButtonDelegate {
        win: WindowId,
        tab: TabId,
        active: bool,
    }

    impl ViewDelegate {
        fn on_theme_changed(&self, view: Option<&mut View>) {
            // Theme changes reset the background; reapply the tab color.
            if let Some(view) = view {
                view.set_background_color(colors(self.active).0);
            }
        }
    }

    impl ButtonDelegate {
        fn on_button_pressed(&self, _button: Option<&mut Button>) {
            // The release that ends a drag of this tab (its window follows the pointer, so the
            // pointer is still on it) is no click: choosing the tab would hide the auto-mode bar
            // until the pointer leaves the bar's area, and at a maximized window's top edge it
            // never does.
            #[cfg(target_os = "windows")]
            if super::mouse_hook::press_ends_drag() {
                return;
            }
            // Activating rebuilds the strip, which destroys this button: do it after the press.
            let mut task = ActivateTab::new(self.win, self.tab);
            post_task(ThreadId::UI, Some(&mut task));
        }

        fn on_button_state_changed(&self, _button: Option<&mut Button>) {
            update_close_later(self.win, self.tab);
        }
    }
}

wrap_button_delegate! {
    struct CloseButtonDelegate {
        win: WindowId,
        tab: TabId,
        active: bool,
    }

    impl ViewDelegate {
        fn on_theme_changed(&self, view: Option<&mut View>) {
            // Theme changes reset the background; reapply the tab color.
            if let Some(view) = view {
                view.set_background_color(colors(self.active).0);
            }
        }
    }

    impl ButtonDelegate {
        fn on_button_pressed(&self, _button: Option<&mut Button>) {
            // Closing rebuilds the strip, which destroys this button: do it after the press.
            let mut task = CloseTab::new(self.win, self.tab);
            post_task(ThreadId::UI, Some(&mut task));
        }

        fn on_button_state_changed(&self, button: Option<&mut Button>) {
            if let Some(button) = button {
                let color = if lit_state(button.state()) {
                    CLOSE_HOVER_COLOR
                } else {
                    colors(self.active).0
                };
                button.set_background_color(color);
            }
            update_close_later(self.win, self.tab);
        }
    }
}

wrap_button_delegate! {
    struct NewTabDelegate {
        win: WindowId,
    }

    impl ViewDelegate {
        fn on_theme_changed(&self, view: Option<&mut View>) {
            // Theme changes reset the background; reapply the strip color.
            if let Some(view) = view {
                view.set_background_color(STRIP_COLOR);
            }
        }
    }

    impl ButtonDelegate {
        fn on_button_pressed(&self, _button: Option<&mut Button>) {
            // Opening a tab rebuilds the strip, which destroys this button: do it after the press.
            let mut task = OpenTab::new(self.win);
            post_task(ThreadId::UI, Some(&mut task));
        }

        fn on_button_state_changed(&self, button: Option<&mut Button>) {
            if let Some(button) = button {
                let lit = lit_state(button.state());
                button.set_background_color(if lit { NEW_TAB_HOVER_COLOR } else { STRIP_COLOR });
            }
        }
    }
}

wrap_task! {
    struct OpenTab {
        win: WindowId,
    }

    impl Task {
        fn execute(&self) {
            tabs::new_tab(self.win);
        }
    }
}

wrap_button_delegate! {
    struct HiddenCountDelegate {}

    impl ViewDelegate {}

    impl ButtonDelegate {
        fn on_button_pressed(&self, _button: Option<&mut Button>) {}
    }
}

wrap_task! {
    struct ActivateTab {
        win: WindowId,
        tab: TabId,
    }

    impl Task {
        fn execute(&self) {
            tabs::activate(self.win, self.tab);
            top_bar::tab_chosen(self.win);
        }
    }
}

wrap_task! {
    struct CloseTab {
        win: WindowId,
        tab: TabId,
    }

    impl Task {
        fn execute(&self) {
            tabs::close(self.win, self.tab);
        }
    }
}

wrap_task! {
    struct UpdateClose {
        win: WindowId,
        tab: TabId,
    }

    impl Task {
        fn execute(&self) {
            update_close(self.win, self.tab);
        }
    }
}
