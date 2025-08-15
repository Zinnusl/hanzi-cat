// Board Level 3 definition
// This file contains LEVEL3_TILES, LEVEL3 static LevelDesc, and LEVEL3_HANZI.
use super::{TileDesc, LevelDesc, ObstacleKind, ModifierKind};

pub const LEVEL3_TILES: [TileDesc; 81] = [
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

pub static LEVEL3_HANZI: [(&str, &str); 10] = [
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

pub static LEVEL3: LevelDesc = LevelDesc {
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

