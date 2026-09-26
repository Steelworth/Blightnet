#!/usr/bin/env bash
# Build Blightnet-x86_64.AppImage next to this repo.
# audio/, assets/, and data/ stay beside the image. They are not packed in.
set -euo pipefail
cd "$(dirname "$0")/.."

BIN="./target/release/blightnet"
if [ ! -x "$BIN" ]; then
  if [ -x "./blightnet" ]; then
    BIN="./blightnet"
  else
    echo "No release binary. Run: cargo build --release" >&2
    exit 1
  fi
fi

ARCH="$(uname -m)"
APP="Blightnet-${ARCH}.AppImage"
DIR="packaging/AppDir"
rm -rf "$DIR"
mkdir -p "$DIR/usr/bin" "$DIR/usr/share/icons/hicolor/256x256/apps"

cp -f "$BIN" "$DIR/usr/bin/blightnet"
chmod +x "$DIR/usr/bin/blightnet"
if [ -f assets/icon.png ]; then
  cp -f assets/icon.png "$DIR/blightnet.png"
  cp -f assets/icon.png "$DIR/usr/share/icons/hicolor/256x256/apps/blightnet.png"
fi

cat > "$DIR/blightnet.desktop" << 'EOF'
[Desktop Entry]
Name=Blightnet
Exec=blightnet
Icon=blightnet
Type=Application
Categories=Game;
Terminal=false
Comment=Native table for Hearthsong and Blight
EOF

cat > "$DIR/AppRun" << 'EOF'
#!/bin/sh
HERE="$(dirname "$(readlink -f "$0" 2>/dev/null || echo "$0")")"
BIN="$HERE/usr/bin/blightnet"
ROOT="$(pwd)"
if [ -n "${APPIMAGE:-}" ]; then
  ROOT="$(dirname "$APPIMAGE")"
fi
d="$ROOT"
i=0
while [ "$i" -lt 5 ]; do
  if [ -f "$d/data/mixer-catalog.json" ]; then
    ROOT="$d"
    break
  fi
  nxt="$(dirname "$d")"
  if [ "$nxt" = "$d" ]; then
    break
  fi
  d="$nxt"
  i=$((i + 1))
done
case "${1:-}" in
  daemon|daemon-status|daemon-stop|node-status|node-stop)
    exec "$BIN" "$1" --root "$ROOT"
    ;;
esac
exec "$BIN" --root "$ROOT" "$@"
EOF
chmod +x "$DIR/AppRun"

TOOL="packaging/appimagetool-${ARCH}.AppImage"
if [ ! -x "$TOOL" ]; then
  echo "Fetching appimagetool…"
  curl -fsSL -o "$TOOL" \
    "https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-${ARCH}.AppImage" \
    || {
      echo "Could not download appimagetool. Install it, then re-run." >&2
      exit 1
    }
  chmod +x "$TOOL"
fi

ARCH="$ARCH" APPIMAGE_EXTRACT_AND_RUN=1 "$TOOL" "$DIR" "$APP"
chmod +x "$APP"
echo "Wrote $APP"
echo "Keep audio/, assets/, and data/ in the same folder, then double-click the image."
echo "If FUSE is missing: APPIMAGE_EXTRACT_AND_RUN=1 ./$APP"
