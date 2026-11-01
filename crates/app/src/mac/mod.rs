// Derived from the cef-rs `cefsimple` example (https://github.com/tauri-apps/cef-rs),
// licensed Apache-2.0 OR MIT.

//! macOS application glue: CEF requires an `NSApplication` subclass that implements its event
//! protocols, and an application delegate that routes quit and reopen through CEF.

use std::cell::Cell;

use cef::application_mac::{CefAppProtocol, CrAppControlProtocol, CrAppProtocol};
use objc2::{
    ClassType, DefinedClass, MainThreadMarker, MainThreadOnly, define_class, extern_methods,
    msg_send,
    rc::Retained,
    runtime::{AnyObject, Bool, NSObject, NSObjectProtocol, ProtocolObject},
};
use objc2_app_kit::{
    NSApp, NSApplication, NSApplicationDelegate, NSApplicationTerminateReply, NSEvent,
};

use crate::shell::client;

define_class! {
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    pub struct MiniAppDelegate;

    unsafe impl NSObjectProtocol for MiniAppDelegate {}

    unsafe impl NSApplicationDelegate for MiniAppDelegate {
        #[unsafe(method(applicationShouldTerminate:))]
        unsafe fn application_should_terminate(&self, _sender: &NSApplication) -> NSApplicationTerminateReply {
            NSApplicationTerminateReply::TerminateNow
        }

        /// Called when the user clicks the app dock icon while the application is
        /// already running.
        #[unsafe(method(applicationShouldHandleReopen:hasVisibleWindows:))]
        unsafe fn application_should_handle_reopen(&self, _sender: &NSApplication, _has_visible_windows: Bool) -> Bool {
            if !client::is_closing() {
                client::show_main_window();
            }
            Bool::NO
        }

        /// Requests that any state restoration archive be created with secure encoding
        /// (macOS 12+ only). See https://crrev.com/c737387656 for details.
        #[unsafe(method(applicationSupportsSecureRestorableState:))]
        unsafe fn application_supports_secure_restorable_state(&self, _sender: &NSApplication) -> Bool {
            Bool::YES
        }
    }
}

impl MiniAppDelegate {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = MiniAppDelegate::alloc(mtm).set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}

/// Instance variables of `MiniApplication`.
#[derive(Default)]
pub struct MiniApplicationIvars {
    handling_send_event: Cell<Bool>,
}

define_class!(
    /// An `NSApplication` subclass that implements the CEF protocols, so Chromium handles
    /// events correctly on macOS.
    #[unsafe(super(NSApplication))]
    #[ivars = MiniApplicationIvars]
    pub struct MiniApplication;

    impl MiniApplication {
        #[unsafe(method(sendEvent:))]
        unsafe fn send_event(&self, event: &NSEvent) {
            let was_sending_event = self.is_handling_send_event();
            if !was_sending_event {
                self.set_handling_send_event(true);
            }

            let _: () = msg_send![super(self), sendEvent:event];

            if !was_sending_event {
                self.set_handling_send_event(false);
            }
        }

        /// `-terminate:` is the entry point for orderly quits (menu, dock, logout). The default
        /// implementation calls `exit()` and never leaves the run loop, but Chromium must
        /// leave it to shut down. Instead, close every browser; the client quits the message
        /// loop after the last one closes and `run_main` performs the shutdown.
        #[unsafe(method(terminate:))]
        unsafe fn terminate(&self, _sender: &AnyObject) {
            if !client::is_closing() {
                client::close_all_browsers(false);
            }
        }
    }

    unsafe impl CrAppControlProtocol for MiniApplication {
        #[unsafe(method(setHandlingSendEvent:))]
        unsafe fn _set_handling_send_event(&self, handling_send_event: Bool) {
            self.ivars().handling_send_event.set(handling_send_event);
        }
    }

    unsafe impl CrAppProtocol for MiniApplication {
        #[unsafe(method(isHandlingSendEvent))]
        unsafe fn _is_handling_send_event(&self) -> Bool {
            self.ivars().handling_send_event.get()
        }
    }

    unsafe impl CefAppProtocol for MiniApplication {}
);

impl MiniApplication {
    extern_methods! {
        #[unsafe(method(sharedApplication))]
        fn shared_application() -> Retained<Self>;

        #[unsafe(method(setHandlingSendEvent:))]
        fn set_handling_send_event(&self, handling_send_event: bool);

        #[unsafe(method(isHandlingSendEvent))]
        fn is_handling_send_event(&self) -> bool;
    }
}

/// Makes `MiniApplication` the shared application. Must run before anything touches `NSApp`.
pub fn setup_application() {
    let _ = MiniApplication::shared_application();

    // If `NSApp` was used before this point it is not a `MiniApplication`.
    assert!(
        NSApp(MainThreadMarker::new().expect("Not running on the main thread"))
            .isKindOfClass(MiniApplication::class())
    );
}

/// Installs the application delegate. Keep the returned value alive while the app runs.
pub fn setup_app_delegate() -> Retained<MiniAppDelegate> {
    let mtm = MainThreadMarker::new().expect("Not running on the main thread");

    let delegate = MiniAppDelegate::new(mtm);
    let delegate_proto =
        ProtocolObject::<dyn NSApplicationDelegate>::from_retained(delegate.clone());
    let app = NSApp(mtm);
    assert!(app.isKindOfClass(MiniApplication::class()));
    app.setDelegate(Some(&delegate_proto));

    delegate
}
