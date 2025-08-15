use super::{LevelDesc, TileDesc};

// Small hanzi set for level 6 (exported by parent module)
pub static LEVEL6_HANZI: &[(&str, &str)] = &[
    ("中", "zhong1"),
    ("国", "guo2"),
    ("学", "xue2"),
    ("校", "xiao4"),
    ("天", "tian1"),
    ("气", "qi4"),
    ("火", "huo3"),
    ("水", "shui3"),
    ("山", "shan1"),
    ("海", "hai3"),
];

pub fn level6() -> &'static LevelDesc {
    let width: u8 = 8;
    let height: u8 = 8;
    let bpm = 138.0;
    // uniform empty tiles
    let tiles_vec = vec![TileDesc::default(); (width as usize) * (height as usize)];
    let tiles: &'static [TileDesc] = Box::leak(tiles_vec.into_boxed_slice());
    let spawn_points: &'static [(u8, u8)] = Box::leak(vec![(3u8, 0u8), (4u8, 0u8), (0u8, 3u8)].into_boxed_slice());
    let goal_region: &'static [(u8, u8)] = Box::leak(vec![(3u8, 7u8), (4u8, 7u8)].into_boxed_slice());

    Box::leak(Box::new(LevelDesc {
        name: "Crystal Isle",
        width,
        height,
        bpm,
        tiles,
        spawn_points,
        goal_region,
    }))
}

