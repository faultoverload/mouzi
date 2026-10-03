# Headless mode memory measurement recipe

Step-by-step procedure to capture the resident-set-size (RSS) of `mouzi --headless` on Linux, macOS, and Windows. Used to verify the 474 MiB → <30 MiB idle-working-set claim in the 2026-09-30 audit.

## Background

The prototype single binary lives at:

- Linux: `/root/cargo-target/release/mouzi` (this sandbox) or `/usr/bin/mouzi` (after `.deb` / `.rpm` install)
- macOS: `/usr/local/bin/mouzi` (after `brew install`)
- Windows: `target\release\mouzi.exe` (cargo build) or the MSI-installed binary

`mouzi --headless` is the **same artifact** as the GUI build. There is no separate package.

### 1.1 What the prototype's binary looks like

From the commit that landed the prototype (`0d3cca8`):

```
$ file target/release/mouzi
ELF 64-bit LSB pie executable, ARM aarch64, dynamically linked,
interpreter /lib/ld-linux-aarch64.so.1, BuildID[sha1]=…, for GNU/Linux 3.7.0,
not stripped

$ size target/release/mouzi
   text       data     bss      dec      hex   filename
17314984    560688   17232  17892904  1110628   target/release/mouzi
# text: 17.3 MiB   data: 560 KiB

$ ldd target/release/mouzi | grep -iE 'webkit|nvidia|gpu|llvm'
        libwebkit2gtk-4.1.so.0 => /lib/aarch64-linux-gnu/libwebkit2gtk-4.1.so.0
```

The link table includes WebKitGTK because the GUI path's transitive deps need it. Crucially, **none of `libLLVM.so.*`, `libnvidia-gpucomp.so.*`, `libwebkit2gtk-extensions.so.*`, or `JavaScriptCore`** show up in this `ldd` output — they are pulled in transitively only when `WebviewWindowBuilder` constructs a WebView, which `mouzi --headless` never does.

## 2. Linux recipe

### 2.1 Start the headless binary

In one terminal:

```sh
mouzi --headless &
HEADLESS_PID=$!
echo "headless PID=$HEADLESS_PID"
sleep 10   # let rusqlite open, tray init settle, no first-second warmup
```

### 2.2 Per-process RSS via `ps`

```sh
ps -p "$HEADLESS_PID" -o pid,rss,vsz,comm
# RSS column is in KiB; multiply by 1024 for bytes, by /1024 for MiB.
```

### 2.3 Whole-process tree (catches stray helpers)

The headless build is supposed to fork **zero** child processes — no WebKitWebProcess, no WebKitNetworkProcess, no helper subprocess at all. Verify:

```sh
pgrep -P "$HEADLESS_PID"     # should be empty
pgrep -af 'webkit|mouzi'    # only mouzi's own PID should match
```

If any child PIDs appear under `pgrep -P`, something in `lib.rs` is reaching past the `headless::run_headless` boundary — file a bug, don't trust the measurement.

### 2.4 Aggregate RSS via `smem` (recommended)

`smem` accounts for shared memory correctly. Install: `sudo apt install smem`.

```sh
sudo smem -P mouzi   # shows Pss (proportional set size), Rss, and per-process totals
```

Compare:

| Metric                          | What it tells you                                         |
| ------------------------------- | --------------------------------------------------------- |
| `Rss` column                    | Naive working-set size (over-counts shared libs)          |
| `Pss` column                    | Working-set size with shared libs apportioned             |
| Sum of `Pss` across the tree    | True memory pressure attributable to mouzi                |

For a sanity check, `Pss` should be `< 30 MiB` if GTK init stays clean, `< 80 MiB` if GTK eagerly maps more libs (still a 5-10× improvement over the 474 MiB baseline).

### 2.5 Quick `/proc` fallback

If `smem` is not installed:

```sh
cat /proc/$HEADLESS_PID/status | grep -E 'VmRSS|VmSize|VmPeak|VmHWM'
# VmRSS is current resident size in KiB; VmHWM is the high-water mark.
```

### 2.6 Mapping breakdown (debug only)

If RSS is suspiciously high (e.g. > 120 MiB):

```sh
pmap -x "$HEADLESS_PID" | head -80
# Look for libLLVM, libnvidia, libwebkit2gtk, libgstreamer, libjavascriptcore
# of size > 1 MiB — none of those should be mapped at all in headless mode.
```

You can also `sort -k3 -n` on the `pmap` output to find the top consumers:

```sh
pmap -x "$HEADLESS_PID" | sort -k3 -n | tail -20
```

### 2.7 Environment variables that affect the measurement

