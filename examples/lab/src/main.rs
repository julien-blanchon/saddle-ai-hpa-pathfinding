#[cfg(feature = "e2e")]
mod e2e;
#[cfg(feature = "e2e")]
mod scenarios;

#[cfg(all(feature = "dev", not(target_arch = "wasm32")))]
use bevy::remote::{RemotePlugin, http::RemoteHttpPlugin};
use bevy::{
    app::ScheduleRunnerPlugin,
    camera::{OrthographicProjection, Projection},
    prelude::*,
};
#[cfg(all(feature = "dev", not(target_arch = "wasm32")))]
use bevy_brp_extras::BrpExtrasPlugin;
use saddle_ai_hpa_pathfinding::{
    AreaMask, AreaTypeId, ComputedPath, GridCoord, HpaPathfindingPlugin, NeighborhoodMode,
    ObstacleShape, PathFilterId, PathFilterProfile, PathInvalidated, PathRequest, PathfindingAgent,
    PathfindingGrid, PathfindingObstacle, PathfindingStats,
};

const DEFAULT_LAB_BRP_PORT: u16 = 15_713;
const GATE_COORD: GridCoord = GridCoord(IVec3::new(16, 12, 0));

#[derive(Component)]
struct SmokeAgent;

#[derive(Component)]
struct DynamicAgent;

#[derive(Component)]
struct WheeledAgent;

#[derive(Component)]
struct UtilityAgent;

#[derive(Component)]
struct StressAgent;

#[derive(Component)]
struct GateVisual;

#[derive(Component)]
struct GateObstacleMarker;

#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct LabControl {
    pub gate_blocked: bool,
}

#[derive(Resource, Debug, Clone, Default)]
pub struct LabDiagnostics {
    pub smoke_ready: bool,
    pub smoke_cost: f32,
    pub smoke_version: u64,
    pub dynamic_cost_before: f32,
    pub dynamic_cost_after: f32,
    pub wheeled_cost: f32,
    pub utility_cost: f32,
    pub invalidations: u64,
    pub stress_completed: u64,
    pub queue_depth: usize,
}

fn main() {
    let mut app = App::new();
    let headless = lab_headless();
    app.insert_resource(ClearColor(Color::srgb(0.12, 0.13, 0.15)));
    app.insert_resource(LabControl::default());
    app.insert_resource(LabDiagnostics::default());
    app.insert_resource(build_lab_config());
    app.insert_resource(build_lab_grid());
    if headless {
        app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(
            std::time::Duration::from_secs_f64(1.0 / 60.0),
        )));
        #[cfg(all(feature = "dev", not(target_arch = "wasm32")))]
        app.add_plugins((
            RemotePlugin::default(),
            RemoteHttpPlugin::default().with_port(lab_brp_port()),
        ));
    } else {
        app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "saddle-ai-hpa-pathfinding crate-local lab".into(),
                resolution: (1400, 920).into(),
                ..default()
            }),
            ..default()
        }));
        #[cfg(all(feature = "dev", not(target_arch = "wasm32")))]
        app.add_plugins((
            RemotePlugin::default(),
            BrpExtrasPlugin::with_http_plugin(
                RemoteHttpPlugin::default().with_port(lab_brp_port()),
            ),
        ));
        #[cfg(feature = "e2e")]
        app.add_plugins(e2e::HpaPathfindingLabE2EPlugin);
    }
    app.add_plugins(HpaPathfindingPlugin::default());
    app.add_systems(Startup, setup);
    app.add_systems(
        Update,
        update_diagnostics.after(saddle_ai_hpa_pathfinding::HpaPathfindingSystems::PublishResults),
    );
    app.add_systems(
        Update,
        track_invalidations.after(saddle_ai_hpa_pathfinding::HpaPathfindingSystems::ValidatePaths),
    );
    app.add_systems(
        Update,
        sync_gate_visual.after(saddle_ai_hpa_pathfinding::HpaPathfindingSystems::DetectChanges),
    );
    app.run();
}

