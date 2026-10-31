//! The frameless, dark top-level window: the tab strip on top, the tabs' browser views below and
//! the address box as an overlay. Window and browser view delegates are derived from the cef-rs
//! `cefsimple` example (https://github.com/tauri-apps/cef-rs), licensed Apache-2.0 OR MIT.

use cef::{sys::cef_color_id_t, *};
use mini_core::settings::{PADDING_DEFAULT, Settings};

use super::{address_box, commands, tab_strip, tabs};

/// Near-black ARGB color behind the page: window, padding frame and the browser before paint.
pub const BACKGROUND_COLOR: u32 = 0xFF11_1111;

/// Initial window size in density-independent pixels; the window opens centered.
const INITIAL_SIZE: Size = Size {
    width: 1280,
    height: 800,
};

/// Opens the top-level window with one tab loading `url`, or with an empty tab showing the
/// address box when `url` is `None`.
pub fn open(client: Client, settings: Settings, url: Option<String>) {
    // The window shows itself once created.
    let mut delegate = MiniWindowDelegate::new(client, settings, url);
    window_create_top_level(Some(&mut delegate));
}

/// Settings for every tab's browser.
pub fn browser_settings() -> BrowserSettings {
    BrowserSettings {
        background_color: BACKGROUND_COLOR,
        ..Default::default()
    }
}

/// Box layout that insets the content by `padding` pixels on every side and stretches it to
/// fill the rest of the window.
fn padded_layout(padding: u32) -> BoxLayoutSettings {
    let padding = i32::try_from(padding).unwrap_or(i32::MAX);
    BoxLayoutSettings {
        inside_border_insets: Insets {
            top: padding,
            left: padding,
            bottom: padding,
            right: padding,
        },
        cross_axis_alignment: AxisAlignment::STRETCH,
        ..Default::default()
    }
}

/// Theme colors that keep the address box dark.
const THEME_COLORS: [(cef_color_id_t, u32); 8] = [
    (
        cef_color_id_t::CEF_ColorTextfieldBackground,
        address_box::BOX_COLOR,
    ),
    (
        cef_color_id_t::CEF_ColorTextfieldForeground,
        address_box::TEXT_COLOR,
    ),
    (
        cef_color_id_t::CEF_ColorTextfieldForegroundPlaceholder,
        address_box::PLACEHOLDER_COLOR,
    ),
    (
        cef_color_id_t::CEF_ColorTextfieldOutline,
        address_box::BORDER_COLOR,
    ),
    (
        cef_color_id_t::CEF_ColorTextfieldSelectionBackground,
        address_box::SELECTION_COLOR,
    ),
    (
        cef_color_id_t::CEF_ColorTextfieldSelectionForeground,
        address_box::TEXT_COLOR,
    ),
    (
        cef_color_id_t::CEF_ColorFocusableBorderFocused,
        address_box::BORDER_COLOR,
    ),
    (
        cef_color_id_t::CEF_ColorFocusableBorderUnfocused,
        address_box::BORDER_COLOR,
    ),
];

