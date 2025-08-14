//! Hanzi Cat WASM game core.
//! A simple rhythm / typing game to learn Hanzi pronunciation (pinyin) with a scrolling lane
//! similar to Guitar Hero. Characters (Hanzi) fall down. Type their pinyin before they reach
//! the bottom. Score points for correct timing & spelling. A friendly minimalist black SVG cat
//! decorates the UI.

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{window, CanvasRenderingContext2d, HtmlCanvasElement, KeyboardEvent};

// Use a smaller allocator if the feature is enabled (optional)
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen(start)]
pub fn wasm_start() {
    // Set better panic hook in debug
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

// Data for a single falling Hanzi note.
#[derive(Clone)]
struct Note {
    hanzi: &'static str,
    pinyin: &'static str,
    spawn_ms: f64, // When it was spawned (performance.now())
    hit: bool,
    sushi_variant: u8, // Which sushi graphic to draw under this note (0-9)
                       // For some simple floating effect we can add a phase offset later.
}

struct Game {
    canvas: HtmlCanvasElement,
    ctx: CanvasRenderingContext2d,
    notes: Vec<Note>,
    // User typing buffer (current attempt for the bottom-most target note)
    typing: String,
    score: i32,
    combo: i32,
    last_spawn_ms: f64,
    spawn_interval_ms: f64,
    speed_px_per_ms: f64,
    width: f64,
    height: f64,
    started_ms: f64,
    cat_drawn: bool,
    lives: i32,
    game_over: bool,
    // 0.0 -> 1.0 progression for dynamic difficulty scaling over a session
    difficulty_progress: f64,
}

thread_local! {
    static GAME: RefCell<Option<Game>> = RefCell::new(None);
}

// Small curated set of Hanzi with pinyin (no tone marks for typing simplicity).
// In future this can be external JSON loaded via fetch + serde.
const HANZI_LIST: &[(&str, &str)] = &[
    // Single characters with tone numbers
    ("你", "ni3"),
    ("好", "hao3"),
    ("猫", "mao1"),
    ("学", "xue2"),
    ("汉", "han4"),
    ("字", "zi4"),
    ("黑", "hei1"),
    ("鱼", "yu2"),
    ("火", "huo3"),
    ("山", "shan1"),
    ("水", "shui3"),
    ("月", "yue4"),
    // Multi-character words
    ("你好", "ni3hao3"),
    ("汉字", "han4zi4"),
    ("黑猫", "hei1mao1"),
    ("学习", "xue2xi2"),
    ("火山", "huo3shan1"),
    ("山水", "shan1shui3"),
    ("月鱼", "yue4yu2"),
];

// Feature/config toggles
const SHOW_SUSHI: bool = true; // Set to false to disable sushi base rendering for performance tests

// Difficulty ramp configuration (session length targeted ~3 minutes)
const DIFFICULTY_TOTAL_MS: f64 = 180_000.0; // time to reach max difficulty
const INITIAL_SPAWN_INTERVAL_MS: f64 = 1400.0;
const FINAL_SPAWN_INTERVAL_MS: f64 = 550.0;
const INITIAL_SPEED_PX_PER_MS: f64 = 0.18;
const FINAL_SPEED_PX_PER_MS: f64 = 0.34;
const MULTI_CHAR_INITIAL: f64 = 0.12; // starting probability of spawning a multi-character word
const MULTI_CHAR_FINAL: f64 = 0.55;   // final probability at max difficulty

// Separate slices for single vs multi-character notes (referencing same static data)
const SINGLE_HANZI: &[(&str, &str)] = &[
    ("你", "ni3"),
    ("好", "hao3"),
    ("猫", "mao1"),
    ("学", "xue2"),
    ("汉", "han4"),
    ("字", "zi4"),
    ("黑", "hei1"),
    ("鱼", "yu2"),
    ("火", "huo3"),
    ("山", "shan1"),
    ("水", "shui3"),
    ("月", "yue4"),
];
const MULTI_HANZI: &[(&str, &str)] = &[
    ("你好", "ni3hao3"),
    ("汉字", "han4zi4"),
    ("黑猫", "hei1mao1"),
    ("学习", "xue2xi2"),
    ("火山", "huo3shan1"),
    ("山水", "shan1shui3"),
    ("月鱼", "yue4yu2"),
];

#[wasm_bindgen]
pub fn start_game() -> Result<(), JsValue> {
    let win = window().ok_or_else(|| JsValue::from_str("no window"))?;
    let doc = win
        .document()
        .ok_or_else(|| JsValue::from_str("no document"))?;

    // Root container (create if not existing)
    let root_id = "hanzi-cat-root";
    let root = if let Some(elem) = doc.get_element_by_id(root_id) {
        elem
    } else {
        let div = doc.create_element("div")?;
        div.set_id(root_id);
        doc.body().unwrap().append_child(&div)?;
        div
    };

    inject_base_style(&doc)?;

    // Clear existing root contents (restart scenario)
    while let Some(child) = root.first_child() {
        root.remove_child(&child)?;
    }

    // Add title bar & score display
    let header = doc.create_element("div")?;
    header.set_class_name("hc-header");
    header.set_inner_html("<span class='hc-title'>Hanzi Cat</span> <span id='hc-score'>Score: 0</span> <span id='hc-combo'>Combo: 0</span> <span id='hc-lives'></span>");
    root.append_child(&header)?;

    // Canvas playfield
    let canvas: HtmlCanvasElement = doc.create_element("canvas")?.dyn_into()?;
    canvas.set_id("hc-canvas");
    root.append_child(&canvas)?;

    // Cat decorative SVG (inline)
    let cat_container = doc.create_element("div")?;
    cat_container.set_class_name("hc-cat-container");
    cat_container.set_inner_html(CAT_SVG);
    root.append_child(&cat_container)?;

    // Typing overlay (higher z-level above cat, separate from canvas)
    let typing_div = doc.create_element("div")?;
    typing_div.set_id("hc-typing");
    root.append_child(&typing_div)?;

    // Resize canvas to window width minus some margin
    let width = win.inner_width()?.as_f64().unwrap_or(800.0) - 40.0;
    let height = win.inner_height()?.as_f64().unwrap_or(600.0) - 120.0; // leave space for header (cat overlays bottom)
    canvas.set_width(width as u32);
    canvas.set_height(height as u32);

    let ctx: CanvasRenderingContext2d = canvas
        .get_context("2d")?
        .ok_or_else(|| JsValue::from_str("no ctx"))?
        .dyn_into()?;

    ctx.set_font("48px 'Noto Serif SC', 'SimSun', serif");
    ctx.set_text_align("center");

    let now = win.performance().unwrap().now();
    let game = Game {
        canvas,
        ctx,
        notes: Vec::new(),
        typing: String::new(),
        score: 0,
        combo: 0,
        last_spawn_ms: now,
        spawn_interval_ms: INITIAL_SPAWN_INTERVAL_MS,
        speed_px_per_ms: INITIAL_SPEED_PX_PER_MS, // pixels per ms
        width,
        height,
        started_ms: now,
        cat_drawn: false,
        lives: 3,
        game_over: false,
        difficulty_progress: 0.0,
    };

    GAME.with(|g| *g.borrow_mut() = Some(game));

    // Input listener
    {
        let closure = Closure::wrap(Box::new(|evt: KeyboardEvent| {
            // Prevent space/arrow from scrolling page if we use them later
            if let Some(code) = evt.code().strip_prefix("Arrow") {
                let _ = code;
                evt.prevent_default();
            }
            handle_key(evt);
        }) as Box<dyn FnMut(_)>);
        doc.add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())?;
        // Leak the closure (intentional for life of page)
        closure.forget();
    }

    // Start animation loop
    start_animation_loop();

    Ok(())
}

