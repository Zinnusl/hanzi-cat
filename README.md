# Hanzi Cat

A tiny Rust → WebAssembly rhythm / reaction & typing game to help you practice Hanzi recognition and pinyin + tone input. Built with size minimization in mind (aggressively optimized release profile and pruned `web-sys` features).

## Gameplay Snapshot
- Hanzi (and occasional multi‑character words) fall toward an animated cat at the bottom.
- Type the pinyin syllable (letters only, no tones marks; use `v` for ü) followed by the tone number `1–5`, then press `Enter`.
- Correct submissions score and advance your combo (combo advances even if timing is a little off; fine timing scoring may evolve later).
- Three pixel hearts = your lives. Misses / incorrect submissions can remove a heart and break combo.
- When a note nears the cat's claws it gains a red danger outline; claws may strike with a red slash effect when notes are in the danger zone.
- A variety of sushi bases render beneath each falling Hanzi for visual flair and separation.
- The in‑progress typing buffer appears in a dedicated overlay element for visibility.
- NEW: An Instructions button (top‑left) opens an accessible overlay with controls & tips.

## Controls
| Action | Key(s) |
| ------ | ------ |
| Type pinyin | letter keys (ASCII) |
| Tone number | `1 2 3 4 5` (5 = neutral when used) |
| Submit | `Enter` |
| Edit buffer | `Backspace` |
| Close instructions overlay | `Esc` or click Close |
| Open instructions overlay | Click "Instructions" button (top‑left) |

## Instructions Overlay
Click the "Instructions" button in the top‑left at any time to view gameplay help. The overlay:
- Is marked with `role="dialog"` and toggles `aria-hidden` for accessibility.
- Traps focus rudimentarily while open.
- Can be closed with the Close button or the `Escape` key.

(Currently the game world keeps running while the overlay is open; a full pause feature may be introduced later.)

## Building From Source
Prerequisites:
- Rust toolchain (latest stable recommended)
- `wasm-pack` installed (https://rustwasm.github.io/wasm-pack/installer/)

Build (development):
```
wasm-pack build --target web --dev
```

Build (release, smaller output):
```
wasm-pack build --target web --release
```
This produces a `pkg/` directory containing the JS glue + `.wasm` binary. Ensure `index.html` (at repo root) can find `pkg/` alongside it.

### Running Locally
Any static file server works. Examples:
```
python -m http.server 5173
# or
npx serve .
```
Then open `http://localhost:5173/` (adjust port if different). Simply opening the file URL may be blocked by browser wasm MIME handling; prefer an HTTP server.

## Size / Performance Notes
Release profile favors small binary size:
- `opt-level = "z"`, `lto = true`, `codegen-units = 1`, `strip = true`, `panic = "abort"`.
Optional dependencies (feature‑gated) keep the core lean. Only enable what you need.

## Repository Layout
| Path | Purpose |
| ---- | ------- |
| `src/lib.rs` | Core game logic exported to JS via `wasm-bindgen` |
| `index.html` | Loader page + Instructions UI |
| `Cargo.toml` | Crate metadata & feature flags |
| `AGENTS.md` | Protocol & change log for autonomous agent contributions |

## Contributing
Keep additions lean. Document any newly required browser APIs in both code comments and (if substantial) `AGENTS.md`. Update README for user‑visible features.

## Planned / Potential Enhancements
- Replay / seed management for deterministic sessions.
- Pause state & overlay integration.
- Audio feedback / rhythm synchronization.
- Extended scoring granularity based on timing windows.

## License
(Choose a license; currently unspecified.)

---
Have fun practicing tones with Hanzi Cat!

