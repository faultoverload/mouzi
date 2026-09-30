---
title: Mouzi memory measurement — main branch baseline (Linux/NVIDIA)
date: 2026-09-30
source: docs/scripts/memory-breakdown.sh
context: Baseline RSS measurements taken on the unmodified main branch before the memory-optimization PRs landed. Reference for evaluating the before/after impact of feat/drop-dead-main-window, feat/destroy-webviews-on-hide, and perf/backend-tightening.
tags: [research, measurement, tauri, memory, mouzi, baseline]
---

# Mouzi memory baseline — main branch on Linux/NVIDIA

## Methodology

Measured on a single Linux/x86_64 host (Arch, KDE/Wayland, NVIDIA GPU) running a **release build** of Mouzi v0.2.1 from upstream `main` (commit `ac89e38`).

Procedure:
1. Build Mouzi in release mode (`npm run tauri build`)
2. Launch Mouzi from the built binary
3. Click the tray icon once to open the popup, click elsewhere to dismiss
4. Open Settings (tray menu → Settings), interact for ~30 seconds (toggled options, edited a rule), close the window
5. Wait 5 seconds for any async cleanup
6. Capture `/proc/$PID/smaps_rollup`, `pmap -q $PID`, and the WebKit helper processes

The `docs/scripts/memory-breakdown.sh` helper script does steps 5-6 and prints a family-level breakdown.

## Top-line numbers (main branch, release build, post-interaction)

| Process | RSS | PSS | Notes |
|---|---|---|---|
| `mouzi` (Rust host) | **187 MiB** | **92 MiB** | Includes dynamically-linked GPU/compiler shared libs |
| `WebKitWebProcess` | **223 MiB** | (shared with mouzi) | **Still alive after popup/settings dismissed** — destroy-on-hide not implemented yet |
| `WebKitNetworkProcess` | 64 MiB | (shared with mouzi) | Expected behavior — shared per-app network service, exits only on Mouzi exit |
| **Total Mouzi stack** | **~474 MiB** | | Includes one WebView that should have been destroyed |

The 200 MiB the user reported in the original report = `mouzi` host RSS alone. The total RSS when WebKit is also counted is ~474 MiB.

## What's actually consuming the host's 187 MiB

This was the key question from the investigation. The breakdown:

| Library family | KB mapped | What it is |
|---|---|---|
| `other` (dominated by GPU stack) | 456,876 KB | `libLLVM.so` (167 MB), `libnvidia-gpucomp.so` (119 MB), `libnvidia-eglcore.so` (38 MB), `libgallium-26.2.3-arch1.1.so` (54 MB) |
| `webkit` | 128,088 KB | `libwebkit2gtk-4.1.so` (91 MB), `libjavascriptcoregtk-4.1.so` (37 MB) |
| `icu (i18n)` | 38,072 KB | Locale message catalog data |
| `gtk` | 10,724 KB | GDK, GObject, GLib |
| `pango/cairo/text` | 3,304 KB | Text rendering |
| `libstdc/libgcc` | 3,056 KB | C++ runtime |
| `sqlite` | 1,636 KB | Bundled SQLite (rusqlite `bundled` feature) |
| `libm` | 1,244 KB | Math library |
| `libsoup (network)` | 604 KB | HTTP client (used by updater plugin) |

**The biggest surprise:** `libLLVM.so` alone is 167 MiB mapped, plus 119 MiB for `libnvidia-gpucomp.so`, plus 38 MiB for `libnvidia-eglcore.so`. The NVIDIA GPU compiler stack + Mesa gallium driver accounts for ~378 MiB of mapped memory.

The kernel counts these shared libraries toward the mouzi process's RSS even though they're shared with other processes on the system. The PSS (Proportional Set Size) is a more honest measure — it accounts for sharing. **Mouzi's actual physical memory cost is 92 MiB (PSS), not 187 MiB (RSS).**

`Anonymous` (heap + stack) is 41 MiB — that's the Rust-side allocation cost, not the shared libraries. Most of this is Tauri's webview-handle state and ICU message caches.

