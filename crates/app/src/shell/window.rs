//! The frameless, dark top-level window: a dark padding frame that drags the window (its top
//! side holds the loading line), the tabs' browser views inside it, and as overlays the top bar
//! (tabs and window buttons) and the address and settings boxes. Window and browser view
//! delegates are derived from the cef-rs `cefsimple` example (https://github.com/tauri-apps/cef-rs), licensed Apache-2.0 OR MIT.

use cef::{sys::cef_color_id_t, *};
use mini_core::{layout, tabs::Tab};

use super::{
    address_box, commands, loading_line, settings_box, tab_strip, tabs, top_bar,
    windows::{self, WindowId},
};

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
pub const INITIAL_SIZE: Size = Size {
    width: 1280,
    height: 800,
};

/// How far right and down from its source window a window opened by moving a tab sits.
const MOVED_WINDOW_OFFSET: i32 = 32;

/// A tab moving into a new window: its entry and live browser view, and the window's bounds on
/// the screen.
pub struct Incoming {
    pub tab: Tab,
    pub view: BrowserView,
    pub bounds: Rect,
}

/// A window opened for a moved tab that could not be built gives the tab back to its source
/// window. The window is never shown and counts as no running window.
fn give_back(state: &windows::WindowState) {
    if let (Some(incoming), Some(origin)) = (state.incoming.take(), state.origin.get()) {
        tabs::adopt_moved(origin, incoming.tab, incoming.view);
    }
}

/// The window and the view inset by the padding frame.
pub struct Frame {
    window: Window,
    content: View,
    /// The saved width of the frame.
    padding: u32,
    /// The width the frame has now: the saved one, or none while the window is maximized.
    inset: u32,
}

/// Runs `f` on the frame of window `win`; `None` before the window is created or once it is gone.
fn with_frame<R>(win: WindowId, f: impl FnOnce(&mut Frame) -> R) -> Option<R> {
    let state = windows::get(win)?;
    let mut frame = state.frame.borrow_mut();
    frame.as_mut().map(f)
}

/// Opens a top-level window with one tab loading `url`, or the home page when `url` is `None`,
/// with the current settings (`settings_box::settings`).
pub fn open(client: Client, url: Option<String>) {
    let win = windows::add();
    // The window shows itself once created.
    let mut delegate = MiniWindowDelegate::new(win, client, url);
    if window_create_top_level(Some(&mut delegate)).is_none() {
        windows::remove(win);
    }
}

/// Opens a top-level window holding the tab `incoming` moved from window `origin`, where the
/// window's tabs return, and returns it. When the window cannot be created, gives `incoming`
/// back, or nothing when the failed window already returned the tab to `origin`.
pub fn open_moved(
    client: Client,
    origin: WindowId,
    incoming: Incoming,
) -> Result<WindowId, Option<Incoming>> {
    let win = windows::add();
    let Some(state) = windows::get(win) else {
        return Err(Some(incoming));
    };
    state.origin.set(Some(origin));
    state.incoming.replace(Some(incoming));
    // The window shows itself and adds the tab once created.
    let mut delegate = MiniWindowDelegate::new(win, client, None);
    if window_create_top_level(Some(&mut delegate)).is_some() {
        return Ok(win);
    }
    let incoming = state.incoming.take();
    windows::remove(win);
    Err(incoming)
}

/// Where a window opened from `window` goes on the screen: a little right of and below it, with
/// its size, inside its screen's work area.
pub fn next_to(window: &Window) -> Rect {
    let source = core_rect(&window.bounds_in_screen());
    let work_area = window
        .display()
        .map_or(source, |display| core_rect(&display.work_area()));
    cef_rect(layout::offset_window(
        source,
        MOVED_WINDOW_OFFSET,
        work_area,
    ))
}

fn core_rect(rect: &Rect) -> layout::Rect {
    layout::Rect {
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: rect.height,
    }
}

