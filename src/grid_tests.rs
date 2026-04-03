use super::{CellData, GridStorage, TransitionKind, TransitionLink};
use crate::{
    config::NeighborhoodMode,
    coord::{GridAabb, GridCoord, WorldRoundingPolicy},
    filters::{AreaMask, AreaTypeId, PathCostOverlay, PathFilterProfile},
};
use bevy::prelude::*;

fn test_grid() -> GridStorage {
    GridStorage::new(
        UVec3::new(8, 8, 2),
        Vec3::ZERO,
        1.0,
        WorldRoundingPolicy::Floor,
    )
}

#[test]
fn area_type_and_walkability_round_trip() {
    let mut grid = test_grid();
    let coord = GridCoord::new(2, 3, 0);
    grid.set_cell(
        coord,
        CellData {
            walkable: false,
            area: AreaTypeId(7),
            traversal_mask: AreaMask::from_bit(4),
            base_cost: 3.5,
            clearance: 2,
        },
    );
    let cell = grid.cell(coord).unwrap();
    assert!(!cell.walkable);
    assert_eq!(cell.area, AreaTypeId(7));
    assert_eq!(cell.traversal_mask, AreaMask::from_bit(4));
    assert_eq!(cell.base_cost, 3.5);
    assert_eq!(cell.clearance, 0);
}

#[test]
fn neighborhood_enumeration_respects_topology() {
    let grid = GridStorage::new(
        UVec3::new(8, 8, 3),
        Vec3::ZERO,
        1.0,
        WorldRoundingPolicy::Floor,
    );
    let profile = PathFilterProfile::default();
    let origin = GridCoord::new(3, 3, 1);

    assert_eq!(
        grid.neighbor_cells(origin, NeighborhoodMode::Cardinal2d, false, &profile, &[])
            .len(),
        4
    );
    assert_eq!(
        grid.neighbor_cells(origin, NeighborhoodMode::Ordinal2d, false, &profile, &[])
            .len(),
        8
    );
    assert_eq!(
        grid.neighbor_cells(origin, NeighborhoodMode::Cardinal3d, false, &profile, &[])
            .len(),
        6
    );
    assert_eq!(
        grid.neighbor_cells(origin, NeighborhoodMode::Ordinal18, false, &profile, &[])
            .len(),
        18
    );
    assert_eq!(
        grid.neighbor_cells(origin, NeighborhoodMode::Ordinal26, false, &profile, &[])
            .len(),
        26
    );
}

#[test]
fn corner_cutting_rule_is_explicit() {
    let mut grid = test_grid();
    let profile = PathFilterProfile::default();
    grid.set_walkable(GridCoord::new(2, 3, 0), false);
    grid.set_walkable(GridCoord::new(3, 2, 0), false);

    let without = grid.neighbor_cells(
        GridCoord::new(2, 2, 0),
        NeighborhoodMode::Ordinal2d,
        false,
        &profile,
        &[],
    );
    let with = grid.neighbor_cells(
        GridCoord::new(2, 2, 0),
        NeighborhoodMode::Ordinal2d,
        true,
        &profile,
        &[],
    );

    assert!(
        !without
            .iter()
            .any(|(coord, _)| *coord == GridCoord::new(3, 3, 0))
    );
    assert!(
        with.iter()
            .any(|(coord, _)| *coord == GridCoord::new(3, 3, 0))
    );
}

#[test]
fn layered_transition_is_validated() {
    let mut grid = GridStorage::new(
        UVec3::new(8, 8, 2),
        Vec3::ZERO,
        1.0,
        WorldRoundingPolicy::Floor,
    );
    let profile = PathFilterProfile::default();
    let from = GridCoord::new(1, 1, 0);
    let to = GridCoord::new(1, 1, 1);
    grid.add_transition(from, TransitionLink::new(to, 2.0, TransitionKind::Stair));

    let neighbors = grid.neighbor_cells(from, NeighborhoodMode::Ordinal2d, false, &profile, &[]);
    assert!(
        neighbors
            .iter()
            .any(|(coord, cost)| *coord == to && (*cost - 2.0).abs() < 0.001)
    );
}

#[test]
fn dirty_region_overlap_inputs_are_grid_aligned() {
    let grid = test_grid();
    let outer = grid.bounds();
    let inner = GridAabb::new(GridCoord::new(2, 2, 0), GridCoord::new(4, 4, 0));
    assert!(inner.clamp_to(outer).unwrap().intersects(inner));
}

#[test]
fn overlays_bias_costs() {
    let mut grid = test_grid();
    let coord = GridCoord::new(3, 3, 0);
    grid.set_cell(
        coord,
        CellData {
            area: AreaTypeId(2),
            ..default()
        },
    );
    let profile = PathFilterProfile::default().with_area_cost(AreaTypeId(2), 2.0);
    let overlay = PathCostOverlay::new(GridAabb::new(coord, coord), 3.0);
    let cost = grid
        .traversal_cost(GridCoord::new(2, 3, 0), coord, &profile, &[overlay])
        .unwrap();
    assert!((cost - 5.0).abs() < 0.001);
}

#[test]
fn clearance_is_recomputed_for_walkable_regions() {
    let grid = GridStorage::new(
        UVec3::new(4, 4, 1),
        Vec3::ZERO,
        1.0,
        WorldRoundingPolicy::Floor,
    );

    assert_eq!(grid.cell(GridCoord::new(0, 0, 0)).unwrap().clearance, 4);
    assert_eq!(grid.cell(GridCoord::new(2, 2, 0)).unwrap().clearance, 2);
    assert_eq!(grid.cell(GridCoord::new(3, 3, 0)).unwrap().clearance, 1);
}

#[test]
fn clearance_updates_after_obstacle_changes() {
    let mut grid = GridStorage::new(
        UVec3::new(4, 4, 1),
        Vec3::ZERO,
        1.0,
        WorldRoundingPolicy::Floor,
    );

    grid.set_walkable(GridCoord::new(1, 1, 0), false);

    assert_eq!(grid.cell(GridCoord::new(0, 0, 0)).unwrap().clearance, 1);
    assert_eq!(grid.cell(GridCoord::new(1, 1, 0)).unwrap().clearance, 0);
    assert_eq!(grid.cell(GridCoord::new(2, 1, 0)).unwrap().clearance, 2);
}
