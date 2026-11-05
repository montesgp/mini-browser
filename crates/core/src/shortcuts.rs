//! Keyboard shortcuts: maps a key press to the browser command it triggers.

/// Windows virtual-key codes used by the shortcuts. CEF fills `KeyEvent.windows_key_code` with
/// these on every platform.
pub mod key {
    pub const TAB: i32 = 0x09;
    pub const LEFT: i32 = 0x25;
    pub const RIGHT: i32 = 0x27;
    pub const F: i32 = 0x46;
    pub const L: i32 = 0x4C;
    pub const Q: i32 = 0x51;
    pub const R: i32 = 0x52;
    pub const T: i32 = 0x54;
    pub const W: i32 = 0x57;
    pub const COMMA: i32 = 0xBC;
    pub const OPEN_BRACKET: i32 = 0xDB;
    pub const CLOSE_BRACKET: i32 = 0xDD;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    FocusAddress,
    NewTab,
    CloseTab,
    NextTab,
    PreviousTab,
    /// Switches the top bar between auto (on hover) and pinned.
    ToggleTopBar,
    Back,
    Forward,
    Reload,
    Quit,
    OpenSettings,
    /// Activates the tab at this 1-based position (`Ctrl/Cmd+1` to `8`).
    GoToTab(usize),
    /// Activates the last tab (`Ctrl/Cmd+9`).
    GoToLastTab,
}

/// Modifier keys held during a key press. `command` is the macOS Command key.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub command: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    MacOs,
    Other,
}

impl Platform {
    /// The platform this crate was compiled for.
    pub const fn current() -> Self {
        if cfg!(target_os = "macos") {
            Self::MacOs
        } else {
            Self::Other
        }
    }
}

/// The command bound to `windows_key_code` pressed with exactly `modifiers`, if any. The
/// primary modifier is Command on macOS and Ctrl elsewhere; extra modifiers never match.
pub fn command_for(
    windows_key_code: i32,
    modifiers: Modifiers,
    platform: Platform,
) -> Option<Command> {
    let none = Modifiers::default();
    let primary = match platform {
        Platform::MacOs => Modifiers {
            command: true,
            ..none
        },
        Platform::Other => Modifiers { ctrl: true, ..none },
    };
    let primary_shift = Modifiers {
        shift: true,
        ..primary
    };
    let ctrl = Modifiers { ctrl: true, ..none };
    let ctrl_shift = Modifiers {
        shift: true,
        ..ctrl
    };
    let (back, forward) = match platform {
        Platform::MacOs => ((key::OPEN_BRACKET, primary), (key::CLOSE_BRACKET, primary)),
        Platform::Other => {
            let alt = Modifiers { alt: true, ..none };
            ((key::LEFT, alt), (key::RIGHT, alt))
        }
    };

    if let Some(command) = tab_number_command(windows_key_code, modifiers, primary) {
        return Some(command);
    }

    let bindings = [
        ((key::L, primary), Command::FocusAddress),
        ((key::T, primary), Command::NewTab),
        ((key::W, primary), Command::CloseTab),
        ((key::TAB, ctrl), Command::NextTab),
        ((key::TAB, ctrl_shift), Command::PreviousTab),
        ((key::F, primary_shift), Command::ToggleTopBar),
        (back, Command::Back),
        (forward, Command::Forward),
        ((key::R, primary), Command::Reload),
        ((key::Q, primary), Command::Quit),
        ((key::COMMA, primary), Command::OpenSettings),
    ];
    bindings
        .into_iter()
        .find(|&(binding, _)| binding == (windows_key_code, modifiers))
        .map(|(_, command)| command)
}

