# Contributing

## Run it

```bash
./start.sh
```

Windows: `start.bat`. This is a **native Rust** window. It does not use a browser.

```bash
cargo run --release
```

Keep `audio/`, `assets/`, and `data/` next to the binary.

## What not to commit

- `uploads/` — local table dumps
- `target/` — Cargo build
- `__pycache__/`, `.env`, editor folders

## Notes

Keep table copy original. Do not paste copyrighted book flavor (D&D or Cyberpunk) into JSON or UI strings.
