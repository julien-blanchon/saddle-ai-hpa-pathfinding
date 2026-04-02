use super::{dynamic_grid, large_snapshot};
use crate::{
    config::PathQueryMode, coord::GridCoord, filters::PathFilterProfile, search::find_path,
};
use bevy::platform::time::Instant;

#[test]
#[ignore = "benchmark-style validation; run with --ignored in release mode"]
fn direct_vs_hierarchy_reports_crossover_timings() {
    let snapshot = large_snapshot(bevy::prelude::UVec3::new(128, 128, 1));
    let profile = PathFilterProfile::default();
    let queries = [
        (GridCoord::new(2, 2, 0), GridCoord::new(24, 24, 0)),
        (GridCoord::new(4, 100, 0), GridCoord::new(118, 8, 0)),
        (GridCoord::new(8, 8, 0), GridCoord::new(120, 116, 0)),
        (GridCoord::new(20, 110, 0), GridCoord::new(110, 20, 0)),
    ];

    let started = Instant::now();
    for (start, goal) in queries {
        assert!(
            find_path(
                &snapshot,
                start,
                goal,
                &profile,
                PathQueryMode::DirectOnly,
                false,
                &[],
            )
            .is_some()
        );
    }
    let direct_elapsed = started.elapsed();

    let started = Instant::now();
    for (start, goal) in queries {
        assert!(
            find_path(
                &snapshot,
                start,
                goal,
                &profile,
                PathQueryMode::Auto,
                false,
                &[],
            )
            .is_some()
        );
    }
    let hierarchical_elapsed = started.elapsed();

    println!(
        "direct={:?} auto={:?} on {} benchmark queries",
        direct_elapsed,
        hierarchical_elapsed,
        queries.len()
    );
}

#[test]
#[ignore = "benchmark-style validation; run with --ignored in release mode"]
fn budgeted_rebuild_reports_cluster_cost() {
    let mut grid = dynamic_grid(bevy::prelude::UVec3::new(128, 128, 1));
    let config = grid.snapshot.config.clone();

    for x in 24..104 {
        grid.set_walkable(GridCoord::new(x, 64, 0), false);
    }

    let started = Instant::now();
    let first = grid.rebuild_budgeted(&config, 4);
    let elapsed_first = started.elapsed();

    let started = Instant::now();
    while !grid.pending_dirty_clusters.is_empty() {
        grid.rebuild_budgeted(&config, 4);
    }
    let elapsed_all = started.elapsed();

    assert!(!first.is_empty());
    println!(
        "first_budget={:?} remaining_total={:?} rebuilt_first_batch={}",
        elapsed_first,
        elapsed_all,
        first.len()
    );
}