fn handle_key(evt: KeyboardEvent) {
    GAME.with(|g| {
        let mut game_opt = g.borrow_mut();
        let game = match game_opt.as_mut() {
            Some(g) => g,
            None => return,
        };

        let key = evt.key();
        if key == "Escape" {
            game.typing.clear();
            return;
        }
        if key == "Backspace" {
            game.typing.pop();
            return;
        }
        if key.len() == 1 {
            let c = key.chars().next().unwrap();
            if c.is_ascii_alphabetic() {
                game.typing.push(c.to_ascii_lowercase());
            } else if c.is_ascii_digit() {
                // Allow multiple tone numbers (1-5) but never two digits in a row, and only after at least one letter
                if matches!(c, '1' | '2' | '3' | '4' | '5') {
                    if !game.typing.is_empty()
                        && game
                            .typing
                            .chars()
                            .last()
                            .map(|lc| lc.is_ascii_alphabetic())
                            .unwrap_or(false)
                    {
                        game.typing.push(c);
                    }
                }
            }
        }

        // Evaluate against the lowest (oldest) active note that isn't hit yet.
        if let Some(target_index) = game.notes.iter().position(|n| !n.hit) {
            let mut remove_note = false;
            let note = &game.notes[target_index];
            if !note.pinyin.starts_with(&game.typing) {
                // Wrong path => reset typing, small combo break
                game.typing.clear();
                game.combo = 0;
            } else if note.pinyin == game.typing {
                // Need to also ensure note is within judge zone (near bottom)
                let now = performance_now();
                let y = note_y(note, now, game.speed_px_per_ms);
                let judge_line = game.height - 100.0; // Where to aim
                if y >= judge_line - 60.0 && y <= judge_line + 40.0 {
                    // Success
                    game.score += 100 + (game.combo * 10);
                    game.combo += 1;
                    remove_note = true;
                } else {
                    // Correct but outside ideal timing window: smaller reward, still keep/increase combo
                    // We still advance combo so players are rewarded for accuracy even if timing is off.
                    game.score += 50 + (game.combo * 5);
                    game.combo += 1;
                    remove_note = true;
                }
                game.typing.clear();
            }
            if remove_note {
                game.notes[target_index].hit = true;
            }
        }
    });
}

