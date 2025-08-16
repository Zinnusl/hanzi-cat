//! Board-based rhythmic gameplay prototype (feature = "board_mode").
//! This module introduces a chess / grid board where Hanzi "pieces" hop tile-to-tile
//! in time with the musical beat instead of falling vertically. It is intentionally
//! scaffolded and non-invasive: nothing here is invoked unless the `board_mode`
//! feature is enabled and `start_board_mode()` is called from JS.
//!
//! Goals (future steps, referenced by top-level TODO IDs in main plan):
//! - c4: Beat clock & scheduled piece spawning / hopping logic
//! - c5: Rendering of board grid and animated piece hops
//! - c6: Obstacles & modifiers influencing movement and Hanzi transformations
//! - c7: Level definitions & progression sequencing
//! - c8: Removal / refactor of legacy falling-note system once parity achieved
//! - c9: Relax size optimization constraints (handled in Cargo.toml / profiles)
//!
//! This file currently focuses on data structures + a minimal ticking harness so we
//! can implement gameplay incrementally.
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, window};

// --- Core Time / Beat Model -------------------------------------------------

/// BeatClock tracks timing relative to BPM for scheduling hops.
struct BeatClock {
    bpm: f64,           // beats per minute
    start_ms: f64,      // performance.now() when started
    last_beat_idx: i64, // index of last processed whole beat
}

impl BeatClock {
    fn new(bpm: f64, now: f64) -> Self {
        Self {
            bpm,
            start_ms: now,
            last_beat_idx: -1,
        }
    }
    fn beat_duration_ms(&self) -> f64 {
        60_000.0 / self.bpm
    }
    fn current_beat(&self, now: f64) -> f64 {
        (now - self.start_ms) / self.beat_duration_ms()
    }
}

// --- Board / Tiles / Obstacles / Modifiers ----------------------------------

/// Kinds of obstacles that occupy or affect tiles.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub enum ObstacleKind {
    Block, // Cannot enter
    Teleport {
        to: (u8, u8),
    }, // Enter -> instantly relocate
    Conveyor {
        dx: i8,
        dy: i8,
    }, // Auto-push piece after landing
    TempoShift {
        mult: f64,
        beats: u32,
    }, // Temporary BPM multiplier effect when stepped on
    /// Ice: slippery tile — a piece that arrives continues moving in its incoming
    /// direction on subsequent beats (momentum = 1). If a piece has no direction
    /// when arriving, it will choose a greedy step toward the goal and then slide.
    Ice,
    /// JumpPad: launches a piece further in a direction. If dx/dy are zero the
    /// pad will launch toward the nearest goal. strength = how many tiles to jump.
    JumpPad {
        dx: i8,
        dy: i8,
        strength: u8,
    },
    Transform, // Placeholder: triggers Hanzi transformation mapping (handled by ModifierKind::TransformMap)
}

/// Tile modifiers (non-exclusive with some obstacles) that adjust piece / hanzi logic.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub enum ModifierKind {
    ScoreMult {
        factor: f64,
        beats: u32,
    },
    SlowHop {
        factor: f64,
        beats: u32,
    },
    TransformMap {
        // Map from original pinyin (or hanzi) to alternate variant
        pairs: &'static [(&'static str, &'static str)],
    },
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TileDesc {
    pub obstacle: Option<ObstacleKind>,
    pub modifier: Option<ModifierKind>,
}

/// Level grid descriptor (immutable). We use a flat vector row-major.
#[allow(dead_code)]
pub struct LevelDesc {
    pub name: &'static str,
    pub width: u8,
    pub height: u8,
    pub bpm: f64,
    pub tiles: &'static [TileDesc],        // length = width * height
    pub spawn_points: &'static [(u8, u8)], // where new hanzi pieces can appear
    pub goal_region: &'static [(u8, u8)],  // reaching here could score / advance
}

impl LevelDesc {
    pub fn tile(&self, x: u8, y: u8) -> &TileDesc {
        let idx = y as usize * self.width as usize + x as usize;
        &self.tiles[idx]
    }
}

/// Active piece on the board (represents a Hanzi / word). For now only one piece hops;
/// future: multiple simultaneous streams. Pieces now carry a small notion of direction
/// and short-lived momentum so tiles like Ice and JumpPad can influence motion.
#[allow(dead_code)]
struct Piece {
    hanzi: &'static str,
    pinyin: &'static str,
    x: u8,
    y: u8,
    target_x: u8,
    target_y: u8,
    hop_start_ms: f64,
    hop_duration_ms: f64,
    arrived: bool,
    /// Normalized movement direction of the last hop (-1/0/1 per axis)
    dir_dx: i8,
    dir_dy: i8,
    /// Short-lived momentum (in tiles) that causes automatic continued movement
    /// (used by Ice tiles). 0 means no momentum.
    momentum: u8,
}

#[allow(dead_code)]
impl Piece {
    fn begin_hop(&mut self, to_x: u8, to_y: u8, now: f64, duration_ms: f64) {
        // Record normalized direction so tile effects (ice / conveyors) can
        // continue movement in the same axis.
        let dx_i = to_x as i8 - self.x as i8;
        let dy_i = to_y as i8 - self.y as i8;
        self.dir_dx = dx_i.signum();
        self.dir_dy = dy_i.signum();
        // Default small momentum so ice can pick it up. Specific tiles may
        // override momentum later (e.g. JumpPad will zero it).
        self.momentum = 1;

        self.target_x = to_x;
        self.target_y = to_y;
        self.hop_start_ms = now;
        self.hop_duration_ms = duration_ms;
        self.arrived = false;
    }
}

#[allow(dead_code)]
impl Piece {
    fn new(
        hanzi: &'static str,
        pinyin: &'static str,
        x: u8,
        y: u8,
        now: f64,
        hop_dur: f64,
    ) -> Self {
        Self {
            hanzi,
            pinyin,
            x,
            y,
            target_x: x,
            target_y: y,
            hop_start_ms: now,
            hop_duration_ms: hop_dur,
            arrived: true,
            dir_dx: 0,
            dir_dy: 0,
            momentum: 0,
        }
    }
}

// Transient claw slash animation effect
struct SlashEffect {
    x: u8,
    y: u8,
    start_ms: f64,
}

/// Runtime board state.
struct BoardState {
    canvas: HtmlCanvasElement,
    ctx: CanvasRenderingContext2d,
    level: &'static LevelDesc,
    beat: BeatClock,
    grid: Vec<Option<(&'static str, &'static str)>>,
    cat_x: u8,
    cat_y: u8,
    // Hop animation (cat reuse) - when true, interpolate between from and target
    cat_from_x: u8,
    cat_from_y: u8,
    cat_target_x: u8,
    cat_target_y: u8,
    cat_hop_start_ms: f64,
    cat_hop_duration_ms: f64,
    cat_hopping: bool,
    level_index: usize,
    // --- Dynamic state for modifiers ---
    score: i64,
    score_multiplier: f64,
    score_mult_end_beat: i64,
    hop_time_factor: f64, // Multiplier on hop duration ( <1 faster, >1 slower )
    hop_time_end_beat: i64,
    // --- Lives / End State ---
    lives: i32,
    game_over: bool,
    // --- Typing ---
    typing: String, // Current pinyin buffer user is entering
    // --- Visual transient effects ---
    slash_effects: Vec<SlashEffect>,
    // Hovered tile (for future selection / interaction); None if outside canvas
    hover_tile: Option<(u8, u8)>,
}

// --- Static Prototype Level --------------------------------------------------
// Board definitions are now in separate files:
mod board_level1;
mod board_level2;
mod board_level3;
mod board_level4;
mod board_level5;
mod board_level6;
mod board_level7;
// child level modules live under src/board/*.rs

// Export per-level hanzi arrays where present for external code
pub use board_level2::LEVEL2_HANZI;
pub use board_level3::LEVEL3_HANZI;
pub use board_level4::LEVEL4_HANZI;
pub use board_level5::LEVEL5_HANZI;
pub use board_level6::LEVEL6_HANZI;
pub use board_level7::LEVEL7_HANZI;

// Runtime-built static levels array. Some level modules provide `levelN()` getters
// (used where tiles are runtime-built), others keep `LEVELN` statics; we unify
// access via this `levels()` function returning &'static [&'static LevelDesc].
fn levels() -> &'static [&'static LevelDesc] {
    use std::sync::OnceLock;
    static LEVELS_STATIC: OnceLock<&'static [&'static LevelDesc]> = OnceLock::new();
    LEVELS_STATIC.get_or_init(|| {
        let l1 = board_level1::level1();
        let l2 = board_level2::level2();
        let l3: &'static LevelDesc = &board_level3::LEVEL3;
        let l4 = board_level4::level4();
        let l5 = board_level5::level5();
        let l6 = board_level6::level6();
        let l7 = board_level7::level7();
        Box::leak(vec![l1, l2, l3, l4, l5, l6, l7].into_boxed_slice())
    })
}

