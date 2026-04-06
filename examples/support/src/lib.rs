use bevy::{app::AppExit, prelude::*};
use saddle_ai_hpa_pathfinding::{
    AreaMask, AreaTypeId, CellData, GridAabb, GridCoord, GridStorage, HpaPathfindingConfig,
    PathFilterId, PathFilterProfile, PathfindingGrid, WorldRoundingPolicy,
};
use saddle_pane::prelude::*;

#[derive(Resource)]
struct ExampleAutoExit(Timer);

#[derive(Resource, Debug, Clone, Copy, Pane)]
#[pane(title = "HPA Pathfinding", position = "top-right")]
pub struct HpaExamplePane {
    #[pane(toggle)]
    pub draw_grid: bool,
    #[pane(toggle)]
    pub draw_clusters: bool,
    #[pane(toggle)]
    pub draw_portals: bool,
    #[pane(toggle)]
    pub draw_abstract_graph: bool,
    #[pane(toggle)]
    pub draw_paths: bool,
    #[pane(toggle)]
    pub draw_heatmap: bool,
    #[pane(toggle)]
    pub gate_blocked: bool,
    #[pane(toggle)]
    pub overlay_enabled: bool,
    #[pane(slider, min = 0.0, max = 127.0, step = 1.0)]
    pub goal_x: i32,
    #[pane(slider, min = 0.0, max = 127.0, step = 1.0)]
    pub goal_y: i32,
    #[pane(slider, min = 0.0, max = 3.0, step = 1.0)]
    pub goal_layer: i32,
    #[pane(slider, min = 0.0, max = 8.0, step = 1.0)]
    pub clearance: i32,
    #[pane(slider, min = 0.0, max = 16.0, step = 0.5)]
    pub overlay_cost: f32,
    #[pane(slider, min = 1.0, max = 32.0, step = 1.0)]
    pub direct_search_distance: i32,
    #[pane(slider, min = 1.0, max = 16.0, step = 1.0)]
    pub max_queries_per_frame: i32,
    #[pane(slider, min = 8.0, max = 256.0, step = 8.0)]
    pub max_sliced_expansions_per_frame: i32,
    #[pane(monitor)]
    pub corridor_len: u32,
    #[pane(monitor)]
    pub waypoint_count: u32,
    #[pane(monitor)]
    pub total_cost: f32,
    #[pane(monitor)]
    pub reachable_cells: u32,
}

impl Default for HpaExamplePane {
    fn default() -> Self {
        Self {
            draw_grid: false,
            draw_clusters: true,
            draw_portals: true,
            draw_abstract_graph: false,
            draw_paths: true,
            draw_heatmap: false,
            gate_blocked: false,
            overlay_enabled: false,
            goal_x: 28,
            goal_y: 21,
            goal_layer: 0,
            clearance: 0,
            overlay_cost: 5.0,
            direct_search_distance: 16,
            max_queries_per_frame: 8,
            max_sliced_expansions_per_frame: 96,
            corridor_len: 0,
            waypoint_count: 0,
            total_cost: 0.0,
            reachable_cells: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum ExampleLayout {
    #[default]
    Single,
    Layered {
        spacing_cells: f32,
    },
}

pub fn configure_visual_app(app: &mut App, title: &str) {
    app.insert_resource(ClearColor(Color::srgb(0.08, 0.09, 0.11)));
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: title.into(),
            resolution: (1440, 920).into(),
            ..default()
        }),
        ..default()
    }));
    app.add_plugins((
        bevy_flair::FlairPlugin,
        bevy_input_focus::InputDispatchPlugin,
        bevy_ui_widgets::UiWidgetsPlugins,
        bevy_input_focus::tab_navigation::TabNavigationPlugin,
        PanePlugin,
    ))
    .register_pane::<HpaExamplePane>();
    if let Some(auto_exit) = auto_exit_from_env() {
        app.insert_resource(auto_exit);
        app.add_systems(Update, auto_exit_example);
    }
}

pub fn sync_config_from_pane(pane: Res<HpaExamplePane>, mut config: ResMut<HpaPathfindingConfig>) {
    if !pane.is_changed() {
        return;
    }

    config.debug_draw_paths = pane.draw_paths;
    config.debug_draw_grid = pane.draw_grid;
    config.debug_draw_clusters = pane.draw_clusters;
    config.debug_draw_portals = pane.draw_portals;
    config.debug_draw_abstract_graph = pane.draw_abstract_graph;
    config.debug_draw_cost_heatmap = pane.draw_heatmap;
    config.direct_search_distance = pane.direct_search_distance.max(1) as u32;
    config.max_queries_per_frame = pane.max_queries_per_frame.max(1) as u32;
    config.max_sliced_expansions_per_frame = pane.max_sliced_expansions_per_frame.max(1) as u32;
}

