// Board Level 5 definition
// This file contains LEVEL5_TILES, LEVEL5 static LevelDesc, and LEVEL5_HANZI.
use super::{TileDesc, LevelDesc, ObstacleKind, ModifierKind};
use std::sync::OnceLock;

pub static LEVEL5_HANZI: [(&str, &str); 9] = [
    ("爱", "ai4"),
    ("和", "he2"),
    ("梦", "meng4"),
    ("星", "xing1"),
    ("雨", "yu3"),
    ("雪", "xue3"),
    ("光", "guang1"),
    ("影", "ying3"),
    ("心", "xin1"),
];

fn build_level5_tiles() -> &'static [TileDesc] {
    use ObstacleKind::*;
    let mut arr: Vec<TileDesc> = vec![TileDesc { obstacle: None, modifier: None }; 81];
    for i in 0..9 {
        arr[i] = TileDesc { obstacle: Some(Block), modifier: None };
        arr[9 * 8 + i] = TileDesc { obstacle: Some(Block), modifier: None };
        arr[9 * i] = TileDesc { obstacle: Some(Block), modifier: None };
        arr[9 * i + 8] = TileDesc { obstacle: Some(Block), modifier: None };
    }
    for i in 2..7 {
        arr[9 * 2 + i] = TileDesc { obstacle: Some(Block), modifier: None };
        arr[9 * 6 + i] = TileDesc { obstacle: Some(Block), modifier: None };
        arr[9 * i + 2] = TileDesc { obstacle: Some(Block), modifier: None };
        arr[9 * i + 6] = TileDesc { obstacle: Some(Block), modifier: None };
    }
    for i in 4..5 {
        arr[9 * 4 + i] = TileDesc { obstacle: Some(Block), modifier: None };
        arr[9 * i + 4] = TileDesc { obstacle: Some(Block), modifier: None };
    }
    arr[16] = TileDesc { obstacle: Some(Teleport { to: (7, 1) }), modifier: None };
    arr[64] = TileDesc { obstacle: Some(Teleport { to: (1, 7) }), modifier: None };
    arr[32] = TileDesc { obstacle: Some(Conveyor { dx: 0, dy: 1 }), modifier: None };
    arr[48] = TileDesc { obstacle: Some(Conveyor { dx: 1, dy: 0 }), modifier: None };
    arr[10] = TileDesc { obstacle: None, modifier: Some(ModifierKind::ScoreMult { factor: 2.0, beats: 4 }) };
    arr[70] = TileDesc { obstacle: None, modifier: Some(ModifierKind::SlowHop { factor: 1.5, beats: 3 }) };
    arr[40] = TileDesc { obstacle: Some(Transform), modifier: Some(ModifierKind::TransformMap { pairs: &[ ("梦", "星"), ("光", "影") ] }) };
    Box::leak(arr.into_boxed_slice())
}

pub fn level5() -> &'static LevelDesc {
    static LD: OnceLock<LevelDesc> = OnceLock::new();
    static TILES: OnceLock<&'static [TileDesc]> = OnceLock::new();
    let tiles = TILES.get_or_init(build_level5_tiles);
    LD.get_or_init(|| LevelDesc {
        name: "Spiral Dream",
        width: 9,
        height: 9,
        bpm: 135.0,
        tiles,
        spawn_points: &[ (1, 1), (1, 7), (7, 1), (7, 7) ],
        goal_region: &[ (4, 4) ],
    })
}

