use super::*;
use crate::{
    cache::PathCache,
    components::{ComputedPath, ObstacleShape, PathRequest, PathfindingAgent, PathfindingObstacle},
    config::{HpaPathfindingConfig, NeighborhoodMode},
    coord::{GridCoord, WorldRoundingPolicy},
    ecs_api::PathfindingGrid,
    filters::PathCostOverlay,
    grid::GridStorage,
};

fn overlay_test_app() -> App {
    let config = HpaPathfindingConfig {
        grid_dimensions: UVec3::new(8, 4, 1),
        neighborhood: NeighborhoodMode::Cardinal2d,
        cache_capacity: 3,
        cache_ttl_frames: 7,
        max_queries_per_frame: 4,
        ..default()
    };
    let mut grid = GridStorage::new(
        config.grid_dimensions,
        Vec3::ZERO,
        1.0,
        WorldRoundingPolicy::Floor,
    );
    for y in 0..4 {
        grid.set_walkable(GridCoord::new(4, y, 0), true);
    }
    for x in 1..8 {
        grid.set_walkable(GridCoord::new(x, 0, 0), false);
    }

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(config.clone());
    app.insert_resource(PathfindingGrid::new(grid, config));
    app.add_plugins(crate::HpaPathfindingPlugin::default());
    app
}

#[test]
fn ecs_queries_forward_overlays_and_apply_cache_limits() {
    let mut app = overlay_test_app();
    let overlay = PathCostOverlay::new(
        crate::coord::GridAabb::new(GridCoord::new(1, 1, 0), GridCoord::new(6, 1, 0)),
        8.0,
    );

    let plain = app
        .world_mut()
        .spawn((
            Name::new("Plain Agent"),
            Transform::from_translation(Vec3::new(0.5, 1.5, 0.0)),
            PathfindingAgent::default(),
            PathRequest::new(GridCoord::new(7, 1, 0)),
        ))
        .id();
    let overlay_agent = app
        .world_mut()
        .spawn((
            Name::new("Overlay Agent"),
            Transform::from_translation(Vec3::new(0.5, 1.5, 0.0)),
            PathfindingAgent::default(),
            PathRequest::new(GridCoord::new(7, 1, 0)).with_overlays(vec![overlay]),
        ))
        .id();

    for _ in 0..64 {
        app.update();
        if app.world().entity(plain).contains::<ComputedPath>()
            && app.world().entity(overlay_agent).contains::<ComputedPath>()
        {
            break;
        }
    }

    let plain_path = app.world().get::<ComputedPath>(plain).unwrap().clone();
    let overlay_path = app
        .world()
        .get::<ComputedPath>(overlay_agent)
        .unwrap()
        .clone();
    let plain_hot_cells = plain_path
        .corridor
        .iter()
        .filter(|coord| coord.y() == 1 && (1..7).contains(&coord.x()))
        .count();
    let overlay_hot_cells = overlay_path
        .corridor
        .iter()
        .filter(|coord| coord.y() == 1 && (1..7).contains(&coord.x()))
        .count();

    assert!(plain_hot_cells >= 5);
    assert!(overlay_hot_cells < plain_hot_cells);

    let cache = app.world().resource::<PathCache>();
    assert_eq!(cache.capacity, 3);
    assert_eq!(cache.ttl_frames, 7);
}

