//! Linux bundles keep `libcef.so` next to the executable. Without an `$ORIGIN` runpath the
//! dynamic loader never looks there and the app fails to start with "libcef.so: cannot open
//! shared object file" (the v0.1.0 Linux release had this bug).

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux") {
        println!("cargo:rustc-link-arg-bins=-Wl,-rpath,$ORIGIN");
    }
}
