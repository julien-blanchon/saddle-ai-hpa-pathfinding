use saddle_ai_hpa_pathfinding_example_support as support;

#[cfg(feature = "e2e")]
mod scenarios;

use bevy::prelude::*;
use saddle_ai_hpa_pathfinding::{
    GridCoord, HpaPathfindingPlugin, PathRequest, PathfindingAgent, PathfindingGrid,
};

#[derive(Component)]
struct GoalMarker;

#[derive(Component)]
struct DebugAgent;

fn main() {
    let config = saddle_ai_hpa_pathfinding::HpaPathfindingConfig {
        debug_draw_clusters: true,
        debug_draw_portals: true,
        debug_draw_abstract_graph: true,
        debug_draw_paths: true,
        debug_draw_cost_heatmap: true,
        debug_draw_grid: false,
        ..support::demo_config(UVec3::new(32, 24, 1))
    };
    let mut storage = saddle_ai_hpa_pathfinding::GridStorage::new(
        config.grid_dimensions,
        config.origin,
        config.cell_size,
        saddle_ai_hpa_pathfinding::WorldRoundingPolicy::Floor,
    );
    // Wall across middle with gap.
    for x in 5..27 {
        if x != 16 {
            storage.set_walkable(GridCoord::new(x, 12, 0), false);
        }
    }
    // Rough-terrain cost zone.
    storage.fill_region(
        saddle_ai_hpa_pathfinding::GridAabb::new(
            GridCoord::new(3, 4, 0),
            GridCoord::new(28, 7, 0),
        ),
        |_coord, cell| {
            cell.base_cost = 3.0;
        },
    );
    let grid = PathfindingGrid::new(storage, config.clone());

    let mut app = App::new();
    app.insert_resource(config);
    app.insert_resource(grid);
    app.insert_resource(support::HpaExamplePane {
        goal_x: 28,
        goal_y: 20,
        draw_grid: false,
        draw_clusters: true,
        draw_portals: true,
        draw_abstract_graph: true,
        draw_paths: true,
        draw_heatmap: true,
        ..default()
    });
    support::configure_visual_app(&mut app, "hpa pathfinding: debug visualization");
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
    app.add_systems(Update, sync_pane);
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
        "Debug Visualization",
        "All layers enabled: clusters, portals, graph, heatmap, paths.\nLeft-click to set goal. Right-click to toggle walls.",
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

    let goal = GridCoord::new(28, 20, 0);
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
        "Debug Agent",
        GridCoord::new(2, 2, 0),
        Color::srgb(0.94, 0.86, 0.22),
    );
    commands.entity(goal_marker).insert(GoalMarker);
    commands.entity(agent).insert((
        DebugAgent,
        PathfindingAgent::default(),
        PathRequest::new(goal),
    ));
}

fn sync_pane(
    pane: Res<support::HpaExamplePane>,
    grid: Res<PathfindingGrid>,
    mut goals: Query<&mut Transform, With<GoalMarker>>,
    mut agents: Query<(&mut PathfindingAgent, &mut PathRequest), With<DebugAgent>>,
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
            9.0,
        );
    }
    for (mut agent, mut request) in &mut agents {
        agent.clearance = pane.clearance.max(0) as u16;
        request.goal = goal;
    }
}
