//! The tab strip: a thin dark row of tab titles at the top of the window, hidden by default.
//! Clicking a title activates its tab; dragging the rest of the strip moves the window.

use std::cell::RefCell;

use cef::*;
use mini_core::tabs::TabId;

use super::{
    tabs,
    window::{self, DarkPanelDelegate},
};

pub const STRIP_HEIGHT: i32 = 28;
const STRIP_COLOR: u32 = 0xFF18_1818;
const TAB_COLOR: u32 = 0xFF20_2020;
const ACTIVE_TAB_COLOR: u32 = 0xFF33_3333;
const TAB_TEXT_COLOR: u32 = 0xFF8C_8C8C;
const ACTIVE_TAB_TEXT_COLOR: u32 = 0xFFF5_F5F5;
const TAB_MAX_WIDTH: i32 = 220;
/// Longer titles are cut and end in an ellipsis.
const TITLE_MAX_CHARS: usize = 30;

thread_local! {
    static STRIP: RefCell<Option<Panel>> = const { RefCell::new(None) };
    /// The strip's label for each tab, in strip order.
    static BUTTONS: RefCell<Vec<(TabId, LabelButton)>> = const { RefCell::new(Vec::new()) };
}

/// Creates the (hidden) strip panel.
pub fn create() -> Option<Panel> {
    let panel = panel_create(Some(&mut DarkPanelDelegate::new(STRIP_COLOR, STRIP_HEIGHT)))?;
    panel.set_to_box_layout(Some(&BoxLayoutSettings {
        horizontal: 1,
        between_child_spacing: 1,
        cross_axis_alignment: AxisAlignment::STRETCH,
        ..Default::default()
    }));
    panel.set_visible(0);
    STRIP.set(Some(panel.clone()));
    Some(panel)
}

pub fn destroy() {
    STRIP.take();
    BUTTONS.take();
}

fn panel() -> Option<Panel> {
    STRIP.with_borrow(Clone::clone)
}

pub fn toggle() {
    let Some(panel) = panel() else {
        return;
    };
    let visible = panel.is_visible() == 0;
    panel.set_visible(visible.into());
    refresh();
    if let Some(parent) = panel.parent_view().and_then(|parent| parent.as_panel()) {
        parent.layout();
    }
    window::update_drag_regions();
}

/// The part of the visible strip right of the tab labels, in window coordinates.
pub fn empty_area() -> Option<Rect> {
    let panel = panel().filter(|panel| panel.is_visible() == 1)?;
    let bounds = panel.bounds();
    let count = i32::try_from(panel.child_view_count()).unwrap_or(0);
    let labels_end = (count > 0)
        .then(|| panel.child_view_at(count - 1))
        .flatten()
        .map_or(0, |last| last.bounds().x + last.bounds().width);
    Some(Rect {
        x: bounds.x + labels_end,
        width: bounds.width - labels_end,
        ..bounds
    })
}

/// Rebuilds the strip from the tab model. Does nothing while the strip is hidden.
pub fn refresh() {
    let Some(panel) = panel().filter(|panel| panel.is_visible() == 1) else {
        return;
    };
    panel.remove_all_child_views();
    let mut buttons = Vec::new();
    for entry in tabs::strip_entries() {
        let mut delegate = TabButtonDelegate::new(entry.id, entry.active);
        let text = CefString::from(short_title(&entry.title).as_str());
        let Some(button) = label_button_create(Some(&mut delegate), Some(&text)) else {
            continue;
        };
        let (background, text_color) = colors(entry.active);
        button.set_enabled_text_colors(text_color);
        button.set_background_color(background);
        button.set_ink_drop_enabled(0);
        button.set_focusable(0);
        button.set_horizontal_alignment(HorizontalAlignment::CENTER);
        button.set_maximum_size(Some(&Size {
            width: TAB_MAX_WIDTH,
            height: STRIP_HEIGHT,
        }));
        set_title(&button, &entry.title);
        panel.add_child_view(Some(&mut View::from(&button)));
        buttons.push((entry.id, button));
    }
    BUTTONS.set(buttons);
    panel.layout();
    window::update_drag_regions();
}

/// Shows a new title on the label of `tab`, without rebuilding the strip.
pub fn retitle(tab: TabId, title: &str) {
    let Some(panel) = panel().filter(|panel| panel.is_visible() == 1) else {
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

fn set_title(button: &LabelButton, title: &str) {
    button.set_text(Some(&CefString::from(short_title(title).as_str())));
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

fn short_title(title: &str) -> String {
    if title.chars().count() <= TITLE_MAX_CHARS {
        return title.to_owned();
    }
    let mut short: String = title.chars().take(TITLE_MAX_CHARS - 1).collect();
    short.push('…');
    short
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
