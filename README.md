# Hanzi Cat

A tiny Rust → WebAssembly rhythm / reaction & typing game to help you practice Hanzi recognition and pinyin + tone input. Built with size minimization in mind (aggressively optimized release profile and pruned `web-sys` features).

## Gameplay Snapshot
(Transition phase: classic falling-note mode has been removed. A new board-based rhythmic prototype is now the default. Typing + scoring systems are being re-integrated.)
- Board-based prototype: Hanzi pieces (starting with "你") spawn at defined points and hop tile-to-tile each beat across an 8×8 grid toward goal tiles.
- Beat-synchronized hop animation with a simple parabolic lift for visual clarity.
- Obstacles demo: blocks (impassable), teleport, conveyors (auto-push), tempo shift (temporary faster hop timing), and a transform tile that can swap one Hanzi to another (e.g., 你→好) to preview upcoming character transformation mechanics.
- Automatic spawning every 4 beats (soft cap of 5 concurrent pieces in current prototype) with greedy Manhattan pathing toward any goal tile.
- Reaching a goal awards placeholder score; combo, lives, and powerups have been removed pending redesigned progression & challenge curves.
- Datasets of single and multi-character Hanzi + pinyin retained for upcoming typing reattachment (typing input not yet hooked into board logic; keystroke audio feedback still functions).
- Instructions overlay (top-right) remains for quick reference and will evolve to include board-specific controls and mechanics as they mature.
- Minimalist procedural beat & keystroke sound effects still active (audio context unlocked on first input) for rhythmic context.
- Expanded Hanzi / word dataset preserved (directions, body parts, nature, basics, common words) for future spawning variety once selection logic is integrated with board mode.

## Controls
| Action | Key(s) |
| ------ | ------ |
| Type pinyin | letter keys (ASCII) |
| Tone number | `1 2 3 4 5` (5 = neutral when used) |
| Submit | `Enter` |
| Edit buffer | `Backspace` |
| Close instructions overlay | `Esc` or click Close |
| Open instructions overlay | Click "Instructions" button (top‑right) |

## Instructions Overlay
Click the "Instructions" button in the top‑right at any time to view gameplay help. The overlay:
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

