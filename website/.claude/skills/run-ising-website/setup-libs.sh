#!/usr/bin/env bash
# No-sudo fallback: make Playwright's headless Chromium runnable without root.
#
# The bundled Chromium needs a few system libraries (libnspr4, libnss3,
# libasound2). The clean way to install them is:
#     sudo npx playwright install-deps chromium
# but if you don't have sudo, this script downloads those .deb packages and
# extracts them into a local dir next to the driver. driver.mjs auto-detects
# the extracted libs and points Chromium at them via LD_LIBRARY_PATH.
#
# Idempotent: re-running just re-verifies. Requires apt-get + dpkg-deb (present
# on Debian/Ubuntu) and network access to the distro archive.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LIBDIR="$HERE/.chromium-libs"
SO="$LIBDIR/root/usr/lib/x86_64-linux-gnu/libnss3.so"

if [ -f "$SO" ]; then
  echo "chromium libs already present at $LIBDIR"
  exit 0
fi

mkdir -p "$LIBDIR/debs"
cd "$LIBDIR/debs"
# libnss3 pulls in libnssutil3/libsmime3; libasound2t64 is the t64 rename of libasound2.
apt-get download libnss3 libnspr4 libasound2t64
for d in *.deb; do dpkg-deb -x "$d" "$LIBDIR/root"; done

echo "extracted:"
find "$LIBDIR/root" -name '*.so*' | grep -iE 'nspr4|nss3|asound' | sort
echo "done — driver.mjs will pick these up automatically"
