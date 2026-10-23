fn main() {
    tauri_build::build();
    // Native save dialogs also link TaskDialogIndirect into the Rust test harness.
    // Select Common Controls v6 for every MSVC executable, including cargo test.
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        println!("cargo:rustc-link-arg=/MANIFESTDEPENDENCY:type='win32' name='Microsoft.Windows.Common-Controls' version='6.0.0.0' processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'");
    }
}
