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

---
This document is a living reference for agents. Update responsibly and keep it tightly aligned with actual repository state.

