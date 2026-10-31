//! Turns CEF key events into [`Command`]s and runs them. Every focus target (page, address box,
//! window) routes its key events here, so each shortcut works wherever the focus is.

use cef::{sys::cef_event_flags_t, *};
use mini_core::shortcuts::{self, Command, Modifiers, Platform};

use super::{address_box, tab_strip, tabs};

/// The command bound to the key and modifiers of `event`, whatever the event type.
fn command_for_event(event: &KeyEvent) -> Option<Command> {
    let has = |flag: cef_event_flags_t| event.modifiers & (flag.0 as u32) != 0;
    let modifiers = Modifiers {
        shift: has(cef_event_flags_t::EVENTFLAG_SHIFT_DOWN),
        ctrl: has(cef_event_flags_t::EVENTFLAG_CONTROL_DOWN),
        alt: has(cef_event_flags_t::EVENTFLAG_ALT_DOWN),
        command: has(cef_event_flags_t::EVENTFLAG_COMMAND_DOWN),
    };
    shortcuts::command_for(event.windows_key_code, modifiers, Platform::current())
}

/// Runs the command bound to a key press. Returns whether `event` belongs to a shortcut: its
/// character and key-up events are swallowed too, so they never reach the page or field that
/// has the focus after the command ran (a `Ctrl+Tab` character would move the page focus).
pub fn handle_event(event: Option<&KeyEvent>) -> bool {
    let Some((event, command)) =
        event.and_then(|event| command_for_event(event).map(|command| (event, command)))
    else {
        return false;
    };
    if event.type_ == KeyEventType::RAWKEYDOWN {
        // Run once CEF is done with the key event: moving the focus while the event is still
        // being dispatched lets it reach the newly focused page.
        let mut task = RunCommand::new(command);
        post_task(ThreadId::UI, Some(&mut task));
    }
    true
}

wrap_task! {
    struct RunCommand {
        command: Command,
    }

    impl Task {
        fn execute(&self) {
            run(self.command);
        }
    }
}

pub fn run(command: Command) {
    match command {
        Command::FocusAddress => address_box::show(&tabs::active_url()),
        Command::NewTab => tabs::open_tab(None),
        Command::CloseTab => tabs::close_active(),
        Command::NextTab => tabs::activate_next(),
        Command::PreviousTab => tabs::activate_previous(),
        Command::ToggleTabStrip => tab_strip::toggle(),
        Command::Back => tabs::with_active_browser(|browser| browser.go_back()),
        Command::Forward => tabs::with_active_browser(|browser| browser.go_forward()),
        Command::Reload => tabs::with_active_browser(|browser| browser.reload()),
        Command::Quit => tabs::quit(),
        // The settings box is not built yet.
        Command::OpenSettings => {}
    }
}