pub fn demo_config(dimensions: UVec3) -> HpaPathfindingConfig {
    let cell_size = if dimensions.x.max(dimensions.y) >= 96 {
        7.0
    } else {
        28.0
    };
    HpaPathfindingConfig {
        grid_dimensions: dimensions,
        origin: Vec3::new(
            -(dimensions.x as f32) * cell_size * 0.5,
            -(dimensions.y as f32) * cell_size * 0.5,
            0.0,
        ),
        cell_size,
        cluster_size: UVec3::new(16, 16, 1),
        hierarchy_levels: if dimensions.x >= 32 || dimensions.y >= 32 {
            2
        } else {
            1
        },
        debug_draw_paths: true,
        debug_draw_clusters: true,
        debug_draw_portals: true,
        ..default()
    }
}

pub fn build_demo_grid(dimensions: UVec3) -> GridStorage {
    let config = demo_config(dimensions);
    let mut grid = GridStorage::new(
        dimensions,
        config.origin,
        config.cell_size,
        WorldRoundingPolicy::Floor,
    );

    // Wall across middle with a gate.
    for x in 6..dimensions.x.saturating_sub(6) as i32 {
        grid.set_walkable(GridCoord::new(x, (dimensions.y / 2) as i32, 0), false);
    }
    let gate = GridCoord::new((dimensions.x / 2) as i32, (dimensions.y / 2) as i32, 0);
    grid.set_walkable(gate, true);

    // Rough-terrain patches so the heatmap is visually interesting.
    if dimensions.x >= 16 && dimensions.y >= 16 {
        grid.fill_region(
            GridAabb::new(GridCoord::new(2, 2, 0), GridCoord::new(6, 6, 0)),
            |_coord, cell| {
                if cell.walkable {
                    cell.base_cost = 2.5;
                    cell.area = AreaTypeId(1);
                }
            },
        );
        let rx = (dimensions.x as i32).saturating_sub(8);
        let ry = (dimensions.y as i32).saturating_sub(7);
        grid.fill_region(
            GridAabb::new(
                GridCoord::new(rx.max(0), ry.max(0), 0),
                GridCoord::new(
                    (dimensions.x as i32).saturating_sub(3).max(rx),
                    (dimensions.y as i32).saturating_sub(3).max(ry),
                    0,
                ),
            ),
            |_coord, cell| {
                if cell.walkable {
                    cell.base_cost = 1.8;
                    cell.area = AreaTypeId(1);
                }
            },
        );
    }

    grid
}

pub fn build_filter_grid() -> PathfindingGrid {
    let config = demo_config(UVec3::new(24, 18, 1));
    let mut grid = PathfindingGrid::new(build_demo_grid(config.grid_dimensions), config);
    grid.fill_region(
        GridAabb::new(GridCoord::new(3, 12, 0), GridCoord::new(20, 15, 0)),
        |_coord, cell| {
            cell.area = AreaTypeId(1);
            cell.base_cost = 1.0;
            cell.traversal_mask = AreaMask::from_bit(1);
        },
    );
    grid.register_filter(
        PathFilterProfile::named("wheeled")
            .with_id(PathFilterId(1))
            .with_area_cost(AreaTypeId(1), 4.0),
    );
    grid.register_filter(
        PathFilterProfile::named("utility")
            .with_id(PathFilterId(2))
            .with_allowed_mask(AreaMask::ALL)
            .with_area_cost(AreaTypeId(1), 1.2),
    );
    grid
}

pub fn build_layered_grid() -> PathfindingGrid {
    let config = HpaPathfindingConfig {
        grid_dimensions: UVec3::new(16, 16, 2),
        cluster_size: UVec3::new(8, 8, 1),
        hierarchy_levels: 1,
        neighborhood: saddle_ai_hpa_pathfinding::NeighborhoodMode::Ordinal18,
        ..demo_config(UVec3::new(16, 16, 2))
    };
    let mut path_grid = PathfindingGrid::new(
        GridStorage::new(
            config.grid_dimensions,
            config.origin,
            config.cell_size,
            WorldRoundingPolicy::Floor,
        ),
        config,
    );
    path_grid.add_transition(
        GridCoord::new(7, 7, 0),
        saddle_ai_hpa_pathfinding::TransitionLink::new(
            GridCoord::new(7, 7, 1),
            1.5,
            saddle_ai_hpa_pathfinding::TransitionKind::Stair,
        ),
    );
    path_grid
}

