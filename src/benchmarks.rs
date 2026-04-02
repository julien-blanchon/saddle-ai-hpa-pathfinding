use crate::{
    config::{HpaPathfindingConfig, NeighborhoodMode},
    coord::{GridCoord, WorldRoundingPolicy},
    ecs_api::PathfindingGrid,
    grid::GridStorage,
    hierarchy::PathfindingSnapshot,
};
use bevy::prelude::*;

pub(crate) fn large_snapshot(dimensions: UVec3) -> PathfindingSnapshot {
    let mut grid = GridStorage::new(dimensions, Vec3::ZERO, 1.0, WorldRoundingPolicy::Floor);
    for y in 8..dimensions.y.saturating_sub(8) as i32 {
        if y % 11 != 0 {
            grid.set_walkable(GridCoord::new((dimensions.x / 3) as i32, y, 0), false);
            grid.set_walkable(GridCoord::new(((dimensions.x * 2) / 3) as i32, y, 0), false);
        }
    }

    PathfindingSnapshot::build(
        grid,
        HpaPathfindingConfig {
            grid_dimensions: dimensions,
            cluster_size: UVec3::new(16, 16, 1),
            hierarchy_levels: 2,
            neighborhood: NeighborhoodMode::Ordinal2d,
            ..default()
        },
        1,
    )
}

pub(crate) fn dynamic_grid(dimensions: UVec3) -> PathfindingGrid {
    let config = HpaPathfindingConfig {
        grid_dimensions: dimensions,
        cluster_size: UVec3::new(16, 16, 1),
        hierarchy_levels: 2,
        neighborhood: NeighborhoodMode::Ordinal2d,
        ..default()
    };
    PathfindingGrid::new(
        GridStorage::new(dimensions, Vec3::ZERO, 1.0, WorldRoundingPolicy::Floor),
        config,
    )
}

#[cfg(test)]
#[path = "benchmarks_tests.rs"]
mod tests;
