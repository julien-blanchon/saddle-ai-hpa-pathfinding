use bevy::prelude::*;
use saddle_ai_hpa_pathfinding::{
    ComputedPath, GridCoord, HpaPathfindingPlugin, PathRequest, PathfindingAgent, PathfindingGrid,
};

fn main() {
    let mut app = App::new();
    let config = saddle_ai_hpa_pathfinding::HpaPathfindingConfig {
        grid_dimensions: UVec3::new(32, 24, 1),
        max_queries_per_frame: 4,
        ..Default::default()
    };
    let path_grid = PathfindingGrid::new(
        {
            let mut grid = saddle_ai_hpa_pathfinding::GridStorage::new(
                config.grid_dimensions,
                Vec3::ZERO,
                1.0,
                saddle_ai_hpa_pathfinding::WorldRoundingPolicy::Floor,
            );
            for x in 10..22 {
                grid.set_walkable(GridCoord::new(x, 12, 0), false);
            }
            grid.set_walkable(GridCoord::new(16, 12, 0), true);
            grid
        },
        config.clone(),
    );

    app.add_plugins(MinimalPlugins);
    app.insert_resource(config);
    app.insert_resource(path_grid);
    app.add_plugins(HpaPathfindingPlugin::default());
    let entity = app
        .world_mut()
        .spawn((
            Name::new("Async Query Agent"),
            Transform::from_xyz(2.0, 2.0, 0.0),
            GlobalTransform::from_xyz(2.0, 2.0, 0.0),
            PathfindingAgent::default(),
            PathRequest::new(GridCoord::new(28, 20, 0)),
        ))
        .id();

    for _ in 0..48 {
        app.update();
        if app.world().entity(entity).contains::<ComputedPath>() {
            break;
        }
    }

    let corridor_len = app
        .world()
        .get::<ComputedPath>(entity)
        .map(|path| path.corridor.len())
        .unwrap_or_default();
    println!("async query corridor={corridor_len}");
}
