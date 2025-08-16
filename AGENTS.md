# AGENTS.md

Guidance for autonomous / semi‑autonomous agents collaborating on the `hanzi-cat` project.

## 1. Project Snapshot
`hanzi-cat` is a Rust → WebAssembly (wasm) rhythm / typing game to help users learn Hanzi pronunciation. The crate is configured to build both an rlib and a cdylib (for wasm) and is optimized for very small wasm output (size‑focused release profile, `wasm-opt` flags in metadata).

Current root files:
- `Cargo.toml` (features, wasm-bindgen setup, size-optimized release profile)
- `Cargo.lock`
- `README.md` (not yet summarized here; consult directly for user‑facing instructions)
- `AGENTS.md` (this file)

## 2. Core Technical Stack
- Language: Rust (edition 2024)
- Target: WebAssembly via `wasm-bindgen`
- Browser APIs accessed through `web-sys` (feature-pruned list for minimal bloat)
- Optional dependencies (gated by feature flags): `serde`, `serde_json`, `getrandom` (JS feature), `wee_alloc`, `console_error_panic_hook`

## 3. Cargo Features / Optional Components
Default feature set: `console_error_panic_hook`

Feature intentions:
- `console_error_panic_hook`: Better panic messages in the browser developer console (enabled by default for debug ergonomics).
- `wee_alloc`: Potential smaller global allocator (not yet enabled by default; evaluate size vs. performance when optimizing final builds).
- `serde`, `serde_json`: For (de)serializing structured data (e.g., level charts, score exports). Keep optional to avoid unconditional code size cost.
- `getrandom` with `js` feature: Deterministic randomness via browser APIs when randomness is needed (e.g., pattern generation).

Agents modifying features MUST:
1. Justify the inclusion (impact on binary size, functionality gained).
2. Update documentation references (README and this file if process changes).
3. Keep the `web-sys` feature list pruned – only add capabilities when code actually needs them.

## 4. Build / Test Guidance
(If explicit scripts or instructions are added to README later, synchronize them here.) Typical workflows:
- Development build (example): `wasm-pack build --target web --dev` (Confirm presence of `wasm-pack` in README before relying on it.)
- Release build (example): `wasm-pack build --target web --release`
- Unit / wasm tests: Use `wasm-bindgen-test` (e.g., `wasm-pack test --headless --chrome`). Add concrete commands to README if missing.

Agents MUST verify actual supported commands before execution; do not assume build tooling beyond what is declared.

## 5. Performance & Size Priorities
- Release profile already tuned: `opt-level = "z"`, `lto = true`, `codegen-units = 1`, `strip = true`, `panic = "abort"`.
- Additional size strategies: feature gating, pruning `web-sys`, optional `wee_alloc` evaluation, avoiding large data tables embedded directly (prefer compressed or external fetch if appropriate).

## 6. Error Handling & Logging
- In debug / default builds, `console_error_panic_hook` improves panic readability. Avoid leaving verbose `web_sys::console::log` spam in performance‑critical loops for release.
- Consider a lightweight logging abstraction if logging proliferates; keep bundle small.

## 7. Memory / Allocator Considerations
- If enabling `wee_alloc`, document measurable size reduction vs. any performance tradeoffs. Provide toggling guidance.

## 8. Agent Operational Protocol
Agents MUST follow these steps for every task:
1. Plan: Add granular TODO items via the task management tool (`add_todos`).
2. Status Tracking: Mark each TODO as `doing` before edits and `done` immediately after completing.
3. File Discovery: Use `ls`, `glob`, `grep`, and `view` to inspect before modifying. Never guess content.
4. Small Edits: Use `replace_in_file` with minimal, precise SEARCH/REPLACE blocks. Do not include unrelated lines.
5. New / Large Files: Use `write_to_file` with complete final content.
6. Validation: (When test/build scripts become available) run them prior to completion. If absent, note the omission explicitly.
7. Documentation Sync: If process, features, or build commands change, update both `README.md` (user perspective) and `AGENTS.md` (agent perspective).
8. Completion: Only call `attempt_completion` after confirming prior tool operations succeeded.
9. Don't run start command

## 9. Editing Guidelines
- Keep comments concise, English only.
- Maintain deterministic ordering of feature lists and imports when practical.
- Avoid speculative abstractions; implement only when immediate value or clear near‑term need exists.
- When adding a new browser API via `web-sys`, cite the code location that requires it in the commit message.

## 10. Suggested TODO Template for Feature Work
Example breakdown (adjust granularity to complexity):
1. Investigate existing modules related to <feature>
2. Define data structures / types
3. Implement core logic
4. Integrate with rendering / event system
5. Add tests (or mark gap if infra missing)
6. Update feature flags / dependencies (if any)
7. Update README + AGENTS docs
8. Final review & size considerations

