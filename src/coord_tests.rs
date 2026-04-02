use super::{GridAabb, GridCoord, GridSpace, WorldRoundingPolicy};
use bevy::prelude::*;

#[test]
fn grid_aabb_contains_and_intersects() {
    let a = GridAabb::new(GridCoord::new(0, 0, 0), GridCoord::new(3, 3, 0));
    let b = GridAabb::new(GridCoord::new(2, 2, 0), GridCoord::new(5, 5, 0));
    let c = GridAabb::new(GridCoord::new(4, 4, 0), GridCoord::new(6, 6, 0));

    assert!(a.contains(GridCoord::new(1, 1, 0)));
    assert!(!a.contains(GridCoord::new(4, 4, 0)));
    assert!(a.intersects(b));
    assert!(!a.intersects(c));
}

#[test]
fn grid_space_round_trip_uses_cell_centers() {
    let space = GridSpace {
        origin: Vec3::new(-4.0, 2.0, 0.0),
        cell_size: 2.0,
        rounding: WorldRoundingPolicy::Floor,
    };
    let coord = GridCoord::new(3, 1, 0);
    let world = space.to_world_center(coord);

    assert_eq!(space.to_grid(world), coord);
}

#[test]
fn grid_space_rounding_modes_are_explicit() {
    let floor_space = GridSpace {
        origin: Vec3::ZERO,
        cell_size: 1.0,
        rounding: WorldRoundingPolicy::Floor,
    };
    let round_space = GridSpace {
        rounding: WorldRoundingPolicy::Round,
        ..floor_space
    };
    let ceil_space = GridSpace {
        rounding: WorldRoundingPolicy::Ceil,
        ..floor_space
    };

    let sample = Vec3::new(1.49, 2.51, 0.1);
    assert_eq!(floor_space.to_grid(sample), GridCoord::new(1, 2, 0));
    assert_eq!(round_space.to_grid(sample), GridCoord::new(1, 3, 0));
    assert_eq!(ceil_space.to_grid(sample), GridCoord::new(2, 3, 1));
}
