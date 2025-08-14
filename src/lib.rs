//! Hanzi Cat core crate.
//!
//! Legacy falling-note gameplay removed (c8-3). Board-based rhythmic mode is now
//! the default gameplay exposed by `start_game()` (feature gate removed in
//! boardfix tasks). Shared Hanzi & pinyin datasets remain available for future
//! gameplay logic expansions.

use wasm_bindgen::prelude::*;

mod board; // always compiled (feature gate removed)

// Optional small allocator for size (feature gated)
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen(start)]
pub fn wasm_start() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

// -----------------------------------------------------------------------------
// Shared Hanzi datasets (retained for future board integration)
// Tone numbers: 1–5 where 5 denotes neutral tone.
// -----------------------------------------------------------------------------

pub const SINGLE_HANZI: &[(&str, &str)] = &[
    ("你", "ni3"), ("好", "hao3"), ("猫", "mao1"), ("学", "xue2"), ("汉", "han4"), ("字", "zi4"),
    ("黑", "hei1"), ("鱼", "yu2"), ("火", "huo3"), ("山", "shan1"), ("水", "shui3"), ("月", "yue4"),
    ("日", "ri4"), ("天", "tian1"), ("人", "ren2"), ("口", "kou3"), ("中", "zhong1"), ("国", "guo2"),
    ("大", "da4"), ("小", "xiao3"), ("上", "shang4"), ("下", "xia4"), ("左", "zuo3"), ("右", "you4"),
    ("心", "xin1"), ("手", "shou3"), ("目", "mu4"), ("耳", "er3"), ("足", "zu2"), ("食", "shi2"),
    ("米", "mi3"), ("花", "hua1"), ("林", "lin2"), ("电", "dian4"), ("雨", "yu3"), ("风", "feng1"),
];

pub const MULTI_HANZI: &[(&str, &str)] = &[
    ("你好", "ni3hao3"), ("汉字", "han4zi4"), ("黑猫", "hei1mao1"), ("学习", "xue2xi2"), ("火山", "huo3shan1"),
    ("山水", "shan1shui3"), ("月鱼", "yue4yu2"), ("中国", "zhong1guo2"), ("天气", "tian1qi4"), ("大小", "da4xiao3"),
    ("上下", "shang4xia4"), ("左右", "zuo3you4"), ("手机", "shou3ji1"), ("电脑", "dian4nao3"), ("朋友", "peng2you3"),
    ("花草", "hua1cao3"), ("学生", "xue2sheng1"), ("老师", "lao3shi1"), ("眼睛", "yan3jing1"), ("耳朵", "er3duo5"),
    ("开心", "kai1xin1"), ("心情", "xin1qing2"), ("米饭", "mi3fan4"), ("国家", "guo2jia1"), ("语言", "yu3yan2"),
    ("手指", "shou3zhi3"), ("风雨", "feng1yu3"), ("火花", "huo3hua1"), ("雨水", "yu3shui3"), ("电风扇", "dian4feng1shan4"),
];

// -----------------------------------------------------------------------------
// Unified entrypoint
// -----------------------------------------------------------------------------

#[wasm_bindgen]
pub fn start_game() -> Result<(), JsValue> {
    // Launch board mode (default gameplay path)
    board::start_board_mode()
}

#[wasm_bindgen]
pub fn purchase_powerup(_kind: &str) -> bool {
    // Powerups belonged to legacy falling-note system; always return false for now.
    false
}

// Internal helper (currently unused) retained for potential timing utilities.
#[allow(dead_code)]
fn performance_now() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now())
        .unwrap_or(0.0)
}

// All legacy rendering, input, animation loop, sushi drawing, difficulty ramp, and
// combo / scoring logic removed in this step (c8-3).
