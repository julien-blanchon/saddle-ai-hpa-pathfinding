use super::PathfindingGrid;
use crate::{
    config::{HpaPathfindingConfig, NeighborhoodMode, PathQueryMode},
    coord::{GridCoord, WorldRoundingPolicy},
    filters::PathFilterProfile,
    grid::GridStorage,
};
use bevy::prelude::*;

fn test_grid() -> PathfindingGrid {
    let config = HpaPathfindingConfig {
        grid_dimensions: UVec3::new(6, 6, 1),
        cluster_size: UVec3::new(3, 3, 1),
        neighborhood: NeighborhoodMode::Cardinal2d,
        ..default()
    };
    let grid = GridStorage::new(
        config.grid_dimensions,
        Vec3::ZERO,
        1.0,
        WorldRoundingPolicy::Floor,
    );
    PathfindingGrid::new(grid, config)
}

#[test]
fn clearance_override_extends_registered_profile() {
    let mut grid = test_grid();
    let filter = grid.register_filter(PathFilterProfile::named("large").with_clearance(1));

    let profile = grid.filter_with_clearance(filter, 3);

    assert_eq!(profile.clearance, 3);
}

#[test]
fn flow_field_and_queries_share_clearance_rules() {
    let mut grid = test_grid();
    grid.set_walkable(GridCoord::new(1, 1, 0), false);

    let default_path = grid.query_path(
        GridCoord::new(0, 0, 0),
        GridCoord::new(5, 5, 0),
        crate::filters::PathFilterId(0),
        PathQueryMode::DirectOnly,
        false,
        &[],
    );
    let wide_path = grid.query_path_with_clearance(
        GridCoord::new(0, 0, 0),
        GridCoord::new(5, 5, 0),
        crate::filters::PathFilterId(0),
        2,
        PathQueryMode::DirectOnly,
        false,
        &[],
    );
    let wide_flow = grid.build_flow_field_with_clearance(
        GridCoord::new(5, 5, 0),
        crate::filters::PathFilterId(0),
        2,
        &[],
    )
    .unwrap();

    assert!(default_path.is_some());
    assert!(wide_path.is_none());
    assert!(wide_flow.integration_cost(GridCoord::new(0, 0, 0)).is_none());
}
