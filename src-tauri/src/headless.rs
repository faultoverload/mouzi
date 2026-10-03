// Headless mode: tray-only operation. No webview windows are created at any point.
// On Linux/macOS the tray icon is sourced from `app.default_window_icon()`, which
// the bundle carries even when no window is constructed.
//
// Same compiled binary as the GUI path — the branch happens in `main.rs` based on
// the `--headless` CLI flag.

use directories::ProjectDirs;
use std::sync::{Arc, Mutex};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, RunEvent};

use crate::commands::get_downloads_folder;
use crate::db::{self, FOLDER_MODE_PAUSED, FOLDER_MODE_SILENT};
use crate::watcher::FolderWatcher;
use crate::AppState;

/// Entry point for `mouzi --headless`.
///
/// Never creates a WebviewWindow. WebKitGTK / WebView2 stay linked but are not
/// dlopen'd because no webview is ever requested.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run_headless() {
    tauri::Builder::default()
        .manage(AppState {
            watcher: Arc::new(Mutex::new(FolderWatcher::new(
                Arc::new(Mutex::new(Default::default())),
                Arc::new(Mutex::new(None)),
            ))),
            ignored_files: Arc::new(Mutex::new(Default::default())),
            pending_open_folder: Arc::new(Mutex::new(None)),
            scheduler: crate::scheduler::Scheduler::new(),
            review_request: Mutex::new(None),
            add_folder_request: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![])
        .setup(|app| {
            // Hide from dock on macOS — tray-only operation.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let app_handle = app.handle().clone();

            // Initialise the same database as the GUI path.
            let beta = env!("CARGO_PKG_VERSION").contains("beta");
            if let Some(proj_dirs) =
                ProjectDirs::from("cc", "mouzi", if beta { "mouzi-beta" } else { "mouzi" })
            {
                let data_dir = proj_dirs.data_dir().to_path_buf();
                std::fs::create_dir_all(&data_dir).ok();
                if let Err(e) = db::init_db(data_dir) {
                    eprintln!("[mouzi --headless] init_db failed: {e}");
                }
            }

            // First-run defaults: same as GUI path minus the popup window show.
            if let Ok(settings) = db::get_settings() {
                if settings.first_run {
                    let downloads = get_downloads_folder();
                    if !beta {
                        let _ = db::add_watched_folder(&downloads, FOLDER_MODE_SILENT);
                    }
                    let _ = db::insert_default_rules(&downloads);
                    let mut new_settings = settings;
                    new_settings.first_run = false;
                    let _ = db::update_settings(&new_settings);
                }
            }

            // Install the system tray BEFORE starting the watcher, so the tooltip
            // can be set immediately after.
            if let Err(e) = build_headless_tray(&app_handle) {
                eprintln!("[mouzi --headless] tray build failed: {e}");
            }

            // Start the folder watcher.
            let state = app.state::<AppState>();
            if let Ok(mut watcher) = state.watcher.lock() {
                if let Err(e) = watcher.watch_folders(app_handle.clone()) {
                    eprintln!("[mouzi --headless] watch_folders failed: {e}");
                }
            }

            // Start scheduled-clean background thread.
            state.scheduler.start(app_handle.clone());

            // Set initial tooltip: rule count.
            let rule_count = db::get_rules().map(|r| r.len()).unwrap_or(0);
            update_headless_tooltip(&app_handle, rule_count);
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building headless tauri application")
        .run(|app, event| {
            if let RunEvent::ExitRequested { .. } = event {
                // Tauri is already tearing down; the FolderWatcher's drop impl will
                // release the notify::RecommendedWatcher handles. We just confirm
                // exit here.
                app.exit(0);
            }
        });
}

/// Build the headless tray. Mirrors `tray::setup_tray` for the GUI path, but
/// contains no webview-window-creating callbacks.
fn build_headless_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    let open_gui = MenuItem::with_id(app, "open_gui", "Open GUI", true, None::<&str>)?;
    let pause = MenuItem::with_id(app, "pause", "Pause", true, None::<&str>)?;
    let resume = MenuItem::with_id(app, "resume", "Resume", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;

    let menu = Menu::with_items(app, &[&pause, &resume, &sep, &open_gui, &sep, &quit])?;

    let mut builder = TrayIconBuilder::with_id("tray-headless")
        .tooltip("mouzi (headless)")
        .show_menu_on_left_click(false)
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open_gui" => {
                // Re-exec the GUI binary without --headless so the regular path runs.
                if let Ok(exe) = std::env::current_exe() {
                    let _ = std::process::Command::new(exe).spawn();
                }
            }
            "pause" => {
                pause_all_folders();
            }
            "resume" => {
                resume_all_folders();
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|_tray, event| {
            // Left-click on the tray is intentionally a no-op in headless mode:
            // there is no window to show, and the "Open GUI" menu item is the
            // explicit way to launch the GUI.
            if let TrayIconEvent::Click {
                button,
                button_state,
                ..
            } = event
            {
                if button == MouseButton::Left && button_state == MouseButtonState::Up {
                    // no-op
                }
            }
        });

    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }
    builder.build(app)?;
    Ok(())
}

fn update_headless_tooltip(app: &tauri::AppHandle, rule_count: usize) {
    let text = format!("mouzi (headless) — {rule_count} rules active");
    if let Some(tray) = app.tray_by_id("tray-headless") {
        let _ = tray.set_tooltip(Some(&text));
    }
}

fn pause_all_folders() {
    let Ok(folders) = db::get_watched_folders() else {
        return;
    };
    for f in folders {
        let id = match f.id {
            Some(id) => id,
            None => continue,
        };
        if f.mode != FOLDER_MODE_PAUSED {
            let _ = db::update_folder_mode(id, FOLDER_MODE_PAUSED);
        }
    }
}

fn resume_all_folders() {
    let Ok(folders) = db::get_watched_folders() else {
        return;
    };
    for f in folders {
        let id = match f.id {
            Some(id) => id,
            None => continue,
        };
        if f.mode == FOLDER_MODE_PAUSED {
            let _ = db::update_folder_mode(id, FOLDER_MODE_SILENT);
        }
    }
}