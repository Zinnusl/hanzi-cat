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

use wasm_bindgen::prelude::*;
use web_sys::{window, CanvasRenderingContext2d, HtmlCanvasElement};

// --- Core Time / Beat Model -------------------------------------------------

/// BeatClock tracks timing relative to BPM for scheduling hops.
struct BeatClock {
    bpm: f64,           // beats per minute
    start_ms: f64,      // performance.now() when started
    last_beat_idx: i64, // index of last processed whole beat
}

impl BeatClock {
    fn new(bpm: f64, now: f64) -> Self { Self { bpm, start_ms: now, last_beat_idx: -1 } }
    fn beat_duration_ms(&self) -> f64 { 60_000.0 / self.bpm }
    fn current_beat(&self, now: f64) -> f64 { (now - self.start_ms) / self.beat_duration_ms() }
}

// --- Board / Tiles / Obstacles / Modifiers ----------------------------------

/// Kinds of obstacles that occupy or affect tiles.
#[derive(Clone, Copy, Debug)]
pub enum ObstacleKind {
    Block,                 // Cannot enter
    Teleport { to: (u8,u8) }, // Enter -> instantly relocate
    Conveyor { dx: i8, dy: i8 }, // Auto-push piece after landing
    TempoShift { mult: f64, beats: u32 }, // Temporary BPM multiplier effect when stepped on
    Transform,             // Placeholder: triggers Hanzi transformation mapping (handled by ModifierKind::TransformMap)
}

/// Tile modifiers (non-exclusive with some obstacles) that adjust piece / hanzi logic.
#[derive(Clone, Copy, Debug)]
pub enum ModifierKind {
    ScoreMult { factor: f64, beats: u32 },
    SlowHop   { factor: f64, beats: u32 },
    TransformMap { // Map from original pinyin (or hanzi) to alternate variant
        pairs: &'static [(&'static str, &'static str)],
    },
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TileDesc {
    pub obstacle: Option<ObstacleKind>,
    pub modifier: Option<ModifierKind>,
}

/// Level grid descriptor (immutable). We use a flat vector row-major.
pub struct LevelDesc {
    pub name: &'static str,
    pub width: u8,
    pub height: u8,
    pub bpm: f64,
    pub tiles: &'static [TileDesc], // length = width * height
    pub spawn_points: &'static [(u8,u8)], // where new hanzi pieces can appear
    pub goal_region: &'static [(u8,u8)],  // reaching here could score / advance
}

impl LevelDesc {
    pub fn tile(&self, x: u8, y: u8) -> &TileDesc {
        let idx = y as usize * self.width as usize + x as usize;
        &self.tiles[idx]
    }
}

/// Active piece on the board (represents a Hanzi / word). For now only one piece hops;
/// future: multiple simultaneous streams.
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
}

impl Piece {
    fn begin_hop(&mut self, to_x: u8, to_y: u8, now: f64, duration_ms: f64) {
        self.target_x = to_x;
        self.target_y = to_y;
        self.hop_start_ms = now;
        self.hop_duration_ms = duration_ms;
        self.arrived = false;
    }
}

impl Piece {
    fn new(hanzi: &'static str, pinyin: &'static str, x: u8, y: u8, now: f64, hop_dur: f64) -> Self {
        Self { hanzi, pinyin, x, y, target_x: x, target_y: y, hop_start_ms: now, hop_duration_ms: hop_dur, arrived: true }
    }
}

/// Runtime board state.
struct BoardState {
    canvas: HtmlCanvasElement,
    ctx: CanvasRenderingContext2d,
    level: &'static LevelDesc,
    beat: BeatClock,
    pieces: Vec<Piece>,
    last_spawn_beat: i64,
    level_index: usize,
    // --- Dynamic state for modifiers ---
    score: i64,
    score_multiplier: f64,
    score_mult_end_beat: i64,
    hop_time_factor: f64,       // Multiplier on hop duration ( <1 faster, >1 slower )
    hop_time_end_beat: i64,
}

// --- Static Prototype Level --------------------------------------------------

