# Headless mode RSS measurement plan

Prototype of a runtime `--headless` CLI flag for `mouzi` (Tauri 2 file
organizer). Single binary: `mouzi` (GUI) and `mouzi --headless` are the same
file. No Cargo features are used to gate the headless path — the branch lives
in `src-tauri/src/main.rs` and calls `mouzi_lib::headless::run_headless()`.

The point of headless mode: skip every webview-window code path so the binary
never dlopen's WebKitGTK / WebView2 / libLLVM. WebKitGTK stays linked into the
binary (unavoidable in a single artifact), but its hundreds of MiB stay unmapped
until the first `WebviewWindowBuilder`.

## Architecture decisions

- **Single binary**, branch on `std::env::args()` in `main.rs`. No `clap` /
  `argh`. No `[features]` in `Cargo.toml`. No `#[cfg(feature = "...")]` on the
  headless path.
- Headless builder keeps plugins minimal: no `single-instance` (the body's
  guidance — verify before re-adding), no `opener`, no `notification` (none of
  the commands touch them), no `updater`. Tray-only: `system_tray` via `setup()`,
  plus an empty `invoke_handler` so the runtime does not even register the IPC.
- Headless first-run logic mirrors `run()` minus `show_settings_window`.
- Pause / Resume tray items update folder modes (`FOLDER_MODE_PAUSED` vs
  `FOLDER_MODE_SILENT`) in the same SQLite DB the GUI uses, so toggling from
  one mode carries over to the other.

## Files changed

| path                            | lines |
| ------------------------------- | ----- |
| `src-tauri/src/main.rs`         | 22    |
| `src-tauri/src/lib.rs`          | +1 (`pub mod headless;`) |
| `src-tauri/src/headless.rs`     | 195 (new) |
| `docs/headless-rss-measurement-plan.md` | this file (new) |

Final line counts and the exact commit SHA are recorded at the bottom of this
document after the prototype is committed.

## Build recipe

```sh
cd /root/mouzi-prototype/src-tauri
export PATH="/workspace/hermesagent/.rustup/toolchains/stable-aarch64-unknown-linux-gnu/bin:$PATH"
export CARGO_HOME=/home/hermesagent/cargo-home
export CARGO_TARGET_DIR=/root/cargo-target    # /tmp is noexec in this sandbox
cargo build --release
```

`/tmp` is mounted `noexec` in this sandbox, so `CARGO_TARGET_DIR` cannot point
there — the build-script binaries need to be executable. Use a directory on the
writable `/root` filesystem instead.

## Binary measurements

```sh
file /root/cargo-target/release/mouzi
size /root/cargo-target/release/mouzi
ldd /root/cargo-target/release/mouzi | grep -iE 'webkit|nvidia|gpu|llvm'
```

Recorded on this sandbox (Linux 6.17.0-1020-oracle, aarch64):

```
$ file /root/cargo-target/release/mouzi
/root/cargo-target/release/mouzi: ELF 64-bit LSB pie executable, ARM aarch64,
version 1 (SYSV), dynamically linked, interpreter /lib/ld-linux-aarch64.so.1,
BuildID[sha1]=c6034030eb939f37827d514c822be66b09e9b34a, for GNU/Linux 3.7.0,
not stripped

$ size /root/cargo-target/release/mouzi
   text       data     bss      dec      hex   filename
17314984    560688   17232  17892904  1110628   /root/cargo-target/release/mouzi
# text: 17.3 MiB   data: 560 KiB

$ ldd /root/cargo-target/release/mouzi | grep -iE 'webkit|nvidia|gpu|llvm'
        libwebkit2gtk-4.1.so.0 => /lib/aarch64-linux-gnu/libwebkit2gtk-4.1.so.0
```

Expected: WebKitGTK appears in the link table (unavoidable in a single binary)
but is **not** mapped at runtime when `mouzi --headless` runs without ever
creating a `WebviewWindow`. `libnvidia-gpucomp` and `libLLVM` are not pulled
in by `mouzi --headless`'s direct dependency closure — they appear as
transitive deps only when `WebviewWindowBuilder` brings in `webkit2gtk`'s
software-fallback path, which our headless path never touches.

## Expected RSS math