#[test]
fn obstacle_changes_invalidate_and_replan_paths() {
    let config = HpaPathfindingConfig {
        grid_dimensions: UVec3::new(10, 6, 1),
        neighborhood: NeighborhoodMode::Cardinal2d,
        max_queries_per_frame: 2,
        ..default()
    };
    let mut grid = GridStorage::new(
        config.grid_dimensions,
        Vec3::ZERO,
        1.0,
        WorldRoundingPolicy::Floor,
    );
    for x in 2..10 {
        if x != 4 && x != 8 {
            grid.set_walkable(GridCoord::new(x, 2, 0), false);
        }
    }

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(config.clone());
    app.insert_resource(PathfindingGrid::new(grid, config));
    app.add_plugins(crate::HpaPathfindingPlugin::default());

    let agent = app
        .world_mut()
        .spawn((
            Name::new("Gate Runner"),
            Transform::from_translation(Vec3::new(1.5, 1.5, 0.0)),
            PathfindingAgent::default(),
            PathRequest::new(GridCoord::new(6, 4, 0)),
        ))
        .id();

    for _ in 0..64 {
        app.update();
        if app.world().entity(agent).contains::<ComputedPath>() {
            break;
        }
    }

    let baseline = app.world().get::<ComputedPath>(agent).unwrap().clone();
    assert!(baseline.corridor.contains(&GridCoord::new(4, 2, 0)));

    app.world_mut().spawn(PathfindingObstacle {
        shape: ObstacleShape::Cell(GridCoord::new(4, 2, 0)),
        area_override: None,
    });

    for _ in 0..96 {
        app.update();
        if app
            .world()
            .get::<ComputedPath>(agent)
            .is_some_and(|path| path.path_version != baseline.path_version)
        {
            break;
        }
    }

    let replanned = app.world().get::<ComputedPath>(agent).unwrap();
    let stats = app.world().resource::<crate::stats::PathfindingStats>();
    assert_ne!(replanned.path_version, baseline.path_version);
    assert_ne!(replanned.corridor, baseline.corridor);
    assert!(!replanned.corridor.contains(&GridCoord::new(4, 2, 0)));
    assert!(stats.total_queries_invalidated > 0);
}

#[test]
fn ecs_queries_respect_agent_clearance_overrides() {
    let config = HpaPathfindingConfig {
        grid_dimensions: UVec3::new(10, 8, 1),
        neighborhood: NeighborhoodMode::Cardinal2d,
        max_queries_per_frame: 4,
        ..default()
    };
    let mut grid = GridStorage::new(
        config.grid_dimensions,
        Vec3::ZERO,
        1.0,
        WorldRoundingPolicy::Floor,
    );
    for y in 0..8 {
        grid.set_walkable(GridCoord::new(4, y, 0), false);
    }
    grid.set_walkable(GridCoord::new(4, 1, 0), true);
    grid.set_walkable(GridCoord::new(4, 5, 0), true);
    grid.set_walkable(GridCoord::new(4, 6, 0), true);

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(config.clone());
    app.insert_resource(PathfindingGrid::new(grid, config));
    app.add_plugins(crate::HpaPathfindingPlugin::default());

    let small_agent = app
        .world_mut()
        .spawn((
            Name::new("Small Agent"),
            Transform::from_translation(Vec3::new(1.5, 2.5, 0.0)),
            PathfindingAgent::default(),
            PathRequest::new(GridCoord::new(8, 2, 0)),
        ))
        .id();
    let large_agent = app
        .world_mut()
        .spawn((
            Name::new("Large Agent"),
            Transform::from_translation(Vec3::new(1.5, 2.5, 0.0)),
            PathfindingAgent {
                clearance: 2,
                ..default()
            },
            PathRequest::new(GridCoord::new(8, 2, 0)),
        ))
        .id();

    for _ in 0..96 {
        app.update();
        if app.world().entity(small_agent).contains::<ComputedPath>()
            && app.world().entity(large_agent).contains::<ComputedPath>()
        {
            break;
        }
    }

    let small_path = app.world().get::<ComputedPath>(small_agent).unwrap();
    let large_path = app.world().get::<ComputedPath>(large_agent).unwrap();

    assert!(small_path.corridor.contains(&GridCoord::new(4, 1, 0)));
    assert!(!large_path.corridor.contains(&GridCoord::new(4, 1, 0)));
    assert!(large_path.corridor.contains(&GridCoord::new(4, 5, 0)));
}
