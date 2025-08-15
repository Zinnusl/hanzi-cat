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
#[derive(Clone, Copy, Debug)]
pub enum ObstacleKind {
    Block,                                // Cannot enter
    Teleport { to: (u8, u8) },            // Enter -> instantly relocate
    Conveyor { dx: i8, dy: i8 },          // Auto-push piece after landing
    TempoShift { mult: f64, beats: u32 }, // Temporary BPM multiplier effect when stepped on
    Transform, // Placeholder: triggers Hanzi transformation mapping (handled by ModifierKind::TransformMap)
}

/// Tile modifiers (non-exclusive with some obstacles) that adjust piece / hanzi logic.
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
    pieces: Vec<Piece>,
    last_spawn_beat: i64,
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

// Level 1: Simple introduction board (9x9), no obstacles or modifiers.
const LEVEL1_TILES: [TileDesc; 81] = {
    let arr: [TileDesc; 81] = [TileDesc {
        obstacle: None,
        modifier: None,
    }; 81];
    arr
};

static LEVEL1: LevelDesc = LevelDesc {
    name: "Opening Board",
    width: 3,
    height: 9,
    bpm: 120.0,
    tiles: &LEVEL1_TILES,
    // All top-row cells are spawn points (requirement: spawn from any top field)
    spawn_points: &[(0, 0), (1, 0), (2, 0)],
    goal_region: &[(1, 8)],
};

// Level 2: Introduces conveyors, tempo shift and transform tile.
const LEVEL2_TILES: [TileDesc; 81] = {
    use ObstacleKind::*;
    let mut arr: [TileDesc; 81] = [TileDesc {
        obstacle: None,
        modifier: None,
    }; 81];
    // Wall of blocks near middle forcing alternate paths (x = 1..=7 at y=3) with new width 9
    arr[9 * 3 + 1] = TileDesc {
        obstacle: Some(Block),
        modifier: None,
    };
    arr[9 * 3 + 2] = TileDesc {
        obstacle: Some(Block),
        modifier: None,
    };
    arr[9 * 3 + 3] = TileDesc {
        obstacle: Some(Block),
        modifier: None,
    };
    arr[9 * 3 + 4] = TileDesc {
        obstacle: Some(Block),
        modifier: None,
    };
    arr[9 * 3 + 5] = TileDesc {
        obstacle: Some(Block),
        modifier: None,
    };
    arr[9 * 3 + 6] = TileDesc {
        obstacle: Some(Block),
        modifier: None,
    };
    arr[9 * 3 + 7] = TileDesc {
        obstacle: Some(Block),
        modifier: None,
    };
    // Conveyors downward lane at x=2 (rows 0..2)
    arr[9 * 0 + 2] = TileDesc {
        obstacle: Some(Conveyor { dx: 0, dy: 1 }),
        modifier: None,
    }; // (2,0)
    arr[9 * 1 + 2] = TileDesc {
        obstacle: Some(Conveyor { dx: 0, dy: 1 }),
        modifier: None,
    }; // (2,1)
    arr[9 * 2 + 2] = TileDesc {
        obstacle: Some(Conveyor { dx: 0, dy: 1 }),
        modifier: None,
    }; // (2,2)
    // Tempo shift tile speeds hops briefly (5,5)
    arr[9 * 5 + 5] = TileDesc {
        obstacle: Some(TempoShift {
            mult: 1.35,
            beats: 4,
        }),
        modifier: None,
    };
    // Transform tile (map 你->好) at (6,6)
    arr[9 * 6 + 6] = TileDesc {
        obstacle: Some(Transform),
        modifier: Some(ModifierKind::TransformMap {
            pairs: &[("你", "好")],
        }),
    };
    arr
};

static LEVEL2: LevelDesc = LevelDesc {
    name: "Conveyor Crossing",
    width: 9,
    height: 9,
    bpm: 126.0,
    tiles: &LEVEL2_TILES,
    // All top-row cells spawn-capable
    spawn_points: &[
        (0, 0),
        (1, 0),
        (2, 0),
        (3, 0),
        (4, 0),
        (5, 0),
        (6, 0),
        (7, 0),
        (8, 0),
    ],
    goal_region: &[(8, 8)],
};

