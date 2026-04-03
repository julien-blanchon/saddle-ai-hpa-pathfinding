use saddle_ai_hpa_pathfinding_example_support as support;

use bevy::prelude::*;
use saddle_ai_hpa_pathfinding::{HpaPathfindingPlugin, PathCostOverlay, PathfindingGrid};

#[derive(Component)]
struct GoalMarker;

#[derive(Component)]
struct FlowArrow;

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
        draw_grid: false,
        draw_clusters: false,
        draw_portals: false,
        overlay_enabled: true,
        ..default()
    });
    support::configure_visual_app(&mut app, "hpa pathfinding: flow field");
    app.add_plugins(HpaPathfindingPlugin::default());
    app.add_systems(Startup, setup);
    app.add_systems(Update, support::sync_config_from_pane);
    app.add_systems(Update, rebuild_flow_field);
    app.run();
}

fn setup(mut commands: Commands, grid: Res<PathfindingGrid>, pane: Res<support::HpaExamplePane>) {
    support::spawn_grid_camera(&mut commands);
    support::spawn_demo_backdrop(
        &mut commands,
        grid.as_ref(),
        support::ExampleLayout::Single,
        "Flow Field Evacuation",
        "Every arrow points at the cheapest next step toward the active goal. Toggle the overlay to bend the field around temporary soft costs.",
    );
    let overlay = if pane.overlay_enabled {
        Some(support::pane_overlay_region(&pane))
    } else {
        None
    };
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
    commands.entity(goal_marker).insert(GoalMarker);
}

fn rebuild_flow_field(
    mut commands: Commands,
    mut pane: ResMut<support::HpaExamplePane>,
    grid: Res<PathfindingGrid>,
    arrows: Query<Entity, With<FlowArrow>>,
    mut goals: Query<&mut Transform, (With<GoalMarker>, Without<FlowArrow>)>,
) {
    if !pane.is_changed() {
        return;
    }

    for entity in &arrows {
        commands.entity(entity).despawn();
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

    let overlays = if pane.overlay_enabled {
        vec![PathCostOverlay::new(
            support::pane_overlay_region(&pane),
            pane.overlay_cost,
        )]
    } else {
        Vec::new()
    };

    let Some(flow_field) = grid.build_flow_field_with_clearance(
        goal,
        saddle_ai_hpa_pathfinding::PathFilterId(0),
        pane.clearance.max(0) as u16,
        &overlays,
    ) else {
        pane.reachable_cells = 0;
        return;
    };

    let mut reachable = 0_u32;
    for coord in grid.grid().bounds().iter() {
        let Some(direction) = flow_field.direction_at(coord) else {
            continue;
        };
        if direction.length_squared() <= f32::EPSILON {
            continue;
        }
        reachable += 1;
        let rotation = Quat::from_rotation_z(direction.y.atan2(direction.x));
        commands.spawn((
            Name::new(format!(
                "Flow Arrow {},{},{}",
                coord.x(),
                coord.y(),
                coord.z()
            )),
            FlowArrow,
            Sprite::from_color(
                Color::srgba(0.86, 0.92, 0.98, 0.78),
                Vec2::new(
                    grid.grid().space.cell_size * 0.34,
                    grid.grid().space.cell_size * 0.10,
                ),
            ),
            Transform {
                translation: support::grid_visual_translation(
                    grid.as_ref(),
                    support::ExampleLayout::Single,
                    coord,
                    2.0,
                ),
                rotation,
                ..default()
            },
        ));
    }

    pane.reachable_cells = reachable;
}