pub fn make_blocked_cell() -> CellData {
    CellData {
        walkable: false,
        ..default()
    }
}

pub fn spawn_grid_camera(commands: &mut Commands) {
    commands.spawn((
        Name::new("Grid Camera"),
        Camera2d,
        Transform::from_xyz(0.0, 0.0, 1000.0),
    ));
}

pub fn spawn_demo_backdrop(
    commands: &mut Commands,
    grid: &PathfindingGrid,
    layout: ExampleLayout,
    title: &str,
    subtitle: &str,
) {
    let size = visual_bounds(grid, layout) + Vec2::splat(grid.grid().space.cell_size * 3.0);
    commands.spawn((
        Name::new("Backdrop"),
        Sprite::from_color(Color::srgb(0.14, 0.15, 0.18), size),
        Transform::from_xyz(0.0, 0.0, 0.1),
    ));
    commands.spawn((
        Name::new("Example Label"),
        Text::new(format!("{title}\n{subtitle}")),
        Node {
            position_type: PositionType::Absolute,
            left: px(18.0),
            top: px(18.0),
            width: px(460.0),
            padding: UiRect::all(px(14.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.03, 0.05, 0.78)),
        TextFont {
            font_size: 17.0,
            ..default()
        },
        TextColor(Color::WHITE),
    ));
}

/// Spawn an on-screen instruction overlay at the bottom-left.
pub fn spawn_instructions(commands: &mut Commands, text: &str) {
    commands.spawn((
        Name::new("Instructions"),
        Text::new(text.to_owned()),
        Node {
            position_type: PositionType::Absolute,
            left: px(18.0),
            bottom: px(18.0),
            width: px(480.0),
            padding: UiRect::all(px(12.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.03, 0.05, 0.72)),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::srgba(0.85, 0.88, 0.92, 0.95)),
    ));
}

pub fn spawn_grid_tiles(
    commands: &mut Commands,
    grid: &PathfindingGrid,
    layout: ExampleLayout,
    overlay_region: Option<GridAabb>,
) {
    let tile_size = Vec2::splat(grid.grid().space.cell_size * 0.92);
    for coord in grid.grid().bounds().iter() {
        let cell = grid
            .grid()
            .cell(coord)
            .expect("coord from bounds should exist");
        let color = cell_visual_color(
            cell,
            overlay_region.is_some_and(|region| region.contains(coord)),
        );
        commands.spawn((
            Name::new(format!(
                "Grid Cell {},{},{}",
                coord.x(),
                coord.y(),
                coord.z()
            )),
            Sprite::from_color(color, tile_size),
            Transform::from_translation(grid_visual_translation(grid, layout, coord, 0.2)),
        ));
    }
}

pub fn spawn_agent_sprite(
    commands: &mut Commands,
    grid: &PathfindingGrid,
    layout: ExampleLayout,
    name: &str,
    coord: GridCoord,
    color: Color,
) -> Entity {
    let cell = grid.grid().space.cell_size;
    let pos = grid_visual_translation(grid, layout, coord, 6.0);
    // Dark outline ring behind the agent for contrast.
    commands.spawn((
        Name::new(format!("{name} Outline")),
        Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.6), Vec2::splat(cell * 0.80)),
        Transform::from_translation(pos - Vec3::Z * 0.1),
    ));
    commands
        .spawn((
            Name::new(name.to_owned()),
            Sprite::from_color(color, Vec2::splat(cell * 0.70)),
            Transform::from_translation(pos),
            GlobalTransform::default(),
        ))
        .id()
}

pub fn spawn_goal_marker(
    commands: &mut Commands,
    grid: &PathfindingGrid,
    layout: ExampleLayout,
    name: &str,
    coord: GridCoord,
    color: Color,
) -> Entity {
    let cell = grid.grid().space.cell_size;
    let pos = grid_visual_translation(grid, layout, coord, 9.0);
    // Dark outline ring behind the goal for contrast.
    commands.spawn((
        Name::new(format!("{name} Outline")),
        Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.5), Vec2::splat(cell * 0.58)),
        Transform::from_translation(pos - Vec3::Z * 0.1),
    ));
    commands
        .spawn((
            Name::new(name.to_owned()),
            Sprite::from_color(color, Vec2::splat(cell * 0.48)),
            Transform::from_translation(pos),
            GlobalTransform::default(),
        ))
        .id()
}

