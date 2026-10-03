#!/usr/bin/env bash
# Build Blightnet-x86_64.AppImage next to this repo.
# audio/, assets/, and data/ stay beside the image. They are not packed in.
# The ffmpeg sidecar is also beside the image, not inside the squashfs.
# Run this on Ubuntu 22.04. A newer host glibc is refused.
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

host_glibc="$(getconf GNU_LIBC_VERSION | awk '{print $2}')"
host_major="${host_glibc%%.*}"
host_rest="${host_glibc#*.}"
host_minor="${host_rest%%.*}"
if [ "$host_major" -gt 2 ] || { [ "$host_major" -eq 2 ] && [ "$host_minor" -gt 35 ]; }; then
  echo "Refusing to bundle: host glibc is ${host_glibc}, need 2.35 or older (Ubuntu 22.04). Do not copy Hellcat libs." >&2
  exit 1
fi

check_elf() {
  local f="$1"
  local label="$2"
  local ver abi_line abi
  while IFS= read -r ver; do
    [ -n "$ver" ] || continue
    local maj min rest
    rest="${ver#GLIBC_}"
    maj="${rest%%.*}"
    min="${rest#*.}"
    min="${min%%.*}"
    if [ "$maj" -gt 2 ] || { [ "$maj" -eq 2 ] && [ "$min" -gt 35 ]; }; then
      echo "${label} needs ${ver} (max GLIBC_2.35)" >&2
      exit 1
    fi
  done < <(objdump -T "$f" 2>/dev/null | grep -o 'GLIBC_[0-9.]*' | sort -u || true)
  abi_line="$(readelf -n "$f" | awk -F'ABI: ' '/OS: Linux/ {print $2; exit}')"
  abi="${abi_line%%,*}"
  abi="${abi%% *}"
  case "$abi" in
    6.*)
      echo "${label} has Linux ABI note ${abi} (6.1 host is a fail)" >&2
      exit 1
      ;;
  esac
}

check_elf "$BIN" "blightnet"

ARCH="$(uname -m)"
APP="Blightnet-${ARCH}.AppImage"
DIR="packaging/AppDir"
rm -rf "$DIR"
mkdir -p "$DIR/usr/bin" "$DIR/usr/lib" "$DIR/usr/share/ca-certificates" \
  "$DIR/usr/share/icons/hicolor/256x256/apps"

cp -f "$BIN" "$DIR/usr/bin/blightnet"
chmod +x "$DIR/usr/bin/blightnet"

copy_soname() {
  local soname="$1"
  local src
  src="$(ldd "$DIR/usr/bin/blightnet" | awk -v s="$soname" '$1 == s { print $3; exit }')"
  if [ -z "$src" ] || [ ! -f "$src" ]; then
    echo "DT_NEEDED ${soname} but it is not on this Ubuntu 22.04 host" >&2
    exit 1
  fi
  cp -L "$src" "$DIR/usr/lib/${soname}"
  chmod 755 "$DIR/usr/lib/${soname}"
  patchelf --set-rpath '$ORIGIN' "$DIR/usr/lib/${soname}"
  check_elf "$DIR/usr/lib/${soname}" "$soname"
  if ! readelf -d "$DIR/usr/lib/${soname}" | grep -q 'RUNPATH'; then
    echo "${soname} has no RUNPATH" >&2
    exit 1
  fi
}

needed="$(objdump -p "$DIR/usr/bin/blightnet" | awk '/NEEDED/ { print $2 }')"
has_needed() { printf '%s\n' "$needed" | grep -qx "$1"; }

# Always bundle the three the payload links when present.
# Opus and speexdsp only if this binary actually DT_NEEDs them.
for soname in libssl.so.3 libcrypto.so.3 libasound.so.2; do
  if has_needed "$soname" || { [ "$soname" = "libcrypto.so.3" ] && has_needed "libssl.so.3"; }; then
    copy_soname "$soname"
  fi
done
for soname in libopus.so.0 libspeexdsp.so.1; do
  if has_needed "$soname"; then
    copy_soname "$soname"
  fi
done

patchelf --set-rpath '$ORIGIN/../lib' "$DIR/usr/bin/blightnet"
if ! readelf -d "$DIR/usr/bin/blightnet" | grep -q 'RUNPATH.*\$ORIGIN/../lib'; then
  echo "blightnet RUNPATH is not \$ORIGIN/../lib" >&2
  exit 1
fi

if [ ! -f /etc/ssl/certs/ca-certificates.crt ]; then
  echo "Missing jammy CA bundle /etc/ssl/certs/ca-certificates.crt" >&2
  exit 1
fi
cp -L /etc/ssl/certs/ca-certificates.crt "$DIR/usr/share/ca-certificates/ca-certificates.crt"

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
export LD_LIBRARY_PATH="$HERE/usr/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
# Sidecar ffmpeg lives beside the AppImage, not on the squashfs mount ($HERE).
if [ -n "${APPIMAGE:-}" ]; then
  PATH="$(dirname "$APPIMAGE"):$PATH"
  export PATH
fi
if [ -f "$HERE/usr/share/ca-certificates/ca-certificates.crt" ]; then
  export SSL_CERT_FILE="$HERE/usr/share/ca-certificates/ca-certificates.crt"
fi
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

TOOL="packaging/appimagetool-13-${ARCH}.AppImage"
if [ ! -x "$TOOL" ]; then
  echo "Fetching appimagetool 13…"
  # Release 13 assets were renamed obsolete-* on GitHub. Still AppImageKit 13, not continuous.
  curl -fsSL -o "$TOOL" \
    "https://github.com/AppImage/AppImageKit/releases/download/13/obsolete-appimagetool-${ARCH}.AppImage" \
    || curl -fsSL -o "$TOOL" \
    "https://github.com/AppImage/AppImageKit/releases/download/13/appimagetool-${ARCH}.AppImage" \
    || {
      echo "Could not download AppImageKit 13 appimagetool. Not using continuous." >&2
      exit 1
    }
  chmod +x "$TOOL"
fi

ARCH="$ARCH" APPIMAGE_EXTRACT_AND_RUN=1 "$TOOL" "$DIR" "$APP"
chmod +x "$APP"

if ! command -v ffmpeg >/dev/null 2>&1; then
  echo "ffmpeg is not on this Ubuntu 22.04 host; sidecar not written" >&2
  exit 1
fi
cp -L "$(command -v ffmpeg)" "./ffmpeg"
chmod +x ./ffmpeg
check_elf "./ffmpeg" "ffmpeg sidecar"

echo "Wrote $APP"
echo "ffmpeg sidecar is beside the image (dirname of the AppImage), not inside it."
echo "Keep audio/, assets/, data/, and ffmpeg in the same folder, then double-click the image."
echo "If FUSE is missing: APPIMAGE_EXTRACT_AND_RUN=1 ./$APP"
