use saddle_ai_hpa_pathfinding_example_support as support;

use bevy::{
    camera::{OrthographicProjection, Projection},
    prelude::*,
};
use saddle_ai_hpa_pathfinding::{
    GridCoord, HpaPathfindingPlugin, PathRequest, PathfindingAgent, PathfindingGrid,
};
use saddle_pane::prelude::*;

#[derive(Component)]
struct WallCell;

fn main() {
    let mut app = App::new();
    let config = saddle_ai_hpa_pathfinding::HpaPathfindingConfig {
        grid_dimensions: UVec3::new(32, 24, 1),
        debug_draw_clusters: true,
        debug_draw_portals: true,
        debug_draw_abstract_graph: true,
        debug_draw_paths: true,
        debug_draw_cost_heatmap: true,
        debug_draw_grid: false,
        ..Default::default()
    };
    let path_grid = PathfindingGrid::new(
        {
            let mut grid = saddle_ai_hpa_pathfinding::GridStorage::new(
                config.grid_dimensions,
                Vec3::new(-16.0, -12.0, 0.0),
                1.0,
                saddle_ai_hpa_pathfinding::WorldRoundingPolicy::Floor,
            );
            for x in 5..27 {
                if x != 16 {
                    grid.set_walkable(GridCoord::new(x, 12, 0), false);
                }
            }
            grid.fill_region(
                saddle_ai_hpa_pathfinding::GridAabb::new(
                    GridCoord::new(3, 4, 0),
                    GridCoord::new(28, 7, 0),
                ),
                |_coord, cell| {
                    cell.base_cost = 3.0;
                },
            );
            grid
        },
        config.clone(),
    );

    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "saddle-ai-hpa-pathfinding debug_viz".into(),
            resolution: (1280, 840).into(),
            ..default()
        }),
        ..default()
    }));
    app.insert_resource(ClearColor(Color::srgb(0.04, 0.05, 0.06)));
    app.insert_resource(support::HpaExamplePane {
        goal_x: 28,
        goal_y: 20,
        draw_clusters: true,
        draw_portals: true,
        draw_heatmap: true,
        ..default()
    });
    app.insert_resource(config);
    app.insert_resource(path_grid.clone());
    app.add_plugins((
        bevy_flair::FlairPlugin,
        bevy_input_focus::InputDispatchPlugin,
        bevy_ui_widgets::UiWidgetsPlugins,
        bevy_input_focus::tab_navigation::TabNavigationPlugin,
        PanePlugin,
    ))
    .register_pane::<support::HpaExamplePane>();
    app.add_plugins(HpaPathfindingPlugin::default());
    app.add_systems(Update, support::sync_config_from_pane);
    app.add_systems(Startup, move |mut commands: Commands| {
        commands.spawn((
            Name::new("Camera"),
            Camera2d,
            Projection::Orthographic(OrthographicProjection {
                scale: 0.05,
                ..OrthographicProjection::default_2d()
            }),
            Transform::from_xyz(0.0, 0.0, 1000.0),
        ));
        for coord in path_grid.grid().bounds().iter() {
            let cell = path_grid.grid().cell(coord).unwrap();
            if cell.walkable {
                continue;
            }
            commands.spawn((
                Name::new("Wall Cell"),
                WallCell,
                Sprite::from_color(Color::srgb(0.28, 0.18, 0.16), Vec2::splat(0.9)),
                Transform::from_translation(path_grid.grid().grid_to_world_center(coord)),
            ));
        }
        commands.spawn((
            Name::new("Debug Agent"),
            Transform::from_xyz(-13.5, -9.5, 1.0),
            GlobalTransform::from_xyz(-13.5, -9.5, 1.0),
            Sprite::from_color(Color::srgb(0.94, 0.86, 0.22), Vec2::splat(0.7)),
            PathfindingAgent::default(),
            PathRequest::new(GridCoord::new(28, 20, 0)),
        ));
    });
    app.run();
}
