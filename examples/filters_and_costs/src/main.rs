use saddle_ai_hpa_pathfinding_example_support as support;

use saddle_ai_hpa_pathfinding::{GridAabb, GridCoord, PathCostOverlay, PathQueryMode, find_path};

fn main() {
    let path_grid = support::build_filter_grid();
    let wheeled = path_grid.filter(saddle_ai_hpa_pathfinding::PathFilterId(1));
    let utility = path_grid.filter(saddle_ai_hpa_pathfinding::PathFilterId(2));
    let overlay = PathCostOverlay::new(
        GridAabb::new(GridCoord::new(8, 8, 0), GridCoord::new(14, 12, 0)),
        5.0,
    );

    let wheeled_path = find_path(
        path_grid.snapshot(),
        GridCoord::new(2, 2, 0),
        GridCoord::new(20, 16, 0),
        &wheeled,
        PathQueryMode::Auto,
        false,
        &[],
    )
    .expect("wheeled path should exist");
    let utility_path = find_path(
        path_grid.snapshot(),
        GridCoord::new(2, 2, 0),
        GridCoord::new(20, 16, 0),
        &utility,
        PathQueryMode::Auto,
        false,
        &[],
    )
    .expect("utility path should exist");
    let utility_overlay_path = find_path(
        path_grid.snapshot(),
        GridCoord::new(2, 2, 0),
        GridCoord::new(20, 16, 0),
        &utility,
        PathQueryMode::Auto,
        false,
        &[overlay],
    )
    .expect("overlay-biased utility path should exist");

    println!(
        "filters wheeled_cost={:.2} utility_cost={:.2} utility_overlay_cost={:.2}",
        wheeled_path.total_cost, utility_path.total_cost, utility_overlay_path.total_cost
    );
}
