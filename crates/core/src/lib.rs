//! Platform-independent logic for mini-browser. Nothing here depends on CEF, so it is unit-tested
//! with a plain `cargo test -p mini-core`.

pub mod address;
pub mod home;
pub mod launch;
pub mod layout;
pub mod popups;
pub mod settings;
pub mod shortcuts;
pub mod strip;
pub mod tabs;
pub mod top_bar;
pub mod update;
pub mod windows;