## 11. Handling Data / Assets
- Prefer embedding only minimal bootstrap data. Larger level charts or dictionaries may be loaded asynchronously (keeping wasm small and enabling updates without rebuild).
- If embedding static tables, evaluate compression tradeoffs.

## 12. Security / Integrity Notes
- Avoid executing unvetted remote scripts in build pipeline.
- Keep dependencies minimal to reduce supply chain surface.
- Document any added randomness sources (e.g., for gameplay fairness reproducibility).

## 13. Future Enhancements (Tracking Section)
Agents may append items here with justification:
- Input latency profiling utilities.
- Deterministic seed management for replay export.
- Asset pipeline notes (if binary assets introduced).

## 14. Change Log for This File
- Initial creation (agent): Established baseline operational protocol and project heuristics.
- Combo logic adjustment: Modified scoring so correct but off-timing inputs still advance combo; updated README to reflect new behavior.
- Tone & words update: Added mandatory tone number input (1–5) for each syllable, introduced occasional multi-character word notes, highlight for current target note, and three-life pixel heart system. Updated README accordingly.
- Animated cat SVG upgrade: Replaced simple circle-based cat with gradient-shaded SVG featuring blinking eyes and a tail wag animation; updated README feature bullet to emphasize animation.
- Bottom-centered clawing cat + reactions: Repositioned cat to bottom center, added paw claw animation when notes enter danger zone, surprised/scared styling (outline + exclamation) for notes, and soft room gradient background; updated README feature list.
- Claw strike visual effect: Added transient triple red slash lines emitted on each paw strike to emphasize impact; updated README feature bullet accordingly.
- Sushi bases: Added 10 distinct sushi piece graphics (salmon, tuna, shrimp, tamago, eel, roe gunkan, cucumber maki, salmon maki, avocado roll, octopus) rendered procedurally on canvas beneath each falling Hanzi; updated README features.
- Recovery assistance: Executed dangling blob extraction after an unrecoverable git reset --hard on an unborn branch (no commits yet); enumerated and exported loose object blobs for potential manual file reconstruction. No commit created; recovery directory contents are ephemeral and not tracked.
- Extended recovery enumeration: Re-ran robust loose object enumeration, extracted all available dangling blobs into recovery directories, applied multi-pass Rust source heuristics (pattern scoring, lowered thresholds) but found no definitive Rust source file; documented manual fallback strategies (pattern grep, editor backups, OS-level recovery).
- Web entrypoint: Added index.html loader (ES module) expecting wasm-pack generated pkg/ with init() and start_game(); documented build & serve instructions for development in file comment.
- Hanzi outline contrast: Added consistent black stroke outline for all falling notes (except red danger outline in claw zone) to improve readability against sushi bases.
- GitHub Pages CI: Introduced gh-pages.yml workflow building via wasm-pack (target web, release) and deploying index.html + pkg/ to GitHub Pages.
- CI fix: Replaced ad-hoc curl pipe install of wasm-pack with jetli/wasm-pack-action@v0.4.0 for reliable installation on GitHub-hosted runners.
- Typed text overlay: Moved in-progress pinyin typing buffer from being drawn on the canvas beneath the cat to a dedicated absolutely positioned DOM element (#hc-typing) layered above the cat for higher visibility; removed old canvas draw block and added CSS with z-index.
- Instructions overlay: Added top-left Instructions button and accessible dialog overlay (role="dialog", aria-hidden toggle, rudimentary focus trap) presenting controls and gameplay tips; updated README and index.html accordingly.
- Keypress sound effects: Added lightweight Web Audio oscillator-based feedback for letters, tone digits, Enter, and Backspace (short envelopes, randomized slight pitch). Skips playback while instructions overlay is open to avoid auditory clutter; no external audio assets added. Updated README and index.html.
- Instructions button visibility fix: Restored missing Instructions button markup, dialog overlay structure, and JS toggle/focus logic in index.html so the previously documented feature is actually present and accessible.
- Constant synth beat: Added minimalist procedural kick/snare/hi-hat loop (~120 BPM) starting on first user key or pointer input (unlocked AudioContext), auto-muting while the Instructions overlay is open; updated README and index.html to mention it.
- Instructions button reposition: Moved Instructions button from top-left to top-right beside repo link; updated CSS positioning and README references.
- Interactive beat modulation: Enhanced constant synth beat with keystroke-driven gain swells and optional micro hi-hat blips on letter keys; updated README feature bullet and extended hcAudio with modulate() API.
- Dynamic difficulty ramp: Added linear time-based scaling (~3 min) reducing spawn interval (1400ms → 550ms), increasing fall speed (0.18 → 0.34 px/ms), and raising multi-character word probability (12% → 55%); introduced constants (INITIAL_/FINAL_*, MULTI_CHAR_*), update_difficulty(), choose_note(), and probability-driven multi-character selection; updated README.
- Powerups system: Introduced purchasable powerups (x2 score multiplier, slow time, shield) with costs (1200/800/600), durations (mult 10s, slow 8s), effects (score_multiplier, speed & spawn interval scaling, shield_charges), purchase_powerup(kind) exported via wasm_bindgen, left-side UI panel (#hc-powerups) with buttons & dynamic enabling based on score, README updated.
- Expanded Hanzi dataset: Added numerous additional single characters (directions, body parts, nature terms, basic radicals) and common multi-character words (中国, 天气, 老师, 朋友, 手机, 电脑, 米饭, 语言, etc.) to increase gameplay variety; synchronized README feature bullet.
- Falling-note removal & entrypoint unification (tasks c8-3..c8-6): Eliminated legacy falling-note arcade system (notes structs, spawn/update/render loops, sushi bases, combo hearts, difficulty ramp, powerups). Replaced `start_game` implementation with unified board-mode launcher under the existing `board_mode` feature gate (left in place temporarily). Removed powerups export (`purchase_powerup`) logic and associated UI plus outdated instructions in index.html. Simplified lib.rs to datasets + minimal entrypoints, preparing for full board mode adoption.
- Board mode default (boardfix tasks): Removed `board_mode` feature gate; board module now always compiled and `start_game()` directly launches board-based rhythmic prototype. Purged feature flag reference from Cargo.toml and updated README to describe board prototype (falling-note classic mode & cat visuals retired for now). AGENTS change log updated accordingly; groundwork laid for reintroducing typing & scoring atop board system.

- Added levels 6 and 7 ("Crystal Isle" and "Neon Bastion"): created `src/board_level6.rs` and `src/board_level7.rs`, registered the modules in `src/board.rs`, exported `LEVEL6_HANZI` / `LEVEL7_HANZI`, and extended `LEVEL_SCORE_THRESHOLDS` and the `levels()` list to include the new entries.

- Synchronized repository layout: moved new level modules under `src/board/` to match existing module path expectations.

- Board set_level: Replaced legacy pieces/last_spawn_beat reset with grid reinitialization and cat repositioning. The set_level() implementation now reconstructs state.grid using pick_random_hanzi for non-block tiles, places the cat on a non-block tile (center-biased), and resets beat/temporary modifiers. Verified cargo build completes (warnings only).

- DOM SVG (#hc-cat) positioning migration: Replaced canvas-drawn per-piece "baby chicken" sprite with an absolutely positioned DOM SVG (#hc-cat) anchored over the game canvas. Positioning now uses a CSS-anchor approach and per-frame set_attribute updates to avoid expanding web-sys feature usage; cargo build verified (warnings only). Visual runtime verification in-browser is still recommended to confirm z-index, pointer-events, and interpolation behavior.

- Cat sizing fix: Adjusted DOM #hc-cat SVG sizing so it fits within a single grid cell. The runtime now computes a square cat_size from the smaller of cell_w and cell_h (scaled by a padding factor) and sets inline width/height on #hc-cat each frame to prevent overflow across board sizes; cargo build verified (warnings only). Visual runtime verification recommended to confirm consistent fit and appearance across levels.

- Board neighbor & player-tile update: When initializing level 0 (and when set_level runs), the player's tile is cleared (left empty). Up-to-8 surrounding tiles are populated with distinct hanzi drawn from the single-hanzi pool to guarantee unique adjacent characters for early gameplay; the remainder of the board is filled using an alternating two-character parity pattern. Implementation: src/board/mod.rs (grid prefill and set_level adjustments).

---
This document is a living reference for agents. Update responsibly and keep it tightly aligned with actual repository state.

- Recent edits: Updated board neighbor refill and pick_random_hanzi in src/board/mod.rs; implemented hop-completion neighbor refresh and landing tile consumption.
- Implementation details: pick_random_hanzi now samples from SINGLE_HANZI with fallback to ("你","ni3"); update_pieces now consumes the landing tile on hop finish and, for level 0, repopulates up-to-8 neighbor tiles with unique entries drawn from SINGLE_HANZI and parity-fills remaining non-block tiles.
- Build: Ran `cargo build --verbose`; compilation completed successfully with warnings (20 warnings).
- Task tracking: Marked todos t18-3 and t18-4 as done.
- Recent agent edits (automated): Applied focused Clippy-oriented cleanups to src/board/*: collapsed a nested keyboard handler if; removed unnecessary parentheses around beat-phase math; introduced a FrameCallback type alias; added a documented #[allow(clippy::missing_const_for_thread_local)] above BOARD_STATE as a minimal mitigation; updated TODO statuses (todo-25, todo-26, todo-27) and committed the changes.