#[cfg(all(feature = "dev", not(target_arch = "wasm32")))]
fn lab_brp_port() -> u16 {
    std::env::var("HPA_PATHFINDING_LAB_BRP_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(DEFAULT_LAB_BRP_PORT)
}

#[cfg(any(not(feature = "dev"), target_arch = "wasm32"))]
fn lab_brp_port() -> u16 {
    DEFAULT_LAB_BRP_PORT
}

fn lab_headless() -> bool {
    std::env::var("HPA_PATHFINDING_LAB_HEADLESS")
        .is_ok_and(|value| value == "1" || value.eq_ignore_ascii_case("true"))
}

fn build_lab_config() -> saddle_ai_hpa_pathfinding::HpaPathfindingConfig {
    let cell_size = 24.0;
    let dimensions = UVec3::new(32, 24, 1);
    saddle_ai_hpa_pathfinding::HpaPathfindingConfig {
        grid_dimensions: dimensions,
        origin: Vec3::new(
            -(dimensions.x as f32 * cell_size) * 0.5,
            -(dimensions.y as f32 * cell_size) * 0.5,
            0.0,
        ),
        cell_size,
        cluster_size: UVec3::new(8, 8, 1),
        hierarchy_levels: 2,
        neighborhood: NeighborhoodMode::Ordinal2d,
        max_queries_per_frame: 16,
        max_sliced_expansions_per_frame: 48,
        debug_draw_clusters: true,
        debug_draw_portals: true,
        debug_draw_abstract_graph: true,
        debug_draw_paths: true,
        debug_draw_grid: false,
        ..default()
    }
}

fn build_lab_grid() -> PathfindingGrid {
    let config = build_lab_config();
    let mut grid = PathfindingGrid::new(
        saddle_ai_hpa_pathfinding::GridStorage::new(
            config.grid_dimensions,
            config.origin,
            config.cell_size,
            config.world_rounding,
        ),
        config,
    );

    for x in 5..27 {
        if x != GATE_COORD.x() {
            grid.set_walkable(GridCoord::new(x, GATE_COORD.y(), 0), false);
        }
    }
    grid.fill_region(
        saddle_ai_hpa_pathfinding::GridAabb::new(GridCoord::new(3, 15, 0), GridCoord::new(24, 18, 0)),
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
            .with_area_cost(AreaTypeId(1), 1.2),
    );
    grid
}

fn setup(mut commands: Commands, grid: Res<PathfindingGrid>) {
    commands.spawn((
        Name::new("Lab Camera"),
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scale: 1.0,
            ..OrthographicProjection::default_2d()
        }),
        Transform::from_xyz(0.0, 0.0, 1000.0),
    ));
    spawn_grid_visuals(&mut commands, grid.as_ref());

    spawn_agent(
        &mut commands,
        "Smoke Agent",
        SmokeAgent,
        PathFilterId(0),
        GridCoord::new(2, 2, 0),
        GridCoord::new(28, 20, 0),
        Color::srgb(0.95, 0.84, 0.24),
        grid.as_ref(),
    );
    spawn_agent(
        &mut commands,
        "Dynamic Agent",
        DynamicAgent,
        PathFilterId(0),
        GridCoord::new(2, 5, 0),
        GridCoord::new(28, 18, 0),
        Color::srgb(0.98, 0.53, 0.22),
        grid.as_ref(),
    );
    spawn_agent(
        &mut commands,
        "Wheeled Agent",
        WheeledAgent,
        PathFilterId(1),
        GridCoord::new(2, 21, 0),
        GridCoord::new(28, 4, 0),
        Color::srgb(0.28, 0.72, 0.97),
        grid.as_ref(),
    );
    spawn_agent(
        &mut commands,
        "Utility Agent",
        UtilityAgent,
        PathFilterId(2),
        GridCoord::new(3, 20, 0),
        GridCoord::new(28, 4, 0),
        Color::srgb(0.34, 0.92, 0.54),
        grid.as_ref(),
    );

    for index in 0..8 {
        spawn_agent(
            &mut commands,
            &format!("Stress Agent {index}"),
            StressAgent,
            PathFilterId(0),
            GridCoord::new(2 + (index % 4), 8 + index / 2, 0),
            GridCoord::new(29 - (index % 3), 20 - (index % 4), 0),
            Color::srgba(0.86, 0.88, 0.92, 0.65),
            grid.as_ref(),
        );
    }
}