// Level 3: Complex board with more obstacles and Hanzi
const LEVEL3_TILES: [TileDesc; 81] = [
    // y = 0
    TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: Some(ObstacleKind::Teleport { to: (0, 8) }), modifier: None },
    // y = 1
    TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: Some(ObstacleKind::Conveyor { dx: 1, dy: 0 }), modifier: None }, TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: None, modifier: None },
    // y = 2
    TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: Some(ObstacleKind::Block), modifier: None }, TileDesc { obstacle: Some(ObstacleKind::Block), modifier: None }, TileDesc { obstacle: Some(ObstacleKind::Block), modifier: None }, TileDesc { obstacle: Some(ObstacleKind::Block), modifier: None }, TileDesc { obstacle: Some(ObstacleKind::Block), modifier: None }, TileDesc { obstacle: Some(ObstacleKind::Block), modifier: None }, TileDesc { obstacle: None, modifier: None },
    // y = 3
    TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: Some(ObstacleKind::Block), modifier: None }, TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: None, modifier: None },
    // y = 4
    TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: Some(ObstacleKind::TempoShift { mult: 1.5, beats: 3 }), modifier: None }, TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: Some(ObstacleKind::Block), modifier: None }, TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: None, modifier: None },
    // y = 5
    TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: Some(ObstacleKind::Conveyor { dx: -1, dy: 0 }), modifier: None }, TileDesc { obstacle: None, modifier: None },
    // y = 6
    TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: Some(ObstacleKind::Block), modifier: None }, TileDesc { obstacle: Some(ObstacleKind::Block), modifier: None }, TileDesc { obstacle: Some(ObstacleKind::Block), modifier: None }, TileDesc { obstacle: Some(ObstacleKind::Block), modifier: None }, TileDesc { obstacle: Some(ObstacleKind::Block), modifier: None }, TileDesc { obstacle: Some(ObstacleKind::Block), modifier: None }, TileDesc { obstacle: None, modifier: None },
    // y = 7
    TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: Some(ObstacleKind::Transform), modifier: Some(ModifierKind::TransformMap { pairs: &[("水", "火"), ("山", "田")] }) }, TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: None, modifier: None },
    // y = 8
    TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: None, modifier: None }, TileDesc { obstacle: None, modifier: None },
];

static LEVEL3_HANZI: [(&str, &str); 10] = [
    ("水", "shui3"),
    ("火", "huo3"),
    ("山", "shan1"),
    ("田", "tian2"),
    ("风", "feng1"),
    ("雨", "yu3"),
    ("日", "ri4"),
    ("月", "yue4"),
    ("木", "mu4"),
    ("石", "shi2"),
];

static LEVEL3: LevelDesc = LevelDesc {
    name: "Maze Challenge",
    width: 9,
    height: 9,
    bpm: 132.0,
    tiles: &LEVEL3_TILES,
    spawn_points: &[
        (0, 0),
        (1, 0),
        (2, 0),
        (3, 0),
        (4, 0),
        (5, 0),
        (6, 0),
        (7, 0),
        (8, 0),
    ],
    goal_region: &[(4, 8), (5, 8), (6, 8)],
};

// Ordered level sequence & score thresholds for progression (score must be >= threshold to enter level index)
static LEVELS: [&LevelDesc; 3] = [&LEVEL1, &LEVEL2, &LEVEL3];
static LEVEL_SCORE_THRESHOLDS: [i64; 3] = [0, 2500, 6000];