fn start_animation_loop() {
    // Self-referential closure pattern using Rc<RefCell<Option<Closure>>>.
    let f: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> = Rc::new(RefCell::new(None));
    let g = f.clone();
    *g.borrow_mut() = Some(Closure::wrap(Box::new(move |timestamp_ms: f64| {
        // Tick & render
        GAME.with(|gstate| {
            if let Some(game) = gstate.borrow_mut().as_mut() {
                tick_and_render(game, timestamp_ms);
            }
        });
        // Schedule next frame using original Rc (f)
        if let Some(win) = window() {
            win.request_animation_frame(f.borrow().as_ref().unwrap().as_ref().unchecked_ref())
                .ok();
        }
    }) as Box<dyn FnMut(f64)>));

    // Kick off loop (use g so that f is still owned inside closure)
    if let Some(win) = window() {
        win.request_animation_frame(g.borrow().as_ref().unwrap().as_ref().unchecked_ref())
            .ok();
    }
    // Leak Rc cycle intentionally (game lifetime = page lifetime)
}

fn tick_and_render(game: &mut Game, now: f64) {
    // Update dynamic difficulty parameters
    update_difficulty(game, now);

    // Spawn new note (unless game over)
    if !game.game_over && now - game.last_spawn_ms >= game.spawn_interval_ms {
        let (h, p) = choose_note(game.difficulty_progress);
        game.notes.push(Note {
            hanzi: h,
            pinyin: p,
            spawn_ms: now,
            hit: false,
            sushi_variant: (rand_index(10) as u8),
        });
        game.last_spawn_ms = now;
    }

    // Clear
    game.ctx.set_fill_style(&JsValue::from_str("#111"));
    game.ctx.fill_rect(0.0, 0.0, game.width, game.height);

    // Draw lane guidelines & judge line
    game.ctx.set_stroke_style(&JsValue::from_str("#333"));
    game.ctx.set_line_width(2.0);
    let judge_line = game.height - 100.0;
    game.ctx.begin_path();
    game.ctx.move_to(0.0, judge_line);
    game.ctx.line_to(game.width, judge_line);
    game.ctx.stroke();

    // Render notes (highlight current target)
    let mut any_removed = false;
    let target_index = game.notes.iter().position(|n| !n.hit);
    let claw_zone = game.height - 140.0; // zone where cat will start claw animation
    let mut claw_active = false;
    for (i, note) in game.notes.iter_mut().enumerate() {
        if note.hit {
            continue;
        }
        let y = note_y(note, now, game.speed_px_per_ms);
        if y > game.height + 60.0 {
            // Missed
            note.hit = true; // mark for removal
            game.combo = 0;
            if game.lives > 0 {
                game.lives -= 1;
            }
            if game.lives <= 0 {
                game.game_over = true;
            }
            continue;
        }
        let is_claw_zone = y >= claw_zone && y < game.height - 20.0;
        if is_claw_zone {
            claw_active = true;
        }
        let alpha = if y < judge_line { 1.0 } else { 0.9 };
        let is_target = matches!(target_index, Some(ti) if ti == i);

        // Draw sushi base under the character (behind text)
        if SHOW_SUSHI {
            draw_sushi(&game.ctx, note.sushi_variant, game.width / 2.0, y - 12.0); // slight upward offset so text sits centered
        }

        if is_claw_zone {
            // Surprised / scared styling retains red outline for danger zone
            game.ctx.set_fill_style(&JsValue::from_str("#ffffff"));
            game.ctx.set_stroke_style(&JsValue::from_str("#ff4d4d"));
            game.ctx.set_line_width(4.0);
            game.ctx.stroke_text(note.hanzi, game.width / 2.0, y).ok();
        } else if is_target {
            // Target note uses gold fill; add black outline for contrast over sushi
            game.ctx.set_fill_style(&JsValue::from_str("#ffd166"));
            game.ctx.set_stroke_style(&JsValue::from_str("#000"));
            game.ctx.set_line_width(6.0);
            game.ctx.stroke_text(note.hanzi, game.width / 2.0, y).ok();
        } else {
            // Non-target note gets white (variable alpha) with black outline for readability
            game.ctx
                .set_fill_style(&JsValue::from_str(&format!("rgba(255,255,255,{alpha})")));
            game.ctx.set_stroke_style(&JsValue::from_str("#000"));
            game.ctx.set_line_width(6.0);
            game.ctx.stroke_text(note.hanzi, game.width / 2.0, y).ok();
        }
        game.ctx.fill_text(note.hanzi, game.width / 2.0, y).ok();
        if is_target {
            // subtle underline highlight
            game.ctx.set_stroke_style(&JsValue::from_str("#ffd166"));
            game.ctx.set_line_width(3.0);
            game.ctx.begin_path();
            game.ctx.move_to(game.width / 2.0 - 40.0, y + 10.0);
            game.ctx.line_to(game.width / 2.0 + 40.0, y + 10.0);
            game.ctx.stroke();
        }
        if is_claw_zone {
            // Add exclamation marks above character for surprised look
            game.ctx.set_fill_style(&JsValue::from_str("#ff5a5a"));
            game.ctx.set_font("26px 'Fira Code', monospace");
            game.ctx
                .fill_text("!!", game.width / 2.0 + 42.0, y - 48.0)
                .ok();
            game.ctx.set_font("48px 'Noto Serif SC', 'SimSun', serif"); // restore
        }
    }
    // Toggle cat clawing class based on zone activity
    if let Some(doc) = window().and_then(|w| w.document()) {
        if let Ok(Some(cat)) = doc.query_selector(".hc-cat-container") {
            if let Ok(el) = cat.dyn_into::<web_sys::HtmlElement>() {
                let existing = el.class_name();
                let has = existing.split_whitespace().any(|c| c == "clawing");
                if claw_active && !has {
                    let new_classes = if existing.is_empty() {
                        "clawing".to_string()
                    } else {
                        format!("{existing} clawing")
                    };
                    el.set_class_name(&new_classes);
                } else if !claw_active && has {
                    let new_classes = existing
                        .split_whitespace()
                        .filter(|c| *c != "clawing")
                        .collect::<Vec<_>>()
                        .join(" ");
                    el.set_class_name(&new_classes);
                }
            }
        }
    }

    // Remove hit notes
    if any_removed {
        game.notes.retain(|n| !n.hit);
    } else {
        game.notes.retain(|n| !n.hit);
    }

    // Typing buffer now rendered in DOM overlay (#hc-typing) above cat; removed from canvas.

    // Update score UI
    if let Some(doc) = window().and_then(|w| w.document()) {
        if let Some(score_el) = doc.get_element_by_id("hc-score") {
            score_el.set_text_content(Some(&format!("Score: {}", game.score)));
        }
        if let Some(combo_el) = doc.get_element_by_id("hc-combo") {
            combo_el.set_text_content(Some(&format!("Combo: {}", game.combo)));
        }
        if let Some(lives_el) = doc.get_element_by_id("hc-lives") {
            let mut hearts = String::new();
            for i in 0..3 {
                // fixed 3 lives display
                let class = if i < game.lives { "full" } else { "empty" };
                hearts.push_str(&format!("<span class='hc-heart {class}'><svg viewBox='0 0 16 16'><path d='M4 1h2l2 2 2-2h2l3 3v2l-7 8-7-8V4z'/></svg></span>"));
            }
            lives_el.set_inner_html(&hearts);
        }
        if let Some(typing_el) = doc.get_element_by_id("hc-typing") {
            typing_el.set_text_content(Some(if game.typing.is_empty() { "" } else { &game.typing }));
        }
    }

    // Game over overlay
    if game.game_over {
        game.ctx
            .set_fill_style(&JsValue::from_str("rgba(0,0,0,0.6)"));
        game.ctx.fill_rect(0.0, 0.0, game.width, game.height);
        game.ctx.set_fill_style(&JsValue::from_str("#f55"));
        game.ctx.set_font("64px 'Fira Code', monospace");
        game.ctx
            .fill_text("GAME OVER", game.width / 2.0, game.height / 2.0)
            .ok();
        game.ctx.set_font("48px 'Noto Serif SC', 'SimSun', serif"); // restore
    }
}

