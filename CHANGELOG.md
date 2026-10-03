# Changelog

All notable changes to Mouzi are documented in this file. Dates are ISO-8601.

The format is loosely based on [Keep a Changelog](https://keepachangelog.com/),
and the project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- **`mouzi --headless` runtime CLI flag** (`src-tauri/src/main.rs` + `src-tauri/src/headless.rs`). The same compiled binary as the GUI path; the branch happens in `main()` on `std::env::args()`. Skips every `WebviewWindow`-creating code path so WebKitGTK / WebView2 / WKWebView never maps its heavyweight dependencies (`libLLVM`, `libnvidia-gpucomp`, `WebKitWebProcess`, `WebKitNetworkProcess`, `JavaScriptCore` VM). Ships a tray-only experience: `Open GUI` / `Pause` / `Resume` / `Quit`. The GUI path's `lib.rs::run()` is unchanged.
- **Systemd user service unit** (`assets/systemd-user/mouzi-headless.service`). `Type=simple`, `ExecStart=/usr/bin/mouzi --headless`, `Restart=on-failure`, hardened with `ProtectSystem=strict` and explicit `Environment=` lines for `WEBKIT_DISABLE_DMABUF_RENDERER`, `WEBKIT_DISABLE_COMPOSITING_MODE`, and `GSK_RENDERER=cairo`. Enables low-RSS long-running daemon mode on Linux.
- **XDG autostart desktop file** (`assets/autostart/mouzi-headless.desktop`). Login-session autostart alternative for desktops without systemd.
- **Headless / Service Mode README section** covering ad-hoc launch, systemd unit install, autostart install, expected memory footprint, and switching between headless and GUI.
- **`docs/headless-memory-measurement.md`** — cross-platform RSS measurement recipe (Linux / macOS / Windows), including environment variables that affect the measurement and a "what to record" table.
- **Unit tests in `src-tauri/src/headless.rs`** for: `--headless` CLI flag recognition, pause/resume helpers, AppState init with a temporary database.