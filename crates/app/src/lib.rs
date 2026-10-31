//! On Windows with the sandbox, CEF's `bootstrap.exe` (renamed to `minibrowser.exe` by the
//! bundle tool) loads this library and calls [`RunWinMain`](win). On every other
//! configuration the library is empty and the `minibrowser` binary is the entry point.

#[cfg(all(target_os = "windows", feature = "sandbox"))]
mod shell;
#[cfg(all(target_os = "windows", feature = "sandbox"))]
mod win;
