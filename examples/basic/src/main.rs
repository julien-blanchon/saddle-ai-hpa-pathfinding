use saddle_ai_hpa_pathfinding_example_support as support;

#[cfg(feature = "e2e")]
mod scenarios;

use bevy::prelude::*;
use saddle_ai_hpa_pathfinding::{
    ComputedPath, GridCoord, HpaPathfindingPlugin, PathRequest, PathfindingAgent, PathfindingGrid,
};

#[derive(Component)]
struct GoalMarker;

#[derive(Component)]
struct DemoAgent;

fn main() {
    let config = support::demo_config(UVec3::new(32, 24, 1));
    let grid = PathfindingGrid::new(
        support::build_demo_grid(config.grid_dimensions),
        config.clone(),
    );

    let mut app = App::new();
    app.insert_resource(config);
    app.insert_resource(grid);
    app.insert_resource(support::HpaExamplePane::default());
    support::configure_visual_app(&mut app, "hpa pathfinding: basic");
    app.add_plugins(HpaPathfindingPlugin::default());
    app.add_systems(Startup, setup);
    app.add_systems(
        Update,
        (
            support::sync_config_from_pane,
            support::click_to_set_goal,
            support::click_to_toggle_wall,
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

fn setup(mut commands: Commands, grid: Res<PathfindingGrid>) {
    support::spawn_grid_camera(&mut commands);
    support::spawn_demo_backdrop(
        &mut commands,
        grid.as_ref(),
        support::ExampleLayout::Single,
        "Basic Route",
        "Left-click to set goal. Right-click to toggle walls.\nUse the pane on the right to tweak parameters.",
    );
    support::spawn_grid_tiles(
        &mut commands,
        grid.as_ref(),
        support::ExampleLayout::Single,
        None,
    );
    support::spawn_instructions(
        &mut commands,
        "Keyboard shortcuts:  G grid  C clusters  P portals  A graph  H heatmap  D paths",
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
        "Scout Agent",
        GridCoord::new(2, 2, 0),
        Color::srgb(0.28, 0.86, 0.74),
    );
    commands.entity(goal_marker).insert(GoalMarker);
    commands.entity(agent).insert((
        DemoAgent,
        PathfindingAgent::default(),
        PathRequest::new(goal),
    ));
}

fn sync_pane(
    pane: Res<support::HpaExamplePane>,
    grid: Res<PathfindingGrid>,
    mut goals: Query<&mut Transform, With<GoalMarker>>,
    mut agents: Query<(&mut PathfindingAgent, &mut PathRequest), With<DemoAgent>>,
) {
    if !pane.is_changed() {
        return;
    }

    let goal = support::clamp_goal_to_grid(grid.as_ref(), &pane);
    let overlays = if pane.overlay_enabled {
        vec![saddle_ai_hpa_pathfinding::PathCostOverlay::new(
            support::pane_overlay_region(&pane),
            pane.overlay_cost,
        )]
    } else {
        Vec::new()
    };

    for mut transform in &mut goals {
        transform.translation = support::grid_visual_translation(
            grid.as_ref(),
            support::ExampleLayout::Single,
            goal,
            9.0,
        );
    }
    for (mut agent, mut request) in &mut agents {
        agent.clearance = pane.clearance.max(0) as u16;
        request.goal = goal;
        request.overlays = overlays.clone();
    }
}

fn sync_monitors(
    mut pane: ResMut<support::HpaExamplePane>,
    paths: Query<&ComputedPath, With<DemoAgent>>,
) {
    let Ok(path) = paths.single() else {
        return;
    };
    let corridor_len = path.corridor.len() as u32;
    let waypoint_count = path.waypoints.len() as u32;
    if pane.corridor_len != corridor_len
        || pane.waypoint_count != waypoint_count
        || (pane.total_cost - path.total_cost).abs() > f32::EPSILON
    {
        pane.corridor_len = corridor_len;
        pane.waypoint_count = waypoint_count;
        pane.total_cost = path.total_cost;
    }
}