// Simple 8x8 blank chess-like board with a couple of placeholder obstacles.
const LEVEL1_TILES: [TileDesc; 64] = {
    use ObstacleKind::*;
    // For readability build array manually; blank except a few blocks.
    let mut arr: [TileDesc; 64] = [TileDesc { obstacle: None, modifier: None }; 64];
    // Place two blocks and one teleport example.
    arr[3] = TileDesc { obstacle: Some(Block), modifier: None };      // (3,0)
    arr[8*4 + 5] = TileDesc { obstacle: Some(Block), modifier: None }; // (5,4)
    arr[8*6 + 2] = TileDesc { obstacle: Some(Teleport { to: (7,7) }), modifier: None }; // (2,6) -> (7,7)
    arr
};

static LEVEL1: LevelDesc = LevelDesc {
    name: "Opening Board",
    width: 8,
    height: 8,
    bpm: 120.0,
    tiles: &LEVEL1_TILES,
    spawn_points: &[(0,0), (7,0)],
    goal_region: &[(3,7), (4,7), (5,7)],
};

// Level 2: Introduces conveyors, tempo shift and transform tile.
const LEVEL2_TILES: [TileDesc; 64] = {
    use ObstacleKind::*;
    let mut arr: [TileDesc; 64] = [TileDesc { obstacle: None, modifier: None }; 64];
    // A wall of blocks near middle forcing alternate paths (x = 1..=6 at y=3)
    arr[8*3 + 1] = TileDesc { obstacle: Some(Block), modifier: None };
    arr[8*3 + 2] = TileDesc { obstacle: Some(Block), modifier: None };
    arr[8*3 + 3] = TileDesc { obstacle: Some(Block), modifier: None };
    arr[8*3 + 4] = TileDesc { obstacle: Some(Block), modifier: None };
    arr[8*3 + 5] = TileDesc { obstacle: Some(Block), modifier: None };
    arr[8*3 + 6] = TileDesc { obstacle: Some(Block), modifier: None };
    // Conveyors pushing downward forming faster lanes
    arr[8*0 + 2] = TileDesc { obstacle: Some(Conveyor { dx:0, dy:1 }), modifier: None }; // (2,0)
    arr[8*1 + 2] = TileDesc { obstacle: Some(Conveyor { dx:0, dy:1 }), modifier: None }; // (2,1)
    arr[8*2 + 2] = TileDesc { obstacle: Some(Conveyor { dx:0, dy:1 }), modifier: None }; // (2,2)
    // Tempo shift tile speeds hops briefly
    arr[8*5 + 5] = TileDesc { obstacle: Some(TempoShift { mult: 1.35, beats: 4 }), modifier: None }; // (5,5)
    // Transform tile (will map 你->好 for demo)
    arr[8*6 + 6] = TileDesc { obstacle: Some(Transform), modifier: Some(ModifierKind::TransformMap { pairs: &[ ("你","好") ] }) };
    arr
};

static LEVEL2: LevelDesc = LevelDesc {
    name: "Conveyor Crossing",
    width: 8,
    height: 8,
    bpm: 126.0,
    tiles: &LEVEL2_TILES,
    spawn_points: &[(0,0), (7,0), (0,7)],
    goal_region: &[(7,7)],
};

// Ordered level sequence & score thresholds for progression (score must be >= threshold to enter level index)
static LEVELS: [&LevelDesc; 2] = [&LEVEL1, &LEVEL2];
static LEVEL_SCORE_THRESHOLDS: [i64; 2] = [0, 2500];

// --- WASM Entry (board mode) -------------------------------------------------

#[wasm_bindgen]
pub fn start_board_mode() -> Result<(), JsValue> {
    let win = window().ok_or_else(|| JsValue::from_str("no window"))?;
    let doc = win.document().ok_or_else(|| JsValue::from_str("no document"))?;

    // Create / reuse canvas with id board-canvas (separate from falling mode for now)
    let canvas: HtmlCanvasElement = if let Some(el) = doc.get_element_by_id("hc-board-canvas") {
        el.dyn_into()?
    } else {
        let c: HtmlCanvasElement = doc.create_element("canvas")?.dyn_into()?;
        c.set_id("hc-board-canvas");
        c.set_width(640);
        c.set_height(640);
        doc.body().unwrap().append_child(&c)?;
        c
    };
    let ctx: CanvasRenderingContext2d = canvas.get_context("2d")?.unwrap().dyn_into()?;
    ctx.set_font("40px 'Noto Serif SC', 'SimSun', serif");
    ctx.set_text_align("center");

    let now = win.performance().unwrap().now();
    let board = BoardState {
        canvas: canvas.clone(),
        ctx,
        level: LEVELS[0],
        beat: BeatClock::new(LEVELS[0].bpm, now),
        pieces: Vec::new(),
        last_spawn_beat: -1,
        level_index: 0,
        score: 0,
        score_multiplier: 1.0,
        score_mult_end_beat: -1,
        hop_time_factor: 1.0,
        hop_time_end_beat: -1,
    };

    BOARD_STATE.with(|b| b.replace(Some(board)));
    start_board_loop();
    Ok(())
}

