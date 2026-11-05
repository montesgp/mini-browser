//! The frameless, dark top-level window: a dark padding frame that drags the window (its top
//! side holds the loading line), the tabs' browser views inside it, and as overlays the top bar
//! (tabs and window buttons) and the address and settings boxes. Window and browser view
//! delegates are derived from the cef-rs `cefsimple` example (https://github.com/tauri-apps/cef-rs), licensed Apache-2.0 OR MIT.

use std::{cell::RefCell, path::PathBuf};

use cef::{sys::cef_color_id_t, *};
use mini_core::{layout, settings::Settings};

use super::{address_box, commands, loading_line, settings_box, tab_strip, tabs, top_bar};

/// Near-black ARGB color behind the page: window, padding frame and the browser before paint.
pub const BACKGROUND_COLOR: u32 = 0xFF11_1111;

/// The window's title, whatever page is open: share pickers and the taskbar show the app name
/// without revealing the page.
const WINDOW_TITLE: &str = "mini-browser";

/// The app icon (`assets/icon/icon.svg`) at 16x16 and 32x32 for the window icon, which CEF
/// expects at 16x16, and at 512x512 for the app icon of the taskbar and the app switcher.
const ICON_16_PNG: &[u8] = include_bytes!("../../../../assets/icon/icon-16.png");
const ICON_32_PNG: &[u8] = include_bytes!("../../../../assets/icon/icon-32.png");
const ICON_PNG: &[u8] = include_bytes!("../../../../assets/icon/icon.png");

/// Initial window size in density-independent pixels; the window opens centered.
const INITIAL_SIZE: Size = Size {
    width: 1280,
    height: 800,
};

/// The window and the view inset by the padding frame.
struct Frame {
    window: Window,
    content: View,
    /// The saved width of the frame.
    padding: u32,
    /// The width the frame has now: the saved one, or none while the window is maximized.
    inset: u32,
}

thread_local! {
    static FRAME: RefCell<Option<Frame>> = const { RefCell::new(None) };
}

/// Opens the top-level window with one tab loading `url`, or the home page when `url` is
/// `None`. `settings_path` is where the settings box saves changes.
pub fn open(client: Client, settings: Settings, settings_path: PathBuf, url: Option<String>) {
    // The window shows itself once created.
    let mut delegate = MiniWindowDelegate::new(client, settings, settings_path, url);
    window_create_top_level(Some(&mut delegate));
}

/// Shows the window in front of the others, restored if it was minimized.
pub fn bring_to_front() {
    let Some(window) = FRAME.with_borrow(|frame| frame.as_ref().map(|frame| frame.window.clone()))
    else {
        return;
    };
    if window.is_minimized() == 1 {
        window.restore();
    }
    window.show();
    window.activate();
    window.bring_to_top();
}

/// The width the padding frame has now (none while the window is maximized).
pub fn padding() -> u32 {
    FRAME.with_borrow(|frame| frame.as_ref().map_or(0, |frame| frame.inset))
}

/// The bounds of the page area, inside the padding frame and below the pinned top bar, in
/// window coordinates.
pub fn page_area() -> Option<layout::Rect> {
    let bounds = FRAME.with_borrow(|frame| frame.as_ref().map(|frame| frame.content.bounds()))?;
    Some(layout::Rect {
        x: bounds.x,
        y: bounds.y,
        width: bounds.width,
        height: bounds.height,
    })
}

/// Changes the saved width of the padding frame and lays the window out again.
pub fn set_padding(padding: u32) {
    FRAME.with_borrow_mut(|frame| {
        if let Some(frame) = frame.as_mut() {
            frame.padding = padding;
        }
    });
    fit_frame();
}

/// Where the window's views start on the screen, in density-independent pixels. On Windows
/// that is the window's client area, read from the system: CEF's own conversions are off for
/// a maximized frameless window, whose client area sits inside a frame that reaches past its
/// screen, on any screen that does not start at (0, 0).
pub fn origin_in_screen(window: &Window) -> Point {
    #[cfg(target_os = "windows")]
    if let Some(origin) = client_origin(window) {
        return origin;
    }
    let bounds = window.bounds_in_screen();
    Point {
        x: bounds.x,
        y: bounds.y,
    }
}

#[cfg(target_os = "windows")]
fn client_origin(window: &Window) -> Option<Point> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetWindowInfo, WINDOWINFO};

    // SAFETY: WINDOWINFO is plain data, valid when zeroed; `cbSize` is set before the call,
    // which fills it for this window's handle.
    let info = unsafe {
        let mut info: WINDOWINFO = std::mem::zeroed();
        info.cbSize = u32::try_from(std::mem::size_of::<WINDOWINFO>()).ok()?;
        if GetWindowInfo(window.window_handle().0.cast(), &mut info) == 0 {
            return None;
        }
        info
    };
    Some(display_convert_screen_point_from_pixels(Some(&Point {
        x: info.rcClient.left,
        y: info.rcClient.top,
    })))
}