These are the variables the systemd unit sets. If you run `mouzi --headless` outside the unit (e.g. via the autostart .desktop on a non-systemd desktop), the variables may differ and the measurement will differ accordingly.

| Variable                            | Effect                                                                    |
| ----------------------------------- | ------------------------------------------------------------------------- |
| `WEBKIT_DISABLE_DMABUF_RENDERER=1`  | Forces the WebKit Cairo fallback path. Already set by `main.rs`.          |
| `WEBKIT_DISABLE_COMPOSITING_MODE=1` | Skips the WebKit compositor. Belt-and-braces fallback.                    |
| `GSK_RENDERER=cairo`                | Forces GTK4 to software rendering — keeps `libnvidia-gpucomp` unmapped.    |

Run with the unit's environment explicitly to reproduce the unit's measurement:

```sh
systemctl --user show mouzi-headless -p Environment   # see what the unit sets
# or, manually:
WEBKIT_DISABLE_DMABUF_RENDERER=1 \
WEBKIT_DISABLE_COMPOSITING_MODE=1 \
GSK_RENDERER=cairo \
  mouzi --headless
```

## 3. macOS recipe

The macOS path uses WKWebView instead of WebKitGTK; the savings are similar but the helper process is `com.apple.WebKit.WebContent` (one per WebView, not the shared-process model).

```sh
mouzi --headless &
HEADLESS_PID=$!
sleep 10

# 1. Per-process RSS
ps -p "$HEADLESS_PID" -o pid,rss,vsz,comm
# RSS is in KiB on macOS too.

# 2. No helper processes expected
pgrep -P "$HEADLESS_PID"   # expect nothing
pgrep -af 'WebContent|mouzi'   # only mouzi itself

# 3. Detailed mapping (lldb-style; vmmap ships with Xcode)
sudo vmmap --summary "$HEADLESS_PID"
```

macOS won't show `libnvidia-gpucomp` (Apple Silicon Macs use Metal, not CUDA), but you should still see a steep drop vs the GUI baseline because WKWebView's per-instance helper process is avoided.

## 4. Windows recipe

Windows uses WebView2; savings come from avoiding `msedgewebview2.exe` helper processes.

```powershell
# Start headless via PowerShell (no console flash)
Start-Process -FilePath mouzi.exe -ArgumentList '--headless' -WindowStyle Hidden

# Find PID
Get-Process -Name mouzi | Select-Object Id,WorkingSet64,VirtualMemorySize64
# WorkingSet64 is bytes; divide by 1MB for MiB.

# Confirm no WebView2 helper processes
Get-Process | Where-Object { $_.ProcessName -like 'msedgewebview2*' }
# Expect: empty.
```

`WorkingSet64` is the Windows analogue of RSS but is updated less aggressively than `ps` on Linux. Wait ≥ 30 s after launch for it to stabilize.

## 5. What to record

When reporting numbers back, please send:

| field           | Linux example                                 |
| --------------- | --------------------------------------------- |
| host            | e.g. "Framework 13, Ryzen 7 7840U, Fedora 41" |
| kernel          | `uname -r` output                             |
| session         | Wayland / X11 / headless                     |
| env             | systemd unit / autostart .desktop / manual    |
| mouzi build     | commit SHA + `git describe`                   |
| headless PID    | —                                             |
| time-after-start| (10 s / 60 s / 5 min — three data points)     |
| VmRSS / RSS     | from `ps` (KiB)                               |
| Pss            | from `smem -P mouzi` (KiB)                    |
| child processes | `pgrep -P $HEADLESS_PID` output               |
| pmap top-20     | if VmRSS > 120 MiB                            |

A single RSS measurement is not enough — capture at 10 s, 60 s, and 5 min after launch to see warm-up vs steady-state behavior.

## 6. Why the prototype's number matters

The prototype's binary itself (`file target/release/mouzi`) shows WebKitGTK in the link table — that is **expected** and unavoidable in a single binary. The point of `mouzi --headless` is that **runtime mapping** never reaches `libwebkit2gtk-4.1.so.0`'s heavyweight initialization paths: no JavaScriptCore VM, no `Soup3` HTTP backend caches, no `WebKitWebProcess` fork, no `WebKitNetworkProcess`. On a stock Linux box, the headless process should hold at the rust+glib baseline (~25 MiB) plus the system-tray attachment (~3 MiB) plus whatever GTK init eagerly maps (~5 MiB).

If you see `libLLVM.so.*` or `libnvidia-gpucomp.so.*` in `pmap` output, that is a regression — the headless path should never reach the WebView code path that would load them. File an issue with `pmap -x $PID | sort -k3 -n | tail -20` output attached.