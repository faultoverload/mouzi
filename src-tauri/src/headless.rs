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

/// Returns true if `args` contains the literal `--headless`. Mirrors the
/// check in `src-tauri/src/main.rs`; exposed here so unit tests can pin the
/// parser in one place. See `tests::headless_flag_recognized_in_argv` /
/// `headless_flag_absent_in_argv` below.
pub fn is_headless_flag(args: &[String]) -> bool {
    args.iter().any(|a| a == "--headless")
}

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

// ---------------------------------------------------------------------------
// Tests
//
// Scope: the headless code path only. No webview is ever created, so no
// integration test against a running webkit — these are pure-Rust unit tests
// against the helpers and the AppState builder. They run with `cargo test` in
// the same binary as the GUI tests (see `beta_tests.rs` for the GUI-side
// fixture pattern we reuse).
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::watcher::FolderWatcher;
    use once_cell::sync::Lazy;
    use std::{
        fs,
        path::PathBuf,
        sync::{Mutex, MutexGuard},
    };

    /// Process-global mutex serialising the headless tests' DB writes.
    /// `db::init_db` uses a `OnceCell` so the second invocation in the same
    /// process returns an error from `DB.set()` (the first init wins) but
    /// leaves the active connection in place — calling it from both
    /// `beta_tests::DATABASE` and this lazy is safe regardless of order.
    /// The per-test cleanup deletes the table rows so each test sees a
    /// known starting state — same pattern as `beta_tests::Fixture::new`.
    static DATABASE: Lazy<Mutex<()>> = Lazy::new(|| {
        let path = std::env::temp_dir().join(format!(
            "mouzi-headless-tests-db-{}",
            std::process::id()
        ));
        let _ = fs::create_dir_all(&path);
        // If beta_tests's lazy already won the race, this returns Err from
        // the OnceCell set inside init_db — we ignore it.
        let _ = db::init_db(path);
        Mutex::new(())
    });

    struct Fixture {
        root: PathBuf,
        _guard: MutexGuard<'static, ()>,
    }
    impl Fixture {
        fn new(label: &str) -> Self {
            let guard = DATABASE.lock().unwrap_or_else(|p| p.into_inner());
            // Wipe rows from any prior test in this process.
            db::get_db()
                .lock()
                .unwrap()
                .execute_batch("DELETE FROM rules; DELETE FROM watched_folders; DELETE FROM folder_baseline; DELETE FROM action_logs; DELETE FROM processed_files;")
                .unwrap();
            let root = std::env::temp_dir().join(format!(
                "mouzi-headless-fixture-{}-{}",
                label,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos())
                    .unwrap_or(0)
            ));
            fs::create_dir_all(&root).unwrap();
            Self {
                root,
                _guard: guard,
            }
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    /// True if any element of `args` is the literal `--headless`. Mirrors the
    /// check in `src-tauri/src/main.rs` (`std::env::args().any(|a| a == "--headless")`).
    /// We test this in isolation so the CLI parser has a single source of truth
    /// even if main.rs is refactored to dispatch through here in the future.
    #[test]
    fn headless_flag_recognized_in_argv() {
        assert!(super::is_headless_flag(&["mouzi".into(), "--headless".into()]));
        assert!(super::is_headless_flag(&[
            "mouzi".into(),
            "--headless".into(),
            "--some-other".into(),
        ]));
    }

    #[test]
    fn headless_flag_absent_in_argv() {
        assert!(!super::is_headless_flag(&["mouzi".into()]));
        assert!(!super::is_headless_flag(&[
            "mouzi".into(),
            "--autostart".into()
        ]));
        // `-headless` (single dash) is intentionally NOT matched — main.rs's
        // original check requires two dashes.
        assert!(!super::is_headless_flag(&["mouzi".into(), "-headless".into()]));
        // Empty argv (shouldn't happen in practice — `argv[0]` is the binary
        // name — but the helper must not panic).
        let empty: Vec<String> = vec![];
        assert!(!super::is_headless_flag(&empty));
    }

    /// Constructing the same `AppState` shape `run_headless` registers with
    /// the Tauri builder, against a freshly-initialized temporary SQLite db.
    /// Must not panic. Verifies that `FolderWatcher::new` and the rest of the
    /// plumbing accept an empty `AppState` without trying to touch the GUI
    /// subsystems.
    #[test]
    fn appstate_init_with_temp_db_does_not_panic() {
        let _f = Fixture::new("appstate");

        let state = AppState {
            watcher: Arc::new(Mutex::new(FolderWatcher::new(
                Arc::new(Mutex::new(Default::default())),
                Arc::new(Mutex::new(None)),
            ))),
            ignored_files: Arc::new(Mutex::new(Default::default())),
            pending_open_folder: Arc::new(Mutex::new(None)),
            scheduler: crate::scheduler::Scheduler::new(),
            review_request: Mutex::new(None),
            add_folder_request: Mutex::new(None),
        };

        // AppState fields are reachable — verify the ones we set in run_headless.
        assert!(state.ignored_files.lock().unwrap().is_empty());
        assert!(state.pending_open_folder.lock().unwrap().is_none());
        assert!(state.review_request.lock().unwrap().is_none());
        assert!(state.add_folder_request.lock().unwrap().is_none());
    }

    /// After `pause_all_folders`, every watched folder's mode is
    /// `FOLDER_MODE_PAUSED`. After `resume_all_folders`, every paused folder
    /// is back to `FOLDER_MODE_SILENT`. Verifies that both helpers
    /// actually iterate through `db::get_watched_folders()` and call
    /// `db::update_folder_mode`.
    #[test]
    fn pause_and_resume_update_all_folder_modes() {
        let f = Fixture::new("pause-resume");

        // Three folders in different starting modes: silent, paused, silent.
        let f1 = f.root.join("downloads");
        let f2 = f.root.join("desktop");
        let f3 = f.root.join("documents");
        fs::create_dir_all(&f1).unwrap();
        fs::create_dir_all(&f2).unwrap();
        fs::create_dir_all(&f3).unwrap();
        let id1 = db::add_watched_folder(f1.to_str().unwrap(), FOLDER_MODE_SILENT).unwrap();
        let id2 = db::add_watched_folder(f2.to_str().unwrap(), FOLDER_MODE_PAUSED).unwrap();
        let id3 = db::add_watched_folder(f3.to_str().unwrap(), FOLDER_MODE_SILENT).unwrap();

        pause_all_folders();

        let folders = db::get_watched_folders().unwrap();
        for folder in &folders {
            assert_eq!(
                folder.mode,
                FOLDER_MODE_PAUSED,
                "folder {:?} expected PAUSED after pause_all_folders",
                folder.path
            );
        }

        resume_all_folders();

        let folders = db::get_watched_folders().unwrap();
        for folder in &folders {
            assert_eq!(
                folder.mode,
                FOLDER_MODE_SILENT,
                "folder {:?} expected SILENT after resume_all_folders",
                folder.path
            );
        }

        // Sanity: the folder ids we added are still in the table.
        let ids: std::collections::HashSet<i64> =
            folders.iter().filter_map(|f| f.id).collect();
        assert!(ids.contains(&id1));
        assert!(ids.contains(&id2));
        assert!(ids.contains(&id3));
    }
}