wrap_window_delegate! {
    struct MiniWindowDelegate {
        client: Client,
        settings: Settings,
        url: Option<String>,
    }

    impl ViewDelegate {
        fn preferred_size(&self, _view: Option<&mut View>) -> Size {
            INITIAL_SIZE
        }

        fn on_theme_changed(&self, view: Option<&mut View>) {
            // Every theme change (including the Chrome theme applied when the window is created)
            // resets the background, so reapply the dark color here.
            if let Some(view) = view {
                view.set_background_color(BACKGROUND_COLOR);
            }
        }

        fn on_layout_changed(&self, _view: Option<&mut View>, _new_bounds: Option<&Rect>) {
            address_box::reposition();
        }
    }

    impl PanelDelegate {}

    impl WindowDelegate {
        fn on_window_created(&self, window: Option<&mut Window>) {
            let Some(window) = window else {
                return;
            };
            let Some(container) = panel_create(Some(&mut DarkPanelDelegate::new(BACKGROUND_COLOR, 0)))
            else {
                return;
            };
            container.set_to_fill_layout();

            window.set_background_color(BACKGROUND_COLOR);
            let layout = window.set_to_box_layout(Some(&padded_layout(PADDING_DEFAULT)));
            if let Some(strip) = tab_strip::create() {
                window.add_child_view(Some(&mut View::from(&strip)));
            }
            let mut container_view = View::from(&container);
            window.add_child_view(Some(&mut container_view));
            if let Some(layout) = layout {
                layout.set_flex_for_view(Some(&mut container_view), 1);
            }
            // Added last so it stays above the other views.
            address_box::create(window, &self.settings.search_template);

            tabs::attach(window.clone(), container, self.client.clone());
            window.center_window(Some(&INITIAL_SIZE));
            window.show();
            tabs::open_tab(self.url.as_deref());
        }

        fn on_window_destroyed(&self, _window: Option<&mut Window>) {
            tabs::detach();
            address_box::destroy();
            tab_strip::destroy();
        }

        fn can_close(&self, _window: Option<&mut Window>) -> i32 {
            // Allow the window to close once every tab's browser says it's OK. Ask all of them,
            // so each one starts closing.
            let ready = tabs::browsers().iter().fold(true, |ready, browser| {
                let closed = browser.host().is_none_or(|host| host.try_close_browser() == 1);
                ready && closed
            });
            ready.into()
        }

        fn on_key_event(&self, _window: Option<&mut Window>, event: Option<&KeyEvent>) -> i32 {
            commands::handle_event(event).into()
        }

        fn on_theme_colors_changed(&self, window: Option<&mut Window>, _chrome_theme: i32) {
            if let Some(window) = window {
                for (id, color) in THEME_COLORS {
                    window.set_theme_color(id as i32, color);
                }
            }
        }

        fn is_frameless(&self, _window: Option<&mut Window>) -> i32 {
            1
        }

        fn with_standard_window_buttons(&self, _window: Option<&mut Window>) -> i32 {
            0
        }

        fn can_resize(&self, _window: Option<&mut Window>) -> i32 {
            1
        }

        fn can_maximize(&self, _window: Option<&mut Window>) -> i32 {
            1
        }

        fn can_minimize(&self, _window: Option<&mut Window>) -> i32 {
            1
        }

        fn window_runtime_style(&self) -> RuntimeStyle {
            RuntimeStyle::DEFAULT
        }
    }
}

// A panel with a fixed dark background and, when `height` is not zero, a fixed height.
wrap_panel_delegate! {
    pub struct DarkPanelDelegate {
        color: u32,
        height: i32,
    }

    impl ViewDelegate {
        fn preferred_size(&self, _view: Option<&mut View>) -> Size {
            // An empty size keeps the layout's own preferred size.
            Size {
                width: 0,
                height: self.height,
            }
        }

        fn on_theme_changed(&self, view: Option<&mut View>) {
            // Theme changes reset the background; reapply the dark color.
            if let Some(view) = view {
                view.set_background_color(self.color);
            }
        }
    }

    impl PanelDelegate {}
}

wrap_browser_view_delegate! {
    pub struct MiniBrowserViewDelegate {}

    impl ViewDelegate {}

    impl BrowserViewDelegate {
        fn delegate_for_popup_browser_view(
            &self,
            _browser_view: Option<&mut BrowserView>,
            _settings: Option<&BrowserSettings>,
            _client: Option<&mut Client>,
            _is_devtools: i32,
        ) -> Option<BrowserViewDelegate> {
            Some(MiniBrowserViewDelegate::new())
        }

        fn on_popup_browser_view_created(
            &self,
            _browser_view: Option<&mut BrowserView>,
            popup_browser_view: Option<&mut BrowserView>,
            is_devtools: i32,
        ) -> i32 {
            // Pages' popups (`window.open`, `target=_blank`) become tabs of this window;
            // DevTools keeps its default window.
            match popup_browser_view {
                Some(popup) if is_devtools == 0 => {
                    tabs::adopt_popup(popup.clone());
                    1
                }
                _ => 0,
            }
        }

        fn chrome_toolbar_type(&self, _browser_view: Option<&mut BrowserView>) -> ChromeToolbarType {
            ChromeToolbarType::NONE
        }

        fn browser_runtime_style(&self) -> RuntimeStyle {
            // A Chrome style window hosts at most one Chrome style browser view but any number
            // of Alloy style ones, and every tab is a browser view. Popups must use the style
            // of their opener, which this delegate also provides.
            RuntimeStyle::ALLOY
        }
    }
}
