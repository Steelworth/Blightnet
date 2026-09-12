# Contributing

## Run it

```bash
./start.sh
```

Windows: double-click `Blightnet.exe` (or `start.bat`). Python 3 is only required on Linux/macOS. The server is stdlib-only.

Do **not** open `index.html` from the folder. Use **http://127.0.0.1:8765**.

## What not to commit

- `uploads/` — files the host shared at the table
- `tools/_go/` — downloaded Go toolchain
- `tools/_raw/` and `tools/_wav/` — source dumps for bundled audio
- `__pycache__/`, `.env`, editor folders

`.gitignore` already covers these.

## Windows executable

From Linux, with network access:

```bash
./tools/build_windows.sh
```

That writes `Blightnet.exe` next to `index.html`.

## Notes

Keep table copy original. Do not paste copyrighted book flavor (D&D or Cyberpunk) into JSON or UI strings.

INDEX **02 UPDATE** pulls from `https://github.com/Steelworth/Blightnet` (`main`). It prefers `git fetch` + fast-forward when this folder is a clone; otherwise it compares Git blob hashes to the GitHub tree and downloads only changed files. `uploads/` is never overwritten.