pub static LEVEL_SCORE_THRESHOLDS: [i64; 7] = [0, 2500, 6000, 12000, 20000, 32000, 50000];

#[wasm_bindgen]
pub fn start_board_mode() -> Result<(), JsValue> {
    let win = window().ok_or_else(|| JsValue::from_str("no window"))?;
    let doc = win
        .document()
        .ok_or_else(|| JsValue::from_str("no document"))?;

    // Create / reuse canvas with id board-canvas (separate from falling mode for now)
    let canvas: HtmlCanvasElement = if let Some(el) = doc.get_element_by_id("hc-board-canvas") {
        el.dyn_into()?
    } else {
        let c: HtmlCanvasElement = doc.create_element("canvas")?.dyn_into()?;
        c.set_id("hc-board-canvas");
        c.set_width(640);
        c.set_height(640);
        // Center the board using CSS
        // Shift board upward so it does not overlap the cat at the bottom center
        c.set_attribute("style", "position:fixed; left:50%; top:38%; transform:translate(-50%,-50%); box-shadow:0 0 32px 0 rgba(0,0,0,0.18); border-radius:18px; border:2px solid #222; background:#181818; z-index:20;").ok();
        doc.body().unwrap().append_child(&c)?;
        c
    };
    let ctx: CanvasRenderingContext2d = canvas.get_context("2d")?.unwrap().dyn_into()?;
    ctx.set_font("40px 'Noto Serif SC', 'SimSun', serif");
    ctx.set_text_align("center");

    let now = win.performance().unwrap().now();
    let mut board = BoardState {
        canvas: canvas.clone(),
        ctx: ctx.clone(),
        level: levels()[0],
        beat: BeatClock::new(levels()[0].bpm, now),
        grid: {
            let lvl = levels()[0];
            let mut g: Vec<Option<(&'static str, &'static str)>> =
                Vec::with_capacity(lvl.width as usize * lvl.height as usize);
            for yy in 0..lvl.height {
                for xx in 0..lvl.width {
                    let tile = lvl.tile(xx, yy);
                    if matches!(tile.obstacle, Some(ObstacleKind::Block)) {
                        g.push(None);
                    } else {
                        let (hanzi, pinyin) = match lvl.name {
                            "Conveyor Crossing" => {
                                let hidx = rand_index(LEVEL2_HANZI.len());
                                LEVEL2_HANZI[hidx]
                            }
                            "Zigzag Express" => {
                                let hidx = rand_index(LEVEL4_HANZI.len());
                                LEVEL4_HANZI[hidx]
                            }
                            "Maze Challenge" => {
                                let hidx = rand_index(LEVEL3_HANZI.len());
                                LEVEL3_HANZI[hidx]
                            }
                            "Spiral Dream" => {
                                let hidx = rand_index(LEVEL5_HANZI.len());
                                LEVEL5_HANZI[hidx]
                            }
                            "Crystal Isle" => {
                                let hidx = rand_index(LEVEL6_HANZI.len());
                                LEVEL6_HANZI[hidx]
                            }
                            "Neon Bastion" => {
                                let hidx = rand_index(LEVEL7_HANZI.len());
                                LEVEL7_HANZI[hidx]
                            }
                            _ => ("你", "ni3"),
                        };
                        g.push(Some((hanzi, pinyin)));
                    }
                }
            }
            g
        },
        cat_x: {
            let lvl = levels()[0];
            let mut cx = lvl.width / 2;
            let mut _cy = lvl.height / 2;
            if matches!(lvl.tile(cx, _cy).obstacle, Some(ObstacleKind::Block)) {
                'search_free: for yy in 0..lvl.height {
                    for xx in 0..lvl.width {
                        if !matches!(lvl.tile(xx, yy).obstacle, Some(ObstacleKind::Block)) {
                            cx = xx;
                            _cy = yy;
                            break 'search_free;
                        }
                    }
                }
            }
            cx
        },
        cat_y: {
            let lvl = levels()[0];
            let mut _cx = lvl.width / 2;
            let mut cy = lvl.height / 2;
            if matches!(lvl.tile(_cx, cy).obstacle, Some(ObstacleKind::Block)) {
                'search_free2: for yy in 0..lvl.height {
                    for xx in 0..lvl.width {
                        if !matches!(lvl.tile(xx, yy).obstacle, Some(ObstacleKind::Block)) {
                            _cx = xx;
                            cy = yy;
                            break 'search_free2;
                        }
                    }
                }
            }
            cy
        },
        cat_from_x: 0,
        cat_from_y: 0,
        cat_target_x: 0,
        cat_target_y: 0,
        cat_hop_start_ms: now,
        cat_hop_duration_ms: 220.0,
        cat_hopping: false,
        level_index: 0,
        score: 0,
        score_multiplier: 1.0,
        score_mult_end_beat: -1,
        hop_time_factor: 1.0,
        hop_time_end_beat: -1,
        // Lives / end state initialization
        lives: 3,
        game_over: false,
        typing: String::new(),
        slash_effects: Vec::new(),
        hover_tile: None,
    };

    // Initialize cat hop fields to current cat position
    board.cat_from_x = board.cat_x;
    board.cat_from_y = board.cat_y;
    board.cat_target_x = board.cat_x;
    board.cat_target_y = board.cat_y;
    board.cat_hop_start_ms = now;
    board.cat_hop_duration_ms = 220.0;
    board.cat_hopping = false;

    // Ensure the player's current tile is empty and that adjacent tiles around the
    // player are populated with distinct hanzi for the first board. The remaining
    // cells are filled with an alternating two-character pattern.
    {
        let lvl = board.level;
        let w = lvl.width as usize;
        let h = lvl.height as usize;
        let cx = board.cat_x as i32;
        let cy = board.cat_y as i32;

        // Collect 8-connected neighbor indices, skip blocked tiles
        let mut neighbors: Vec<usize> = Vec::new();
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let nx = cx + dx;
                let ny = cy + dy;
                if nx < 0 || ny < 0 {
                    continue;
                }
                let nxu = nx as u8;
                let nyu = ny as u8;
                if nxu >= lvl.width || nyu >= lvl.height {
                    continue;
                }
                if matches!(lvl.tile(nxu, nyu).obstacle, Some(ObstacleKind::Block)) {
                    continue;
                }
                neighbors.push(ny as usize * w + nx as usize);
            }
        }

        // Clear player's tile
        if cx >= 0 && cy >= 0 && (cx as u8) < lvl.width && (cy as u8) < lvl.height {
            let cat_idx = cy as usize * w + cx as usize;
            if cat_idx < board.grid.len() {
                board.grid[cat_idx] = None;
            }
        }

        if board.level_index == 0 && !neighbors.is_empty() {
            let pool = crate::SINGLE_HANZI;
            let pool_len = pool.len();
            if pool_len > 0 {
                // Choose a contiguous run from the pool (random start) and take
                // enough unique entries for neighbors plus two for the alternating pattern.
                let mut selected: Vec<(&'static str, &'static str)> = Vec::new();
                let mut start = rand_index(pool_len);
                while selected.len() < neighbors.len() + 2 && selected.len() < pool_len {
                    let cand = pool[start % pool_len];
                    if !selected.iter().any(|(h, _)| *h == cand.0) {
                        selected.push(cand);
                    }
                    start = (start + 1) % pool_len;
                }

                // Assign unique characters to the neighbor tiles.
                for (i, &idx) in neighbors.iter().enumerate() {
                    if i < selected.len() {
                        board.grid[idx] = Some(selected[i]);
                    } else {
                        let (h, p) = pick_random_hanzi(lvl);
                        board.grid[idx] = Some((h, p));
                    }
                }

                // Pick two characters for the alternating fill pattern.
                let (pat0, pat1) = if selected.len() >= neighbors.len() + 2 {
                    (selected[neighbors.len()], selected[neighbors.len() + 1])
                } else {
                    (crate::SINGLE_HANZI[0], crate::SINGLE_HANZI[1 % pool_len])
                };

                // Fill remaining empty, non-block tiles with an (x+y) parity pattern.
                for y in 0..h {
                    for x in 0..w {
                        let idx = y * w + x;
                        // Do not fill the player's tile (cat) so it remains empty.
                        if x == board.cat_x as usize && y == board.cat_y as usize {
                            continue;
                        }
                        if board.grid[idx].is_none()
                            && !matches!(
                                lvl.tile(x as u8, y as u8).obstacle,
                                Some(ObstacleKind::Block)
                            )
                        {
                            let parity = (x + y) % 2;
                            board.grid[idx] = Some(if parity == 0 { pat0 } else { pat1 });
                        }
                    }
                }
            }
        }
    }

    BOARD_STATE.with(|b| b.replace(Some(board)));

    // Ensure typing overlay exists
    if doc.get_element_by_id("hc-typing").is_none() {
        if let Some(body) = doc.body() {
            let div = doc.create_element("div")?;
            div.set_id("hc-typing");
            div.set_text_content(Some(""));
            // Basic styling (absolute overlay centered above board) can be added via CSS later
            div.set_attribute("style", "position:fixed; bottom:220px; left:50%; transform:translateX(-50%); font-family:'Fira Code', monospace; font-size:20px; padding:4px 10px; background:rgba(0,0,0,0.35); border:1px solid #333; border-radius:6px; color:#ffd166; z-index:30;").ok();
            body.append_child(&div)?;
        }
    }
    // Ensure score overlay exists (top-left)
    if doc.get_element_by_id("hc-score").is_none() {
        if let Some(body) = doc.body() {
            let div = doc.create_element("div")?;
            div.set_id("hc-score");
            div.set_text_content(Some("Score: 0"));
            div.set_attribute("style", "position:fixed; top:10px; left:12px; font-family:'Fira Code', monospace; font-size:15px; padding:4px 8px; background:rgba(0,0,0,0.42); border:1px solid #333; border-radius:6px; color:#ffd166; z-index:45; letter-spacing:0.5px;").ok();
            body.append_child(&div)?;
        }
    }
    // Ensure lives overlay exists (top-left, next to score)
    if doc.get_element_by_id("hc-lives").is_none() {
        if let Some(body) = doc.body() {
            let div = doc.create_element("div")?;
            div.set_id("hc-lives");
            // Render hearts (Minecraft-style) - start with 3 filled hearts
            div.set_inner_html("<span style='color:#ff4d4d;font-size:16px;margin-right:6px;'>♥</span><span style='color:#ff4d4d;font-size:16px;margin-right:6px;'>♥</span><span style='color:#ff4d4d;font-size:16px;'>♥</span>");
            div.set_attribute("style", "position:fixed; top:10px; left:170px; font-family:'Fira Code', monospace; font-size:15px; padding:4px 8px; background:rgba(0,0,0,0.42); border:1px solid #333; border-radius:6px; z-index:44; letter-spacing:0.5px;").ok();
            body.append_child(&div)?;
        }
    }

    // Keyboard listener for pinyin typing
    {
        let closure = Closure::wrap(Box::new(move |evt: web_sys::KeyboardEvent| {
            BOARD_STATE.with(|state_cell| {
                if let Some(state) = state_cell.borrow_mut().as_mut() {
                    let key = evt.key();
                    if key == "Escape" {
                        state.typing.clear();
                    } else if key == "Backspace" {
                        state.typing.pop();
                    } else if key == "Enter" {
                        if !state.typing.is_empty() {
                            let typed = state.typing.clone();
                            // Look for matching adjacent tile (up, right, down, left)
                            let dirs: [(i8, i8); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];
                            let mut found: Option<((u8, u8), usize)> = None;
                            for (dx, dy) in dirs.iter() {
                                let nx_i = state.cat_x as i8 + *dx;
                                let ny_i = state.cat_y as i8 + *dy;
                                if nx_i < 0 || ny_i < 0 {
                                    continue;
                                }
                                let nx = nx_i as u8;
                                let ny = ny_i as u8;
                                if nx >= state.level.width || ny >= state.level.height {
                                    continue;
                                }
                                // skip blocked tiles
                                if matches!(
                                    state.level.tile(nx, ny).obstacle,
                                    Some(ObstacleKind::Block)
                                ) {
                                    continue;
                                }
                                let idx = ny as usize * state.level.width as usize + nx as usize;
                                if let Some((_, pinyin)) = state.grid[idx] {
                                    if pinyin == typed.as_str() {
                                        found = Some(((nx, ny), idx));
                                        break;
                                    }
                                }
                            }
                            if let Some(((mx, my), gidx)) = found {
                                // Queue a hop animation (reuse canonical cat) instead of
                                // instant teleport. We'll still consume the tile and
                                // award score immediately; the visual hop will play out.
                                let now_ts = window()
                                    .and_then(|w| w.performance())
                                    .map(|p| p.now())
                                    .unwrap_or(0.0);

                                state.cat_from_x = state.cat_x;
                                state.cat_from_y = state.cat_y;
                                state.cat_target_x = mx;
                                state.cat_target_y = my;
                                state.cat_hop_start_ms = now_ts;
                                state.cat_hop_duration_ms = 220.0 * state.hop_time_factor;
                                state.cat_hopping = true;

                                // Consume tile and award score immediately (visual slash plays)
                                state.grid[gidx] = None;
                                let per = (180.0 * state.score_multiplier) as i64;
                                state.score += per;
                                state.slash_effects.push(SlashEffect {
                                    x: mx,
                                    y: my,
                                    start_ms: now_ts,
                                });
                            }
                            state.typing.clear();
                        }
                    } else if key.len() == 1 {
                        let c = key.chars().next().unwrap();
                        if c.is_ascii_alphabetic() {
                            state.typing.push(c.to_ascii_lowercase());
                        } else if c.is_ascii_digit() && matches!(c, '1' | '2' | '3' | '4' | '5') {
                            if state
                                .typing
                                .chars()
                                .last()
                                .map(|lc| lc.is_ascii_alphabetic())
                                .unwrap_or(false)
                            {
                                state.typing.push(c);
                            }
                        }
                    }
                    // Update DOM element
                    if let Some(doc) = window().and_then(|w| w.document()) {
                        if let Some(el) = doc.get_element_by_id("hc-typing") {
                            el.set_text_content(Some(&state.typing));
                        }
                    }
                }
            });
        }) as Box<dyn FnMut(_)>);
        doc.add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    // Mouse move listener for hover tile tracking (visual placeholder only)
    {
        let canvas_move = canvas.clone();
        let closure = Closure::wrap(Box::new(move |evt: web_sys::MouseEvent| {
            // Use offset coordinates relative to the event target (canvas) to avoid
            // depending on js_sys / DomRect. offset_x/offset_y are available on
            // MouseEvent and are simpler for canvas-local coordinates.
            let x = evt.offset_x() as f64;
            let y = evt.offset_y() as f64;
            BOARD_STATE.with(|cell| {
                if let Some(st) = cell.borrow_mut().as_mut() {
                    let cw = canvas_move.width() as f64 / st.level.width as f64;
                    let ch = canvas_move.height() as f64 / st.level.height as f64;
                    if x >= 0.0
                        && y >= 0.0
                        && x < canvas_move.width() as f64
                        && y < canvas_move.height() as f64
                    {
                        let tx = (x / cw).floor() as u8;
                        let ty = (y / ch).floor() as u8;
                        st.hover_tile = Some((tx, ty));
                    } else {
                        st.hover_tile = None;
                    }
                }
            });
        }) as Box<dyn FnMut(_)>);
        canvas.add_event_listener_with_callback("mousemove", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }
    // Mouse leave clears hover
    {
        let canvas_leave = canvas.clone();
        let closure = Closure::wrap(Box::new(move |_evt: web_sys::MouseEvent| {
            BOARD_STATE.with(|cell| {
                if let Some(st) = cell.borrow_mut().as_mut() {
                    st.hover_tile = None;
                }
            });
        }) as Box<dyn FnMut(_)>);
        canvas_leave
            .add_event_listener_with_callback("mouseleave", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    start_board_loop();
    Ok(())
}

// RefCell::new isn't const on this toolchain; allow Clippy lint until a const initializer is feasible.
thread_local! {
    static BOARD_STATE: std::cell::RefCell<Option<BoardState>> = std::cell::RefCell::new(None);
}

type FrameCallback = std::rc::Rc<std::cell::RefCell<Option<Closure<dyn FnMut(f64)>>>>;

fn start_board_loop() {
    use wasm_bindgen::JsCast;
    let f: FrameCallback = std::rc::Rc::new(std::cell::RefCell::new(None));
    let g = f.clone();
    *g.borrow_mut() = Some(Closure::wrap(Box::new(move |ts: f64| {
        BOARD_STATE.with(|state_cell| {
            if let Some(state) = state_cell.borrow_mut().as_mut() {
                board_tick(state, ts);
            }
        });
        if let Some(w) = window() {
            let _ =
                w.request_animation_frame(f.borrow().as_ref().unwrap().as_ref().unchecked_ref());
        }
    }) as Box<dyn FnMut(f64)>));
    if let Some(w) = window() {
        let _ = w.request_animation_frame(g.borrow().as_ref().unwrap().as_ref().unchecked_ref());
    }
}

// --- Tick & Rendering (prototype) -------------------------------------------

fn board_tick(state: &mut BoardState, now: f64) {
    // Beat detection (whole beats only for now)
    let cur_beat = state.beat.current_beat(now);
    let whole = cur_beat.floor() as i64;
    if whole > state.beat.last_beat_idx {
        for b in state.beat.last_beat_idx + 1..=whole {
            on_new_beat(state, b, now);
        }
        state.beat.last_beat_idx = whole;
    }
    // Expire temporary effects
    expire_effects(state, whole);
    update_pieces(state, now, whole);
    check_level_progression(state, now, whole);
    // Expire slash effects (>300ms)
    state.slash_effects.retain(|e| now - e.start_ms < 300.0);
    render_board(state, now);
    // Keep DOM overlays (typing + score + lives) updated each frame
    if let Some(win) = window() {
        if let Some(doc) = win.document() {
            if let Some(el) = doc.get_element_by_id("hc-typing") {
                el.set_text_content(Some(&state.typing));
            }
            if let Some(score_el) = doc.get_element_by_id("hc-score") {
                score_el.set_text_content(Some(&format!("Score: {}", state.score)));
            }
            if let Some(lives_el) = doc.get_element_by_id("hc-lives") {
                // Build hearts HTML (3 hearts max)
                let max_hearts: i32 = 3;
                let mut html = String::new();
                let filled = (state.lives.max(0).min(max_hearts)) as usize;
                for _ in 0..filled {
                    html.push_str(
                        "<span style='color:#ff4d4d;font-size:16px;margin-right:6px;'>♥</span>",
                    );
                }
                for _ in filled..(max_hearts as usize) {
                    html.push_str(
                        "<span style='color:#6b6b6b;font-size:16px;margin-right:6px;'>♡</span>",
                    );
                }
                lives_el.set_inner_html(&html);
            }
        }
    }
}

fn on_new_beat(state: &mut BoardState, _beat_idx: i64, _now: f64) {
    // Grid-based refill: on each beat, refill any empty (None) cells
    // with a randomly chosen hanzi/pinyin appropriate for the current level.
    // Skip tiles that are blocked and avoid overwriting the player's tile or
    // the cat's destination tile while a hop animation is in progress.
    if state.game_over {
        return;
    }
    let lvl = state.level;
    for y in 0..lvl.height {
        for x in 0..lvl.width {
            // skip blocked tiles
            if matches!(lvl.tile(x, y).obstacle, Some(ObstacleKind::Block)) {
                continue;
            }

            // Do not refill the player's current tile; it must remain empty.
            if x == state.cat_x && y == state.cat_y {
                continue;
            }

            // While the cat is mid-hop, avoid refilling the target tile so the
            // arriving tile remains empty until arrival handling runs.
            if state.cat_hopping && x == state.cat_target_x && y == state.cat_target_y {
                continue;
            }

            let idx = y as usize * lvl.width as usize + x as usize;
            if state.grid[idx].is_none() {
                let (h, p) = pick_random_hanzi(lvl);
                state.grid[idx] = Some((h, p));
            }
        }
    }
    // Future: consider refilling only a subset per beat to tune pacing.
}

fn update_pieces(state: &mut BoardState, now: f64, _whole_beat: i64) {
    // Advance cat hop animation (if any). We keep this function to preserve the
    // previous call site but now use it to finish the hop and update canonical
    // cat coordinates when the animation completes.
    if state.cat_hopping {
        let elapsed = now - state.cat_hop_start_ms;
        let dur = if state.cat_hop_duration_ms <= 0.0 {
            1.0
        } else {
            state.cat_hop_duration_ms
        };
        let t = (elapsed / dur).clamp(0.0, 1.0);
        if t >= 1.0 {
            // Finish hop animation and update canonical coordinates
            state.cat_hopping = false;
            state.cat_x = state.cat_target_x;
            state.cat_y = state.cat_target_y;

            // Consume the landing tile so it remains empty after arrival.
            let lvl = state.level;
            let w = lvl.width as usize;
            let idx = state.cat_y as usize * w + state.cat_x as usize;
            if idx < state.grid.len() {
                state.grid[idx] = None;
            }

            // For the first level, refresh up-to-8 neighbor tiles with unique
            // hanzi drawn from SINGLE_HANZI and then parity-fill remaining empties.
            if state.level_index == 0 {
                let lvl = state.level;
                let w = lvl.width as usize;
                let h = lvl.height as usize;
                let cx = state.cat_x as i32;
                let cy = state.cat_y as i32;

                // Collect neighbor indices (8-connected), skipping blocked tiles.
                let mut neighbors: Vec<usize> = Vec::new();
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        let nx = cx + dx;
                        let ny = cy + dy;
                        if nx < 0 || ny < 0 {
                            continue;
                        }
                        let nxu = nx as u8;
                        let nyu = ny as u8;
                        if nxu >= lvl.width || nyu >= lvl.height {
                            continue;
                        }
                        if matches!(lvl.tile(nxu, nyu).obstacle, Some(ObstacleKind::Block)) {
                            continue;
                        }
                        neighbors.push(ny as usize * w + nx as usize);
                    }
                }

                if !neighbors.is_empty() {
                    let pool = crate::SINGLE_HANZI;
                    let pool_len = pool.len();
                    if pool_len > 0 {
                        let mut selected: Vec<(&'static str, &'static str)> = Vec::new();
                        let mut start = rand_index(pool_len);
                        while selected.len() < neighbors.len() + 2 && selected.len() < pool_len {
                            let cand = pool[start % pool_len];
                            if !selected.iter().any(|(h, _)| *h == cand.0) {
                                selected.push(cand);
                            }
                            start = (start + 1) % pool_len;
                        }

                        for (i, &nidx) in neighbors.iter().enumerate() {
                            if i < selected.len() {
                                state.grid[nidx] = Some(selected[i]);
                            } else {
                                let (h, p) = pick_random_hanzi(lvl);
                                state.grid[nidx] = Some((h, p));
                            }
                        }

                        let (pat0, pat1) = if selected.len() >= neighbors.len() + 2 {
                            (selected[neighbors.len()], selected[neighbors.len() + 1])
                        } else {
                            (crate::SINGLE_HANZI[0], crate::SINGLE_HANZI[1 % pool_len])
                        };

                        for y in 0..h {
                            for x in 0..w {
                                let idx = y * w + x;
                                // Do not fill the player's tile (cat) so it remains empty.
                                if x == state.cat_x as usize && y == state.cat_y as usize {
                                    continue;
                                }
                                if state.grid[idx].is_none()
                                    && !matches!(
                                        lvl.tile(x as u8, y as u8).obstacle,
                                        Some(ObstacleKind::Block)
                                    )
                                {
                                    let parity = (x + y) % 2;
                                    state.grid[idx] = Some(if parity == 0 { pat0 } else { pat1 });
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn render_board(state: &mut BoardState, now: f64) {
    // Render background with a subtle beat pulse.
    let beat_phase = {
        let cb = state.beat.current_beat(now);
        cb - cb.floor()
    };
    let pulse = ((beat_phase * std::f64::consts::TAU).sin() * 0.5 + 0.5) * 0.25;
    let cell_w = state.canvas.width() as f64 / state.level.width as f64;
    let cell_h = state.canvas.height() as f64 / state.level.height as f64;
    let bg = (15.0 + pulse * 40.0) as i32;
    let color = format!(
        "rgb({},{},{})",
        (bg + 18).clamp(0, 255),
        (bg + 14).clamp(0, 255),
        (bg + 12).clamp(0, 255)
    );
    state.ctx.set_fill_style_str(&color);
    state.ctx.fill_rect(
        0.0,
        0.0,
        state.canvas.width() as f64,
        state.canvas.height() as f64,
    );

    // Top accent band (spawn row visual)
    state.ctx.set_fill_style_str("rgba(255,220,120,0.08)");
    state
        .ctx
        .fill_rect(0.0, 0.0, state.canvas.width() as f64, cell_h);

    // Highlight goal region tiles
    state.ctx.set_fill_style_str("rgba(120,200,255,0.10)");
    for &(gx, gy) in state.level.goal_region.iter() {
        let px = gx as f64 * cell_w;
        let py = gy as f64 * cell_h;
        state.ctx.fill_rect(px, py, cell_w, cell_h);
    }

    // Grid lines
    state.ctx.set_stroke_style_str("#222");
    state.ctx.set_line_width(2.0);
    for x in 0..=state.level.width {
        let fx = x as f64 * cell_w;
        line(&state.ctx, fx, 0.0, fx, state.canvas.height() as f64);
    }
    for y in 0..=state.level.height {
        let fy = y as f64 * cell_h;
        line(&state.ctx, 0.0, fy, state.canvas.width() as f64, fy);
    }

    // Hover highlight
    if let Some((hx, hy)) = state.hover_tile {
        if hx < state.level.width && hy < state.level.height {
            let px = hx as f64 * cell_w;
            let py = hy as f64 * cell_h;
            state.ctx.set_stroke_style_str("rgba(255,240,150,0.55)");
            state.ctx.set_line_width(3.0);
            state
                .ctx
                .stroke_rect(px + 1.5, py + 1.5, cell_w - 3.0, cell_h - 3.0);
        }
    }

    // Obstacles (draw before cell content so they sit beneath Hanzi when appropriate)
    for y in 0..state.level.height {
        for x in 0..state.level.width {
            let t = state.level.tile(x, y);
            if let Some(obs) = &t.obstacle {
                draw_obstacle(&state.ctx, obs, x, y, cell_w, cell_h);
            }
        }
    }

    // Draw cell hanzi (centered). Use a consistent layered stroke+fill like the piece renderer.
    state.ctx.set_shadow_color("rgba(0,0,0,0.55)");
    state.ctx.set_shadow_blur(12.0);
    state.ctx.set_shadow_offset_x(0.0);
    state.ctx.set_shadow_offset_y(3.0);

    for y in 0..state.level.height {
        for x in 0..state.level.width {
            let idx = y as usize * state.level.width as usize + x as usize;
            if let Some((hanzi, _pinyin)) = state.grid[idx] {
                let cx = x as f64 * cell_w + cell_w / 2.0;
                let cy = y as f64 * cell_h + cell_h / 2.0 + 8.0; // small vertical offset
                state.ctx.set_line_width(6.0);
                state.ctx.set_stroke_style_str("rgba(0,0,0,0.85)");
                state.ctx.stroke_text(hanzi, cx, cy).ok();
                // crisp fill
                state.ctx.set_shadow_blur(0.0);
                state.ctx.set_fill_style_str("#ffffff");
                state.ctx.fill_text(hanzi, cx, cy).ok();
                state.ctx.set_line_width(2.0);
                state.ctx.set_stroke_style_str("rgba(255,210,120,0.55)");
                state.ctx.stroke_text(hanzi, cx, cy).ok();
                // restore shadow for next glyph
                state.ctx.set_shadow_blur(12.0);
            }
        }
    }

    // Clear shadows after drawing text
    state.ctx.set_shadow_blur(0.0);
    state.ctx.set_shadow_offset_x(0.0);
    state.ctx.set_shadow_offset_y(0.0);

    // Compute the cat center (as before) and position the canonical DOM SVG (#hc-cat)
    // over the canvas. We preserve the SVG's internal animation by moving the element
    // instead of rasterizing it to the canvas.
    let (cat_cx, cat_cy) = if state.cat_hopping {
        let elapsed = now - state.cat_hop_start_ms;
        let dur = if state.cat_hop_duration_ms <= 0.0 {
            1.0
        } else {
            state.cat_hop_duration_ms
        };
        let t = (elapsed / dur).clamp(0.0, 1.0);
        // ease-in-out-ish (simple quadratic ease)
        let ease_t = 1.0 - (1.0 - t).powf(2.0);
        let from_x = state.cat_from_x as f64;
        let from_y = state.cat_from_y as f64;
        let to_x = state.cat_target_x as f64;
        let to_y = state.cat_target_y as f64;
        let ix = from_x + (to_x - from_x) * ease_t;
        let iy = from_y + (to_y - from_y) * ease_t;
        // vertical arc for hop
        let hop_h = (t * std::f64::consts::PI).sin() * 0.20 * cell_h;
        (
            ix * cell_w + cell_w / 2.0,
            iy * cell_h + cell_h / 2.0 - hop_h,
        )
    } else {
        (
            state.cat_x as f64 * cell_w + cell_w / 2.0,
            state.cat_y as f64 * cell_h + cell_h / 2.0,
        )
    };

    // Position the DOM cat SVG (#hc-cat) over the canvas at the computed tile center.
    // The canvas is positioned using fixed left/top + transform:translate(-50%,-50%).
    // We'll place the cat with the same anchor and apply pixel offsets relative to
    // the canvas center to avoid requiring additional web-sys features.
    if let Some(win) = window() {
        if let Some(doc) = win.document() {
            if let Some(el) = doc.get_element_by_id("hc-cat") {
                let canvas_w = state.canvas.width() as f64;
                let canvas_h = state.canvas.height() as f64;
                // offset from canvas center in canvas pixels
                let offset_x = cat_cx - (canvas_w / 2.0);
                let offset_y = cat_cy - (canvas_h / 2.0);
                // Use the same left/top anchor used for the canvas (50% / 38%) so the
                // cat sits correctly above the canvas. We apply a translation that
                // adjusts from the anchor by the computed offsets.
                // Compute a square pixel size for the DOM cat so it fits within a
                // single grid cell with some padding. Use the smaller of cell_w
                // and cell_h to remain consistent across non-square boards.
                let cat_size = (cell_w.min(cell_h) * 0.75).round() as i32;
                let style = format!(
                    "position:fixed; left:50%; top:38%; transform:translate(calc(-50% + {ox}px), calc(-50% + {oy}px)); pointer-events:none; z-index:40; width:{w}px; height:{h}px;",
                    ox = offset_x,
                    oy = offset_y,
                    w = cat_size,
                    h = cat_size
                );
                el.set_attribute("style", &style).ok();
            }
        }
    }

    // Slash effects (tile-space, same visual as before)
    for eff in &state.slash_effects {
        let age = now - eff.start_ms;
        let alpha = 1.0 - (age / 300.0).clamp(0.0, 1.0);
        if alpha <= 0.0 {
            continue;
        }
        let px = eff.x as f64 * cell_w;
        let py = eff.y as f64 * cell_h;
        let inset = 6.0;
        let left = px + inset;
        let top = py + inset;
        let right = px + cell_w - inset;
        let bottom = py + cell_h - inset;
        state.ctx.set_line_width(4.0);
        state
            .ctx
            .set_stroke_style_str(&format!("rgba(255,80,80,{alpha})"));
        for i in 0..3 {
            let offset = i as f64 * 6.0;
            state.ctx.begin_path();
            state.ctx.move_to(left + offset, top);
            state.ctx.line_to(right + offset - 18.0, bottom);
            state.ctx.stroke();
        }
    }

    // GAME OVER overlay (unchanged)
    if state.game_over {
        state.ctx.set_fill_style_str("rgba(0,0,0,0.55)");
        state.ctx.fill_rect(
            0.0,
            0.0,
            state.canvas.width() as f64,
            state.canvas.height() as f64,
        );
        state.ctx.set_fill_style_str("#ffffff");
        state.ctx.set_font("72px 'Noto Serif SC', serif");
        state.ctx.set_text_align("center");
        state.ctx.set_line_width(6.0);
        state.ctx.set_stroke_style_str("#000000");
        let cx = state.canvas.width() as f64 / 2.0;
        let cy = state.canvas.height() as f64 / 2.0;
        state.ctx.stroke_text("GAME OVER", cx, cy).ok();
        state.ctx.fill_text("GAME OVER", cx, cy).ok();
        state.ctx.set_font("20px 'Fira Code', monospace");
        state
            .ctx
            .fill_text("Refresh to try again", cx, cy + 44.0)
            .ok();
    }
}

fn draw_obstacle(
    ctx: &CanvasRenderingContext2d,
    obs: &ObstacleKind,
    x: u8,
    y: u8,
    cw: f64,
    ch: f64,
) {
    let px = x as f64 * cw;
    let py = y as f64 * ch;
    match *obs {
        ObstacleKind::Block => {
            // Solid block with subtle inner X pattern
            ctx.set_fill_style_str("#552222");
            ctx.fill_rect(px + 2.0, py + 2.0, cw - 4.0, ch - 4.0);
            ctx.set_stroke_style_str("rgba(255,200,200,0.15)");
            ctx.set_line_width(3.0);
            ctx.begin_path();
            ctx.move_to(px + 4.0, py + 4.0);
            ctx.line_to(px + cw - 4.0, py + ch - 4.0);
            ctx.move_to(px + cw - 4.0, py + 4.0);
            ctx.line_to(px + 4.0, py + ch - 4.0);
            ctx.stroke();
        }
        ObstacleKind::Teleport { .. } => {
            // Portal: ring + inner glow square
            ctx.set_stroke_style_str("#4aa3ff");
            ctx.set_line_width(4.0);
            ctx.begin_path();
            let cx = px + cw / 2.0;
            let cy = py + ch / 2.0;
            let r = (cw.min(ch)) * 0.33;
            ctx.arc(cx, cy, r, 0.0, std::f64::consts::TAU).ok();
            ctx.stroke();
            ctx.set_fill_style_str("rgba(70,140,255,0.25)");
            let side = r * 1.1;
            ctx.fill_rect(cx - side / 2.0, cy - side / 2.0, side, side);
        }
        ObstacleKind::Conveyor { dx, dy } => {
            // Belt: darker base + directional chevrons
            ctx.set_fill_style_str("#334433");
            ctx.fill_rect(px + 2.0, py + 2.0, cw - 4.0, ch - 4.0);
            ctx.set_fill_style_str("#88cc88");
            // Draw 3 small chevrons along movement axis
            let chevrons = 3;
            for i in 0..chevrons {
                let t = (i as f64 + 0.5) / chevrons as f64;
                let (cx, cy) = (px + 2.0 + (cw - 4.0) * t, py + 2.0 + (ch - 4.0) * t);
                ctx.begin_path();
                let size = 6.0;
                match (dx, dy) {
                    (1, 0) => {
                        // right
                        ctx.move_to(cx - size, py + ch / 2.0 - size * 0.8);
                        ctx.line_to(cx, py + ch / 2.0);
                        ctx.line_to(cx - size, py + ch / 2.0 + size * 0.8);
                    }
                    (-1, 0) => {
                        // left
                        ctx.move_to(cx + size, py + ch / 2.0 - size * 0.8);
                        ctx.line_to(cx, py + ch / 2.0);
                        ctx.line_to(cx + size, py + ch / 2.0 + size * 0.8);
                    }
                    (0, 1) => {
                        // down
                        ctx.move_to(px + cw / 2.0 - size * 0.8, cy - size);
                        ctx.line_to(px + cw / 2.0, cy);
                        ctx.line_to(px + cw / 2.0 + size * 0.8, cy - size);
                    }
                    (0, -1) => {
                        // up
                        ctx.move_to(px + cw / 2.0 - size * 0.8, cy + size);
                        ctx.line_to(px + cw / 2.0, cy);
                        ctx.line_to(px + cw / 2.0 + size * 0.8, cy + size);
                    }
                    _ => {}
                }
                ctx.fill();
            }
        }
        ObstacleKind::TempoShift { .. } => {
            // Metronome tile: base + swinging arm representation
            ctx.set_fill_style_str("#444455");
            ctx.fill_rect(px + 2.0, py + 2.0, cw - 4.0, ch - 4.0);
            ctx.set_stroke_style_str("#b0b0ff");
            ctx.set_line_width(3.0);
            let base_w = cw * 0.28;
            let base_x = px + cw / 2.0 - base_w / 2.0;
            let base_y = py + ch * 0.68;
            ctx.begin_path();
            ctx.move_to(base_x, base_y);
            ctx.line_to(base_x + base_w, base_y);
            ctx.stroke();
            // Arm
            ctx.begin_path();
            ctx.move_to(px + cw / 2.0, base_y);
            ctx.line_to(px + cw / 2.0 + cw * 0.18, py + ch * 0.30);
            ctx.stroke();
        }
        ObstacleKind::Ice => {
            // Ice: slippery pale tile with a snowflake-like symbol
            ctx.set_fill_style_str("#224466");
            ctx.fill_rect(px + 2.0, py + 2.0, cw - 4.0, ch - 4.0);
            ctx.set_stroke_style_str("rgba(200,230,255,0.9)");
            ctx.set_line_width(2.0);
            // simple snowflake lines
            let cx = px + cw / 2.0;
            let cy = py + ch / 2.0;
            let r = (cw.min(ch)) * 0.18;
            ctx.begin_path();
            ctx.move_to(cx - r, cy);
            ctx.line_to(cx + r, cy);
            ctx.move_to(cx, cy - r);
            ctx.line_to(cx, cy + r);
            ctx.move_to(cx - r * 0.7, cy - r * 0.7);
            ctx.line_to(cx + r * 0.7, cy + r * 0.7);
            ctx.move_to(cx - r * 0.7, cy + r * 0.7);
            ctx.line_to(cx + r * 0.7, cy - r * 0.7);
            ctx.stroke();
        }
        ObstacleKind::JumpPad { dx, dy, strength } => {
            // JumpPad: bright pad with arrow showing direction and a strength number
            ctx.set_fill_style_str("#554488");
            ctx.fill_rect(px + 2.0, py + 2.0, cw - 4.0, ch - 4.0);
            ctx.set_fill_style_str("#ffdd88");
            // arrow center
            let cx = px + cw / 2.0;
            let cy = py + ch / 2.0;
            let size = (cw.min(ch)) * 0.18;
            ctx.begin_path();
            match (dx, dy) {
                (1, 0) => {
                    ctx.move_to(cx - size, cy - size);
                    ctx.line_to(cx + size, cy);
                    ctx.line_to(cx - size, cy + size);
                }
                (-1, 0) => {
                    ctx.move_to(cx + size, cy - size);
                    ctx.line_to(cx - size, cy);
                    ctx.line_to(cx + size, cy + size);
                }
                (0, 1) => {
                    ctx.move_to(cx - size, cy - size);
                    ctx.line_to(cx, cy + size);
                    ctx.line_to(cx + size, cy - size);
                }
                (0, -1) => {
                    ctx.move_to(cx - size, cy + size);
                    ctx.line_to(cx, cy - size);
                    ctx.line_to(cx + size, cy + size);
                }
                _ => {
                    // no direction: draw a burst
                    ctx.arc(cx, cy, size, 0.0, std::f64::consts::TAU).ok();
                }
            }
            ctx.fill();
            // strength number in corner
            ctx.set_fill_style_str("#ffffff");
            ctx.set_font("12px 'Fira Code', monospace");
            ctx.fill_text(&format!("{}", strength), px + cw - 14.0, py + ch - 8.0)
                .ok();
        }
        ObstacleKind::Transform => {
            // Transform tile: gradient-like base + double arrow
            ctx.set_fill_style_str("#333355");
            ctx.fill_rect(px + 2.0, py + 2.0, cw - 4.0, ch - 4.0);
            ctx.set_stroke_style_str("#aac");
            ctx.set_line_width(3.0);
            ctx.begin_path();
            let mid_y = py + ch / 2.0;
            ctx.move_to(px + cw * 0.25, mid_y - 6.0);
            ctx.line_to(px + cw * 0.75, mid_y - 6.0);
            ctx.move_to(px + cw * 0.75 - 8.0, mid_y - 12.0);
            ctx.line_to(px + cw * 0.75, mid_y - 6.0);
            ctx.line_to(px + cw * 0.75 - 8.0, mid_y);
            ctx.move_to(px + cw * 0.75, mid_y + 6.0);
            ctx.line_to(px + cw * 0.25, mid_y + 6.0);
            ctx.move_to(px + cw * 0.25 + 8.0, mid_y);
            ctx.line_to(px + cw * 0.25, mid_y + 6.0);
            ctx.line_to(px + cw * 0.25 + 8.0, mid_y + 12.0);
            ctx.stroke();
        }
    }
}

#[allow(dead_code)]
fn apply_tile_effects(piece: &mut Piece, state: &mut BoardState, current_beat: i64, _now: f64) {
    let tile = state.level.tile(piece.x, piece.y);
    // Obstacles with post-arrival effects
    if let Some(obs) = &tile.obstacle {
        match obs {
            ObstacleKind::Teleport { to: (tx, ty) } => {
                piece.x = *tx;
                piece.y = *ty; // instant relocate
            }
            ObstacleKind::Conveyor { dx, dy } => {
                let nx = piece.x as i8 + *dx;
                let ny = piece.y as i8 + *dy;
                if nx >= 0
                    && ny >= 0
                    && (nx as u8) < state.level.width
                    && (ny as u8) < state.level.height
                {
                    let nxu = nx as u8;
                    let nyu = ny as u8;
                    if !matches!(
                        state.level.tile(nxu, nyu).obstacle,
                        Some(ObstacleKind::Block)
                    ) {
                        // Queue immediate hop (small duration)
                        piece.begin_hop(nxu, nyu, _now, piece.hop_duration_ms * 0.8);
                    }
                }
            }
            ObstacleKind::TempoShift { mult, beats } => {
                state.hop_time_factor *= 1.0 / mult; // faster tempo => shorter hops
                state.hop_time_end_beat = current_beat + *beats as i64;
            }
            ObstacleKind::Ice => {
                // If the piece has a known direction, enable sliding momentum.
                if piece.dir_dx == 0 && piece.dir_dy == 0 {
                    // choose a greedy direction toward goal so the piece will slide
                    if let Some((nx, ny)) = choose_next_step(state.level, piece.x, piece.y) {
                        piece.dir_dx = (nx as i8 - piece.x as i8).signum();
                        piece.dir_dy = (ny as i8 - piece.y as i8).signum();
                    }
                }
                piece.momentum = 1;
            }
            ObstacleKind::JumpPad { dx, dy, strength } => {
                // Launch the piece `strength` tiles in the given direction,
                // or toward the first goal if dx/dy == 0.
                let mut ldx = *dx;
                let mut ldy = *dy;
                if ldx == 0 && ldy == 0 {
                    // pick direction toward nearest goal
                    if let Some(&(gx, gy)) = state.level.goal_region.first() {
                        ldx = (gx as i8 - piece.x as i8).signum();
                        ldy = (gy as i8 - piece.y as i8).signum();
                    }
                }
                let mut tx = piece.x as i8;
                let mut ty = piece.y as i8;
                for _ in 0..*strength {
                    let nx = tx + ldx;
                    let ny = ty + ldy;
                    if nx < 0
                        || ny < 0
                        || (nx as u8) >= state.level.width
                        || (ny as u8) >= state.level.height
                    {
                        break;
                    }
                    if matches!(
                        state.level.tile(nx as u8, ny as u8).obstacle,
                        Some(ObstacleKind::Block)
                    ) {
                        break;
                    }
                    tx = nx;
                    ty = ny;
                }
                // Queue a faster hop to the landing tile
                piece.begin_hop(tx as u8, ty as u8, _now, piece.hop_duration_ms * 0.6);
                piece.momentum = 0; // jump breaks sliding momentum
            }
            ObstacleKind::Block => { /* cannot stand here normally (shouldn't happen) */ }
            ObstacleKind::Transform => { /* handled via modifier if present */ }
        }
    }
    if let Some(modf) = &tile.modifier {
        match modf {
            ModifierKind::ScoreMult { factor, beats } => {
                state.score_multiplier *= *factor;
                state.score_mult_end_beat = current_beat + *beats as i64;
            }
            ModifierKind::SlowHop { factor, beats } => {
                state.hop_time_factor *= *factor;
                state.hop_time_end_beat = current_beat + *beats as i64;
            }
            ModifierKind::TransformMap { pairs } => {
                for (from, to) in *pairs {
                    if piece.hanzi == *from {
                        piece.hanzi = to; // keep pinyin placeholder; future: pinyin map
                        break;
                    }
                }
            }
        }
    }
}

fn expire_effects(state: &mut BoardState, current_beat: i64) {
    if state.score_mult_end_beat >= 0 && current_beat >= state.score_mult_end_beat {
        state.score_multiplier = 1.0;
        state.score_mult_end_beat = -1;
    }
    if state.hop_time_end_beat >= 0 && current_beat >= state.hop_time_end_beat {
        state.hop_time_factor = 1.0;
        state.hop_time_end_beat = -1;
    }
}

fn check_level_progression(state: &mut BoardState, now: f64, current_beat: i64) {
    // If next level exists and score threshold reached, advance.
    if state.level_index + 1 < levels().len() {
        let next_idx = state.level_index + 1;
        if state.score >= LEVEL_SCORE_THRESHOLDS[next_idx] {
            set_level(state, next_idx, now, current_beat);
        }
    }
}

fn set_level(state: &mut BoardState, new_index: usize, now: f64, _current_beat: i64) {
    // Switch to the new level descriptor and reinitialize dynamic per-level state.
    state.level_index = new_index;
    state.level = levels()[new_index];

    // Rebuild the grid for the new level. Block tiles remain None; other tiles
    // are filled with a random hanzi/pinyin appropriate to the level.
    let lvl = state.level;
    state.grid.clear();
    state.grid.reserve(lvl.width as usize * lvl.height as usize);
    for yy in 0..lvl.height {
        for xx in 0..lvl.width {
            let tile = lvl.tile(xx, yy);
            if matches!(tile.obstacle, Some(ObstacleKind::Block)) {
                state.grid.push(None);
            } else {
                let (h, p) = pick_random_hanzi(lvl);
                state.grid.push(Some((h, p)));
            }
        }
    }

    // Position the cat near the center or on the first non-block tile found.
    let mut cx = lvl.width / 2;
    let mut cy = lvl.height / 2;
    if matches!(lvl.tile(cx, cy).obstacle, Some(ObstacleKind::Block)) {
        'search_free: for yy in 0..lvl.height {
            for xx in 0..lvl.width {
                if !matches!(lvl.tile(xx, yy).obstacle, Some(ObstacleKind::Block)) {
                    cx = xx;
                    cy = yy;
                    break 'search_free;
                }
            }
        }
    }
    state.cat_x = cx;
    state.cat_y = cy;
    // Reset hop animation so cat remains consistent with new level
    state.cat_from_x = state.cat_x;
    state.cat_from_y = state.cat_y;
    state.cat_target_x = state.cat_x;
    state.cat_target_y = state.cat_y;
    state.cat_hop_start_ms = now;
    state.cat_hop_duration_ms = 220.0;
    state.cat_hopping = false;

    // Ensure player's tile is empty and neighbors are uniquely populated for level 0.
    {
        let lvl = state.level;
        let w = lvl.width as usize;
        let h = lvl.height as usize;
        let cx = state.cat_x as i32;
        let cy = state.cat_y as i32;

        // Collect neighbor indices (8-connected)
        let mut neighbors: Vec<usize> = Vec::new();
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let nx = cx + dx;
                let ny = cy + dy;
                if nx < 0 || ny < 0 {
                    continue;
                }
                let nxu = nx as u8;
                let nyu = ny as u8;
                if nxu >= lvl.width || nyu >= lvl.height {
                    continue;
                }
                if matches!(lvl.tile(nxu, nyu).obstacle, Some(ObstacleKind::Block)) {
                    continue;
                }
                neighbors.push(ny as usize * w + nx as usize);
            }
        }

        // Clear player's tile
        if cx >= 0 && cy >= 0 && (cx as u8) < lvl.width && (cy as u8) < lvl.height {
            let cat_idx = cy as usize * w + cx as usize;
            if cat_idx < state.grid.len() {
                state.grid[cat_idx] = None;
            }
        }

        if new_index == 0 && !neighbors.is_empty() {
            let pool = crate::SINGLE_HANZI;
            let pool_len = pool.len();
            if pool_len > 0 {
                let mut selected: Vec<(&'static str, &'static str)> = Vec::new();
                let mut start = rand_index(pool_len);
                while selected.len() < neighbors.len() + 2 && selected.len() < pool_len {
                    let cand = pool[start % pool_len];
                    if !selected.iter().any(|(h, _)| *h == cand.0) {
                        selected.push(cand);
                    }
                    start = (start + 1) % pool_len;
                }

                for (i, &idx) in neighbors.iter().enumerate() {
                    if i < selected.len() {
                        state.grid[idx] = Some(selected[i]);
                    } else {
                        let (h, p) = pick_random_hanzi(lvl);
                        state.grid[idx] = Some((h, p));
                    }
                }

                let (pat0, pat1) = if selected.len() >= neighbors.len() + 2 {
                    (selected[neighbors.len()], selected[neighbors.len() + 1])
                } else {
                    (crate::SINGLE_HANZI[0], crate::SINGLE_HANZI[1 % pool_len])
                };

                for y in 0..h {
                    for x in 0..w {
                        let idx = y * w + x;
                        // Do not fill the player's tile (cat) so it remains empty.
                        if x == state.cat_x as usize && y == state.cat_y as usize {
                            continue;
                        }
                        if state.grid[idx].is_none()
                            && !matches!(
                                lvl.tile(x as u8, y as u8).obstacle,
                                Some(ObstacleKind::Block)
                            )
                        {
                            let parity = (x + y) % 2;
                            state.grid[idx] = Some(if parity == 0 { pat0 } else { pat1 });
                        }
                    }
                }
            }
        }
    }

    // Reset beat clock to the new level's BPM
    state.beat = BeatClock {
        bpm: state.level.bpm,
        start_ms: now,
        last_beat_idx: -1,
    };

    // Reset temporary modifiers
    state.hop_time_factor = 1.0;
    state.hop_time_end_beat = -1;
    state.score_multiplier = 1.0;
    state.score_mult_end_beat = -1;
}

fn rand_index(len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    let now = window()
        .and_then(|w| w.performance())
        .map(|p| p.now())
        .unwrap_or(0.0);
    // Simple linear transform and modulus for prototype randomness (not crypto secure)
    (now as u64 as usize)
        .wrapping_mul(1664525)
        .wrapping_add(1013904223)
        % len
}

/// Pick a random hanzi / pinyin tuple appropriate for the given level.
/// Centralizes the per-level selection logic used in multiple places.
fn pick_random_hanzi(level: &LevelDesc) -> (&'static str, &'static str) {
    match level.name {
        "Conveyor Crossing" => {
            let hidx = rand_index(LEVEL2_HANZI.len());
            LEVEL2_HANZI[hidx]
        }
        "Zigzag Express" => {
            let hidx = rand_index(LEVEL4_HANZI.len());
            LEVEL4_HANZI[hidx]
        }
        "Maze Challenge" => {
            let hidx = rand_index(LEVEL3_HANZI.len());
            LEVEL3_HANZI[hidx]
        }
        "Spiral Dream" => {
            let hidx = rand_index(LEVEL5_HANZI.len());
            LEVEL5_HANZI[hidx]
        }
        "Crystal Isle" => {
            let hidx = rand_index(LEVEL6_HANZI.len());
            LEVEL6_HANZI[hidx]
        }
        "Neon Bastion" => {
            let hidx = rand_index(LEVEL7_HANZI.len());
            LEVEL7_HANZI[hidx]
        }
        _ => {
            let pool = crate::SINGLE_HANZI;
            let len = pool.len();
            if len == 0 {
                ("你", "ni3")
            } else {
                let idx = rand_index(len);
                pool[idx]
            }
        }
    }
}

/// Decide next step for a piece taking into account momentum (ice), jump pads, and
/// simple heuristics. Returns the next tile to hop to if any.
#[allow(dead_code)]
fn choose_next_for_piece(level: &LevelDesc, p: &Piece) -> Option<(u8, u8)> {
    let x = p.x;
    let y = p.y;

    // If we have momentum, attempt to continue in that direction
    if p.momentum > 0 && (p.dir_dx != 0 || p.dir_dy != 0) {
        let nx = x as i8 + p.dir_dx;
        let ny = y as i8 + p.dir_dy;
        if nx >= 0 && ny >= 0 && (nx as u8) < level.width && (ny as u8) < level.height {
            let nxu = nx as u8;
            let nyu = ny as u8;
            if !matches!(level.tile(nxu, nyu).obstacle, Some(ObstacleKind::Block)) {
                return Some((nxu, nyu));
            }
        }
        // blocked: drop momentum and fallthrough to normal logic
    }

    // Prefer moving onto an adjacent JumpPad if present (it will launch the piece)
    let dirs: [(i8, i8); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
    for (dx, dy) in dirs {
        let nx = x as i8 + dx;
        let ny = y as i8 + dy;
        if nx < 0 || ny < 0 || (nx as u8) >= level.width || (ny as u8) >= level.height {
            continue;
        }
        let tile = level.tile(nx as u8, ny as u8);
        if let Some(ObstacleKind::JumpPad { .. }) = tile.obstacle {
            return Some((nx as u8, ny as u8));
        }
    }

    // Fallback to greedy nearest-goal step
    choose_next_step(level, x, y)
}

#[allow(dead_code)]
fn choose_next_step(level: &LevelDesc, x: u8, y: u8) -> Option<(u8, u8)> {
    // Greedy: pick neighbor (4-dir) that reduces Manhattan distance to ANY goal tile and is not blocked.
    let dirs: [(i8, i8); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
    let mut best: Option<((u8, u8), i32)> = None;
    let dist_to_goal =
        |gx: u8, gy: u8| -> i32 { (gx as i32 - x as i32).abs() + (gy as i32 - y as i32).abs() };
    let cur_best_dist = level
        .goal_region
        .iter()
        .map(|&(gx, gy)| dist_to_goal(gx, gy))
        .min()
        .unwrap_or(0);
    for (dx, dy) in dirs {
        let nx = x as i8 + dx;
        let ny = y as i8 + dy;
        if nx < 0 || ny < 0 || nx as u8 >= level.width || ny as u8 >= level.height {
            continue;
        }
        let nxu = nx as u8;
        let nyu = ny as u8;
        // skip blocked
        if matches!(level.tile(nxu, nyu).obstacle, Some(ObstacleKind::Block)) {
            continue;
        }
        let nd = level
            .goal_region
            .iter()
            .map(|&(gx, gy)| (gx as i32 - nx as i32).abs() + (gy as i32 - ny as i32).abs())
            .min()
            .unwrap_or(i32::MAX);
        if nd <= cur_best_dist {
            // allow equal to avoid deadlock
            if let Some((_, bestd)) = &best {
                if nd > *bestd {
                    continue;
                }
            }
            best = Some(((nxu, nyu), nd));
        }
    }
    best.map(|(pos, _)| pos)
}

fn line(ctx: &CanvasRenderingContext2d, x1: f64, y1: f64, x2: f64, y2: f64) {
    ctx.begin_path();
    ctx.move_to(x1, y1);
    ctx.line_to(x2, y2);
    ctx.stroke();
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to build a simple LevelDesc with optional blocked tiles and leaked static slices
    fn make_level_with_tiles(
        width: u8,
        height: u8,
        obstacle_positions: &[(u8, u8)],
        goals: &[(u8, u8)],
    ) -> LevelDesc {
        let mut tiles_vec = vec![
            TileDesc {
                obstacle: None,
                modifier: None
            };
            (width as usize * height as usize)
        ];
        for &(ox, oy) in obstacle_positions.iter() {
            let idx = oy as usize * width as usize + ox as usize;
            tiles_vec[idx] = TileDesc {
                obstacle: Some(ObstacleKind::Block),
                modifier: None,
            };
        }
        let tiles_static: &'static [TileDesc] = Box::leak(tiles_vec.into_boxed_slice());
        let spawn_static: &'static [(u8, u8)] = Box::leak(vec![(0u8, 0u8)].into_boxed_slice());
        let goal_static: &'static [(u8, u8)] = Box::leak(goals.to_vec().into_boxed_slice());
        LevelDesc {
            name: "test-level",
            width,
            height,
            bpm: 120.0,
            tiles: tiles_static,
            spawn_points: spawn_static,
            goal_region: goal_static,
        }
    }

    #[test]
    fn test_beatclock() {
        let start = 1_000.0;
        let clock = BeatClock::new(120.0, start);
        assert!((clock.beat_duration_ms() - 500.0).abs() < 1e-6);
        assert!((clock.current_beat(start) - 0.0).abs() < 1e-9);
        assert!((clock.current_beat(start + 500.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_level_tile_access() {
        let lvl = make_level_with_tiles(2, 2, &[], &[(1, 1)]);
        let t = lvl.tile(1, 0);
        assert!(t.obstacle.is_none());
        let g = lvl.tile(1, 1);
        assert!(g.obstacle.is_none());
    }

    #[test]
    fn test_choose_next_step_prefers_unblocked_direction() {
        // Create 3x3 level with (1,0) blocked so (0,0) should move down to (0,1)
        let lvl = make_level_with_tiles(3, 3, &[(1, 0)], &[(2, 2)]);
        let step = choose_next_step(&lvl, 0, 0);
        assert_eq!(step, Some((0, 1)));
    }

    #[test]
    fn test_choose_next_for_piece_momentum() {
        let lvl = make_level_with_tiles(3, 3, &[], &[(2, 2)]);
        let mut p = Piece::new("你", "ni3", 1, 1, 0.0, 200.0);
        p.dir_dx = 1;
        p.dir_dy = 0;
        p.momentum = 1;
        let next = choose_next_for_piece(&lvl, &p);
        assert_eq!(next, Some((2, 1)));
    }
}