fn note_y(note: &Note, now: f64, speed_px_per_ms: f64) -> f64 {
    (now - note.spawn_ms) * speed_px_per_ms - 50.0 // offset so it starts above view slightly
}

// Draw one of 10 sushi variants at (x, y) center baseline.
// Each variant kept intentionally simple (few path operations) to limit draw cost.
fn draw_sushi(ctx: &CanvasRenderingContext2d, variant: u8, x: f64, y: f64) {
    // Common shadow
    ctx.set_fill_style(&JsValue::from_str("rgba(0,0,0,0.35)"));
    ctx.begin_path();
    ctx.ellipse(x, y + 28.0, 46.0, 9.0, 0.0, 0.0, std::f64::consts::TAU)
        .ok();
    ctx.fill();

    // Rice base
    ctx.set_fill_style(&JsValue::from_str("#f7f7f7"));
    ctx.set_stroke_style(&JsValue::from_str("#e0e0e0"));
    ctx.set_line_width(2.0);
    rounded_rect(ctx, x - 44.0, y - 4.0, 88.0, 32.0, 14.0);
    ctx.fill();
    ctx.stroke();

    match variant % 10 {
        0 => {
            // Salmon nigiri stripes
            topping_capsule(ctx, x, y - 10.0, 84.0, 22.0, "#ff8c42", "#e05a12");
            fish_stripes(ctx, x, y - 10.0, "rgba(255,255,255,0.55)");
        }
        1 => {
            // Tuna nigiri
            topping_capsule(ctx, x, y - 10.0, 84.0, 22.0, "#e43d53", "#b41224");
            fish_stripes(ctx, x, y - 10.0, "rgba(255,255,255,0.45)");
        }
        2 => {
            // Shrimp
            topping_capsule(ctx, x, y - 10.0, 84.0, 22.0, "#ffb18b", "#ff9060");
            shrimp_bands(ctx, x, y - 10.0);
        }
        3 => {
            // Tamago (egg)
            topping_capsule(ctx, x, y - 10.0, 84.0, 24.0, "#ffe56b", "#e0c240");
            nori_wrap(ctx, x, y - 6.0);
        }
        4 => {
            // Eel (unagi)
            topping_capsule(ctx, x, y - 10.0, 84.0, 22.0, "#7b4a21", "#5a3315");
            glaze_lines(ctx, x, y - 10.0);
            nori_wrap(ctx, x, y - 6.0);
        }
        5 => {
            // Roe gunkan
            gunkan_base(ctx, x, y);
            roe_dots(ctx, x, y - 2.0);
        }
        6 => {
            // Cucumber maki
            maki_roll(ctx, x, y, "#6abf4b");
        }
        7 => {
            // Salmon maki
            maki_roll(ctx, x, y, "#ff8742");
        }
        8 => {
            // Avocado roll
            maki_roll(ctx, x, y, "#b4d85a");
        }
        _ => {
            // Octopus nigiri
            topping_capsule(ctx, x, y - 10.0, 84.0, 22.0, "#d8a4c9", "#b476a3");
            suction_dots(ctx, x, y - 10.0);
            nori_wrap(ctx, x, y - 6.0);
        }
    }
}

