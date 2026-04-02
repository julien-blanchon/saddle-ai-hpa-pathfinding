use super::{ObstacleRuntimeState, sync_obstacles};
use crate::{
    components::{ObstacleShape, PathfindingObstacle},
    coord::{GridCoord, WorldRoundingPolicy},
    ecs_api::PathfindingGrid,
    filters::AreaTypeId,
    grid::GridStorage,
};
use bevy::prelude::*;

fn obstacle_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(PathfindingGrid::new(
        GridStorage::new(
            UVec3::new(6, 6, 1),
            Vec3::ZERO,
            1.0,
            WorldRoundingPolicy::Floor,
        ),
        crate::config::HpaPathfindingConfig {
            grid_dimensions: UVec3::new(6, 6, 1),
            ..default()
        },
    ));
    app.insert_resource(ObstacleRuntimeState::default());
    app.add_systems(Update, sync_obstacles);
    app
}

#[test]
fn obstacle_component_blocks_and_restores_cells() {
    let mut app = obstacle_app();
    let coord = GridCoord::new(2, 2, 0);
    let entity = app
        .world_mut()
        .spawn(PathfindingObstacle {
            shape: ObstacleShape::Cell(coord),
            area_override: None,
        })
        .id();

    app.update();
    assert!(
        !app.world()
            .resource::<PathfindingGrid>()
            .grid()
            .cell(coord)
            .unwrap()
            .walkable
    );

    app.world_mut().despawn(entity);
    app.update();
    assert!(
        app.world()
            .resource::<PathfindingGrid>()
            .grid()
            .cell(coord)
            .unwrap()
            .walkable
    );
}

#[test]
fn overlapping_obstacles_keep_cells_blocked_until_last_removal() {
    let mut app = obstacle_app();
    let coord = GridCoord::new(3, 3, 0);
    let blocker = app
        .world_mut()
        .spawn(PathfindingObstacle {
            shape: ObstacleShape::Cell(coord),
            area_override: None,
        })
        .id();
    let tagged = app
        .world_mut()
        .spawn(PathfindingObstacle {
            shape: ObstacleShape::Cell(coord),
            area_override: Some(AreaTypeId(7)),
        })
        .id();

    app.update();
    let cell = app
        .world()
        .resource::<PathfindingGrid>()
        .grid()
        .cell(coord)
        .unwrap()
        .clone();
    assert!(!cell.walkable);
    assert_eq!(cell.area, AreaTypeId(7));

    app.world_mut().despawn(blocker);
    app.update();
    assert!(
        !app.world()
            .resource::<PathfindingGrid>()
            .grid()
            .cell(coord)
            .unwrap()
            .walkable
    );

    app.world_mut().despawn(tagged);
    app.update();
    assert!(
        app.world()
            .resource::<PathfindingGrid>()
            .grid()
            .cell(coord)
            .unwrap()
            .walkable
    );
}
