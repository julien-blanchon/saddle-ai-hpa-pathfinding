use super::{ClusterKey, EdgeKind, PathfindingSnapshot, cluster_size_for_level};
use crate::{
    config::{HpaPathfindingConfig, NeighborhoodMode},
    coord::GridCoord,
    grid::{GridStorage, TransitionKind, TransitionLink},
};
use bevy::prelude::*;

fn grid_2d() -> GridStorage {
    GridStorage::new(
        UVec3::new(16, 16, 1),
        Vec3::ZERO,
        1.0,
        crate::coord::WorldRoundingPolicy::Floor,
    )
}

fn count_portals(snapshot: &PathfindingSnapshot, left: ClusterKey, right: ClusterKey) -> usize {
    snapshot
        .edges
        .iter()
        .enumerate()
        .flat_map(|(node_id, edges)| edges.iter().map(move |edge| (node_id, edge)))
        .filter(|(node_id, edge)| {
            edge.kind == EdgeKind::InterCluster
                && *node_id < edge.to
                && matches!(
                    (
                        snapshot.cluster_key_for_coord(1, snapshot.nodes[*node_id].coord),
                        snapshot.cluster_key_for_coord(1, snapshot.nodes[edge.to].coord),
                    ),
                    (a, b) if (a == left && b == right) || (a == right && b == left)
                )
        })
        .count()
}

#[test]
fn cluster_partitioning_covers_map_edges() {
    let snapshot = PathfindingSnapshot::build(
        grid_2d(),
        HpaPathfindingConfig {
            grid_dimensions: UVec3::new(16, 16, 1),
            cluster_size: UVec3::new(6, 6, 1),
            hierarchy_levels: 1,
            neighborhood: NeighborhoodMode::Ordinal2d,
            ..default()
        },
        1,
    );
    let level = snapshot.level(1).unwrap();
    assert!(
        level
            .clusters
            .contains_key(&ClusterKey::new(1, GridCoord::new(2, 2, 0)))
    );
}

#[test]
fn entrance_detection_creates_inter_cluster_edges() {
    let snapshot = PathfindingSnapshot::build(
        grid_2d(),
        HpaPathfindingConfig {
            grid_dimensions: UVec3::new(16, 16, 1),
            cluster_size: UVec3::new(8, 8, 1),
            hierarchy_levels: 1,
            ..default()
        },
        1,
    );

    assert!(
        snapshot
            .edges
            .iter()
            .flatten()
            .any(|edge| edge.kind == EdgeKind::InterCluster)
    );
}

#[test]
fn hierarchy_builds_super_clusters() {
    let snapshot = PathfindingSnapshot::build(
        grid_2d(),
        HpaPathfindingConfig {
            grid_dimensions: UVec3::new(32, 32, 1),
            cluster_size: UVec3::new(8, 8, 1),
            hierarchy_levels: 2,
            ..default()
        },
        1,
    );
    assert_eq!(
        cluster_size_for_level(UVec3::new(8, 8, 1), 2),
        UVec3::new(16, 16, 2)
    );
    assert!(snapshot.level(2).is_some());
    assert_eq!(
        snapshot.level(2).unwrap().cluster_size,
        UVec3::new(16, 16, 2)
    );
}

#[test]
fn layered_portal_nodes_exist() {
    let mut grid = GridStorage::new(
        UVec3::new(8, 8, 2),
        Vec3::ZERO,
        1.0,
        crate::coord::WorldRoundingPolicy::Floor,
    );
    grid.add_transition(
        GridCoord::new(3, 3, 0),
        TransitionLink::new(GridCoord::new(3, 3, 1), 2.0, TransitionKind::Stair),
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

    assert!(snapshot.edges.iter().flatten().any(|edge| edge.cost >= 2.0));
}

#[test]
fn long_full_border_merges_down_to_endpoint_portals() {
    let snapshot = PathfindingSnapshot::build(
        GridStorage::new(
            UVec3::new(16, 8, 1),
            Vec3::ZERO,
            1.0,
            crate::coord::WorldRoundingPolicy::Floor,
        ),
        HpaPathfindingConfig {
            grid_dimensions: UVec3::new(16, 8, 1),
            cluster_size: UVec3::new(8, 8, 1),
            hierarchy_levels: 1,
            ..default()
        },
        1,
    );

    assert_eq!(
        count_portals(
            &snapshot,
            ClusterKey::new(1, GridCoord::new(0, 0, 0)),
            ClusterKey::new(1, GridCoord::new(1, 0, 0)),
        ),
        2
    );
}

#[test]
fn border_gaps_split_portal_groups() {
    let mut grid = GridStorage::new(
        UVec3::new(16, 8, 1),
        Vec3::ZERO,
        1.0,
        crate::coord::WorldRoundingPolicy::Floor,
    );
    grid.set_walkable(GridCoord::new(7, 3, 0), false);
    grid.set_walkable(GridCoord::new(7, 4, 0), false);

    let snapshot = PathfindingSnapshot::build(
        grid,
        HpaPathfindingConfig {
            grid_dimensions: UVec3::new(16, 8, 1),
            cluster_size: UVec3::new(8, 8, 1),
            hierarchy_levels: 1,
            ..default()
        },
        1,
    );

    assert_eq!(
        count_portals(
            &snapshot,
            ClusterKey::new(1, GridCoord::new(0, 0, 0)),
            ClusterKey::new(1, GridCoord::new(1, 0, 0)),
        ),
        4
    );
}
