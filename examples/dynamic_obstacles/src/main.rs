use saddle_ai_hpa_pathfinding_example_support as support;

use saddle_ai_hpa_pathfinding::{GridCoord, PathFilterProfile, PathQueryMode, find_path};

fn main() {
    let config = support::demo_config(bevy::prelude::UVec3::new(32, 24, 1));
    let mut path_grid = saddle_ai_hpa_pathfinding::PathfindingGrid::new(
        support::build_demo_grid(config.grid_dimensions),
        config.clone(),
    );

    let before = find_path(
        path_grid.snapshot(),
        GridCoord::new(2, 2, 0),
        GridCoord::new(28, 21, 0),
        &PathFilterProfile::default(),
        PathQueryMode::Auto,
        false,
        &[],
    )
    .expect("initial path should exist");

    path_grid.set_cell(GridCoord::new(16, 12, 0), support::make_blocked_cell());
    path_grid.rebuild_budgeted(&config, 8);

    let after = find_path(
        path_grid.snapshot(),
        GridCoord::new(2, 2, 0),
        GridCoord::new(28, 21, 0),
        &PathFilterProfile::default(),
        PathQueryMode::Auto,
        false,
        &[],
    )
    .expect("replanned path should exist");

    println!(
        "dynamic obstacle path cost before={:.2} after={:.2}",
        before.total_cost, after.total_cost
    );
}
