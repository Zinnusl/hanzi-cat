// Board Level 4 definition
// This file contains LEVEL4_TILES, LEVEL4 static LevelDesc, and LEVEL4_HANZI.
use super::{TileDesc, LevelDesc, ObstacleKind, ModifierKind};
use std::sync::OnceLock;

pub static LEVEL4_HANZI: [(&str, &str); 6] = [
    ("爸爸", "baba4"),
    ("妈妈", "mama1"),
    ("老师", "laoshi1"),
    ("学生", "xuesheng2"),
    ("朋友", "pengyou2"),
    ("同学", "tongxue2"),
];

fn build_level4_tiles() -> &'static [TileDesc] {
    use ObstacleKind::*;
    let mut arr: Vec<TileDesc> = vec![TileDesc { obstacle: None, modifier: None }; 63];
    for y in 0..9 {
        for x in 0..7 {
            let path_x = if y % 2 == 0 { x == (y % 7) } else { x == 6 - (y % 7) };
            if !path_x {
                arr[7 * y + x] = TileDesc {
                    obstacle: Some(Block),
                    modifier: None,
                };
            }
        }
    }
    for y in (0..9).step_by(2) {
        for x in 0..7 {
            if matches!(arr[7 * y + x].obstacle, None) {
                arr[7 * y + x].obstacle = Some(Conveyor { dx: 1, dy: 0 });
            }
        }
    }
    arr[7 * 4 + 3] = TileDesc {
        obstacle: Some(TempoShift { mult: 1.5, beats: 3 }),
        modifier: None,
    };
    arr[7 * 5 + 1] = TileDesc {
        obstacle: None,
        modifier: Some(ModifierKind::ScoreMult { factor: 2.0, beats: 4 }),
    };
    arr[7 * 7 + 6] = TileDesc {
        obstacle: Some(Teleport { to: (0, 1) }),
        modifier: None,
    };
    Box::leak(arr.into_boxed_slice())
}

pub fn level4() -> &'static LevelDesc {
    static LD: OnceLock<LevelDesc> = OnceLock::new();
    static TILES: OnceLock<&'static [TileDesc]> = OnceLock::new();
    let tiles = TILES.get_or_init(|| build_level4_tiles());
    LD.get_or_init(|| LevelDesc {
        name: "Zigzag Express",
        width: 7,
        height: 9,
        bpm: 128.0,
        tiles,
        spawn_points: &[(0, 0), (0, 1), (0, 2), (0, 3), (0, 4), (0, 5), (0, 6), (0, 7), (0, 8)],
        goal_region: &[(6, 8)],
    })
}

