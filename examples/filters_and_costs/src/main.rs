use saddle_ai_hpa_pathfinding_example_support as support;

use bevy::prelude::*;
use saddle_ai_hpa_pathfinding::{
    ComputedPath, GridCoord, HpaPathfindingPlugin, PathCostOverlay, PathFilterId, PathRequest,
    PathfindingAgent, PathfindingGrid,
};

#[derive(Component)]
struct GoalMarker;

#[derive(Component)]
struct UtilityAgent;

#[derive(Component)]
struct WheeledAgent;

fn main() {
    let grid = support::build_filter_grid();
    let config = grid.snapshot.config.clone();

    let mut app = App::new();
    app.insert_resource(config);
    app.insert_resource(grid);
    app.insert_resource(support::HpaExamplePane {
        goal_x: 20,
        goal_y: 16,
        overlay_enabled: true,
        overlay_cost: 5.0,
        ..default()
    });
    support::configure_visual_app(&mut app, "hpa pathfinding: filters and costs");
    app.add_plugins(HpaPathfindingPlugin::default());
    app.add_systems(Startup, setup);
    app.add_systems(Update, support::sync_config_from_pane);
    app.add_systems(Update, (sync_pane, sync_monitors));
    app.run();
}

fn setup(mut commands: Commands, grid: Res<PathfindingGrid>, pane: Res<support::HpaExamplePane>) {
    let overlay = if pane.overlay_enabled {
        Some(support::pane_overlay_region(&pane))
    } else {
        None
    };
    support::spawn_grid_camera(&mut commands);
    support::spawn_demo_backdrop(
        &mut commands,
        grid.as_ref(),
        support::ExampleLayout::Single,
        "Filters And Cost Overlays",
        "The green utility agent tolerates rough terrain better, while the blue wheeled agent strongly avoids it. Toggle the amber overlay to add request-local soft penalties.",
    );
    support::spawn_grid_tiles(
        &mut commands,
        grid.as_ref(),
        support::ExampleLayout::Single,
        overlay,
    );

    let goal = support::clamp_goal_to_grid(grid.as_ref(), &pane);
    let goal_marker = support::spawn_goal_marker(
        &mut commands,
        grid.as_ref(),
        support::ExampleLayout::Single,
        "Goal Marker",
        goal,
        Color::srgb(0.97, 0.85, 0.28),
    );
    let utility = support::spawn_agent_sprite(
        &mut commands,
        grid.as_ref(),
        support::ExampleLayout::Single,
        "Utility Agent",
        GridCoord::new(2, 2, 0),
        Color::srgb(0.34, 0.90, 0.58),
    );
    let wheeled = support::spawn_agent_sprite(
        &mut commands,
        grid.as_ref(),
        support::ExampleLayout::Single,
        "Wheeled Agent",
        GridCoord::new(2, 3, 0),
        Color::srgb(0.34, 0.66, 0.98),
    );

    commands.entity(goal_marker).insert(GoalMarker);
    commands.entity(utility).insert((
        UtilityAgent,
        PathfindingAgent {
            filter: PathFilterId(2),
            ..default()
        },
        PathRequest::new(goal),
    ));
    commands.entity(wheeled).insert((
        WheeledAgent,
        PathfindingAgent {
            filter: PathFilterId(1),
            ..default()
        },
        PathRequest::new(goal),
    ));
}

fn sync_pane(
    pane: Res<support::HpaExamplePane>,
    grid: Res<PathfindingGrid>,
    mut goals: Query<&mut Transform, With<GoalMarker>>,
    mut utility_agents: Query<
        (&mut PathfindingAgent, &mut PathRequest),
        (With<UtilityAgent>, Without<WheeledAgent>),
    >,
    mut wheeled_agents: Query<
        (&mut PathfindingAgent, &mut PathRequest),
        (With<WheeledAgent>, Without<UtilityAgent>),
    >,
) {
    if !pane.is_changed() {
        return;
    }

    let goal = support::clamp_goal_to_grid(grid.as_ref(), &pane);
    let overlays = if pane.overlay_enabled {
        vec![PathCostOverlay::new(
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
            8.0,
        );
    }
    for (mut agent, mut request) in &mut utility_agents {
        agent.clearance = pane.clearance.max(0) as u16;
        request.goal = goal;
        request.overlays = overlays.clone();
    }
    for (mut agent, mut request) in &mut wheeled_agents {
        agent.clearance = pane.clearance.max(0) as u16;
        request.goal = goal;
        request.overlays = overlays.clone();
    }
}

fn sync_monitors(
    mut pane: ResMut<support::HpaExamplePane>,
    utility_paths: Query<&ComputedPath, With<UtilityAgent>>,
) {
    let Ok(path) = utility_paths.single() else {
        return;
    };
    pane.corridor_len = path.corridor.len() as u32;
    pane.waypoint_count = path.waypoints.len() as u32;
    pane.total_cost = path.total_cost;
}
