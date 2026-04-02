use saddle_ai_hpa_pathfinding_example_support as support;

use bevy::prelude::*;
use saddle_ai_hpa_pathfinding::{GridCoord, PathFilterProfile, PathQueryMode, find_path};

fn main() {
    let config = support::demo_config(UVec3::new(128, 128, 1));
    let mut grid = saddle_ai_hpa_pathfinding::GridStorage::new(
        config.grid_dimensions,
        Vec3::ZERO,
        1.0,
        saddle_ai_hpa_pathfinding::WorldRoundingPolicy::Floor,
    );

    for y in 10..118 {
        if y % 19 != 0 {
            grid.set_walkable(GridCoord::new(48, y, 0), false);
            grid.set_walkable(GridCoord::new(86, y, 0), false);
        }
    }

    let snapshot = saddle_ai_hpa_pathfinding::PathfindingGrid::new(grid, config);
    for (start, goal) in [
        (GridCoord::new(2, 2, 0), GridCoord::new(124, 120, 0)),
        (GridCoord::new(8, 116, 0), GridCoord::new(116, 8, 0)),
        (GridCoord::new(24, 24, 0), GridCoord::new(100, 100, 0)),
    ] {
        let path = find_path(
            snapshot.snapshot(),
            start,
            goal,
            &PathFilterProfile::default(),
            PathQueryMode::Auto,
            false,
            &[],
        )
        .expect("large-grid path should exist");
        println!("large-grid {:?}->{:?}: {:.2}", start, goal, path.total_cost);
    }
}
