use crate::{
    config::{HpaPathfindingConfig, NeighborhoodMode, PathQueryMode},
    coord::{GridCoord, WorldRoundingPolicy},
    filters::PathFilterProfile,
    grid::{GridStorage, TransitionKind, TransitionLink},
    hierarchy::PathfindingSnapshot,
    search::{SlicedGridSearch, find_path, line_of_sight, nearest_walkable_cell},
};
use bevy::prelude::*;

fn build_snapshot(mut grid: GridStorage) -> PathfindingSnapshot {
    for x in 4..12 {
        grid.set_walkable(GridCoord::new(x, 7, 0), false);
    }
    grid.set_walkable(GridCoord::new(8, 7, 0), true);
    PathfindingSnapshot::build(
        grid,
        HpaPathfindingConfig {
            grid_dimensions: UVec3::new(16, 16, 1),
            cluster_size: UVec3::new(8, 8, 1),
            hierarchy_levels: 2,
            neighborhood: NeighborhoodMode::Ordinal2d,
            ..default()
        },
        3,
    )
}

#[test]
fn direct_a_star_finds_corridor_detour() {
    let grid = GridStorage::new(
        UVec3::new(16, 16, 1),
        Vec3::ZERO,
        1.0,
        WorldRoundingPolicy::Floor,
    );
    let snapshot = build_snapshot(grid);
    let path = find_path(
        &snapshot,
        GridCoord::new(2, 2, 0),
        GridCoord::new(13, 12, 0),
        &PathFilterProfile::default(),
        PathQueryMode::DirectOnly,
        false,
        &[],
    )
    .unwrap();

    assert_eq!(
        path.corridor.first().copied(),
        Some(GridCoord::new(2, 2, 0))
    );
    assert_eq!(
        path.corridor.last().copied(),
        Some(GridCoord::new(13, 12, 0))
    );
}

#[test]
fn same_cluster_fallback_returns_path() {
    let grid = GridStorage::new(
        UVec3::new(16, 16, 1),
        Vec3::ZERO,
        1.0,
        WorldRoundingPolicy::Floor,
    );
    let snapshot = build_snapshot(grid);

    let path = find_path(
        &snapshot,
        GridCoord::new(1, 1, 0),
        GridCoord::new(6, 5, 0),
        &PathFilterProfile::default(),
        PathQueryMode::Auto,
        false,
        &[],
    )
    .unwrap();

    assert!(!path.corridor.is_empty());
}

#[test]
fn hierarchical_path_crosses_clusters() {
    let grid = GridStorage::new(
        UVec3::new(16, 16, 1),
        Vec3::ZERO,
        1.0,
        WorldRoundingPolicy::Floor,
    );
    let snapshot = build_snapshot(grid);

    let path = find_path(
        &snapshot,
        GridCoord::new(1, 1, 0),
        GridCoord::new(14, 14, 0),
        &PathFilterProfile::default(),
        PathQueryMode::CoarseOnly,
        false,
        &[],
    )
    .unwrap();

    assert!(path.touched_clusters.len() >= 2);
}

#[test]
fn partial_path_returns_best_effort_when_goal_unreachable() {
    let mut grid = GridStorage::new(
        UVec3::new(8, 8, 1),
        Vec3::ZERO,
        1.0,
        WorldRoundingPolicy::Floor,
    );
    for y in 0..8 {
        grid.set_walkable(GridCoord::new(4, y, 0), false);
    }
    let snapshot = PathfindingSnapshot::build(
        grid,
        HpaPathfindingConfig {
            grid_dimensions: UVec3::new(8, 8, 1),
            cluster_size: UVec3::new(4, 4, 1),
            hierarchy_levels: 1,
            ..default()
        },
        1,
    );

    let path = find_path(
        &snapshot,
        GridCoord::new(1, 1, 0),
        GridCoord::new(7, 7, 0),
        &PathFilterProfile::default(),
        PathQueryMode::DirectOnly,
        true,
        &[],
    )
    .unwrap();

    assert!(path.is_partial);
    assert_ne!(path.corridor.last().copied(), Some(GridCoord::new(7, 7, 0)));
}

#[test]
fn no_path_returns_none_without_partial_mode() {
    let mut grid = GridStorage::new(
        UVec3::new(8, 8, 1),
        Vec3::ZERO,
        1.0,
        WorldRoundingPolicy::Floor,
    );
    for y in 0..8 {
        grid.set_walkable(GridCoord::new(4, y, 0), false);
    }
    let snapshot = PathfindingSnapshot::build(
        grid,
        HpaPathfindingConfig {
            grid_dimensions: UVec3::new(8, 8, 1),
            cluster_size: UVec3::new(4, 4, 1),
            hierarchy_levels: 1,
            ..default()
        },
        1,
    );

    assert!(
        find_path(
            &snapshot,
            GridCoord::new(1, 1, 0),
            GridCoord::new(7, 7, 0),
            &PathFilterProfile::default(),
            PathQueryMode::DirectOnly,
            false,
            &[],
        )
        .is_none()
    );
}

