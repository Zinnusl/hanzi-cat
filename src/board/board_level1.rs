// Board Level 1 definition
// This file contains LEVEL1_TILES and LEVEL1 static LevelDesc.
use super::{TileDesc, LevelDesc};
use std::sync::OnceLock;

fn build_level1_tiles() -> &'static [TileDesc] {
    let mut arr: Vec<TileDesc> = vec![TileDesc { obstacle: None, modifier: None }; 81];
    // No special tiles for level1 in original definition
    Box::leak(arr.into_boxed_slice())
}

pub fn level1() -> &'static LevelDesc {
    static LD: OnceLock<LevelDesc> = OnceLock::new();
    static TILES: OnceLock<&'static [TileDesc]> = OnceLock::new();
    let tiles = TILES.get_or_init(|| build_level1_tiles());
    LD.get_or_init(|| LevelDesc {
        name: "Opening Board",
        width: 3,
        height: 9,
        bpm: 120.0,
        tiles,
        spawn_points: &[(0, 0), (1, 0), (2, 0)],
        goal_region: &[(1, 8)],
    })
}

