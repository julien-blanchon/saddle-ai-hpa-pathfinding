use saddle_ai_hpa_pathfinding_example_support as support;

use bevy::prelude::*;
use saddle_ai_hpa_pathfinding::{
    ComputedPath, GridCoord, HpaPathfindingPlugin, PathRequest, PathfindingAgent, PathfindingGrid,
};

#[derive(Component)]
struct QueueAgent;

fn main() {
    let mut config = support::demo_config(UVec3::new(32, 24, 1));
    config.max_queries_per_frame = 2;
    let grid = PathfindingGrid::new(
        support::build_demo_grid(config.grid_dimensions),
        config.clone(),
    );

    let mut app = App::new();
    app.insert_resource(config);
    app.insert_resource(grid);
    app.insert_resource(support::HpaExamplePane {
        goal_x: 28,
        goal_y: 20,
        max_queries_per_frame: 2,
        ..default()
    });
    support::configure_visual_app(&mut app, "hpa pathfinding: async queries");
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
        "Async Query Queue",
        "Lower the per-frame budget to watch the request queue drain more slowly. Multiple agents share the same committed hierarchy snapshot without blocking the frame.",
    );
    support::spawn_grid_tiles(
        &mut commands,
        grid.as_ref(),
        support::ExampleLayout::Single,
        None,
    );

    for (index, (start, goal, color)) in [
        (
            GridCoord::new(2, 2, 0),
            GridCoord::new(28, 20, 0),
            Color::srgb(0.94, 0.78, 0.32),
        ),
        (
            GridCoord::new(3, 5, 0),
            GridCoord::new(27, 17, 0),
            Color::srgb(0.32, 0.88, 0.64),
        ),
        (
            GridCoord::new(4, 8, 0),
            GridCoord::new(25, 4, 0),
            Color::srgb(0.38, 0.74, 0.98),
        ),
        (
            GridCoord::new(5, 10, 0),
            GridCoord::new(26, 21, 0),
            Color::srgb(0.90, 0.48, 0.30),
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let entity = support::spawn_agent_sprite(
            &mut commands,
            grid.as_ref(),
            support::ExampleLayout::Single,
            &format!("Queue Agent {index}"),
            start,
            color,
        );
        commands.entity(entity).insert((
            QueueAgent,
            PathfindingAgent::default(),
            PathRequest::new(goal),
        ));
    }
}

fn sync_monitors(
    mut pane: ResMut<support::HpaExamplePane>,
    ready_paths: Query<&ComputedPath, With<QueueAgent>>,
) {
    pane.reachable_cells = ready_paths.iter().count() as u32;
    if let Some(path) = ready_paths.iter().next() {
        pane.corridor_len = path.corridor.len() as u32;
        pane.waypoint_count = path.waypoints.len() as u32;
        pane.total_cost = path.total_cost;
    }
}