fn spawn_grid_visuals(commands: &mut Commands, grid: &PathfindingGrid) {
    let tile_size = Vec2::splat(grid.grid().space.cell_size * 0.92);
    commands.spawn((
        Name::new("Grid Backdrop"),
        Sprite::from_color(
            Color::srgb(0.34, 0.36, 0.40),
            Vec2::new(
                grid.grid().dimensions.x as f32 * grid.grid().space.cell_size,
                grid.grid().dimensions.y as f32 * grid.grid().space.cell_size,
            ),
        ),
        Transform::from_xyz(0.0, 0.0, -0.6),
    ));

    for coord in grid.grid().bounds().iter() {
        let cell = grid.grid().cell(coord).unwrap();
        let mut color = Color::srgb(0.42, 0.44, 0.48);
        if cell.area == AreaTypeId(1) {
            color = Color::srgb(0.28, 0.40, 0.56);
        }
        if !cell.walkable {
            color = Color::srgb(0.82, 0.34, 0.26);
        }
        commands.spawn((
            Name::new("Cell"),
            Sprite::from_color(color, tile_size),
            Transform::from_translation(grid.grid().grid_to_world_center(coord).with_z(-0.2)),
        ));
    }

    commands.spawn((
        Name::new("Gate Visual"),
        GateVisual,
        Sprite::from_color(Color::srgba(0.95, 0.18, 0.22, 0.0), tile_size),
        Transform::from_translation(grid.grid().grid_to_world_center(GATE_COORD).with_z(0.5)),
    ));
    commands.spawn((Name::new("Gate Obstacle"), GateObstacleMarker));
}

fn spawn_agent<T: Component>(
    commands: &mut Commands,
    name: &str,
    marker: T,
    filter: PathFilterId,
    start: GridCoord,
    goal: GridCoord,
    color: Color,
    grid: &PathfindingGrid,
) {
    let position = grid.grid().grid_to_world_center(start).with_z(1.0);
    let agent_size = Vec2::splat(grid.grid().space.cell_size * 0.62);
    commands.spawn((
        Name::new(name.to_string()),
        marker,
        Sprite::from_color(color, agent_size),
        Transform::from_translation(position),
        GlobalTransform::from_translation(position),
        PathfindingAgent {
            filter,
            clearance: 0,
            request_priority: if filter == PathFilterId(0) { 0 } else { 1 },
        },
        PathRequest::new(goal),
    ));
}

pub fn set_gate_blocked(world: &mut World, blocked: bool) {
    world.resource_mut::<LabControl>().gate_blocked = blocked;
    let mut query = world.query_filtered::<Entity, With<GateObstacleMarker>>();
    let Some(entity) = query.iter(world).next() else {
        return;
    };
    if blocked {
        world.entity_mut(entity).insert(PathfindingObstacle {
            shape: ObstacleShape::Cell(GATE_COORD),
            area_override: None,
        });
    } else {
        world.entity_mut(entity).remove::<PathfindingObstacle>();
    }
}

fn sync_gate_visual(control: Res<LabControl>, mut query: Query<&mut Sprite, With<GateVisual>>) {
    if !control.is_changed() {
        return;
    }
    for mut sprite in &mut query {
        sprite.color = if control.gate_blocked {
            Color::srgba(0.95, 0.18, 0.22, 0.85)
        } else {
            Color::srgba(0.95, 0.18, 0.22, 0.0)
        };
    }
}

fn track_invalidations(
    mut diagnostics: ResMut<LabDiagnostics>,
    mut invalidated: MessageReader<PathInvalidated>,
) {
    diagnostics.invalidations += invalidated.read().count() as u64;
}

fn update_diagnostics(
    stats: Res<PathfindingStats>,
    smoke: Query<&ComputedPath, With<SmokeAgent>>,
    dynamic: Query<&ComputedPath, With<DynamicAgent>>,
    wheeled: Query<&ComputedPath, With<WheeledAgent>>,
    utility: Query<&ComputedPath, With<UtilityAgent>>,
    stress: Query<&ComputedPath, With<StressAgent>>,
    mut diagnostics: ResMut<LabDiagnostics>,
    control: Res<LabControl>,
) {
    diagnostics.queue_depth = stats.queue_depth;
    diagnostics.stress_completed = stress.iter().count() as u64;

    if let Ok(path) = smoke.single() {
        diagnostics.smoke_ready = true;
        diagnostics.smoke_cost = path.total_cost;
        diagnostics.smoke_version = path.path_version.0;
    }

    if let Ok(path) = dynamic.single() {
        if !control.gate_blocked {
            diagnostics.dynamic_cost_before = path.total_cost;
        } else {
            diagnostics.dynamic_cost_after = path.total_cost;
        }
    }

    if let Ok(path) = wheeled.single() {
        diagnostics.wheeled_cost = path.total_cost;
    }

    if let Ok(path) = utility.single() {
        diagnostics.utility_cost = path.total_cost;
    }
}
