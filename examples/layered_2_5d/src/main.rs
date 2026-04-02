use saddle_ai_hpa_pathfinding_example_support as support;

use saddle_ai_hpa_pathfinding::{GridCoord, PathFilterProfile, PathQueryMode, find_path};

fn main() {
    let path_grid = support::build_layered_grid();
    let path = find_path(
        path_grid.snapshot(),
        GridCoord::new(2, 2, 0),
        GridCoord::new(12, 12, 1),
        &PathFilterProfile::default(),
        PathQueryMode::Auto,
        false,
        &[],
    )
    .expect("layered path should exist");

    println!(
        "layered path end={:?} touched_clusters={}",
        path.corridor.last(),
        path.touched_clusters.len()
    );
}
