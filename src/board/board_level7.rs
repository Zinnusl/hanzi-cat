use super::{LevelDesc, TileDesc, ObstacleKind, ModifierKind};

// Larger hanzi set for level 7 (boss level)
pub static LEVEL7_HANZI: &[(&str, &str)] = &[
    ("老", "lao3"),
    ("师", "shi1"),
    ("朋", "peng2"),
    ("友", "you3"),
    ("电", "dian4"),
    ("脑", "nao3"),
    ("手", "shou3"),
    ("机", "ji1"),
    ("语", "yu3"),
    ("言", "yan2"),
    ("食", "shi2"),
    ("物", "wu4"),
];

pub fn level7() -> &'static LevelDesc {
    let width: u8 = 10;
    let height: u8 = 8;
    let bpm = 150.0;
    // Build tiles with a few obstacles and modifiers
    let mut tiles_vec = vec![TileDesc::default(); (width as usize) * (height as usize)];
    // Place some blocks forming narrow corridors
    let block_positions = [
        (2u8, 2u8),
        (2u8, 3u8),
        (2u8, 4u8),
        (7u8, 2u8),
        (7u8, 3u8),
        (7u8, 4u8),
    ];
    for (x, y) in block_positions.iter() {
        let idx = *y as usize * width as usize + *x as usize;
        tiles_vec[idx].obstacle = Some(ObstacleKind::Block);
    }
    // Add a conveyor on row 1 pushing right
    tiles_vec[width as usize + 3].obstacle = Some(ObstacleKind::Conveyor { dx: 1, dy: 0 });
    tiles_vec[width as usize + 4].obstacle = Some(ObstacleKind::Conveyor { dx: 1, dy: 0 });
    tiles_vec[width as usize + 5].obstacle = Some(ObstacleKind::Conveyor { dx: 1, dy: 0 });

    // Add a tempo shift tile near center
    tiles_vec[4 * width as usize + 4].obstacle = Some(ObstacleKind::TempoShift { mult: 1.5, beats: 6 });

    // Modifier: score multiplier tile near goal
    tiles_vec[6 * width as usize + 6].modifier = Some(ModifierKind::ScoreMult { factor: 2.0, beats: 6 });

    let tiles: &'static [TileDesc] = Box::leak(tiles_vec.into_boxed_slice());
    let spawn_points: &'static [(u8, u8)] = Box::leak(vec![(0u8, 0u8), (9u8, 0u8), (5u8, 0u8)].into_boxed_slice());
    let goal_region: &'static [(u8, u8)] = Box::leak(vec![(4u8, 7u8), (5u8, 7u8), (6u8, 7u8)].into_boxed_slice());

    Box::leak(Box::new(LevelDesc {
        name: "Neon Bastion",
        width,
        height,
        bpm,
        tiles,
        spawn_points,
        goal_region,
    }))
}