/// The top-level window `win`.
pub fn window(win: WindowId) -> Option<Window> {
    with_frame(win, |frame| frame.window.clone())
}

/// Shows window `win` in front of the others, restored if it was minimized.
pub fn bring_to_front(win: WindowId) {
    let Some(window) = window(win) else {
        return;
    };
    if window.is_minimized() == 1 {
        window.restore();
    }
    window.show();
    window.activate();
    window.bring_to_top();
}

/// The width the padding frame of window `win` has now (none while it is maximized).
pub fn padding(win: WindowId) -> u32 {
    with_frame(win, |frame| frame.inset).unwrap_or(0)
}

/// The bounds of the page area, inside the padding frame and below the pinned top bar, in
/// window coordinates.
pub fn page_area(win: WindowId) -> Option<layout::Rect> {
    let bounds = with_frame(win, |frame| frame.content.bounds())?;
    Some(layout::Rect {
        x: bounds.x,
        y: bounds.y,
        width: bounds.width,
        height: bounds.height,
    })
}

/// Changes the saved width of the padding frame of window `win` and lays it out again.
pub fn set_padding(win: WindowId, padding: u32) {
    with_frame(win, |frame| frame.padding = padding);
    fit_frame(win);
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
pub fn shift_pages(win: WindowId, offset: i32) {
    let Some(container) = with_frame(win, |frame| frame.content.as_panel()).flatten() else {
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
fn fit_frame(win: WindowId) {
    let Some((window, mut content, inset)) = with_frame(win, |frame| {
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
    })
    .flatten() else {
        return;
    };
    loading_line::set_height(win, inset);
    apply_layout(&window, &mut content, inset);
    window.layout();
    update_drag_regions(win);
}

wrap_task! {
    struct FitFrame {
        win: WindowId,
    }

    impl Task {
        fn execute(&self) {
            fit_frame(self.win);
        }
    }
}

/// Sets the window's layout: `content` inset by `padding` and stretched over the rest.
fn apply_layout(window: &Window, content: &mut View, padding: u32) {
    if let Some(layout) = window.set_to_box_layout(Some(&padded_layout(padding))) {
        layout.set_flex_for_view(Some(content), 1);
    }
}

/// Makes the padding frame, and the shown top bar outside its tabs and buttons, drag window
/// `win`. Call it whenever the window size, the padding or the top bar changes.
pub fn update_drag_regions(win: WindowId) {
    let Some((window, padding)) = with_frame(win, |frame| (frame.window.clone(), frame.inset))
    else {
        return;
    };
    let size = window.size();
    let frame = layout::frame_rects(size.width, size.height, i32::try_from(padding).unwrap_or(0));
    let mut regions: Vec<DraggableRegion> = frame
        .into_iter()
        .map(cef_rect)
        .chain(top_bar::empty_area(win))
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
pub fn update_drag_regions_later(win: WindowId) {
    let mut task = UpdateDragRegions::new(win);
    post_task(ThreadId::UI, Some(&mut task));
}

wrap_task! {
    struct UpdateDragRegions {
        win: WindowId,
    }

    impl Task {
        fn execute(&self) {
            update_drag_regions(self.win);
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

// `win` is the window's entry in the registry.
wrap_window_delegate! {
    struct MiniWindowDelegate {
        win: WindowId,
        client: Client,
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
            address_box::reposition(self.win);
            settings_box::reposition(self.win);
            top_bar::reposition(self.win);
            tab_strip::fit(self.win);
            update_drag_regions(self.win);
        }
    }

    impl PanelDelegate {}

    impl WindowDelegate {
        fn on_window_bounds_changed(&self, _window: Option<&mut Window>, _new_bounds: Option<&Rect>) {
            // Maximizing or restoring the window hides or shows the frame; lay the window out
            // once CEF is done with the bounds change.
            let mut task = FitFrame::new(self.win);
            post_task(ThreadId::UI, Some(&mut task));
        }

        fn on_window_created(&self, window: Option<&mut Window>) {
            let (Some(window), Some(state)) = (window, windows::get(self.win)) else {
                return;
            };
            let Some(settings) = settings_box::settings() else {
                give_back(&state);
                return;
            };
            let win = self.win;
            let Some(container) = panel_create(Some(&mut DarkPanelDelegate::new(BACKGROUND_COLOR, 0)))
            else {
                give_back(&state);
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
            if let Some(band) = loading_line::create(win, settings.padding) {
                window.add_child_view(Some(&mut View::from(&band)));
            }
            // The bar is the first overlay, so the others stay above it; its spacer holds the
            // page below the pinned bar.
            if let Some(spacer) = top_bar::create(win, window, settings.top_bar) {
                window.add_child_view(Some(&mut View::from(&spacer)));
            }
            let mut container_view = View::from(&container);
            window.add_child_view(Some(&mut container_view));
            apply_layout(window, &mut container_view, settings.padding);
            state.frame.replace(Some(Frame {
                window: window.clone(),
                content: container_view,
                padding: settings.padding,
                inset: settings.padding,
            }));
            // Added last so they stay above the other views.
            address_box::create(win, window, &settings.search_template);
            settings_box::create(win, window);

            tabs::attach(win, window.clone(), container, self.client.clone());
            // Views reports no middle clicks or drags; on Windows a hook on this thread does.
            #[cfg(target_os = "windows")]
            super::mouse_hook::install(win, window);
            // A window opened by moving a tab sits next to its source and shows that tab.
            let incoming = state.incoming.take();
            match incoming {
                Some(incoming) => {
                    window.set_bounds(Some(&incoming.bounds));
                    window.show();
                    window.activate();
                    tabs::adopt_moved(win, incoming.tab, incoming.view);
                    tabs::focus_active_page_later(win);
                }
                None => {
                    window.center_window(Some(&INITIAL_SIZE));
                    window.show();
                    tabs::open_tab(win, self.url.as_deref());
                }
            }
        }

        fn on_window_destroyed(&self, _window: Option<&mut Window>) {
            #[cfg(target_os = "windows")]
            super::mouse_hook::uninstall(self.win);
            // The window's state (tabs, bar, boxes, frame) goes with its entry.
            windows::remove(self.win);
        }

        fn on_window_activation_changed(&self, _window: Option<&mut Window>, active: i32) {
            if active == 1 {
                windows::activate(self.win);
            }
        }

        fn can_close(&self, _window: Option<&mut Window>) -> i32 {
            // Allow the window to close once every tab's browser says it's OK. Ask all of them,
            // so each one starts closing; the last one closes the window instead of returning
            // home.
            tabs::start_quitting(self.win);
            let ready = tabs::browsers(self.win).iter().fold(true, |ready, browser| {
                let closed = browser.host().is_none_or(|host| host.try_close_browser() == 1);
                ready && closed
            });
            ready.into()
        }

        fn on_key_event(&self, _window: Option<&mut Window>, event: Option<&KeyEvent>) -> i32 {
            (settings_box::handle_escape(self.win, event)
                || settings_box::handle_step_keys(self.win, event)
                || commands::handle_event(self.win, event))
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
            browser_view: Option<&mut BrowserView>,
            popup_browser_view: Option<&mut BrowserView>,
            is_devtools: i32,
        ) -> i32 {
            // Pages' popups (`window.open`, `target=_blank`) become tabs of their opener's
            // window; DevTools keeps its default window.
            let opener = browser_view
                .and_then(|view| view.browser())
                .and_then(|browser| windows::of_browser_or_last(&browser))
                .or_else(windows::last_active);
            match (popup_browser_view, opener) {
                (Some(popup), Some(win)) if is_devtools == 0 => {
                    tabs::adopt_popup(win, popup.clone());
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
