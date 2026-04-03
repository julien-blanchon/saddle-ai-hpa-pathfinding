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
    pub debug_enabled: bool,
    #[pane(toggle)]
    pub draw_grid: bool,
    #[pane(toggle)]
    pub draw_clusters: bool,
    #[pane(toggle)]
    pub draw_portals: bool,
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
            debug_enabled: true,
            draw_grid: false,
            draw_clusters: true,
            draw_portals: true,
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
    app.insert_resource(ClearColor(Color::srgb(0.05, 0.06, 0.08)));
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

    config.debug_draw_paths = pane.debug_enabled;
    config.debug_draw_grid = pane.draw_grid;
    config.debug_draw_clusters = pane.draw_clusters;
    config.debug_draw_portals = pane.draw_portals;
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
    for x in 6..dimensions.x.saturating_sub(6) as i32 {
        grid.set_walkable(GridCoord::new(x, (dimensions.y / 2) as i32, 0), false);
    }
    let gate = GridCoord::new((dimensions.x / 2) as i32, (dimensions.y / 2) as i32, 0);
    grid.set_walkable(gate, true);
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
        Sprite::from_color(Color::srgb(0.09, 0.10, 0.12), size),
        Transform::from_xyz(0.0, 0.0, -50.0),
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

pub fn spawn_grid_tiles(
    commands: &mut Commands,
    grid: &PathfindingGrid,
    layout: ExampleLayout,
    overlay_region: Option<GridAabb>,
) {
    let tile_size = Vec2::splat(grid.grid().space.cell_size * 0.88);
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
            Transform::from_translation(grid_visual_translation(grid, layout, coord, -5.0)),
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
    commands
        .spawn((
            Name::new(name.to_owned()),
            Sprite::from_color(color, Vec2::splat(grid.grid().space.cell_size * 0.56)),
            Transform::from_translation(grid_visual_translation(grid, layout, coord, 5.0)),
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
    commands
        .spawn((
            Name::new(name.to_owned()),
            Sprite::from_color(color, Vec2::splat(grid.grid().space.cell_size * 0.28)),
            Transform::from_translation(grid_visual_translation(grid, layout, coord, 8.0)),
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

fn cell_visual_color(cell: &CellData, overlay: bool) -> Color {
    let mut color = if !cell.walkable {
        Color::srgb(0.68, 0.24, 0.20)
    } else if cell.area == AreaTypeId(1) {
        Color::srgb(0.20, 0.34, 0.52)
    } else {
        Color::srgb(0.20, 0.24, 0.28)
    };

    if overlay && cell.walkable {
        color = Color::srgb(0.58, 0.42, 0.18);
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