/// How far the window's views reach past the top of its screen's work area. 0 when they fit.
pub fn overhang(window: &Window) -> i32 {
    window.display().map_or(0, |display| {
        display.work_area().y - origin_in_screen(window).y
    })
}

/// Moves the tabs' pages down by `offset` pixels without resizing them, so they do not lay out
/// and repaint again; the container cuts off their bottom. 0 puts them back.
pub fn shift_pages(offset: i32) {
    let Some(container) = FRAME.with_borrow(|frame| frame.as_ref()?.content.as_panel()) else {
        return;
    };
    container.set_to_box_layout(Some(&pages_layout(offset)));
    container.layout();
}

/// Box layout that gives the visible page the container's size, `offset` pixels down. The
/// negative bottom inset keeps the page's height while it moves.
fn pages_layout(offset: i32) -> BoxLayoutSettings {
    BoxLayoutSettings {
        inside_border_insets: Insets {
            top: offset,
            bottom: -offset,
            ..Default::default()
        },
        cross_axis_alignment: AxisAlignment::STRETCH,
        default_flex: 1,
        ..Default::default()
    }
}

/// Gives the frame the width it has for the window's state: the saved one while the window is
/// restored, none while it is maximized. Call it whenever the window's bounds change.
fn fit_frame() {
    let Some((window, mut content, inset)) = FRAME.with_borrow_mut(|frame| {
        let frame = frame.as_mut()?;
        let maximized = frame.window.is_maximized() == 1;
        let overhang = if maximized {
            overhang(&frame.window)
        } else {
            0
        };
        let inset = layout::frame_padding(frame.padding, maximized, overhang);
        if inset == frame.inset {
            return None;
        }
        frame.inset = inset;
        Some((frame.window.clone(), frame.content.clone(), inset))
    }) else {
        return;
    };
    loading_line::set_height(inset);
    apply_layout(&window, &mut content, inset);
    window.layout();
    update_drag_regions();
}

wrap_task! {
    struct FitFrame;

    impl Task {
        fn execute(&self) {
            fit_frame();
        }
    }
}

/// Sets the window's layout: `content` inset by `padding` and stretched over the rest.
fn apply_layout(window: &Window, content: &mut View, padding: u32) {
    if let Some(layout) = window.set_to_box_layout(Some(&padded_layout(padding))) {
        layout.set_flex_for_view(Some(content), 1);
    }
}

/// Makes the padding frame, and the shown top bar outside its tabs and buttons, drag the
/// window. Call it whenever the window size, the padding or the top bar changes.
pub fn update_drag_regions() {
    let Some((window, padding)) = FRAME.with_borrow(|frame| {
        frame
            .as_ref()
            .map(|frame| (frame.window.clone(), frame.inset))
    }) else {
        return;
    };
    let size = window.size();
    let frame = layout::frame_rects(size.width, size.height, i32::try_from(padding).unwrap_or(0));
    let mut regions: Vec<DraggableRegion> = frame
        .into_iter()
        .map(cef_rect)
        .chain(top_bar::empty_area())
        .map(|bounds| DraggableRegion {
            bounds,
            draggable: 1,
        })
        .collect();
    regions.retain(|region| region.bounds.width > 0 && region.bounds.height > 0);
    window.set_draggable_regions(Some(&regions));
}

/// Runs [`update_drag_regions`] once CEF has finished its current work. CEF updates the window's
/// draggable regions from the page as it loads, which can cover the top of the frame.
pub fn update_drag_regions_later() {
    let mut task = UpdateDragRegions::new();
    post_task(ThreadId::UI, Some(&mut task));
}

wrap_task! {
    struct UpdateDragRegions;

    impl Task {
        fn execute(&self) {
            update_drag_regions();
        }
    }
}

/// Converts a core rectangle to a CEF one.
pub fn cef_rect(rect: layout::Rect) -> Rect {
    Rect {
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: rect.height,
    }
}

/// Settings for every tab's browser.
pub fn browser_settings() -> BrowserSettings {
    BrowserSettings {
        background_color: BACKGROUND_COLOR,
        ..Default::default()
    }
}

/// A CEF image with one PNG per scale factor, or `None` if CEF cannot decode one of them.
fn icon_image(pngs: &[(f32, &[u8])]) -> Option<Image> {
    let icon = image_create()?;
    pngs.iter()
        .all(|(scale, png)| icon.add_png(*scale, Some(png)) == 1)
        .then_some(icon)
}