fn rounded_rect(ctx: &CanvasRenderingContext2d, x: f64, y: f64, w: f64, h: f64, r: f64) {
    ctx.begin_path();
    ctx.move_to(x + r, y);
    ctx.line_to(x + w - r, y);
    ctx.quadratic_curve_to(x + w, y, x + w, y + r);
    ctx.line_to(x + w, y + h - r);
    ctx.quadratic_curve_to(x + w, y + h, x + w - r, y + h);
    ctx.line_to(x + r, y + h);
    ctx.quadratic_curve_to(x, y + h, x, y + h - r);
    ctx.line_to(x, y + r);
    ctx.quadratic_curve_to(x, y, x + r, y);
}

fn topping_capsule(
    ctx: &CanvasRenderingContext2d,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fill: &str,
    stroke: &str,
) {
    ctx.set_fill_style(&JsValue::from_str(fill));
    ctx.set_stroke_style(&JsValue::from_str(stroke));
    ctx.set_line_width(2.0);
    rounded_rect(ctx, x - w / 2.0, y - h / 2.0, w, h, h * 0.45);
    ctx.fill();
    ctx.stroke();
}

fn fish_stripes(ctx: &CanvasRenderingContext2d, x: f64, y: f64, stroke: &str) {
    ctx.set_stroke_style(&JsValue::from_str(stroke));
    ctx.set_line_width(3.0);
    for i in 0..2 {
        ctx.begin_path();
        let yy = y - 4.0 + (i as f64) * 8.0;
        ctx.move_to(x - 36.0, yy);
        ctx.quadratic_curve_to(x, yy - 6.0, x + 36.0, yy);
        ctx.stroke();
    }
}

fn shrimp_bands(ctx: &CanvasRenderingContext2d, x: f64, y: f64) {
    ctx.set_stroke_style(&JsValue::from_str("#ff9060"));
    ctx.set_line_width(2.0);
    for i in 0..3 {
        ctx.begin_path();
        let yy = y - 6.0 + (i as f64) * 6.0;
        ctx.move_to(x - 34.0, yy);
        ctx.line_to(x + 34.0, yy + 2.0);
        ctx.stroke();
    }
}

fn glaze_lines(ctx: &CanvasRenderingContext2d, x: f64, y: f64) {
    ctx.set_stroke_style(&JsValue::from_str("rgba(255,255,255,0.35)"));
    ctx.set_line_width(3.0);
    for i in 0..2 {
        ctx.begin_path();
        let yy = y - 4.0 + (i as f64) * 8.0;
        ctx.move_to(x - 34.0, yy);
        ctx.quadratic_curve_to(x, yy + 6.0, x + 34.0, yy);
        ctx.stroke();
    }
}

fn nori_wrap(ctx: &CanvasRenderingContext2d, x: f64, y: f64) {
    ctx.set_fill_style(&JsValue::from_str("#1f1f1f"));
    ctx.fill_rect(x - 20.0, y - 12.0, 40.0, 24.0);
    ctx.set_fill_style(&JsValue::from_str("#262626"));
    ctx.fill_rect(x - 20.0, y - 2.0, 40.0, 4.0);
}

