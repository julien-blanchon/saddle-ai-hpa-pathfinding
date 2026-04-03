use saddle_ai_hpa_pathfinding_example_support as support;

use bevy::prelude::*;
use saddle_ai_hpa_pathfinding::{
    ComputedPath, GridCoord, HpaPathfindingPlugin, ObstacleShape, PathRequest, PathfindingAgent,
    PathfindingGrid, PathfindingObstacle,
};

const GATE: GridCoord = GridCoord(IVec3::new(16, 12, 0));

#[derive(Component)]
struct GoalMarker;

#[derive(Component)]
struct DemoAgent;

#[derive(Component)]
struct GateMarker;

fn main() {
    let config = support::demo_config(UVec3::new(32, 24, 1));
    let grid = PathfindingGrid::new(
        support::build_demo_grid(config.grid_dimensions),
        config.clone(),
    );

    let mut app = App::new();
    app.insert_resource(config);
    app.insert_resource(grid);
    app.insert_resource(support::HpaExamplePane {
        goal_x: 28,
        goal_y: 21,
        ..default()
    });
    support::configure_visual_app(&mut app, "hpa pathfinding: dynamic obstacles");
    app.add_plugins(HpaPathfindingPlugin::default());
    app.add_systems(Startup, setup);
    app.add_systems(Update, support::sync_config_from_pane);
    app.add_systems(Update, (sync_pane, sync_monitors, sync_gate_visual));
    app.run();
}

fn setup(mut commands: Commands, grid: Res<PathfindingGrid>) {
    support::spawn_grid_camera(&mut commands);
    support::spawn_demo_backdrop(
        &mut commands,
        grid.as_ref(),
        support::ExampleLayout::Single,
        "Dynamic Gate",
        "Toggle the central gate live to force the route back through side corridors and watch the published path invalidate and recover.",
    );
    support::spawn_grid_tiles(
        &mut commands,
        grid.as_ref(),
        support::ExampleLayout::Single,
        None,
    );

    let goal = GridCoord::new(28, 21, 0);
    let goal_marker = support::spawn_goal_marker(
        &mut commands,
        grid.as_ref(),
        support::ExampleLayout::Single,
        "Goal Marker",
        goal,
        Color::srgb(0.97, 0.85, 0.28),
    );
    let agent = support::spawn_agent_sprite(
        &mut commands,
        grid.as_ref(),
        support::ExampleLayout::Single,
        "Detour Agent",
        GridCoord::new(2, 2, 0),
        Color::srgb(0.32, 0.84, 0.94),
    );
    let gate = support::spawn_goal_marker(
        &mut commands,
        grid.as_ref(),
        support::ExampleLayout::Single,
        "Gate Marker",
        GATE,
        Color::srgba(0.94, 0.28, 0.22, 0.05),
    );
    commands.entity(goal_marker).insert(GoalMarker);
    commands.entity(gate).insert(GateMarker);
    commands.entity(agent).insert((
        DemoAgent,
        PathfindingAgent::default(),
        PathRequest::new(goal),
    ));
}

fn sync_pane(
    pane: Res<support::HpaExamplePane>,
    grid: Res<PathfindingGrid>,
    mut commands: Commands,
    mut goals: Query<&mut Transform, With<GoalMarker>>,
    gates: Query<(Entity, Option<&PathfindingObstacle>), With<GateMarker>>,
    mut agents: Query<(&mut PathfindingAgent, &mut PathRequest), With<DemoAgent>>,
) {
    if !pane.is_changed() {
        return;
    }

    let goal = support::clamp_goal_to_grid(grid.as_ref(), &pane);
    for mut transform in &mut goals {
        transform.translation = support::grid_visual_translation(
            grid.as_ref(),
            support::ExampleLayout::Single,
            goal,
            8.0,
        );
    }
    for (mut agent, mut request) in &mut agents {
        agent.clearance = pane.clearance.max(0) as u16;
        request.goal = goal;
    }
    for (entity, obstacle) in &gates {
        if pane.gate_blocked && obstacle.is_none() {
            commands.entity(entity).insert(PathfindingObstacle {
                shape: ObstacleShape::Cell(GATE),
                area_override: None,
            });
        } else if !pane.gate_blocked && obstacle.is_some() {
            commands.entity(entity).remove::<PathfindingObstacle>();
        }
    }
}

fn sync_gate_visual(
    pane: Res<support::HpaExamplePane>,
    mut gates: Query<&mut Sprite, With<GateMarker>>,
) {
    if !pane.is_changed() {
        return;
    }

    for mut sprite in &mut gates {
        sprite.color = if pane.gate_blocked {
            Color::srgba(0.94, 0.28, 0.22, 0.92)
        } else {
            Color::srgba(0.94, 0.28, 0.22, 0.08)
        };
    }
}

fn sync_monitors(
    mut pane: ResMut<support::HpaExamplePane>,
    paths: Query<&ComputedPath, With<DemoAgent>>,
) {
    let Ok(path) = paths.single() else {
        return;
    };
    pane.corridor_len = path.corridor.len() as u32;
    pane.waypoint_count = path.waypoints.len() as u32;
    pane.total_cost = path.total_cost;
}
