use saddle_ai_hpa_pathfinding_example_support as support;

use bevy::prelude::*;
use saddle_ai_hpa_pathfinding::{GridCoord, PathFilterProfile, PathQueryMode, find_path};

fn main() {
    let config = support::demo_config(UVec3::new(32, 24, 1));
    let path_grid = saddle_ai_hpa_pathfinding::PathfindingGrid::new(
        support::build_demo_grid(config.grid_dimensions),
        config,
    );
    let path = find_path(
        path_grid.snapshot(),
        GridCoord::new(2, 2, 0),
        GridCoord::new(28, 21, 0),
        &PathFilterProfile::default(),
        PathQueryMode::Auto,
        false,
        &[],
    )
    .expect("path should exist");

    println!(
        "basic path corridor={} waypoints={} cost={:.2}",
        path.corridor.len(),
        path.waypoints.len(),
        path.total_cost
    );
}
