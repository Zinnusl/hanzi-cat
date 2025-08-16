// Board Level 2 definition
// This file contains a runtime-built tiles slice and a level2() getter.
use super::{TileDesc, LevelDesc, ObstacleKind, ModifierKind};
use std::sync::OnceLock;

fn build_level2_tiles() -> &'static [TileDesc] {
    use ObstacleKind::*;
    let mut arr: Vec<TileDesc> = vec![TileDesc { obstacle: None, modifier: None }; 81];
    arr[9 * 3 + 1] = TileDesc { obstacle: Some(Block), modifier: None };
    arr[9 * 3 + 2] = TileDesc { obstacle: Some(Block), modifier: None };
    arr[9 * 3 + 3] = TileDesc { obstacle: Some(Block), modifier: None };
    arr[9 * 3 + 4] = TileDesc { obstacle: Some(Block), modifier: None };
    arr[9 * 3 + 5] = TileDesc { obstacle: Some(Block), modifier: None };
    arr[9 * 3 + 6] = TileDesc { obstacle: Some(Block), modifier: None };
    arr[9 * 3 + 7] = TileDesc { obstacle: Some(Block), modifier: None };
    arr[2] = TileDesc { obstacle: Some(Conveyor { dx: 0, dy: 1 }), modifier: None };
    arr[9 + 2] = TileDesc { obstacle: Some(Conveyor { dx: 0, dy: 1 }), modifier: None };
    arr[9 * 2 + 2] = TileDesc { obstacle: Some(Conveyor { dx: 0, dy: 1 }), modifier: None };
    arr[9 * 5 + 5] = TileDesc { obstacle: Some(TempoShift { mult: 1.35, beats: 4 }), modifier: None };
    arr[9 * 6 + 6] = TileDesc { obstacle: Some(Transform), modifier: Some(ModifierKind::TransformMap { pairs: &[ ("你", "好") ] }) };
    Box::leak(arr.into_boxed_slice())
}

pub fn level2() -> &'static LevelDesc {
    static LD: OnceLock<LevelDesc> = OnceLock::new();
    static TILES: OnceLock<&'static [TileDesc]> = OnceLock::new();
    let tiles = TILES.get_or_init(build_level2_tiles);
    LD.get_or_init(|| LevelDesc {
        name: "Conveyor Crossing",
        width: 9,
        height: 9,
        bpm: 126.0,
        tiles,
        spawn_points: &[
            (0, 0), (1, 0), (2, 0), (3, 0), (4, 0), (5, 0), (6, 0), (7, 0), (8, 0),
        ],
        goal_region: &[(8, 8)],
    })
}

pub static LEVEL2_HANZI: [(&str, &str); 6] = [
    ("你", "ni3"),
    ("好", "hao3"),
    ("天", "tian1"),
    ("气", "qi4"),
    ("中", "zhong1"),
    ("国", "guo2"),
];

