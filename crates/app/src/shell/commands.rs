//! Turns CEF key events into [`Command`]s and runs them in the window they were pressed in.
//! Every focus target (page, address box, window) routes its key events here, so each shortcut
//! works wherever the focus is.

use cef::{sys::cef_event_flags_t, *};
use mini_core::shortcuts::{self, Command, Modifiers, Platform};

use super::{address_box, home, settings_box, tabs, windows::WindowId};

/// The command bound to the key and modifiers of `event`, whatever the event type.
fn command_for_event(event: &KeyEvent) -> Option<Command> {
    // The flag's inner type is `i32` on Windows and `u32` on macOS/Linux, so the cast is
    // only redundant on some platforms.
    #[allow(clippy::unnecessary_cast)]
    let has = |flag: cef_event_flags_t| event.modifiers & (flag.0 as u32) != 0;
    let modifiers = Modifiers {
        shift: has(cef_event_flags_t::EVENTFLAG_SHIFT_DOWN),
        ctrl: has(cef_event_flags_t::EVENTFLAG_CONTROL_DOWN),
        alt: has(cef_event_flags_t::EVENTFLAG_ALT_DOWN),
        command: has(cef_event_flags_t::EVENTFLAG_COMMAND_DOWN),
    };
    shortcuts::command_for(event.windows_key_code, modifiers, Platform::current())
}

/// Runs the command bound to a key press in window `win`. Returns whether `event` belongs to a
/// shortcut: its character and key-up events are swallowed too, so they never reach the page or
/// field that has the focus after the command ran (a `Ctrl+Tab` character would move the page
/// focus).
pub fn handle_event(win: WindowId, event: Option<&KeyEvent>) -> bool {
    let Some((event, command)) =
        event.and_then(|event| command_for_event(event).map(|command| (event, command)))
    else {
        return false;
    };
    if event.type_ == KeyEventType::RAWKEYDOWN {
        // Run once CEF is done with the key event: moving the focus while the event is still
        // being dispatched lets it reach the newly focused page.
        let mut task = RunCommand::new(win, command);
        post_task(ThreadId::UI, Some(&mut task));
    }
    true
}

wrap_task! {
    struct RunCommand {
        win: WindowId,
        command: Command,
    }

    impl Task {
        fn execute(&self) {
            run(self.win, self.command);
        }
    }
}

/// Runs `command` in window `win`.
pub fn run(win: WindowId, command: Command) {
    match command {
        // The home page has its own address field.
        Command::FocusAddress if tabs::active_is_home(win) => home::focus_address_field(win),
        Command::FocusAddress => address_box::show(win, &tabs::active_url(win)),
        Command::NewTab => tabs::new_tab(win),
        Command::CloseTab => tabs::close_active(win),
        Command::NextTab => tabs::activate_next(win),
        Command::PreviousTab => tabs::activate_previous(win),
        Command::ToggleTopBar => settings_box::toggle_top_bar(),
        Command::Back => tabs::with_active_browser(win, |browser| browser.go_back()),
        Command::Forward => tabs::with_active_browser(win, |browser| browser.go_forward()),
        Command::Reload => tabs::with_active_browser(win, |browser| browser.reload()),
        Command::Quit => tabs::quit(win),
        Command::OpenSettings => settings_box::toggle(win),
        Command::GoToTab(position) => tabs::activate_position(win, position),
        Command::GoToLastTab => tabs::activate_last(win),
    }
}
