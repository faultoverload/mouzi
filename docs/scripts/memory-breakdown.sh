#!/usr/bin/env bash
# Mouzi memory breakdown — diagnose host RSS growth after opening settings.
# Run twice: once before opening settings, once after closing settings.
# Compare the "Top mappings" sections to see which libraries account for the delta.
#
# Usage: ./docs/scripts/memory-breakdown.sh [pid]
#        (or just run it; it auto-discovers the mouzi pid)
set -euo pipefail

PID="${1:-$(pgrep -x mouzi | head -1)}"
if [ -z "$PID" ] || ! kill -0 "$PID" 2>/dev/null; then
  echo "mouzi not running (pid empty or dead)" >&2
  exit 1
fi

echo "=== mouzi PID: $PID ==="
echo "smaps_rollup:"
grep -E '^(Rss|Pss|Shared_Clean|Shared_Dirty|Private_Clean|Private_Dirty|Anon)' \
  /proc/$PID/smaps_rollup 2>/dev/null | sed 's/^/  /' \
  || echo "  (smaps_rollup not available on this kernel)"

echo ""
echo "WebKit helper processes:"
ps -eo pid,ppid,rss,comm | awk -v p="$PID" '$2 == p { printf "  %-7s RSS=%6d KB (%.1f MiB)  %s\n", $1, $3, $3/1024, $4 }'

echo ""
echo "Top 15 .so mappings by RSS (KB) — first row of pmap shows the total:"
pmap -q "$PID" 2>/dev/null | tail -n +2 | awk '
  /\.so/ {
    # Field format: start K K offset device inode path
    # Total RSS is column 2 in pmaps format "Address Kbytes ..."
    # pmap -q uses Kbytes as column 2
    path = $NF
    # Get basename for short label
    n = split(path, parts, "/")
    base = parts[n]
    sub(/\.so.*/, ".so", base)
    rss = $2
    total[base] += rss
    count[base]++
  }
  END {
    for (b in total) print total[b], count[b], b
  }
' | sort -rn | head -15 | awk '{ printf "  %6d KB  (%d mappings)  %s\n", $1, $2, $3 }'

echo ""
echo "Total mapped .so count:"
pmap -q "$PID" 2>/dev/null | grep -c '\.so' || true

echo ""
echo "Grouped by major dep family (rough totals — same library may appear under multiple):"
pmap -q "$PID" 2>/dev/null | awk '
  /\.so/ {
    path = $NF
    if (path ~ /libwebkit2gtk|libjavascriptcore/) family="webkit"
    else if (path ~ /libsoup/) family="libsoup (network)"
    else if (path ~ /libgtk|libgdk|libgobject|libgio/) family="gtk"
    else if (path ~ /libpango|libcairo|libharfbuzz/) family="pango/cairo/text"
    else if (path ~ /libicu/) family="icu (i18n)"
    else if (path ~ /libsqlite/) family="sqlite"
    else if (path ~ /libstdc|libgcc/) family="libstdc/libgcc"
    else if (path ~ /libc\.so/) family="libc"
    else if (path ~ /libm\.so/) family="libm"
    else if (path ~ /libwebkit/) family="webkit (other)"
    else if (path ~ /librsvg/) family="rsvg"
    else if (path ~ /libnotify/) family="libnotify"
    else family="other"
    total[family] += $2
  }
  END {
    for (f in total) printf "  %6d KB  %s\n", total[f], f
  }
' | sort -rn | head -15

echo ""
echo "Anon RSS (heap + stack, NOT shared libs):"
grep -E '^RssAnon' /proc/$PID/smaps_rollup 2>/dev/null \
  || awk '/^VmRSS:/ && !/^VmRSS/{rss+=$2} END{print "  VmRSS lines summed:", rss, "KB"}' /proc/$PID/status