/// Box layout that insets the content by `padding` pixels on the left, right and bottom and
/// stretches it to fill the rest of the window. The top side of the frame is the loading line's
/// band, itself `padding` pixels tall.
fn padded_layout(padding: u32) -> BoxLayoutSettings {
    let padding = i32::try_from(padding).unwrap_or(i32::MAX);
    BoxLayoutSettings {
        inside_border_insets: Insets {
            top: 0,
            left: padding,
            bottom: padding,
            right: padding,
        },
        cross_axis_alignment: AxisAlignment::STRETCH,
        ..Default::default()
    }
}

/// Theme colors that keep the address box dark. The field's outline and focus ring are its
/// edge; Views keeps focus rings at a minimum contrast with the field, so they cannot be hidden.
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
        settings_path: PathBuf,
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
            settings_box::reposition();
            top_bar::reposition();
            tab_strip::fit();
            update_drag_regions();
        }
    }

    impl PanelDelegate {}

    impl WindowDelegate {
        fn on_window_bounds_changed(&self, _window: Option<&mut Window>, _new_bounds: Option<&Rect>) {
            // Maximizing or restoring the window hides or shows the frame; lay the window out
            // once CEF is done with the bounds change.
            let mut task = FitFrame::new();
            post_task(ThreadId::UI, Some(&mut task));
        }

        fn on_window_created(&self, window: Option<&mut Window>) {
            let Some(window) = window else {
                return;
            };
            let Some(container) = panel_create(Some(&mut DarkPanelDelegate::new(BACKGROUND_COLOR, 0)))
            else {
                return;
            };
            container.set_to_box_layout(Some(&pages_layout(0)));

            window.set_title(Some(&CefString::from(WINDOW_TITLE)));
            if let Some(mut icon) = icon_image(&[(1.0, ICON_16_PNG), (2.0, ICON_32_PNG)]) {
                window.set_window_icon(Some(&mut icon));
            }
            if let Some(mut icon) = icon_image(&[(1.0, ICON_PNG)]) {
                window.set_window_app_icon(Some(&mut icon));
            }
            window.set_background_color(BACKGROUND_COLOR);
            if let Some(band) = loading_line::create(self.settings.padding) {
                window.add_child_view(Some(&mut View::from(&band)));
            }
            // The bar is the first overlay, so the others stay above it; its spacer holds the
            // page below the pinned bar.
            if let Some(spacer) = top_bar::create(window, self.settings.top_bar) {
                window.add_child_view(Some(&mut View::from(&spacer)));
            }
            let mut container_view = View::from(&container);
            window.add_child_view(Some(&mut container_view));
            apply_layout(window, &mut container_view, self.settings.padding);
            FRAME.set(Some(Frame {
                window: window.clone(),
                content: container_view,
                padding: self.settings.padding,
                inset: self.settings.padding,
            }));
            // Added last so they stay above the other views.
            address_box::create(window, &self.settings.search_template);
            settings_box::create(window, self.settings.clone(), self.settings_path.clone());

            tabs::attach(window.clone(), container, self.client.clone());
            window.center_window(Some(&INITIAL_SIZE));
            window.show();
            tabs::open_tab(self.url.as_deref());
        }

        fn on_window_destroyed(&self, _window: Option<&mut Window>) {
            tabs::detach();
            address_box::destroy();
            settings_box::destroy();
            top_bar::destroy();
            tab_strip::destroy();
            loading_line::destroy();
            FRAME.take();
        }

        fn can_close(&self, _window: Option<&mut Window>) -> i32 {
            // Allow the window to close once every tab's browser says it's OK. Ask all of them,
            // so each one starts closing; the last one closes the window instead of returning
            // home.
            tabs::start_quitting();
            let ready = tabs::browsers().iter().fold(true, |ready, browser| {
                let closed = browser.host().is_none_or(|host| host.try_close_browser() == 1);
                ready && closed
            });
            ready.into()
        }

        fn on_key_event(&self, _window: Option<&mut Window>, event: Option<&KeyEvent>) -> i32 {
            (settings_box::handle_escape(event)
                || settings_box::handle_step_keys(event)
                || commands::handle_event(event))
                .into()
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
            // CEF ignores a size with a zero side and falls back to the layout's own preferred
            // size, so a fixed height needs a non-zero width; the window stretches it anyway.
            Size {
                width: i32::from(self.height != 0),
                height: self.height,
            }
        }

        fn height_for_width(&self, _view: Option<&mut View>, _width: i32) -> i32 {
            // The window's box layout asks for the height this way, which otherwise comes from
            // the panel's own layout (its tallest child). 0 keeps that default.
            self.height
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
