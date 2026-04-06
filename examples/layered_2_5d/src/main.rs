use saddle_ai_hpa_pathfinding_example_support as support;

use bevy::prelude::*;
use saddle_ai_hpa_pathfinding::{
    ComputedPath, GridCoord, HpaPathfindingPlugin, PathRequest, PathfindingAgent, PathfindingGrid,
};

#[cfg(feature = "e2e")]
mod scenarios;

#[derive(Component)]
struct GoalMarker;

#[derive(Component)]
struct LayeredAgent;

const LAYOUT: support::ExampleLayout = support::ExampleLayout::Layered { spacing_cells: 2.0 };

fn main() {
    let grid = support::build_layered_grid();
    let config = grid.snapshot.config.clone();

    let mut app = App::new();
    app.insert_resource(config);
    app.insert_resource(grid);
    app.insert_resource(support::HpaExamplePane {
        goal_x: 12,
        goal_y: 12,
        goal_layer: 1,
        draw_portals: true,
        ..default()
    });
    support::configure_visual_app(&mut app, "hpa pathfinding: layered 2.5d");
    app.add_plugins(HpaPathfindingPlugin::default());
    app.add_systems(Startup, setup);
    app.add_systems(
        Update,
        (
            support::sync_config_from_pane,
            support::keyboard_debug_shortcuts,
        ),
    );
    app.add_systems(Update, (sync_pane, sync_monitors));
    #[cfg(feature = "e2e")]
    app.add_plugins(support::e2e_support::ExampleE2EPlugin::new(
        scenarios::list,
        scenarios::by_name,
    ));
    app.run();
}

fn setup(mut commands: Commands, grid: Res<PathfindingGrid>, pane: Res<support::HpaExamplePane>) {
    support::spawn_grid_camera(&mut commands);
    support::spawn_demo_backdrop(
        &mut commands,
        grid.as_ref(),
        LAYOUT,
        "Layered 2.5D Route",
        "Two layers connected by stairs. The route crosses layers\nvia an explicit transition link.",
    );
    support::spawn_grid_tiles(&mut commands, grid.as_ref(), LAYOUT, None);
    support::spawn_layer_labels(&mut commands, grid.as_ref(), LAYOUT);

    let goal = support::clamp_goal_to_grid(grid.as_ref(), &pane);
    let goal_marker = support::spawn_goal_marker(
        &mut commands,
        grid.as_ref(),
        LAYOUT,
        "Upper Goal",
        goal,
        Color::srgb(0.97, 0.85, 0.28),
    );
    let agent = support::spawn_agent_sprite(
        &mut commands,
        grid.as_ref(),
        LAYOUT,
        "Layered Agent",
        GridCoord::new(2, 2, 0),
        Color::srgb(0.34, 0.86, 0.96),
    );

    support::spawn_instructions(
        &mut commands,
        "Keyboard shortcuts:  G grid  C clusters  P portals  A graph  H heatmap  D paths",
    );

    commands.entity(goal_marker).insert(GoalMarker);
    commands.entity(agent).insert((
        LayeredAgent,
        PathfindingAgent::default(),
        PathRequest::new(goal),
    ));
}

fn sync_pane(
    pane: Res<support::HpaExamplePane>,
    grid: Res<PathfindingGrid>,
    mut goals: Query<&mut Transform, With<GoalMarker>>,
    mut agents: Query<(&mut PathfindingAgent, &mut PathRequest), With<LayeredAgent>>,
) {
    if !pane.is_changed() {
        return;
    }

    let goal = support::clamp_goal_to_grid(grid.as_ref(), &pane);
    for mut transform in &mut goals {
        transform.translation = support::grid_visual_translation(grid.as_ref(), LAYOUT, goal, 9.0);
    }
    for (mut agent, mut request) in &mut agents {
        agent.clearance = pane.clearance.max(0) as u16;
        request.goal = goal;
    }
}

fn sync_monitors(
    mut pane: ResMut<support::HpaExamplePane>,
    paths: Query<&ComputedPath, With<LayeredAgent>>,
) {
    let Ok(path) = paths.single() else {
        return;
    };
    pane.corridor_len = path.corridor.len() as u32;
    pane.waypoint_count = path.waypoints.len() as u32;
    pane.total_cost = path.total_cost;
}