pub fn spawn_layer_labels(commands: &mut Commands, grid: &PathfindingGrid, layout: ExampleLayout) {
    if grid.grid().dimensions.z <= 1 {
        return;
    }

    let height = grid.grid().dimensions.y as f32 * grid.grid().space.cell_size * 0.5
        + grid.grid().space.cell_size;
    for layer in 0..grid.grid().dimensions.z {
        let offset = layer_offset_x(grid, layout, layer as i32);
        commands.spawn((
            Name::new(format!("Layer Label {layer}")),
            Text2d::new(format!("Layer {layer}")),
            TextFont {
                font_size: 20.0,
                ..default()
            },
            TextColor(Color::srgb(0.92, 0.94, 0.98)),
            Transform::from_xyz(offset, height, 10.0),
        ));
    }
}

pub fn grid_visual_translation(
    grid: &PathfindingGrid,
    layout: ExampleLayout,
    coord: GridCoord,
    z: f32,
) -> Vec3 {
    let cell_size = grid.grid().space.cell_size;
    let x = grid.grid().space.origin.x + (coord.x() as f32 + 0.5) * cell_size;
    let y = grid.grid().space.origin.y + (coord.y() as f32 + 0.5) * cell_size;
    Vec3::new(x + layer_offset_x(grid, layout, coord.z()), y, z)
}

pub fn clamp_goal_to_grid(grid: &PathfindingGrid, pane: &HpaExamplePane) -> GridCoord {
    let dimensions = grid.grid().dimensions;
    GridCoord::new(
        pane.goal_x.clamp(0, dimensions.x.saturating_sub(1) as i32),
        pane.goal_y.clamp(0, dimensions.y.saturating_sub(1) as i32),
        pane.goal_layer
            .clamp(0, dimensions.z.saturating_sub(1) as i32),
    )
}

pub fn pane_overlay_region(pane: &HpaExamplePane) -> GridAabb {
    GridAabb::new(
        GridCoord::new(8, 8, pane.goal_layer.max(0)),
        GridCoord::new(14, 12, pane.goal_layer.max(0)),
    )
}

// ---------------------------------------------------------------------------
// Interactive systems for examples
// ---------------------------------------------------------------------------

/// Left-click on the grid to set the goal position. The pane goal_x / goal_y
/// fields are updated, which triggers the normal pane-change flow.
pub fn click_to_set_goal(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    grid: Res<PathfindingGrid>,
    mut pane: ResMut<HpaExamplePane>,
) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };
    let Ok((camera, camera_transform)) = cameras.single() else {
        return;
    };
    let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else {
        return;
    };
    let coord = grid
        .grid()
        .world_to_grid(Vec3::new(world_pos.x, world_pos.y, 0.0));
    let dims = grid.grid().dimensions;
    if coord.x() >= 0 && coord.y() >= 0 && coord.x() < dims.x as i32 && coord.y() < dims.y as i32 {
        pane.goal_x = coord.x();
        pane.goal_y = coord.y();
    }
}

/// Right-click to toggle walkability of the clicked cell.
pub fn click_to_toggle_wall(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    mut grid: ResMut<PathfindingGrid>,
) {
    if !buttons.just_pressed(MouseButton::Right) {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };
    let Ok((camera, camera_transform)) = cameras.single() else {
        return;
    };
    let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else {
        return;
    };
    let coord = grid
        .grid()
        .world_to_grid(Vec3::new(world_pos.x, world_pos.y, 0.0));
    let dims = grid.grid().dimensions;
    if coord.x() >= 0 && coord.y() >= 0 && coord.x() < dims.x as i32 && coord.y() < dims.y as i32 {
        let walkable = grid.grid().cell(coord).map(|c| c.walkable).unwrap_or(false);
        grid.set_walkable(coord, !walkable);
    }
}