## Why these numbers matter

The original 200 MiB complaint conflates two distinct costs:

1. **Shared library mappings that the kernel attributes to mouzi** (~140 MiB of the host RSS). These are loaded once and stay loaded until the process exits — they cannot be freed. **Not a leak.**

2. **Anonymous heap** (~40 MiB). Tauri's webview-handle state, ICU message caches, and any in-process state. This *could* be reduced by caching settings/rules in process memory and avoiding per-event SQLite queries.

3. **WebKitWebProcess** (223 MiB). The actual webview rendering process. Stays alive because the popup/settings windows are not destroyed on focus loss. **Addressed by `feat/destroy-webviews-on-hide`.**

4. **WebKitNetworkProcess** (64 MiB). Shared network service. **Not a leak** — exits only on Mouzi shutdown. Not addressable without a fundamental architecture change.

## Expected impact of the three PRs

| PR | Linux (NVIDIA) expected delta | Why |
|---|---|---|
| `feat/drop-dead-main-window` | **~5-15 MiB host RSS, ~30-60 MiB peak WebKitWebProcess** | No `main` window means no startup webview is created. On a fresh launch where the user never opens a window, the host stays at ~40-50 MiB and no `WebKitWebProcess` exists. |
| `feat/destroy-webviews-on-hide` | **~150-220 MiB peak WebKitWebProcess eliminated when popup/settings is closed** | The popup and settings webviews are destroyed on focus loss. On Linux/WebKitGTK the shared `WebKitWebProcess` does NOT exit (architectural limitation), but the per-webview DOM/JS heap is freed. |
| `perf/backend-tightening` (unlanded) | **~5-10 MiB host RSS, plus WAL mode DB contention relief** | Lazy updater (skip linking `reqwest` + signature verifier for libwebpki-roots). Bounded `ignored_files` HashMap (was unbounded). SQLite WAL mode lets watcher/scheduler reads proceed concurrently. |

## Platform-specific notes

- **Linux/WebKitGTK (this baseline):** Shared `WebKitWebProcess` model — destroying one WebView doesn't reap the helper process. GPU driver libraries load on first webview creation and stay until shutdown. Most RSS "growth" is shared library mappings, not leaks.
- **Windows/WebView2:** Per-webview process model — destroying a WebView reaps its helper. PR 2's full benefit applies.
- **macOS/WKWebView:** Per-webview process model — destroying a WebView reaps its helper. PR 2's full benefit applies.

## What was investigated but did not lead to code changes

- **`WebKitNetworkProcess` not closing after popup/settings dismiss:** Expected WebKitGTK behavior. The network process is a per-app shared service. Documented as expected, not a bug.
- **Host RSS growth from 40 MiB → 100 MiB on first WebView open:** Caused by `libLLVM.so`, `libnvidia-gpucomp.so`, `libnvidia-eglcore.so`, `libgallium-26.2.3-arch1.1.so` loading on GPU initialization. Cannot be reduced without disabling hardware acceleration (`WEBKIT_FORCE_SOFTWARE_RENDERING=1`), which would slow the UI noticeably.
- **Anonymous heap growth (13 MiB → 45 MiB after first WebView):** Mostly ICU message catalog caching and Tauri's webview-handle state. Could be reduced by lazy-loading i18n locales (10-15 MiB), but not pursued in this round.

## Reproducing this baseline

```bash
cd /path/to/mouzi
git checkout main
npm install
npm run tauri build
./target/release/mouzi &
sleep 3
# Click tray to open popup, click elsewhere to dismiss
# Open settings, interact, close, wait 5s
./docs/scripts/memory-breakdown.sh
```

For an after-PR comparison, repeat with `feat/drop-dead-main-window` checked out and built.

## Related notes

- [[2026-09-30-mouzi-memory-optimization-audit]] — the full audit that drove these PRs
- `docs/scripts/memory-breakdown.sh` — the measurement tool used to capture this data