#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
GO_DIR="$ROOT/tools/_go"
GO_VER="1.24.6"
ARCH="$(uname -m)"
case "$ARCH" in
  x86_64|amd64) GO_ARCH="amd64" ;;
  aarch64|arm64) GO_ARCH="arm64" ;;
  *) echo "Unsupported host arch: $ARCH" >&2; exit 1 ;;
esac
TARBALL="go${GO_VER}.linux-${GO_ARCH}.tar.gz"
URL="https://go.dev/dl/${TARBALL}"

if [[ ! -x "$GO_DIR/go/bin/go" ]]; then
  echo "Downloading Go ${GO_VER}…"
  mkdir -p "$GO_DIR"
  curl -fsSL "$URL" -o "$GO_DIR/$TARBALL"
  tar -C "$GO_DIR" -xzf "$GO_DIR/$TARBALL"
fi

export GOTOOLCHAIN=local
export GOOS=windows
export GOARCH=amd64
export CGO_ENABLED=0
"$GO_DIR/go/bin/go" -C "$ROOT/tools/winlaunch" build -trimpath -ldflags="-s -w" -o "$ROOT/Blightnet.exe"
echo "Wrote $ROOT/Blightnet.exe"
ls -lh "$ROOT/Blightnet.exe"