fn gunkan_base(ctx: &CanvasRenderingContext2d, x: f64, y: f64) {
    // Seaweed wall
    ctx.set_fill_style(&JsValue::from_str("#1c1c1c"));
    rounded_rect(ctx, x - 38.0, y - 6.0, 76.0, 28.0, 14.0);
    ctx.fill();
    // Inner rice top
    ctx.set_fill_style(&JsValue::from_str("#f4f4f4"));
    ctx.begin_path();
    ctx.ellipse(x, y - 2.0, 32.0, 10.0, 0.0, 0.0, std::f64::consts::TAU)
        .ok();
    ctx.fill();
}

fn roe_dots(ctx: &CanvasRenderingContext2d, x: f64, y: f64) {
    ctx.set_fill_style(&JsValue::from_str("#ff7129"));
    for i in 0..14 {
        let ang = (i as f64) * 0.45;
        let rx = x + ang.cos() * 24.0 * 0.6;
        let ry = y + ang.sin() * 6.0 * 0.6;
        ctx.begin_path();
        ctx.ellipse(rx, ry, 4.5, 4.5, 0.0, 0.0, std::f64::consts::TAU)
            .ok();
        ctx.fill();
    }
}

fn maki_roll(ctx: &CanvasRenderingContext2d, x: f64, y: f64, center_color: &str) {
    // Outer nori
    ctx.set_fill_style(&JsValue::from_str("#181818"));
    ctx.begin_path();
    ctx.ellipse(x, y + 4.0, 30.0, 22.0, 0.0, 0.0, std::f64::consts::TAU)
        .ok();
    ctx.fill();
    // Rice ring
    ctx.set_fill_style(&JsValue::from_str("#f5f5f5"));
    ctx.begin_path();
    ctx.ellipse(x, y + 4.0, 24.0, 17.0, 0.0, 0.0, std::f64::consts::TAU)
        .ok();
    ctx.fill();
    // Center filling
    ctx.set_fill_style(&JsValue::from_str(center_color));
    ctx.begin_path();
    ctx.ellipse(x, y + 4.0, 14.0, 10.0, 0.0, 0.0, std::f64::consts::TAU)
        .ok();
    ctx.fill();
}

fn suction_dots(ctx: &CanvasRenderingContext2d, x: f64, y: f64) {
    ctx.set_fill_style(&JsValue::from_str("#f2d2e7"));
    for i in 0..6 {
        let xx = x - 30.0 + (i as f64) * 12.0;
        ctx.begin_path();
        ctx.ellipse(xx, y, 4.0, 4.0, 0.0, 0.0, std::f64::consts::TAU)
            .ok();
        ctx.fill();
    }
}

fn performance_now() -> f64 {
    window()
        .and_then(|w| w.performance())
        .map(|p| p.now())
        .unwrap_or(0.0)
}

// Simple pseudo-random index using performance.now; NOT CRYPTO. (No dependency to keep small.)
fn rand_index(len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    let now = performance_now();
    (now as usize).wrapping_mul(1103515245).wrapping_add(12345) % len
}

// Pseudo random unit value in [0,1)
fn rand_unit() -> f64 {
    let now = performance_now();
    // Simple LCG style transformation then normalize
    let v = ((now as u64).wrapping_mul(6364136223846793005).wrapping_add(1) >> 17) as u64;
    (v % 1_000_000) as f64 / 1_000_000.0
}

fn lerp(a: f64, b: f64, t: f64) -> f64 { a + (b - a) * t }

// Update difficulty based on elapsed time; linear scaling for now (simple & predictable)
fn update_difficulty(game: &mut Game, now: f64) {
    let elapsed = now - game.started_ms;
    let progress = (elapsed / DIFFICULTY_TOTAL_MS).clamp(0.0, 1.0);
    game.difficulty_progress = progress;
    game.spawn_interval_ms = lerp(INITIAL_SPAWN_INTERVAL_MS, FINAL_SPAWN_INTERVAL_MS, progress);
    game.speed_px_per_ms = lerp(INITIAL_SPEED_PX_PER_MS, FINAL_SPEED_PX_PER_MS, progress);
}

// Choose note respecting probability of multi-character entries as difficulty rises.
fn choose_note(progress: f64) -> (&'static str, &'static str) {
    let multi_prob = lerp(MULTI_CHAR_INITIAL, MULTI_CHAR_FINAL, progress);
    if rand_unit() < multi_prob && !MULTI_HANZI.is_empty() {
        MULTI_HANZI[rand_index(MULTI_HANZI.len())]
    } else {
        // fallback to single if empty or probability branch fails
        SINGLE_HANZI[rand_index(SINGLE_HANZI.len())]
    }
}

