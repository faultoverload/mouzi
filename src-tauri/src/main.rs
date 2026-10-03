// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Work around WebKitGTK DMABuf renderer bug on Wayland.
    // Without this, GTK window creation fails with:
    //   "Error 71 (Protocol error) dispatching to Wayland display."
    // Upstream: https://bugs.webkit.org/show_bug.cgi?id=280210
    //           https://github.com/tauri-apps/tauri/issues/10702
    #[cfg(target_os = "linux")]
    if std::env::var("WEBKIT_DISABLE_DMABUF_RENDERER").is_err() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }

    // Branch on a runtime CLI flag — single binary, no Cargo features.
    // `mouzi --headless` skips every webview-window code path in `run()`.
    if std::env::args().any(|a| a == "--headless") {
        mouzi_lib::headless::run_headless();
    } else {
        mouzi_lib::run();
    }
}