/// `Ctrl/Cmd` with a digit from the main row or the keypad: `1` to `8` go to that tab, `9` to
/// the last one, as in Chrome.
fn tab_number_command(
    windows_key_code: i32,
    modifiers: Modifiers,
    primary: Modifiers,
) -> Option<Command> {
    if modifiers != primary {
        return None;
    }
    let digit = match windows_key_code {
        0x31..=0x39 => windows_key_code - 0x30,
        0x61..=0x69 => windows_key_code - 0x60,
        _ => return None,
    };
    Some(match digit {
        9 => Command::GoToLastTab,
        n => Command::GoToTab(usize::try_from(n).ok()?),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primary_and_a_digit_go_to_a_tab() {
        let ctrl = Modifiers {
            ctrl: true,
            ..Modifiers::default()
        };
        let command = Modifiers {
            command: true,
            ..Modifiers::default()
        };
        assert_eq!(
            command_for(0x31, ctrl, Platform::Other),
            Some(Command::GoToTab(1))
        );
        assert_eq!(
            command_for(0x38, ctrl, Platform::Other),
            Some(Command::GoToTab(8))
        );
        assert_eq!(
            command_for(0x39, ctrl, Platform::Other),
            Some(Command::GoToLastTab)
        );
        assert_eq!(
            command_for(0x62, ctrl, Platform::Other),
            Some(Command::GoToTab(2))
        ); // keypad 2
        assert_eq!(
            command_for(0x69, ctrl, Platform::Other),
            Some(Command::GoToLastTab)
        ); // keypad 9
        assert_eq!(
            command_for(0x33, command, Platform::MacOs),
            Some(Command::GoToTab(3))
        );
        assert_eq!(command_for(0x30, ctrl, Platform::Other), None); // 0
        assert_eq!(
            command_for(0x31, Modifiers::default(), Platform::Other),
            None
        );
        assert_eq!(
            command_for(
                0x31,
                Modifiers {
                    shift: true,
                    ..ctrl
                },
                Platform::Other
            ),
            None
        );
        assert_eq!(command_for(0x31, ctrl, Platform::MacOs), None);
    }

    const NONE: Modifiers = Modifiers {
        shift: false,
        ctrl: false,
        alt: false,
        command: false,
    };
    const CTRL: Modifiers = Modifiers { ctrl: true, ..NONE };
    const CTRL_SHIFT: Modifiers = Modifiers {
        ctrl: true,
        shift: true,
        ..NONE
    };
    const ALT: Modifiers = Modifiers { alt: true, ..NONE };
    const CMD: Modifiers = Modifiers {
        command: true,
        ..NONE
    };
    const CMD_SHIFT: Modifiers = Modifiers {
        command: true,
        shift: true,
        ..NONE
    };

    fn other(key: i32, modifiers: Modifiers) -> Option<Command> {
        command_for(key, modifiers, Platform::Other)
    }

    fn mac(key: i32, modifiers: Modifiers) -> Option<Command> {
        command_for(key, modifiers, Platform::MacOs)
    }

    /// Asserts `command` is bound to `key` with the primary modifier on both platforms.
    fn assert_primary(key: i32, command: Command) {
        assert_eq!(other(key, CTRL), Some(command));
        assert_eq!(mac(key, CMD), Some(command));
    }

    #[test]
    fn primary_l_focuses_the_address_box() {
        assert_primary(key::L, Command::FocusAddress);
    }

    #[test]
    fn primary_t_opens_a_new_tab() {
        assert_primary(key::T, Command::NewTab);
    }

    #[test]
    fn primary_w_closes_the_tab() {
        assert_primary(key::W, Command::CloseTab);
    }

    #[test]
    fn ctrl_tab_cycles_forward_on_both_platforms() {
        assert_eq!(other(key::TAB, CTRL), Some(Command::NextTab));
        assert_eq!(mac(key::TAB, CTRL), Some(Command::NextTab));
    }

    #[test]
    fn ctrl_shift_tab_cycles_backward_on_both_platforms() {
        assert_eq!(other(key::TAB, CTRL_SHIFT), Some(Command::PreviousTab));
        assert_eq!(mac(key::TAB, CTRL_SHIFT), Some(Command::PreviousTab));
    }

    #[test]
    fn primary_shift_f_toggles_the_top_bar() {
        assert_eq!(other(key::F, CTRL_SHIFT), Some(Command::ToggleTopBar));
        assert_eq!(mac(key::F, CMD_SHIFT), Some(Command::ToggleTopBar));
    }

    #[test]
    fn alt_arrows_go_back_and_forward_outside_macos() {
        assert_eq!(other(key::LEFT, ALT), Some(Command::Back));
        assert_eq!(other(key::RIGHT, ALT), Some(Command::Forward));
    }

    #[test]
    fn cmd_brackets_go_back_and_forward_on_macos() {
        assert_eq!(mac(key::OPEN_BRACKET, CMD), Some(Command::Back));
        assert_eq!(mac(key::CLOSE_BRACKET, CMD), Some(Command::Forward));
    }

    #[test]
    fn primary_r_reloads() {
        assert_primary(key::R, Command::Reload);
    }

    #[test]
    fn primary_q_quits() {
        assert_primary(key::Q, Command::Quit);
    }

    #[test]
    fn primary_comma_opens_settings() {
        assert_primary(key::COMMA, Command::OpenSettings);
    }

    #[test]
    fn extra_modifiers_never_match() {
        let ctrl_alt = Modifiers { alt: true, ..CTRL };
        let ctrl_cmd = Modifiers { ctrl: true, ..CMD };
        let alt_shift = Modifiers { shift: true, ..ALT };
        assert_eq!(other(key::L, ctrl_alt), None);
        assert_eq!(other(key::L, CTRL_SHIFT), None);
        assert_eq!(other(key::T, ctrl_cmd), None);
        assert_eq!(other(key::LEFT, alt_shift), None);
        assert_eq!(other(key::TAB, ctrl_alt), None);
        assert_eq!(mac(key::L, CMD_SHIFT), None);
        assert_eq!(mac(key::L, ctrl_cmd), None);
        assert_eq!(mac(key::TAB, ctrl_cmd), None);
    }

    #[test]
    fn the_other_platforms_primary_modifier_never_matches() {
        assert_eq!(mac(key::L, CTRL), None);
        assert_eq!(mac(key::F, CTRL_SHIFT), None);
        assert_eq!(other(key::L, CMD), None);
        assert_eq!(other(key::F, CMD_SHIFT), None);
    }

    #[test]
    fn platform_specific_history_keys_do_not_cross_platforms() {
        assert_eq!(mac(key::LEFT, ALT), None);
        assert_eq!(mac(key::RIGHT, ALT), None);
        assert_eq!(other(key::OPEN_BRACKET, CTRL), None);
        assert_eq!(other(key::CLOSE_BRACKET, CTRL), None);
    }

    #[test]
    fn unmodified_or_unbound_keys_do_nothing() {
        assert_eq!(other(key::L, NONE), None);
        assert_eq!(other(key::TAB, NONE), None);
        assert_eq!(mac(key::LEFT, NONE), None);
        assert_eq!(other(0x41, CTRL), None);
        assert_eq!(mac(0x41, CMD), None);
    }

    #[test]
    fn cmd_tab_is_not_bound_on_macos() {
        assert_eq!(mac(key::TAB, CMD), None);
    }
}