When `mouzi --headless` starts on the nvidia box, it should NOT load:

- `WebKitWebProcess` (one copy per app, never reaped)
- `WebKitNetworkProcess`
- `libLLVM.so.*` (~60 MiB mapped by the first WebView invocation)
- `libnvidia-gpucomp.so.*`
- The JavaScriptCore VM (~40 MiB on cold start)
- `Soup3` HTTP backend caches
- `libgstreamer-*` decoders loaded by WebKit media subsystem

These stay in the link table but stay off the working set. Expected host RSS
breakdown when running `mouzi --headless` on the user's nvidia box:

| component                                | expected RSS |
| ---------------------------------------- | -----------: |
| mouzi process, no webview                |        ~25 MiB |
| system_tray / libayatana-appindicator    |         ~3 MiB |
| glib / gtk init (tray only, no windows)  |        ~5 MiB |
| rusqlite + watcher                       |         ~3 MiB |
| **host total**                           |    **<40 MiB** |

If the GUI path also gets exercised in the same session (e.g. user clicks
"Open GUI" on the headless tray), `WebKitWebProcess` and friends WILL be
spawned and the 474 MiB baseline reappears. This prototype intentionally
solves the steady-state idle case only.

If GTK display init on the user's box eagerly maps some libraries that the
audit attributed to WebView, the host total may climb to 50-80 MiB but should
still be a 5-10× improvement over the 474 MiB baseline. Anything above 120
MiB is a regression worth investigating — paste `pmap -x <pid> | sort -k3 -n
| tail -20` output and we'll trace which library is the culprit.

## RSS measurement recipe (user-side, nvidia box)

```sh
# Start headless mode in one terminal
./mouzi --headless &

# Capture PID
PID=$(pgrep -f 'mouzi --headless')
echo "PID=$PID"

# 1. Quick RSS check via ps
ps -p "$PID" -o pid,rss,vsz,comm

# 2. Detailed breakdown by mapped region
pmap -x "$PID" | head -50
# Look for WebKit, nvidia, llvm, soup, javascriptcore, gtk

# 3. Aggregate RSS across mouzi's child processes too (if any)
pgrep -P "$PID" | xargs -r ps -o pid,rss,comm -p
pgrep -f 'mouzi|webkit' | xargs -r ps -o pid,rss,comm -p

# 4. Use smap_rollup if installed (more accurate shared-memory accounting)
sudo smem -P mouzi
# or
cat /proc/"$PID"/status | grep -E 'VmRSS|VmSize|VmPeak'
```

Steady-state RSS is what to record — start the binary, wait ~10 seconds for
the SQLite db to be opened and the system tray to settle, then capture the
numbers. Do not measure during the first second (rusqlite is warming up).

## Why not a Cargo feature?

The user explicitly chose the runtime flag approach. From the body:

> Architecture (THIS is what we're building):
> - **Single binary.** `mouzi` (GUI) and `mouzi --headless` are the SAME compiled artifact.
> - Do NOT add `[features]` to Cargo.toml. Do NOT use `#[cfg(feature = ...)]` to gate the headless path.

Cargo features would have given us a smaller binary, but:

- the user wants one binary for operational simplicity (one package to install,
  one binary to flag when launching with autostart), and
- a `--headless` flag can be passed via the existing autostart arg list without
  rebuilding (the autostart plugin already passes `--autostart`).

The audit's 474 MiB → <30 MiB target is achievable because WebKitGTK's shared
process model means simply not creating a WebView avoids the process entirely,
not just the ~5 MiB of code we'd save by feature-gating.

## Pointer to the audit

The full memory-audit context lives in
`~/Documents/Obsidian Vault/Research/2026-09-30-mouzi-memory-optimization-audit.md`.
Headline numbers: baseline 474 MiB total RSS, ~95% of which is WebKitGTK +
libLLVM + libnvidia-gpucomp mapped by the first WebView invocation.

## Verification of compile and link state

After `cargo build --release` lands, the final lines of this file record:

- exact `git rev-parse HEAD` of the prototype commit,
- `file target/release/mouzi` output,
- `size target/release/mouzi` text+data columns,
- `ldd target/release/mouzi | grep -iE 'webkit|nvidia|gpu|llvm'` output,
- final lines of the cargo build log.