// Inject base style only once.
fn inject_base_style(doc: &web_sys::Document) -> Result<(), JsValue> {
    if doc.get_element_by_id("hanzi-cat-style").is_some() {
        return Ok(());
    }
    let style = doc.create_element("style")?;
    style.set_id("hanzi-cat-style");
    style.set_text_content(Some(BASE_CSS));
    // Append to body (avoids needing the head() API feature); fallback silently if body missing.
    if let Some(body) = doc.body() {
        body.append_child(&style)?;
    }
    Ok(())
}

// Minimal black cat SVG (stylized). License: CC0 / generated.
const CAT_SVG: &str = r#"<svg class='hc-cat' viewBox='0 0 200 200' role='img' aria-label='Cat'>
  <defs>
    <linearGradient id='catGrad' x1='0%' y1='0%' x2='0%' y2='100%'>
      <stop offset='0%' stop-color='#0a0a0a'/>
      <stop offset='100%' stop-color='#020202'/>
    </linearGradient>
    <linearGradient id='eyeGrad' x1='0%' y1='0%' x2='100%' y2='0%'>
      <stop offset='0%' stop-color='#ffe9a6'/>
      <stop offset='100%' stop-color='#ffd166'/>
    </linearGradient>
  </defs>
  <!-- Tail behind body -->
  <path class='hc-cat-tail' d='M42 128 Q20 120 28 108 Q38 95 52 104 Q40 90 48 78 Q58 64 70 74 Q82 84 78 104 Q74 124 60 132 Z' fill='url(#catGrad)'/>
  <!-- Body -->
  <ellipse cx='100' cy='125' rx='62' ry='50' fill='url(#catGrad)'/>
  <!-- Head base -->
  <circle cx='100' cy='82' r='55' fill='url(#catGrad)'/>
  <!-- Ears -->
  <path d='M53 58 L70 30 L78 64 Z' fill='url(#catGrad)'/>
  <path d='M147 58 L130 30 L122 64 Z' fill='url(#catGrad)'/>
  <!-- Inner ear accents -->
  <path d='M66 53 L71 39 L75 58 Z' fill='#1d1d1d'/>
  <path d='M134 53 L129 39 L125 58 Z' fill='#1d1d1d'/>
  <!-- Eyes + pupils (blink group) -->
  <g class='blink'>
    <ellipse cx='78' cy='82' rx='14' ry='13' fill='url(#eyeGrad)'/>
    <ellipse cx='122' cy='82' rx='14' ry='13' fill='url(#eyeGrad)'/>
    <circle cx='78' cy='82' r='5' fill='#1b1b1b'/>
    <circle cx='122' cy='82' r='5' fill='#1b1b1b'/>
    <circle cx='76' cy='80' r='2' fill='#fff' opacity='.8'/>
    <circle cx='120' cy='80' r='2' fill='#fff' opacity='.8'/>
  </g>
  <!-- Nose + mouth -->
  <path d='M96 97 Q100 100 104 97 Q100 101 100 102 Q100 101 96 97 Z' fill='#444'/>
  <path d='M92 103 Q100 118 108 103 Q108 112 100 120 Q92 112 92 103 Z' fill='#ff5a5a' stroke='#333' stroke-width='2'/>
  <path d='M97 108 L100 115 L103 108 Z' fill='#fff' opacity='0.85'/>
  <!-- Whiskers -->
  <path d='M46 90 L70 94' stroke='#303030' stroke-width='3' stroke-linecap='round'/>
  <path d='M46 102 L70 98' stroke='#303030' stroke-width='3' stroke-linecap='round'/>
  <path d='M154 90 L130 94' stroke='#303030' stroke-width='3' stroke-linecap='round'/>
  <path d='M154 102 L130 98' stroke='#303030' stroke-width='3' stroke-linecap='round'/>
  <!-- Chest highlight -->
  <ellipse cx='100' cy='140' rx='20' ry='12' fill='#111'/>
  <!-- Paws (animated when clawing) -->
  <g class='hc-paw left'><path d='M70 160 Q60 170 64 182 Q68 194 80 190 Q92 186 90 172 Q88 158 70 160 Z M74 176 Q72 170 78 170 Q84 170 82 176 Q80 182 74 176 Z' fill='#0a0a0a' stroke='#222' stroke-width='2'/></g>
  <g class='hc-paw right'><path d='M130 160 Q140 170 136 182 Q132 194 120 190 Q108 186 110 172 Q112 158 130 160 Z M126 176 Q128 170 122 170 Q116 170 118 176 Q120 182 126 176 Z' fill='#0a0a0a' stroke='#222' stroke-width='2'/></g>
  <!-- Claw swipe lines (3 staggered red slashes) -->
  <g class='hc-claw-swipes'>
    <line x1='90' y1='150' x2='92' y2='95' class='cl1'/>
    <line x1='100' y1='150' x2='102' y2='95' class='cl2'/>
    <line x1='110' y1='150' x2='112' y2='95' class='cl3'/>
  </g>