#[test]
fn nearest_walkable_and_los_utilities_work() {
    let mut grid = GridStorage::new(
        UVec3::new(8, 8, 1),
        Vec3::ZERO,
        1.0,
        WorldRoundingPolicy::Floor,
    );
    grid.set_walkable(GridCoord::new(3, 3, 0), false);
    let snapshot = PathfindingSnapshot::build(
        grid,
        HpaPathfindingConfig {
            grid_dimensions: UVec3::new(8, 8, 1),
            cluster_size: UVec3::new(4, 4, 1),
            hierarchy_levels: 1,
            ..default()
        },
        1,
    );

    let nearest = nearest_walkable_cell(
        &snapshot,
        GridCoord::new(3, 3, 0),
        &PathFilterProfile::default(),
    )
    .unwrap();
    assert_ne!(nearest, GridCoord::new(3, 3, 0));
    assert!(line_of_sight(
        &snapshot,
        GridCoord::new(0, 0, 0),
        GridCoord::new(2, 2, 0),
        &PathFilterProfile::default(),
        &[],
    ));
}

#[test]
fn layered_transition_path_is_supported() {
    let mut grid = GridStorage::new(
        UVec3::new(8, 8, 2),
        Vec3::ZERO,
        1.0,
        WorldRoundingPolicy::Floor,
    );
    grid.add_transition(
        GridCoord::new(2, 2, 0),
        TransitionLink::new(GridCoord::new(2, 2, 1), 2.0, TransitionKind::Stair),
    );
    let snapshot = PathfindingSnapshot::build(
        grid,
        HpaPathfindingConfig {
            grid_dimensions: UVec3::new(8, 8, 2),
            cluster_size: UVec3::new(4, 4, 1),
            hierarchy_levels: 1,
            neighborhood: NeighborhoodMode::Ordinal18,
            ..default()
        },
        1,
    );

    let path = find_path(
        &snapshot,
        GridCoord::new(1, 1, 0),
        GridCoord::new(3, 3, 1),
        &PathFilterProfile::default(),
        PathQueryMode::DirectOnly,
        false,
        &[],
    )
    .unwrap();

    assert_eq!(path.corridor.last().copied(), Some(GridCoord::new(3, 3, 1)));
}

#[test]
fn sliced_search_finishes_over_multiple_advances() {
    let grid = GridStorage::new(
        UVec3::new(16, 16, 1),
        Vec3::ZERO,
        1.0,
        WorldRoundingPolicy::Floor,
    );
    let snapshot = build_snapshot(grid);
    let mut sliced = SlicedGridSearch::new(
        snapshot,
        GridCoord::new(1, 1, 0),
        GridCoord::new(14, 14, 0),
        PathFilterProfile::default(),
        false,
        Vec::new(),
    )
    .unwrap();

    let mut result = None;
    for _ in 0..64 {
        if let Some(done) = sliced.advance(4) {
            result = done;
            break;
        }
    }

    assert!(result.is_some());
}

#[test]
fn filter_profiles_can_choose_a_cheaper_detour() {
    let mut grid = GridStorage::new(
        UVec3::new(8, 4, 1),
        Vec3::ZERO,
        1.0,
        WorldRoundingPolicy::Floor,
    );
    for x in 1..7 {
        let mut cell = grid
            .cell(GridCoord::new(x, 1, 0))
            .cloned()
            .unwrap_or_default();
        cell.area = crate::filters::AreaTypeId(3);
        grid.set_cell(GridCoord::new(x, 1, 0), cell);
    }

    let snapshot = PathfindingSnapshot::build(
        grid,
        HpaPathfindingConfig {
            grid_dimensions: UVec3::new(8, 4, 1),
            cluster_size: UVec3::new(4, 4, 1),
            hierarchy_levels: 1,
            neighborhood: NeighborhoodMode::Cardinal2d,
            ..default()
        },
        1,
    );

    let direct_bias =
        PathFilterProfile::default().with_area_cost(crate::filters::AreaTypeId(3), 1.1);
    let detour_bias =
        PathFilterProfile::default().with_area_cost(crate::filters::AreaTypeId(3), 8.0);

    let direct_path = find_path(
        &snapshot,
        GridCoord::new(0, 1, 0),
        GridCoord::new(7, 1, 0),
        &direct_bias,
        PathQueryMode::DirectOnly,
        false,
        &[],
    )
    .unwrap();
    let detour_path = find_path(
        &snapshot,
        GridCoord::new(0, 1, 0),
        GridCoord::new(7, 1, 0),
        &detour_bias,
        PathQueryMode::DirectOnly,
        false,
        &[],
    )
    .unwrap();

    let direct_hot_cells = direct_path
        .corridor
        .iter()
        .filter(|coord| coord.y() == 1 && (1..7).contains(&coord.x()))
        .count();
    let detour_hot_cells = detour_path
        .corridor
        .iter()
        .filter(|coord| coord.y() == 1 && (1..7).contains(&coord.x()))
        .count();

    assert!(direct_hot_cells >= 5);
    assert!(detour_hot_cells < direct_hot_cells);
}
