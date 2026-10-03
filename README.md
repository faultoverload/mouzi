# Mouzi 🧹🐁

> **Your downloads, tamed.**

Mouzi is a silent, elegant file organizer that lives in your system tray and keeps your Downloads folder (and any other folder) automatically tidy. It runs quietly in the background, monitors selected folders, and moves, renames, or sorts files based on customizable rules.

[![Product Hunt](https://img.shields.io/badge/Product%20Hunt-🚀%20Launch-orange?logo=producthunt&color=ff6154)](https://www.producthunt.com/products/mouzi?launch=mouzi)
[![Reddit](https://img.shields.io/badge/Reddit-r%2FMouzi-FF4500?logo=reddit)](https://www.reddit.com/r/Mouzi/)
[![X](https://img.shields.io/badge/X-@hsrvibe-black?logo=x&logoColor=white)](https://x.com/hsrvibe)

[![Windows](https://img.shields.io/badge/Windows-10%2F11-blue?logo=windows)](https://mouzi.cc)
[![Linux](https://img.shields.io/badge/Linux-AppImage%2Fdeb%2Frpm-yellow?logo=linux)](https://mouzi.cc)
[![Tauri](https://img.shields.io/badge/Built%20with-Tauri-FFC131?logo=tauri)](https://tauri.app)
[![Rust](https://img.shields.io/badge/Backend-Rust-000000?logo=rust)](https://www.rust-lang.org)
[![React](https://img.shields.io/badge/Frontend-React-61DAFB?logo=react)](https://react.dev)
[![License](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)
[![GitGem](https://gitgem.org/api/badge/github/hsr88/mouzi.svg)](https://gitgem.org/github/hsr88/mouzi)

[![Download Mouzi](https://a.fsdn.com/con/app/sf-download-button)](https://sourceforge.net/projects/mozui/files/latest/download)

---

## 📸 Screenshots
<img width="640" height="360" alt="mouzigiflinux_maly" src="https://github.com/user-attachments/assets/32dc0286-fdb0-411e-8237-f589c2f17082" />

<img width="500" height="361" alt="resized-1_1781356999" src="https://github.com/user-attachments/assets/22555e17-b58a-4a70-9da2-47d8f778b9ea" />
<img width="500" height="361" alt="resized-2_1781357019" src="https://github.com/user-attachments/assets/75ddb288-ff70-4b78-927a-b31e31cbcecd" />
<img width="500" height="314" alt="resized-mouzilinux" src="https://github.com/user-attachments/assets/2ed8b18f-4833-40f2-ab19-9d0a63014f88" />
<img width="500" height="315" alt="resized-mozuilinux2" src="https://github.com/user-attachments/assets/fb6bca80-0b5e-4622-8efc-3ceca186a829" />




---

## ✨ Features

### 🔇 Silent by Default
- Runs 24/7 in the background with minimal resource usage (~5 MB RAM)
- Automatically organizes new files as they arrive
- Shows a subtle Windows toast notification with the count of organized files
- Silent autostart with Windows

### 📁 Smart Rules Engine
- **Images** (`.jpg`, `.png`, `.gif`, `.webp`...) → `Downloads/Images/`
- **Documents** (`.pdf`, `.docx`, `.xlsx`...) → `Downloads/Documents/`
- **Archives** (`.zip`, `.rar`, `.7z`...) → `Downloads/Archives/`
- **Installers** (`.exe`, `.msi`...) → `Downloads/Installers/`
- **Music** / **Video** → dedicated folders
- **Catch-all** rule for everything else

### 🛠️ Fully Customizable
- Create your own rules with extensions, regex patterns, and destination folders
- Use dynamic placeholders in paths: `{year}`, `{month}`, `{day}`, `{extension}`, `{filename}`
- Reorder rules by priority - first match wins

### 🚫 Ignore Rules (.mouziignore)
- Per-folder ignore patterns — like `.gitignore` for your files
- Set up via Settings UI or write a `.mouziignore` file manually
- Supports wildcards (`*.tmp`), exact names (`.DS_Store`), and folders (`node_modules/`)

### 📂 Folder Modes
Each watched folder can run in one of three modes:
- **Silent** - automatically organize files as they arrive (default)
- **Manual** - collect files and only process them when you click **Organize Now**
- **Paused** - watch the folder but don't move anything

### 📦 Google Takeout Import
- Import `.zip`, `.tgz`, or `.tar.gz` archives directly from Google Takeout
- Files are extracted to a staging folder and sorted using your existing rules
- No need to manually unzip and reorganize everything

### 📜 History & Undo
- Every action is logged locally in SQLite
- Undo any single move with one click
- Clear history anytime

### 🌍 Multi-language
Auto-detects your system language. Supported:
- 🇬🇧 English
- 🇵🇱 Polish
- 🇮🇹 Italian
- 🇩🇪 German
- 🇫🇷 French
- 🇷🇺 Russian
- 🇯🇵 Japanese
- 🇻🇳 Vietnamese
- 🇪🇸 Spanish
- 🇺🇦 Ukrainian

*(Falls back to English if system language is not supported)*

### 🕶️ Dark Mode
- Follows system theme, or force Light / Dark mode from settings

### 🔒 Privacy First
- **100% offline** - zero cloud, zero file name uploads
- **No telemetry** by default
- **System files ignored** - `desktop.ini`, `Thumbs.db`, `.DS_Store`, and other OS hidden files are never touched
- **Portable version available** - run without installing, leaves no trace in the registry
- All data stored locally in your user profile folder

### 🪶 Headless / Service Mode

`mouzi --headless` runs the same binary as the GUI path but **never creates a WebviewWindow** — no WebKitWebProcess, no WebKitNetworkProcess, no libLLVM, no libnvidia-gpucomp mapped at runtime. The branch happens in `main.rs` on `std::env::args()`; no separate package is shipped. Use it for:

- **Long-running daemons** that organize `~/Downloads` on a server, NAS, or remote box.
- **Low-RSS idle** on laptops or shared machines where ~474 MiB baseline (the GUI's idle working set, dominated by WebKit helper processes and `libLLVM`) is too much.
- **Hybrid-GPU nvidia/Intel boxes** where every WebKit invocation eagerly pulls `libnvidia-gpucomp` into the working set even if you never open a window.

#### Run ad-hoc

```sh
mouzi --headless   # same binary as `mouzi`; no separate install
```

The tray menu shows up at the same icon location as the GUI path and offers `Open GUI` (spawns the GUI in a sibling process), `Pause` / `Resume` (toggle folder modes in the shared SQLite db), and `Quit`.

#### Run as a systemd user service (Linux)

Install the unit shipped in `assets/systemd-user/mouzi-headless.service`:

```sh
mkdir -p ~/.config/systemd/user
cp assets/systemd-user/mouzi-headless.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now mouzi-headless
systemctl --user status mouzi-headless    # verify "active (running)"
```

The unit sets `WEBKIT_DISABLE_DMABUF_RENDERER=1`, `WEBKIT_DISABLE_COMPOSITING_MODE=1`, and `GSK_RENDERER=cairo` so the GTK/WebKit init paths stay on software rendering and `libnvidia-gpucomp` stays out of the process working set even on hybrid-GPU laptops. The unit's `ExecStart` assumes `/usr/bin/mouzi` — edit the file if you built from source and installed elsewhere (e.g. `~/.local/bin/mouzi`).

#### Run as an XDG autostart entry

For login-session autostart without systemd, copy the desktop file:

```sh
mkdir -p ~/.config/autostart
cp assets/autostart/mouzi-headless.desktop ~/.config/autostart/
```

This is the same approach Mouzi's own `tauri-plugin-autostart` uses for the GUI build, just with `Exec=mouzi --headless` instead of `Exec=mouzi --autostart`.

#### Expected memory footprint

The 2026-09-30 memory audit (`~/Documents/Obsidian Vault/Research/2026-09-30-mouzi-memory-optimization-audit.md`) traced ~95% of the GUI path's 474 MiB baseline to WebKit helper processes and the LLVM/nvidia libraries that get mapped alongside the first WebView. By skipping the WebView entirely, `mouzi --headless` should hold the host working set to **<30 MiB** for the mouzi process itself plus no WebKit processes at all. Practical upper bound if your GTK init eagerly maps a few more shared libs: 50–80 MiB. Anything above 120 MiB is a regression worth investigating with `pmap -x <pid> | sort -k3 -n | tail -20`.

Numbers from a real nvidia box will be captured after this lands — see [`docs/headless-memory-measurement.md`](docs/headless-memory-measurement.md) for the measurement recipe.

#### Switching between headless and GUI

Both modes share the same SQLite database (under `~/.local/share/mouzi/`), so rules and folder modes set in one carry over to the other. To reconfigure after starting headless:

1. Click the tray menu → **Open GUI**. A GUI process spawns alongside the running headless one (single-instance plugin will refuse if you already have a GUI running).
2. Edit rules / add folders / change settings in the GUI as usual.
3. **Save** in the GUI. The headless process picks up the new state on its next folder scan (≤ 1 s) without needing to be restarted.

Or, if you don't want two processes running, quit the headless one from the tray (`Quit` menu entry) before launching the GUI directly:

```sh
systemctl --user stop mouzi-headless     # if using the unit
mouzi                                    # launch GUI; rules/state already saved
```

---

## 🌍 Help translate Mouzi

Want to use Mouzi in another language or improve an existing translation? Community translations are welcome, and you do not need to work on the rest of the application.

- Read the [step-by-step translation guide](CONTRIBUTING.md#translations).
- Browse the current [locale files](src/i18n/locales).
- Translation updates can be submitted directly as a pull request.

If you want to add a new language, open an issue first so we can confirm the language code and avoid duplicate work.

---

## 📥 Download

### Windows

| Installer | Size | Best For |
|-----------|------|----------|
| [`Mouzi_0.2.1_x64-setup.exe`](https://mouzi.cc/download) | ~4.8 MB | Regular users (auto-installer) |
| [`Mouzi_0.2.1_x64_en-US.msi`](https://mouzi.cc/download) | ~6.8 MB | Enterprise / Active Directory |
| [`Mouzi_0.2.1_x64-portable.exe`](https://mouzi.cc/download) | ~18.5 MB | Power users (no install) |

> ⚠️ **Windows 10/11.** Requires the [Microsoft Edge WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) (pre-installed on most systems).

### Linux

| Package | Size | Best For |
|---------|------|----------|
| [`Mouzi_0.2.1_amd64.AppImage`](https://mouzi.cc/download/linux) | ~86.6 MB | Universal — works on most distros |
| [`Mouzi_0.2.1_amd64.deb`](https://mouzi.cc/download/linux) | ~8.8 MB | Debian, Ubuntu, Mint, Pop!_OS |
| [`Mouzi-0.2.1-1.x86_64.rpm`](https://mouzi.cc/download/linux) | ~8.9 MB | Fedora, openSUSE, RHEL |

> 🐧 **Linux requirements:** `libwebkit2gtk-4.1` and `libayatana-appindicator3`. Most modern distros have these pre-installed.

**SHA-256 checksums:** every GitHub release includes a generated `SHA256SUMS.txt`
covering the exact published artifacts. Use that file instead of copying a checksum
from an older release.

---

## 🚀 Quick Start

1. **Download** Mouzi for your OS using the links above.
2. **Windows:** Install and Mouzi starts automatically with a tray icon (📂).
   **Linux:** Run the AppImage directly, or install the `.deb`/`.rpm` package.
3. **Left-click** the tray icon to open the popup - see recent actions, stats, and organize files manually.
4. **Right-click** the tray icon for the menu: `Organize Now`, `Settings`, `Quit`.
5. Drop a file into your `Downloads` folder and watch it disappear into the right subfolder within 2 seconds.

---

## ⚙️ How Rules Work

Rules are evaluated top-to-bottom. The first rule that matches a file wins.

| Condition | Example Match |
|-----------|---------------|
| Extensions | `jpg`, `png`, `gif` |
| Regex pattern | `.*faktura.*` matches `faktura_2025.pdf` |

**Destination path placeholders:**
```
Downloads/Documents/{year}/{month}/
→ Downloads/Documents/2026/05/
```

---

## 🛡️ Community Use Case

A computer repair technician uses Mouzi to automatically move ScreenConnect installers out of the Downloads folder before less-technical users can open them—adding an extra layer of friction against remote-access scams.

[Read the case study: How Mouzi is being used to reduce ScreenConnect scam risk →](https://straycode.dev/blog/how-a-simple-downloads-organizer-is-being-used-to-stop-screenconnect-scams)

> Mouzi is a file organizer, not antivirus or endpoint security software. Filename-based rules should be used as an additional safeguard, not as a replacement for security software and user education.

---

## 📐 Architecture

```
+---------------------------------------------+
|  Frontend (React 19 + TypeScript + Tailwind) |
|  +- Popup window (300x420, frameless)        |
|  +- Settings window (900x650)                |
+---------------------------------------------+
|  Tauri 2.x Bridge                            |
+---------------------------------------------+
|  Backend (Rust)                              |
|  +- File Watcher (notify crate)              |
|  +- Rules Engine                             |
|  +- Scheduler (time-based organization)      |
|  +- SQLite Database (rusqlite)               |
|  +- System Tray & Notifications              |
+---------------------------------------------+
```

---

## 🛠️ Development

### Prerequisites
- [Rust](https://rustup.rs/) (latest stable)
- [Node.js](https://nodejs.org/) 22+
- **Windows:** Windows SDK / MSVC (Visual Studio Build Tools)
- **Linux:** `libwebkit2gtk-4.1-dev`, `libayatana-appindicator3-dev`, `fuse` (see [Tauri Linux prerequisites](https://v2.tauri.app/start/prerequisites/))

### Setup

```bash
# Clone the repo
git clone https://github.com/hsr88/mouzi.git
cd mouzi

# Install frontend dependencies
npm install

# Run in development mode (hot-reload for both frontend & Rust)
npm run tauri dev
```

### Build from Source

```bash
# Production build (MSI + NSIS installer)
npm run tauri build
```

Output will be in `src-tauri/target/release/bundle/`.

---

## 🆕 Mouzi 0.2.1

This patch completes the new interface translations in all 11 supported languages and adds missing Spanish archive-import text. The organization features include:

- Preview matched rules and destinations, then approve selected operations.
- Suggest mode and per-folder processing of only new files.
- File-size and modification-date conditions; rename-only templates.
- History filters, organization runs and verified undo of selected files or runs.
- Warnings for misleading executable filenames and optional **Add to Mouzi** in Windows Explorer.
- Simplified Chinese, defensive extension-list validation and theme contrast fixes.

Stable settings are preserved. Older History entries without verification records require manual restoration. New controls are translated into all 11 supported interface languages. Windows files remain unsigned with Authenticode; signed updater packages are separate.

[Full release notes](https://github.com/hsr88/mouzi/releases/tag/v0.2.1) · [Documentation](https://mouzi.cc/docs)

---

## 📋 Roadmap

### Already implemented

MVP with default rules, multi-language support, dark mode, history & undo, start with Windows, custom folders with local rules, folder modes (silent / manual / paused), system files ignored, secure automatic update checks, `.mouziignore`, portable version, browser temp files ignored, grace period option, file lock check, single-instance guard, first-run popup visibility, clickable and custom notifications, skip 0 KB placeholder files, Linux port, Google Takeout archive import, Wayland crash workaround, rule destination picker, quick rule toggles, extensionless-file detection, selective batches, extension normalization, Recycle Bin/Trash actions and history deletion confirmation.

### Upcoming

- [ ] Optional screenshot cleanup: move screenshots older than a chosen number of days from system and custom screenshot folders to the Recycle Bin
- [ ] npm wrapper (`npm install -g mouzi`) for cross-platform CLI install
- [ ] Local AI tagging (ONNX runtime for content classification)
- [ ] Rule learning from user manual moves
- [ ] macOS port

---

## ☕ Support

If Mouzi saves you time and keeps your Downloads folder sane, consider supporting its development:

[![ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/hsr)

You can also support Mouzi through [GitHub Sponsors](https://github.com/sponsors/hsr88).

Or visit the project homepage: **[mouzi.cc](https://mouzi.cc)**

---
## See Also

### [Ordir](https://github.com/landnthrn/ordir)

Order folders any way you want inside Windows File Explorer, and add custom thumbnails.

---
## 📄 License

Mouzi is released under the [MIT License](LICENSE).

---

## 🙏 Acknowledgements

Built with [Tauri](https://tauri.app), [React](https://react.dev), [Tailwind CSS](https://tailwindcss.com), and [Rust](https://www.rust-lang.org).

---

<p align="center">
  <sub>Made with ❤️ for people who download too much stuff.</sub>
</p>