</svg>"#;

const BASE_CSS: &str = r#"
#hanzi-cat-root { position:relative; font-family: 'Noto Serif SC', 'SimSun', serif; color: #eee; padding: 10px; }
.hc-header { display:flex; gap:1.5rem; align-items:center; font-size:1.2rem; margin-bottom:8px; }
.hc-title { font-weight:700; letter-spacing:1px; color:#ffd166; text-shadow:0 0 6px #222; }
#hc-score, #hc-combo { font-family: 'Fira Code', monospace; }
#hc-canvas { width:100%; max-width:100%; background:#111; border:2px solid #222; border-radius:8px; box-shadow:0 4px 12px rgba(0,0,0,0.5) inset, 0 0 12px rgba(0,0,0,0.4); }
.hc-cat-container { position:absolute; bottom:8px; left:50%; transform:translateX(-50%); width:220px; opacity:0.95; pointer-events:none; }
.hc-cat { width:100%; height:auto; filter: drop-shadow(0 0 4px #000) drop-shadow(0 4px 8px rgba(0,0,0,0.6)); }
.hc-paw { transform-origin: 100px 160px; }
.hc-cat-container.clawing .hc-paw.left { animation: paw-left 0.45s cubic-bezier(.55,.1,.25,1) infinite; }
.hc-cat-container.clawing .hc-paw.right { animation: paw-right 0.45s cubic-bezier(.55,.1,.25,1) infinite; }
.hc-cat-container.clawing .hc-cat { animation: cat-bounce 0.9s ease-in-out infinite; }
.hc-cat .blink { animation: blink 6s infinite; transform-origin: 100px 82px; }
.hc-cat-tail { animation: tail-wag 3.8s ease-in-out infinite; transform-origin: 52px 110px; }
@keyframes blink { 0%,4%,6%,100% { transform:scaleY(1); } 5% { transform:scaleY(.15); } }
@keyframes tail-wag { 0%,100% { transform:rotate(0deg); } 50% { transform:rotate(-10deg); } }
@keyframes cat-bounce { 0%,55%,100% { transform:translateY(0); } 25% { transform:translateY(2px); } 35% { transform:translateY(-3px); } 45% { transform:translateY(1px); } }
@keyframes paw-left {
  0% { transform:translate(-8px,0) rotate(0deg); }
  18% { transform:translate(-10px,4px) rotate(0deg); } /* anticipation down */
  28% { transform:translate(-8px,-46px) rotate(-10deg); } /* rapid strike */
  42% { transform:translate(-9px,-34px) rotate(-4deg); } /* recoil */
  55% { transform:translate(-8px,-40px) rotate(-6deg); } /* settle */
  100% { transform:translate(-8px,0) rotate(0deg); }
}
@keyframes paw-right {
  0% { transform:translate(8px,0) rotate(0deg); }
  18% { transform:translate(10px,4px) rotate(0deg); }
  28% { transform:translate(8px,-46px) rotate(10deg); }
  42% { transform:translate(9px,-34px) rotate(4deg); }
  55% { transform:translate(8px,-40px) rotate(6deg); }
  100% { transform:translate(8px,0) rotate(0deg); }
}
.hc-claw-swipes line { stroke:#ff4040; stroke-width:5; stroke-linecap:round; filter:drop-shadow(0 0 4px #ff2a2a); opacity:0; stroke-dasharray:70; stroke-dashoffset:70; }
.hc-cat-container.clawing .hc-claw-swipes .cl1 { animation: claw-line 0.6s linear infinite; }
.hc-cat-container.clawing .hc-claw-swipes .cl2 { animation: claw-line 0.6s linear infinite 0.06s; }
.hc-cat-container.clawing .hc-claw-swipes .cl3 { animation: claw-line 0.6s linear infinite 0.12s; }
@keyframes claw-line { 0% { stroke-dashoffset:70; opacity:0; } 12% { stroke-dashoffset:0; opacity:1; } 30% { opacity:1; } 48% { opacity:0; } 100% { stroke-dashoffset:70; opacity:0; } }
body { margin:0; background:linear-gradient(#272a33 0 65%, #1a1c20 65% 100%); overflow:hidden; }
@media (max-width: 600px) { .hc-cat-container { display:none; } }
.hc-heart { width:22px; height:22px; display:inline-block; }
.hc-heart.full svg path { fill:#e02828; stroke:#890000; stroke-width:1; }
.hc-heart.empty svg path { fill:#2a0000; stroke:#550000; stroke-width:1; }
.hc-heart svg { width:100%; height:100%; shape-rendering:crispEdges; image-rendering:pixelated; }
#hc-typing { position:absolute; left:50%; bottom:190px; transform:translateX(-50%); font:28px 'Fira Code', monospace; color:#6cf; text-shadow:0 0 6px #000,0 0 12px rgba(0,0,0,0.6); letter-spacing:1px; pointer-events:none; z-index:10; }
"#;