// --- WASM Entry (board mode) -------------------------------------------------

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
    let board = BoardState {
        canvas: canvas.clone(),
        ctx: ctx.clone(),
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
        // Lives / end state initialization
        lives: 3,
        game_over: false,
        typing: String::new(),
        slash_effects: Vec::new(),
        hover_tile: None,
    };

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
            div.set_attribute("style", "position:fixed; top:10px; left:110px; font-family:'Fira Code', monospace; font-size:15px; padding:4px 8px; background:rgba(0,0,0,0.42); border:1px solid #333; border-radius:6px; z-index:44; letter-spacing:0.5px;").ok();
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
                            let mut matched_positions: Vec<(u8, u8)> = Vec::new();
                            // Collect indices of pieces to remove
                            let mut remove_flags: Vec<bool> = vec![false; state.pieces.len()];
                            for (i, p) in state.pieces.iter().enumerate() {
                                if p.pinyin == typed {
                                    remove_flags[i] = true;
                                    matched_positions.push((p.x, p.y));
                                }
                            }
                            if !matched_positions.is_empty() {
                                // Score: base per piece * multiplier
                                let per = (180.0 * state.score_multiplier) as i64;
                                state.score += per * matched_positions.len() as i64;
                                // Add effects
                                let now_ts = window()
                                    .and_then(|w| w.performance())
                                    .map(|p| p.now())
                                    .unwrap_or(0.0);
                                for (x, y) in matched_positions {
                                    state.slash_effects.push(SlashEffect {
                                        x,
                                        y,
                                        start_ms: now_ts,
                                    });
                                }
                                // Retain only non-removed pieces
                                let mut idx = 0usize;
                                state.pieces.retain(|_| {
                                    let keep = !remove_flags[idx];
                                    idx += 1;
                                    keep
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
                    if let Some(win) = window() {
                        if let Some(doc) = win.document() {
                            if let Some(el) = doc.get_element_by_id("hc-typing") {
                                el.set_text_content(Some(&state.typing));
                            }
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
        use wasm_bindgen::JsCast;
        let canvas_move = canvas.clone();
        let closure = Closure::wrap(Box::new(move |evt: web_sys::MouseEvent| {
            use wasm_bindgen::JsCast;
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
        use wasm_bindgen::JsCast;
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

thread_local! {
    static BOARD_STATE: std::cell::RefCell<Option<BoardState>> = std::cell::RefCell::new(None);
}

fn start_board_loop() {
    use wasm_bindgen::JsCast;
    let f: std::rc::Rc<std::cell::RefCell<Option<Closure<dyn FnMut(f64)>>>> =
        std::rc::Rc::new(std::cell::RefCell::new(None));
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

// Hanzi and pinyin for LEVEL2
static LEVEL2_HANZI: [(&str, &str); 6] = [
    ("你", "ni3"),
    ("好", "hao3"),
    ("天", "tian1"),
    ("气", "qi4"),
    ("中", "zhong1"),
    ("国", "guo2"),
];

fn on_new_beat(state: &mut BoardState, beat_idx: i64, now: f64) {
    // Base hop duration (one half-beat @ 120 BPM ~= 250ms, scaled by hop_time_factor)
    let base_hop_ms = 220.0 * state.hop_time_factor;

    // Spawn rule: every 4 beats up to a soft cap of 5 concurrent pieces.
    // Do not spawn when game is over.
    if !state.game_over && beat_idx % 4 == 0 && state.pieces.len() < 5 {
        if !state.level.spawn_points.is_empty() {
            let idx = rand_index(state.level.spawn_points.len());
            let (sx, sy) = state.level.spawn_points[idx];
            // For LEVEL2, spawn random Hanzi from LEVEL2_HANZI
            let (hanzi, pinyin) = if state.level.name == "Conveyor Crossing" {
                let hidx = rand_index(LEVEL2_HANZI.len());
                LEVEL2_HANZI[hidx]
            } else {
                ("你", "ni3")
            };
            let piece = Piece::new(hanzi, pinyin, sx, sy, now, base_hop_ms);
            state.pieces.push(piece);
        }
    }

    // Schedule a hop for each arrived piece toward goal on each beat.
    for p in &mut state.pieces {
        if !p.arrived {
            continue;
        }
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
        if p.arrived {
            continue;
        }
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
    for idx in arrived_indices.into_iter().rev() {
        // remove from end to avoid shifting earlier indices
        if idx < state.pieces.len() {
            let mut piece = state.pieces.swap_remove(idx);
            apply_tile_effects(&mut piece, state, whole_beat, now);
            state.pieces.push(piece); // order not important for prototype
        }
    }
    // Goal handling: treat pieces reaching any goal tile as misses (lose life)
    let mut misses = 0usize;
    state.pieces.retain(|p| {
        let goal = state
            .level
            .goal_region
            .iter()
            .any(|&(gx, gy)| gx == p.x && gy == p.y);
        if goal {
            misses += 1;
            // Add a slash effect where piece reached goal
            let now_ms = window()
                .and_then(|w| w.performance())
                .map(|p| p.now())
                .unwrap_or(0.0);
            state.slash_effects.push(SlashEffect {
                x: p.x,
                y: p.y,
                start_ms: now_ms,
            });
            false
        } else {
            true
        }
    });
    if misses > 0 {
        state.lives -= misses as i32;
        if state.lives <= 0 {
            state.game_over = true;
        }
    }
}

fn render_board(state: &mut BoardState, now: f64) {
    // Beat pulse backdrop (subtle) based on fractional beat progress.
    let beat_phase = {
        let cb = state.beat.current_beat(now);
        (cb - cb.floor()) as f64
    };
    let pulse = ((beat_phase * std::f64::consts::TAU).sin() * 0.5 + 0.5) * 0.25; // 0..0.25
    let cell_w = state.canvas.width() as f64 / state.level.width as f64;
    let cell_h = state.canvas.height() as f64 / state.level.height as f64;
    let bg = (15.0 + pulse * 40.0) as i32; // animate brightness a bit
    // For compilation robustness avoid the CanvasGradient/Result handling which
    // can differ across web-sys versions. Use a single solid fill color based
    // on the calculated brightness. This preserves a similar backdrop while
    // eliminating type mismatches.
    let color = format!(
        "rgb({},{},{})",
        (bg + 18).clamp(0, 255),
        (bg + 14).clamp(0, 255),
        (bg + 12).clamp(0, 255)
    );
    state
        .ctx
        .set_fill_style(&wasm_bindgen::JsValue::from_str(&color));
    state.ctx.fill_rect(
        0.0,
        0.0,
        state.canvas.width() as f64,
        state.canvas.height() as f64,
    );

    // Spawn row accent (assumes all spawns top row). Draw subtle translucent band.
    let cell_w = state.canvas.width() as f64 / state.level.width as f64;
    let cell_h = state.canvas.height() as f64 / state.level.height as f64;
    state
        .ctx
        .set_fill_style(&wasm_bindgen::JsValue::from_str("rgba(255,220,120,0.08)"));
    state
        .ctx
        .fill_rect(0.0, 0.0, state.canvas.width() as f64, cell_h);

    // Goal region highlight tiles (before grid so lines appear above)
    state
        .ctx
        .set_fill_style(&wasm_bindgen::JsValue::from_str("rgba(120,200,255,0.10)"));
    for &(gx, gy) in state.level.goal_region.iter() {
        let px = gx as f64 * cell_w;
        let py = gy as f64 * cell_h;
        state.ctx.fill_rect(px, py, cell_w, cell_h);
    }

    // Draw grid
    state
        .ctx
        .set_stroke_style(&wasm_bindgen::JsValue::from_str("#222"));
    state.ctx.set_line_width(2.0);
    for x in 0..=state.level.width {
        let fx = x as f64 * cell_w;
        line(&state.ctx, fx, 0.0, fx, state.canvas.height() as f64);
    }
    for y in 0..=state.level.height {
        let fy = y as f64 * cell_h;
        line(&state.ctx, 0.0, fy, state.canvas.width() as f64, fy);
    }

    // Hover highlight (draw after grid, before pieces/obstacles for now)
    if let Some((hx, hy)) = state.hover_tile {
        if hx < state.level.width && hy < state.level.height {
            let px = hx as f64 * cell_w;
            let py = hy as f64 * cell_h;
            state
                .ctx
                .set_stroke_style(&wasm_bindgen::JsValue::from_str("rgba(255,240,150,0.55)"));
            state.ctx.set_line_width(3.0);
            state
                .ctx
                .stroke_rect(px + 1.5, py + 1.5, cell_w - 3.0, cell_h - 3.0);
        }
    }

    // Obstacles
    for y in 0..state.level.height {
        for x in 0..state.level.width {
            let t = state.level.tile(x, y);
            if let Some(obs) = &t.obstacle {
                draw_obstacle(&state.ctx, obs, x, y, cell_w, cell_h);
            }
        }
    }

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
        // Drop shadow + layered strokes for clarity and depth
        state.ctx.set_shadow_color("rgba(0,0,0,0.55)");
        state.ctx.set_shadow_blur(14.0);
        state.ctx.set_shadow_offset_x(0.0);
        state.ctx.set_shadow_offset_y(4.0);
        state.ctx.set_line_width(6.0);
        state
            .ctx
            .set_stroke_style(&wasm_bindgen::JsValue::from_str("rgba(0,0,0,0.85)"));
        state.ctx.stroke_text(p.hanzi, cx, cy).ok();
        // Remove shadow for fill to stay crisp
        state.ctx.set_shadow_blur(0.0);
        state.ctx.set_shadow_offset_x(0.0);
        state.ctx.set_shadow_offset_y(0.0);
        state
            .ctx
            .set_fill_style(&wasm_bindgen::JsValue::from_str("#ffffff"));
        state.ctx.fill_text(p.hanzi, cx, cy).ok();
        // Accent inner glow stroke
        state.ctx.set_line_width(2.0);
        state
            .ctx
            .set_stroke_style(&wasm_bindgen::JsValue::from_str("rgba(255,210,120,0.55)"));
        state.ctx.stroke_text(p.hanzi, cx, cy).ok();
    }

    // Slash effects (draw after pieces for overlay)
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
            .set_stroke_style(&wasm_bindgen::JsValue::from_str(&format!(
                "rgba(255,80,80,{alpha})"
            )));
        // Three parallel diagonal slashes
        for i in 0..3 {
            let offset = i as f64 * 6.0;
            state.ctx.begin_path();
            state.ctx.move_to(left + offset, top);
            state.ctx.line_to(right + offset - 18.0, bottom);
            state.ctx.stroke();
        }
    }

    // GAME OVER overlay
    if state.game_over {
        state
            .ctx
            .set_fill_style(&wasm_bindgen::JsValue::from_str("rgba(0,0,0,0.55)"));
        state.ctx.fill_rect(
            0.0,
            0.0,
            state.canvas.width() as f64,
            state.canvas.height() as f64,
        );
        state
            .ctx
            .set_fill_style(&wasm_bindgen::JsValue::from_str("#ffffff"));
        state.ctx.set_font("72px 'Noto Serif SC', serif");
        state.ctx.set_text_align("center");
        state.ctx.set_line_width(6.0);
        state
            .ctx
            .set_stroke_style(&wasm_bindgen::JsValue::from_str("#000000"));
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
    match obs {
        ObstacleKind::Block => {
            // Solid block with subtle inner X pattern
            ctx.set_fill_style(&JsValue::from_str("#552222"));
            ctx.fill_rect(px + 2.0, py + 2.0, cw - 4.0, ch - 4.0);
            ctx.set_stroke_style(&JsValue::from_str("rgba(255,200,200,0.15)"));
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
            ctx.set_stroke_style(&JsValue::from_str("#4aa3ff"));
            ctx.set_line_width(4.0);
            ctx.begin_path();
            let cx = px + cw / 2.0;
            let cy = py + ch / 2.0;
            let r = (cw.min(ch)) * 0.33;
            ctx.arc(cx, cy, r, 0.0, std::f64::consts::TAU).ok();
            ctx.stroke();
            ctx.set_fill_style(&JsValue::from_str("rgba(70,140,255,0.25)"));
            let side = r * 1.1;
            ctx.fill_rect(cx - side / 2.0, cy - side / 2.0, side, side);
        }
        ObstacleKind::Conveyor { dx, dy } => {
            // Belt: darker base + directional chevrons
            ctx.set_fill_style(&JsValue::from_str("#334433"));
            ctx.fill_rect(px + 2.0, py + 2.0, cw - 4.0, ch - 4.0);
            ctx.set_fill_style(&JsValue::from_str("#88cc88"));
            // Draw 3 small chevrons along movement axis
            let chevrons = 3;
            for i in 0..chevrons {
                let t = (i as f64 + 0.5) / chevrons as f64;
                let (cx, cy) = (px + 2.0 + (cw - 4.0) * t, py + 2.0 + (ch - 4.0) * t);
                ctx.begin_path();
                let size = 6.0;
                match (*dx, *dy) {
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
            ctx.set_fill_style(&JsValue::from_str("#444455"));
            ctx.fill_rect(px + 2.0, py + 2.0, cw - 4.0, ch - 4.0);
            ctx.set_stroke_style(&JsValue::from_str("#b0b0ff"));
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
        ObstacleKind::Transform => {
            // Transform tile: gradient-like base + double arrow
            ctx.set_fill_style(&JsValue::from_str("#333355"));
            ctx.fill_rect(px + 2.0, py + 2.0, cw - 4.0, ch - 4.0);
            ctx.set_stroke_style(&JsValue::from_str("#aac"));
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
    state.beat = BeatClock {
        bpm: state.level.bpm,
        start_ms: now,
        last_beat_idx: -1,
    };
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
