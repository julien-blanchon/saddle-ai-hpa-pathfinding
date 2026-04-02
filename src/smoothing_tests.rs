use super::smooth_corridor;
use crate::{
    config::PathSmoothingMode,
    coord::{GridCoord, WorldRoundingPolicy},
    filters::PathFilterProfile,
    grid::GridStorage,
};
use bevy::prelude::*;

fn open_grid() -> GridStorage {
    GridStorage::new(
        UVec3::new(8, 8, 1),
        Vec3::ZERO,
        1.0,
        WorldRoundingPolicy::Floor,
    )
}

#[test]
fn line_of_sight_smoothing_removes_redundant_bends() {
    let grid = open_grid();
    let corridor = vec![
        GridCoord::new(0, 0, 0),
        GridCoord::new(1, 0, 0),
        GridCoord::new(2, 1, 0),
        GridCoord::new(3, 2, 0),
        GridCoord::new(4, 3, 0),
    ];

    let smoothed = smooth_corridor(
        &grid,
        &corridor,
        &PathFilterProfile::default(),
        &[],
        PathSmoothingMode::LineOfSight,
    );

    assert_eq!(smoothed.len(), 2);
}

#[test]
fn smoothing_does_not_cut_through_blocked_cells() {
    let mut grid = open_grid();
    grid.set_walkable(GridCoord::new(1, 1, 0), false);
    let corridor = vec![
        GridCoord::new(0, 0, 0),
        GridCoord::new(0, 1, 0),
        GridCoord::new(1, 2, 0),
        GridCoord::new(2, 3, 0),
        GridCoord::new(3, 3, 0),
    ];

    let smoothed = smooth_corridor(
        &grid,
        &corridor,
        &PathFilterProfile::default(),
        &[],
        PathSmoothingMode::LineOfSight,
    );

    assert!(smoothed.len() > 2);
}
