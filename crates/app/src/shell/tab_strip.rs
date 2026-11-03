//! The tab strip: the top bar's row of tab titles. Clicking a title activates its tab; dragging
//! the rest of the strip moves the window. Labels share the strip's width evenly and long titles
//! end in an ellipsis; when there are more tabs than fit, the strip shows the ones around the
//! active tab followed by how many are hidden (`+3`).

use std::cell::{Cell, RefCell};

use cef::*;
use mini_core::{
    strip::{StripLayout, strip_layout},
    tabs::TabId,
};

use super::{
    tabs,
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
/// Width of the hidden-tab count that follows the labels.
const HIDDEN_COUNT_WIDTH: i32 = 36;
const FONT: &str = "Segoe UI, Helvetica, Arial, 13px";

thread_local! {
    static STRIP: RefCell<Option<Panel>> = const { RefCell::new(None) };
    /// The strip's label for each shown tab, in strip order.
    static BUTTONS: RefCell<Vec<(TabId, LabelButton)>> = const { RefCell::new(Vec::new()) };
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

/// Where the labels and the hidden-tab count end, in strip coordinates.
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
    let size = Size {
        width: layout.tab_width,
        height: STRIP_HEIGHT,
    };
    let mut buttons = Vec::new();
    for entry in entries.iter().skip(layout.first).take(layout.count) {
        let mut delegate = TabButtonDelegate::new(entry.id, entry.active);
        let text = CefString::from(entry.title.as_str());
        let Some(button) = label_button_create(Some(&mut delegate), Some(&text)) else {
            continue;
        };
        let (background, text_color) = colors(entry.active);
        button.set_font_list(Some(&CefString::from(FONT)));
        button.set_enabled_text_colors(text_color);
        button.set_background_color(background);
        button.set_ink_drop_enabled(0);
        button.set_focusable(0);
        button.set_horizontal_alignment(HorizontalAlignment::CENTER);
        // Every label gets the same width; a longer title is elided with an ellipsis.
        button.set_minimum_size(Some(&size));
        button.set_maximum_size(Some(&size));
        set_title(&button, &entry.title);
        panel.add_child_view(Some(&mut View::from(&button)));
        buttons.push((entry.id, button));
    }
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
            .find(|(id, _)| *id == tab)
            .map(|(_, button)| button.clone())
    }) else {
        return;
    };
    set_title(&button, title);
    panel.layout();
    window::update_drag_regions();
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
        }
    }
}
