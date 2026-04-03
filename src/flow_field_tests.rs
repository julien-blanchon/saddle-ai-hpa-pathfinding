use super::build_flow_field;
use crate::{
    config::{HpaPathfindingConfig, NeighborhoodMode},
    coord::{GridCoord, WorldRoundingPolicy},
    filters::PathFilterProfile,
    grid::GridStorage,
    hierarchy::PathfindingSnapshot,
};
use bevy::prelude::*;

fn build_snapshot(grid: GridStorage, neighborhood: NeighborhoodMode) -> PathfindingSnapshot {
    PathfindingSnapshot::build(
        grid,
        HpaPathfindingConfig {
            grid_dimensions: UVec3::new(10, 6, 1),
            cluster_size: UVec3::new(5, 3, 1),
            hierarchy_levels: 1,
            neighborhood,
            ..default()
        },
        1,
    )
}

#[test]
fn flow_field_routes_through_the_available_gap() {
    let mut grid = GridStorage::new(
        UVec3::new(10, 6, 1),
        Vec3::ZERO,
        1.0,
        WorldRoundingPolicy::Floor,
    );
    for y in 0..6 {
        if y != 3 {
            grid.set_walkable(GridCoord::new(4, y, 0), false);
        }
    }

    let snapshot = build_snapshot(grid, NeighborhoodMode::Cardinal2d);
    let flow = build_flow_field(
        &snapshot,
        GridCoord::new(8, 3, 0),
        &PathFilterProfile::default(),
        &[],
    )
    .unwrap();

    let mut cursor = GridCoord::new(1, 1, 0);
    let mut visited = vec![cursor];
    for _ in 0..32 {
        if cursor == flow.goal {
            break;
        }
        cursor = flow.next_step(cursor).expect("reachable next step");
        visited.push(cursor);
    }

    assert_eq!(visited.last().copied(), Some(GridCoord::new(8, 3, 0)));
    assert!(visited.contains(&GridCoord::new(4, 3, 0)));
}

#[test]
fn flow_field_respects_clearance_constraints() {
    let mut grid = GridStorage::new(
        UVec3::new(10, 6, 1),
        Vec3::ZERO,
        1.0,
        WorldRoundingPolicy::Floor,
    );
    grid.set_walkable(GridCoord::new(1, 1, 0), false);

    let snapshot = build_snapshot(grid, NeighborhoodMode::Cardinal2d);
    let default_field = build_flow_field(
        &snapshot,
        GridCoord::new(8, 4, 0),
        &PathFilterProfile::default(),
        &[],
    )
    .unwrap();
    let wide_field = build_flow_field(
        &snapshot,
        GridCoord::new(8, 4, 0),
        &PathFilterProfile::default().with_clearance(2),
        &[],
    )
    .unwrap();

    assert!(
        default_field
            .integration_cost(GridCoord::new(0, 0, 0))
            .is_some()
    );
    assert!(
        wide_field
            .integration_cost(GridCoord::new(0, 0, 0))
            .is_none()
    );
    assert!(
        wide_field
            .integration_cost(GridCoord::new(2, 2, 0))
            .is_some()
    );
}