thread_local! {
    static BOARD_STATE: std::cell::RefCell<Option<BoardState>> = std::cell::RefCell::new(None);
}

fn start_board_loop() {
    use wasm_bindgen::JsCast;
    let f: std::rc::Rc<std::cell::RefCell<Option<Closure<dyn FnMut(f64)>>>> = std::rc::Rc::new(std::cell::RefCell::new(None));
    let g = f.clone();
    *g.borrow_mut() = Some(Closure::wrap(Box::new(move |ts: f64| {
        BOARD_STATE.with(|state_cell| {
            if let Some(state) = state_cell.borrow_mut().as_mut() { board_tick(state, ts); }
        });
        if let Some(w) = window() {
            let _ = w.request_animation_frame(f.borrow().as_ref().unwrap().as_ref().unchecked_ref());
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
        for b in state.beat.last_beat_idx + 1 ..= whole { on_new_beat(state, b, now); }
        state.beat.last_beat_idx = whole;
    }
    // Expire temporary effects
    expire_effects(state, whole);
    update_pieces(state, now, whole);
    check_level_progression(state, now, whole);
    render_board(state, now);
}

fn on_new_beat(state: &mut BoardState, beat_idx: i64, now: f64) {
    // Base hop duration (one half-beat @ 120 BPM ~= 250ms, scaled by hop_time_factor)
    let base_hop_ms = 220.0 * state.hop_time_factor;

    // Spawn rule: every 4 beats up to a soft cap of 5 concurrent pieces.
    if beat_idx % 4 == 0 && state.pieces.len() < 5 {
        if let Some(&(sx, sy)) = state.level.spawn_points.get((beat_idx as usize / 4) % state.level.spawn_points.len()) {
            let piece = Piece::new("你", "ni3", sx, sy, now, base_hop_ms);
            state.pieces.push(piece);
        }
    }

    // Schedule a hop for each arrived piece toward goal on each beat.
    for p in &mut state.pieces {
        if !p.arrived { continue; }
        if let Some((nx, ny)) = choose_next_step(state.level, p.x, p.y) {
            p.begin_hop(nx, ny, now, base_hop_ms);
        }
    }

    // Future hooks: tempo-shift recalculation (c6) could adjust hop_time_factor here.
}

fn update_pieces(state: &mut BoardState, now: f64, whole_beat: i64) {
    // Phase 1: progress hops and record arrivals
    let mut arrived_indices: Vec<usize> = Vec::new();
    for (idx, p) in state.pieces.iter_mut().enumerate() {
        if p.arrived { continue; }
        let t = ((now - p.hop_start_ms) / p.hop_duration_ms).clamp(0.0, 1.0);
        if t >= 1.0 {
            p.arrived = true;
            p.x = p.target_x;
            p.y = p.target_y;
            arrived_indices.push(idx);
        }
    }
    // Phase 2: apply tile effects without aliasing mutable borrow of state
    arrived_indices.sort_unstable();
    for idx in arrived_indices.into_iter().rev() { // remove from end to avoid shifting earlier indices
        if idx < state.pieces.len() {
            let mut piece = state.pieces.swap_remove(idx);
            apply_tile_effects(&mut piece, state, whole_beat, now);
            state.pieces.push(piece); // order not important for prototype
        }
    }
    // Goal handling: score & remove pieces that reach any goal tile.
    let mut total_scored = 0;
    state.pieces.retain(|p| {
        let goal = state.level.goal_region.iter().any(|&(gx,gy)| gx == p.x && gy == p.y);
        if goal { total_scored += (200.0 * state.score_multiplier) as i64; false } else { true }
    });
    state.score += total_scored;
}

fn render_board(state: &mut BoardState, now: f64) {
    // Beat pulse backdrop (subtle) based on fractional beat progress.
    let beat_phase = {
        let cb = state.beat.current_beat(now);
        (cb - cb.floor()) as f64
    };
    let pulse = ( (beat_phase * std::f64::consts::TAU).sin() * 0.5 + 0.5 ) * 0.25; // 0..0.25
    let cell_w = state.canvas.width() as f64 / state.level.width as f64;
    let cell_h = state.canvas.height() as f64 / state.level.height as f64;
    let bg = (15.0 + pulse * 40.0) as i32; // animate brightness a bit
    state.ctx.set_fill_style(&wasm_bindgen::JsValue::from_str(&format!("rgb({},{},{})", bg, bg, bg + 8)));
    state.ctx.fill_rect(0.0, 0.0, state.canvas.width() as f64, state.canvas.height() as f64);

    // Draw grid
    state.ctx.set_stroke_style(&wasm_bindgen::JsValue::from_str("#222"));
    state.ctx.set_line_width(2.0);
    for x in 0..=state.level.width { let fx = x as f64 * cell_w; line(&state.ctx, fx, 0.0, fx, state.canvas.height() as f64); }
    for y in 0..=state.level.height { let fy = y as f64 * cell_h; line(&state.ctx, 0.0, fy, state.canvas.width() as f64, fy); }

    // Obstacles
    for y in 0..state.level.height { for x in 0..state.level.width {
        let t = state.level.tile(x, y);
        if let Some(obs) = &t.obstacle { draw_obstacle(&state.ctx, obs, x, y, cell_w, cell_h); }
    }}

    // Pieces
    for p in &state.pieces {
        // Interpolate position if moving (parabolic hop arc for readability)
        let (mut px, mut py) = (p.x as f64, p.y as f64);
        let mut hop_lift = 0.0;
        if !p.arrived {
            let frac = ((now - p.hop_start_ms) / p.hop_duration_ms).clamp(0.0, 1.0);
            px = p.x as f64 + (p.target_x as f64 - p.x as f64) * frac;
            py = p.y as f64 + (p.target_y as f64 - p.y as f64) * frac;
            hop_lift = (-(frac - 0.5).powi(2) + 0.25) * cell_h * 0.55; // simple arc
        }
        let cx = px * cell_w + cell_w / 2.0;
        let cy = py * cell_h + cell_h / 2.0 + 14.0 - hop_lift; // apply lift
        state.ctx.set_fill_style(&wasm_bindgen::JsValue::from_str("#fff"));
        state.ctx.set_stroke_style(&wasm_bindgen::JsValue::from_str("#000"));
        state.ctx.set_line_width(6.0);
        state.ctx.stroke_text(p.hanzi, cx, cy).ok();
        state.ctx.fill_text(p.hanzi, cx, cy).ok();
    }
}

fn draw_obstacle(ctx: &CanvasRenderingContext2d, obs: &ObstacleKind, x: u8, y: u8, cw: f64, ch: f64) {
    let px = x as f64 * cw;
    let py = y as f64 * ch;
    match obs {
        ObstacleKind::Block => {
            ctx.set_fill_style(&JsValue::from_str("#552222"));
            ctx.fill_rect(px+2.0, py+2.0, cw-4.0, ch-4.0);
        }
        ObstacleKind::Teleport { .. } => {
            ctx.set_fill_style(&JsValue::from_str("#224466"));
            ctx.fill_rect(px+4.0, py+4.0, cw-8.0, ch-8.0);
        }
        ObstacleKind::Conveyor { dx, dy } => {
            ctx.set_fill_style(&JsValue::from_str("#334433"));
            ctx.fill_rect(px+2.0, py+2.0, cw-4.0, ch-4.0);
            ctx.set_fill_style(&JsValue::from_str("#88cc88"));
            let arrow = match (*dx, *dy) {
                (1,0) => "→", (-1,0) => "←", (0,1) => "↓", (0,-1) => "↑", _ => "·" };
            ctx.set_font("22px 'Fira Code', monospace");
            ctx.fill_text(arrow, px + cw/2.0, py + ch/2.0 + 8.0).ok();
            ctx.set_font("40px 'Noto Serif SC', 'SimSun', serif");
        }
        ObstacleKind::TempoShift { .. } => {
            ctx.set_fill_style(&JsValue::from_str("#444455"));
            ctx.fill_rect(px+2.0, py+2.0, cw-4.0, ch-4.0);
        }
        ObstacleKind::Transform => {
            ctx.set_fill_style(&JsValue::from_str("#333355"));
            ctx.fill_rect(px+2.0, py+2.0, cw-4.0, ch-4.0);
        }
    }
}

fn apply_tile_effects(piece: &mut Piece, state: &mut BoardState, current_beat: i64, _now: f64) {
    let tile = state.level.tile(piece.x, piece.y);
    // Obstacles with post-arrival effects
    if let Some(obs) = &tile.obstacle {
        match obs {
            ObstacleKind::Teleport { to: (tx, ty) } => {
                piece.x = *tx; piece.y = *ty; // instant relocate
            }
            ObstacleKind::Conveyor { dx, dy } => {
                let nx = piece.x as i8 + *dx; let ny = piece.y as i8 + *dy;
                if nx >= 0 && ny >= 0 && (nx as u8) < state.level.width && (ny as u8) < state.level.height {
                    let nxu = nx as u8; let nyu = ny as u8;
                    if !matches!(state.level.tile(nxu, nyu).obstacle, Some(ObstacleKind::Block)) {
                        // Queue immediate hop (small duration)
                        piece.begin_hop(nxu, nyu, _now, piece.hop_duration_ms * 0.8);
                    }
                }
            }
            ObstacleKind::TempoShift { mult, beats } => {
                state.hop_time_factor *= 1.0 / mult; // faster tempo => shorter hops
                state.hop_time_end_beat = current_beat + *beats as i64;
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
    if state.level_index + 1 < LEVELS.len() {
        let next_idx = state.level_index + 1;
        if state.score >= LEVEL_SCORE_THRESHOLDS[next_idx] {
            set_level(state, next_idx, now, current_beat);
        }
    }
}

fn set_level(state: &mut BoardState, new_index: usize, now: f64, current_beat: i64) {
    state.level_index = new_index;
    state.level = LEVELS[new_index];
    state.pieces.clear();
    state.last_spawn_beat = current_beat; // prevent immediate double-spawn
    state.beat = BeatClock { bpm: state.level.bpm, start_ms: now, last_beat_idx: -1 };
    state.hop_time_factor = 1.0;
    state.hop_time_end_beat = -1;
    state.score_multiplier = 1.0;
    state.score_mult_end_beat = -1;
}

fn choose_next_step(level: &LevelDesc, x: u8, y: u8) -> Option<(u8,u8)> {
    // Greedy: pick neighbor (4-dir) that reduces Manhattan distance to ANY goal tile and is not blocked.
    let dirs: [(i8,i8);4] = [(1,0),(-1,0),(0,1),(0,-1)];
    let mut best: Option<((u8,u8), i32)> = None;
    let dist_to_goal = |gx: u8, gy: u8| -> i32 { (gx as i32 - x as i32).abs() + (gy as i32 - y as i32).abs() };
    let cur_best_dist = level.goal_region.iter().map(|&(gx,gy)| dist_to_goal(gx,gy)).min().unwrap_or(0);
    for (dx,dy) in dirs {
        let nx = x as i8 + dx; let ny = y as i8 + dy;
        if nx < 0 || ny < 0 || nx as u8 >= level.width || ny as u8 >= level.height { continue; }
        let nxu = nx as u8; let nyu = ny as u8;
        // skip blocked
        if matches!(level.tile(nxu, nyu).obstacle, Some(ObstacleKind::Block)) { continue; }
        let nd = level.goal_region.iter().map(|&(gx,gy)| {
            (gx as i32 - nx as i32).abs() + (gy as i32 - ny as i32).abs()
        }).min().unwrap_or(i32::MAX);
        if nd <= cur_best_dist { // allow equal to avoid deadlock
            if let Some((_, bestd)) = &best { if nd > *bestd { continue; } }
            best = Some(((nxu, nyu), nd));
        }
    }
    best.map(|(pos,_)| pos)
}

fn line(ctx: &CanvasRenderingContext2d, x1: f64, y1: f64, x2: f64, y2: f64) {
    ctx.begin_path();
    ctx.move_to(x1, y1);
    ctx.line_to(x2, y2);
    ctx.stroke();
}

