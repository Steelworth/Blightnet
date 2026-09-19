#!/usr/bin/env bash
cd "$(dirname "$0")" || exit 1

need_build=0
if [ ! -x "./target/release/blightnet" ]; then
  need_build=1
elif [ -d src ]; then
  newest_src=$(find src Cargo.toml Cargo.lock data/mixer-catalog.json -type f -printf '%T@\n' 2>/dev/null | sort -n | tail -1)
  bin_time=$(stat -c '%Y' ./target/release/blightnet 2>/dev/null || echo 0)
  if [ -n "$newest_src" ]; then
    src_int=${newest_src%.*}
    if [ "$src_int" -gt "$bin_time" ]; then
      need_build=1
    fi
  fi
fi

if [ "$need_build" -eq 1 ]; then
  echo "Building native Blightnet…"
  if ! command -v cargo >/dev/null 2>&1; then
    echo "Rust/cargo is not on PATH. Install it from https://rustup.rs then run start.sh again."
    [ -t 0 ] && read -r -p "Press Enter to close."
    exit 1
  fi
  # GUI launchers often skip ~/.cargo/bin
  export PATH="$HOME/.cargo/bin:$PATH"
  cargo build --release || {
    echo "Build failed."
    [ -t 0 ] && read -r -p "Press Enter to close."
    exit 1
  }
fi

BIN="./target/release/blightnet"
cp -f "$BIN" ./blightnet 2>/dev/null || true
chmod +x "$BIN" ./blightnet 2>/dev/null || true

echo "Starting Blightnet…"
set +e
"$BIN" "$@" >blightnet.log 2>&1
status=$?
if [ "$status" -ne 0 ]; then
  echo
  echo "Blightnet exited with status $status."
  echo "Last lines of blightnet.log:"
  tail -n 40 blightnet.log 2>/dev/null || true
  [ -t 0 ] && read -r -p "Press Enter to close."
  exit "$status"
fi