/// Keyboard shortcuts for toggling debug visualization layers.
///   G = grid, C = clusters, P = portals, A = abstract graph,
///   H = heatmap, D = paths (draw paths)
pub fn keyboard_debug_shortcuts(keys: Res<ButtonInput<KeyCode>>, mut pane: ResMut<HpaExamplePane>) {
    if keys.just_pressed(KeyCode::KeyG) {
        pane.draw_grid = !pane.draw_grid;
    }
    if keys.just_pressed(KeyCode::KeyC) {
        pane.draw_clusters = !pane.draw_clusters;
    }
    if keys.just_pressed(KeyCode::KeyP) {
        pane.draw_portals = !pane.draw_portals;
    }
    if keys.just_pressed(KeyCode::KeyA) {
        pane.draw_abstract_graph = !pane.draw_abstract_graph;
    }
    if keys.just_pressed(KeyCode::KeyH) {
        pane.draw_heatmap = !pane.draw_heatmap;
    }
    if keys.just_pressed(KeyCode::KeyD) {
        pane.draw_paths = !pane.draw_paths;
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn cell_visual_color(cell: &CellData, overlay: bool) -> Color {
    let mut color = if !cell.walkable {
        Color::srgb(0.72, 0.28, 0.22)
    } else if cell.area == AreaTypeId(1) {
        Color::srgb(0.24, 0.38, 0.54)
    } else {
        Color::srgb(0.34, 0.36, 0.42)
    };

    if overlay && cell.walkable {
        color = Color::srgb(0.50, 0.40, 0.16);
    }
    color
}

fn visual_bounds(grid: &PathfindingGrid, layout: ExampleLayout) -> Vec2 {
    let width = grid.grid().dimensions.x as f32 * grid.grid().space.cell_size;
    let height = grid.grid().dimensions.y as f32 * grid.grid().space.cell_size;
    match layout {
        ExampleLayout::Single => Vec2::new(width, height),
        ExampleLayout::Layered { spacing_cells } => {
            let layers = grid.grid().dimensions.z.max(1) as f32;
            let spacing = spacing_cells * grid.grid().space.cell_size;
            Vec2::new(width * layers + spacing * (layers - 1.0), height)
        }
    }
}

fn layer_offset_x(grid: &PathfindingGrid, layout: ExampleLayout, layer: i32) -> f32 {
    match layout {
        ExampleLayout::Single => 0.0,
        ExampleLayout::Layered { spacing_cells } => {
            let layers = grid.grid().dimensions.z.max(1) as f32;
            let stride = grid.grid().dimensions.x as f32 * grid.grid().space.cell_size
                + spacing_cells * grid.grid().space.cell_size;
            (layer as f32 - (layers - 1.0) * 0.5) * stride
        }
    }
}

fn auto_exit_from_env() -> Option<ExampleAutoExit> {
    let seconds = std::env::var("HPA_PATHFINDING_EXAMPLE_EXIT_AFTER_SECONDS")
        .ok()?
        .parse::<f32>()
        .ok()?;
    Some(ExampleAutoExit(Timer::from_seconds(
        seconds.max(0.1),
        TimerMode::Once,
    )))
}

fn auto_exit_example(
    time: Res<Time>,
    mut auto_exit: ResMut<ExampleAutoExit>,
    mut exit: MessageWriter<AppExit>,
) {
    if auto_exit.0.tick(time.delta()).just_finished() {
        exit.write(AppExit::Success);
    }
}

// ---------------------------------------------------------------------------
// E2E support (behind `e2e` feature)
// ---------------------------------------------------------------------------

#[cfg(feature = "e2e")]
pub mod e2e_support {
    use bevy::prelude::*;
    use saddle_bevy_e2e::{action::Action, scenario::Scenario};

    /// Reusable E2E plugin for individual examples.
    ///
    /// Pass two function pointers: one to list available scenario names,
    /// one to build a scenario by name. The plugin parses CLI args and
    /// initialises the runner automatically.
    pub struct ExampleE2EPlugin {
        list_fn: fn() -> Vec<&'static str>,
        build_fn: fn(&str) -> Option<Scenario>,
    }

    impl ExampleE2EPlugin {
        pub fn new(
            list_fn: fn() -> Vec<&'static str>,
            build_fn: fn(&str) -> Option<Scenario>,
        ) -> Self {
            Self { list_fn, build_fn }
        }
    }

    impl Plugin for ExampleE2EPlugin {
        fn build(&self, app: &mut App) {
            app.add_plugins(saddle_bevy_e2e::E2EPlugin);

            let args: Vec<String> = std::env::args().collect();
            let (scenario_name, handoff) = parse_e2e_args(&args);

            if let Some(name) = scenario_name {
                if let Some(mut scenario) = (self.build_fn)(&name) {
                    if handoff {
                        scenario.actions.push(Action::Handoff);
                    }
                    saddle_bevy_e2e::init_scenario(app, scenario);
                } else {
                    error!(
                        "[e2e] Unknown scenario '{name}'. Available: {:?}",
                        (self.list_fn)()
                    );
                }
            }
        }
    }

    fn parse_e2e_args(args: &[String]) -> (Option<String>, bool) {
        let mut scenario_name = None;
        let mut handoff = false;

        for arg in args.iter().skip(1) {
            if arg == "--handoff" {
                handoff = true;
            } else if !arg.starts_with('-') && scenario_name.is_none() {
                scenario_name = Some(arg.clone());
            }
        }

        if !handoff {
            handoff =
                std::env::var("E2E_HANDOFF").is_ok_and(|v| v == "1" || v == "true");
        }

        (scenario_name, handoff)
    }
}
