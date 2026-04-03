use saddle_ai_hpa_pathfinding_example_support as support;

use bevy::prelude::*;
use saddle_ai_hpa_pathfinding::{
    ComputedPath, GridCoord, HpaPathfindingPlugin, PathRequest, PathfindingAgent, PathfindingGrid,
};

#[derive(Component)]
struct LongRouteAgent;

#[derive(Component)]
struct CrossMapAgent;

#[derive(Component)]
struct DiagonalAgent;

fn main() {
    let config = support::demo_config(UVec3::new(128, 128, 1));
    let mut storage = saddle_ai_hpa_pathfinding::GridStorage::new(
        config.grid_dimensions,
        config.origin,
        config.cell_size,
        saddle_ai_hpa_pathfinding::WorldRoundingPolicy::Floor,
    );

    for y in 10..118 {
        if y % 19 != 0 {
            storage.set_walkable(GridCoord::new(48, y, 0), false);
            storage.set_walkable(GridCoord::new(86, y, 0), false);
        }
    }

    let grid = PathfindingGrid::new(storage, config.clone());
    let mut app = App::new();
    app.insert_resource(config);
    app.insert_resource(grid);
    app.insert_resource(support::HpaExamplePane {
        goal_x: 124,
        goal_y: 120,
        draw_grid: false,
        draw_clusters: true,
        draw_portals: false,
        ..default()
    });
    support::configure_visual_app(&mut app, "hpa pathfinding: large grid");
    app.add_plugins(HpaPathfindingPlugin::default());
    app.add_systems(Startup, setup);
    app.add_systems(Update, support::sync_config_from_pane);
    app.add_systems(Update, sync_monitors);
    app.run();
}

fn setup(mut commands: Commands, grid: Res<PathfindingGrid>) {
    support::spawn_grid_camera(&mut commands);
    support::spawn_demo_backdrop(
        &mut commands,
        grid.as_ref(),
        support::ExampleLayout::Single,
        "Large Grid Stress Route",
        "Three long-haul requests share the same 128x128 hierarchy. This is the visual stress example for long-distance routing and portal reuse.",
    );
    support::spawn_grid_tiles(
        &mut commands,
        grid.as_ref(),
        support::ExampleLayout::Single,
        None,
    );

    let long_route = support::spawn_agent_sprite(
        &mut commands,
        grid.as_ref(),
        support::ExampleLayout::Single,
        "Long Route Agent",
        GridCoord::new(2, 2, 0),
        Color::srgb(0.96, 0.78, 0.30),
    );
    commands.entity(long_route).insert((
        LongRouteAgent,
        PathfindingAgent::default(),
        PathRequest::new(GridCoord::new(124, 120, 0)),
    ));

    let cross_map = support::spawn_agent_sprite(
        &mut commands,
        grid.as_ref(),
        support::ExampleLayout::Single,
        "Cross Map Agent",
        GridCoord::new(8, 116, 0),
        Color::srgb(0.34, 0.88, 0.62),
    );
    commands.entity(cross_map).insert((
        CrossMapAgent,
        PathfindingAgent::default(),
        PathRequest::new(GridCoord::new(116, 8, 0)),
    ));

    let diagonal = support::spawn_agent_sprite(
        &mut commands,
        grid.as_ref(),
        support::ExampleLayout::Single,
        "Diagonal Agent",
        GridCoord::new(24, 24, 0),
        Color::srgb(0.40, 0.70, 0.98),
    );
    commands.entity(diagonal).insert((
        DiagonalAgent,
        PathfindingAgent::default(),
        PathRequest::new(GridCoord::new(100, 100, 0)),
    ));
}

fn sync_monitors(
    mut pane: ResMut<support::HpaExamplePane>,
    paths: Query<&ComputedPath, With<LongRouteAgent>>,
) {
    let Ok(path) = paths.single() else {
        return;
    };
    pane.corridor_len = path.corridor.len() as u32;
    pane.waypoint_count = path.waypoints.len() as u32;
    pane.total_cost = path.total_cost;
}
