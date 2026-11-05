//! The tab strip: the top bar's row of tabs. Clicking a title activates its tab and hides the
//! auto-mode bar; the `×` of the active tab and of the hovered tab closes that tab; dragging the
//! rest of the strip moves the window. Tabs share the strip's width evenly and long titles end in
//! an ellipsis; when there are more tabs than fit, the strip shows the ones around the active tab
//! followed by how many are hidden (`+3`). A `+` button right after the last shown tab opens a
//! new tab at the end. Views reports no middle clicks or drags to button delegates
//! (`on_button_pressed` has no event flags, and buttons only react to the left button), so on
//! Windows `mouse_hook` closes a tab on a middle click and reorders tabs dragged along the strip,
//! hit-testing the labels with [`tab_at`].

use std::cell::{Cell, RefCell};

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

thread_local! {
    static STRIP: RefCell<Option<Panel>> = const { RefCell::new(None) };
    /// The views of each shown tab, in strip order.
    static BUTTONS: RefCell<Vec<TabViews>> = const { RefCell::new(Vec::new()) };
    /// The layout the labels were built for, and for how many tabs.
    static SHOWN: Cell<Option<(StripLayout, usize)>> = const { Cell::new(None) };
}

/// Creates the strip panel.
pub fn create() -> Option<Panel> {
    let panel = panel_create(Some(&mut DarkPanelDelegate::new(STRIP_COLOR, STRIP_HEIGHT)))?;
    panel.set_to_box_layout(Some(&BoxLayoutSettings {
        horizontal: 1,
        between_child_spacing: TAB_SPACING,
        cross_axis_alignment: AxisAlignment::STRETCH,
        ..Default::default()
    }));
    STRIP.set(Some(panel.clone()));
    Some(panel)
}

pub fn destroy() {
    STRIP.take();
    BUTTONS.take();
    SHOWN.take();
}

pub fn panel() -> Option<Panel> {
    STRIP.with_borrow(Clone::clone)
}

/// Where the labels, the new-tab button and the hidden-tab count end, in strip coordinates.
pub fn labels_end() -> i32 {
    let Some(panel) = panel() else {
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
pub fn tab_at(x: i32) -> Option<(TabId, i32)> {
    let (layout, _) = SHOWN.get()?;
    let index = strip::tab_at(x, &layout, TAB_SPACING)?;
    let id =
        BUTTONS.with_borrow(|buttons| buttons.get(index - layout.first).map(|views| views.id))?;
    Some((id, strip::tab_left(index, &layout, TAB_SPACING)?))
}

/// The position among the tabs of a tab dragged with the pointer at `x` (strip coordinates),
/// held `grab` pixels from its label's left edge.
#[cfg(target_os = "windows")]
pub fn drag_index(x: i32, grab: i32) -> Option<usize> {
    let (layout, _) = SHOWN.get()?;
    strip::drag_index(x, grab, &layout, TAB_SPACING)
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
pub fn fit() {
    let Some(panel) = panel() else {
        return;
    };
    let entries = tabs::strip_entries();
    if SHOWN.get() != Some((layout_for(&panel, &entries), entries.len())) {
        refresh();
    }
}

/// Rebuilds the strip from the tab model.
pub fn refresh() {
    let Some(panel) = panel() else {
        return;
    };
    panel.remove_all_child_views();
    let entries = tabs::strip_entries();
    let layout = layout_for(&panel, &entries);
    SHOWN.set(Some((layout, entries.len())));
    let mut buttons = Vec::new();
    for entry in entries.iter().skip(layout.first).take(layout.count) {
        let Some(tab) = tab_views(entry, layout.tab_width) else {
            continue;
        };
        panel.add_child_view(Some(&mut View::from(&tab.panel)));
        buttons.push(tab);
    }
    add_new_tab_button(&panel);
    if layout.hidden > 0 {
        add_hidden_count(&panel, layout.hidden);
    }
    BUTTONS.set(buttons);
    panel.layout();
    window::update_drag_regions();
}

/// Shows a new title on the label of `tab`, without rebuilding the strip.
pub fn retitle(tab: TabId, title: &str) {
    let Some(panel) = panel() else {
        return;
    };
    let Some(button) = BUTTONS.with_borrow(|buttons| {
        buttons
            .iter()
            .find(|views| views.id == tab)
            .map(|views| views.title.clone())
    }) else {
        return;
    };
    set_title(&button, title);
    panel.layout();
    window::update_drag_regions();
}

/// The panel of a tab `width` pixels wide, with its title and close button.
fn tab_views(entry: &tabs::StripEntry, width: i32) -> Option<TabViews> {
    let (background, text_color) = colors(entry.active);
    let panel = panel_create(Some(&mut DarkPanelDelegate::new(background, 0)))?;
    panel.set_to_box_layout(Some(&BoxLayoutSettings {
        horizontal: 1,
        cross_axis_alignment: AxisAlignment::STRETCH,
        ..Default::default()
    }));
    panel.set_background_color(background);
    let mut delegate = TabButtonDelegate::new(entry.id, entry.active);
    let text = CefString::from(entry.title.as_str());
    let title = label_button_create(Some(&mut delegate), Some(&text))?;
    style_button(&title, FONT, background, text_color);
    set_title(&title, &entry.title);
    let mut delegate = CloseButtonDelegate::new(entry.id, entry.active);
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
fn update_close(tab: TabId) {
    let Some(views) =
        BUTTONS.with_borrow(|buttons| buttons.iter().find(|views| views.id == tab).cloned())
    else {
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
fn update_close_later(tab: TabId) {
    let mut task = UpdateClose::new(tab);
    post_task(ThreadId::UI, Some(&mut task));
}

/// Adds the `+` button that opens a new tab at the end, like `Ctrl/Cmd+T`.
fn add_new_tab_button(panel: &Panel) {
    let Some(button) = label_button_create(
        Some(&mut NewTabDelegate::new()),
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
            // Activating rebuilds the strip, which destroys this button: do it after the press.
            let mut task = ActivateTab::new(self.tab);
            post_task(ThreadId::UI, Some(&mut task));
        }

        fn on_button_state_changed(&self, _button: Option<&mut Button>) {
            update_close_later(self.tab);
        }
    }
}

wrap_button_delegate! {
    struct CloseButtonDelegate {
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
            let mut task = CloseTab::new(self.tab);
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
            update_close_later(self.tab);
        }
    }
}

wrap_button_delegate! {
    struct NewTabDelegate {}

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
            let mut task = OpenTab::new();
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
    struct OpenTab;

    impl Task {
        fn execute(&self) {
            tabs::new_tab();
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
        tab: TabId,
    }

    impl Task {
        fn execute(&self) {
            tabs::activate(self.tab);
            top_bar::tab_chosen();
        }
    }
}

wrap_task! {
    struct CloseTab {
        tab: TabId,
    }

    impl Task {
        fn execute(&self) {
            tabs::close(self.tab);
        }
    }
}

wrap_task! {
    struct UpdateClose {
        tab: TabId,
    }

    impl Task {
        fn execute(&self) {
            update_close(self.tab);
        }
    }
}
