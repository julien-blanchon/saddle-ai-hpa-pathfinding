# Saddle AI HPA Pathfinding

Reusable hierarchical grid pathfinding for Bevy.

The crate owns a grid and hierarchy snapshot, not an AI behavior stack. It plans global routes across 2D, layered 2.5D, and 3D voxel-like grids with HPA*-style portal abstraction, dirty-region rebuilds, async query orchestration, cacheable filter profiles, and opt-in debug drawing.

For apps where the runtime should stay active for the full app lifetime, prefer `HpaPathfindingPlugin::always_on(Update)`. Use `HpaPathfindingPlugin::new(...)` when activation should follow explicit schedules such as `OnEnter` / `OnExit`.

## Quick Start

```toml
[dependencies]
saddle-ai-hpa-pathfinding = { git = "https://github.com/julien-blanchon/saddle-ai-hpa-pathfinding" }
```

```rust,no_run
use bevy::prelude::*;
use saddle_ai_hpa_pathfinding::{
    GridCoord, HpaPathfindingConfig, HpaPathfindingPlugin, NeighborhoodMode, PathCostOverlay,
    PathFilterProfile, PathRequest, PathfindingAgent, PathfindingGrid,
};

#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum DemoState {
    #[default]
    Active,
}

fn main() {
    let grid = PathfindingGrid::from_config(HpaPathfindingConfig {
        grid_dimensions: UVec3::new(32, 32, 1),
        cluster_size: UVec3::new(8, 8, 1),
        neighborhood: NeighborhoodMode::Ordinal2d,
        ..default()
    });

    App::new()
        .add_plugins(DefaultPlugins)
        .init_state::<DemoState>()
        .insert_resource(grid)
        .add_plugins(HpaPathfindingPlugin::new(
            OnEnter(DemoState::Active),
            OnExit(DemoState::Active),
            Update,
        ))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, mut grid: ResMut<PathfindingGrid>) {
        grid.register_filter(PathFilterProfile::named("default"));

        commands.spawn((
            Name::new("Agent"),
            PathfindingAgent::default(),
            PathRequest::new(GridCoord::new(28, 28, 0)).with_overlays(vec![PathCostOverlay::new(
                saddle_ai_hpa_pathfinding::GridAabb::new(
                    GridCoord::new(12, 12, 0),
                    GridCoord::new(18, 18, 0),
                ),
                4.0,
            )]),
            Transform::from_xyz(1.5, 1.5, 0.0),
            GlobalTransform::default(),
        ));
}
```

## Public API

- Plugin: `HpaPathfindingPlugin`
- System sets:
  `HpaPathfindingSystems::{DetectChanges, RebuildHierarchy, EnqueueQueries, ProcessQueries, ValidatePaths, PublishResults, DebugDraw}`
- Core resources:
  `PathfindingGrid`, `HpaPathfindingConfig`, `PathfindingStats`
- Core coordinate and filter types:
  `GridCoord`, `GridAabb`, `PathQueryId`, `PathVersion`, `AreaTypeId`, `PathFilterId`, `PathFilterProfile`, `PathCostOverlay`
- ECS components:
  `PathfindingAgent`, `PathRequest`, `PendingPathQuery`, `ComputedPath`, `PathfindingObstacle`
- Messages:
  `GridRegionChanged`, `PathReady`, `PathInvalidated`
- Pure query helpers:
  `find_path`, `estimate_cost`, `nearest_walkable_cell`, `line_of_sight`,
  plus `PathfindingGrid::{query_path, query_path_sliced, nearest_walkable, raycast_line_of_sight, estimate_cost}`

## Feature Boundaries

Included:

- hierarchical route planning on reusable grids
- filter- and mask-aware path queries
- query-scoped cost overlays through pure calls and ECS `PathRequest`
- dirty-region rebuilds and cache invalidation
- ECS obstacle synchronization for blocked dynamic regions
- async and sliced query orchestration
- diagnostics, timings, and debug overlays

Not included:

- local steering or crowd avoidance
- animation or locomotion playback
- game-specific AI state machines or planners
- triangle navmesh generation

## Examples

| Example | Purpose | Run |
| --- | --- | --- |
| `basic` | Minimal sync query and ECS request flow on a 2D grid | `cargo run -p saddle-ai-hpa-pathfinding-example-basic` |
| `dynamic_obstacles` | Region edits, dirty rebuilds, and path invalidation | `cargo run -p saddle-ai-hpa-pathfinding-example-dynamic-obstacles` |
| `layered_2_5d` | Layered grid with explicit vertical transitions | `cargo run -p saddle-ai-hpa-pathfinding-example-layered-2-5d` |
| `large_grid` | Large-map hierarchy and stats overlay | `cargo run -p saddle-ai-hpa-pathfinding-example-large-grid` |
| `async_queries` | Async ECS query queue, deduplication, and publication | `cargo run -p saddle-ai-hpa-pathfinding-example-async-queries` |
| `filters_and_costs` | Filter profiles, terrain masks, and query-time overlays | `cargo run -p saddle-ai-hpa-pathfinding-example-filters-and-costs` |
| `debug_viz` | Debug layers for clusters, portals, paths, and the cost heatmap | `cargo run -p saddle-ai-hpa-pathfinding-example-debug-viz` |
| `saddle-ai-hpa-pathfinding-lab` | Crate-local showcase with BRP and E2E hooks | `cargo run -p saddle-ai-hpa-pathfinding-lab` |

## Crate-Local Lab

`shared/ai/saddle-ai-hpa-pathfinding/examples/lab` is the richer verification surface for this crate. It keeps BRP and E2E scenarios inside the shared crate instead of pushing them into project-level sandboxes.

```bash
cargo run -p saddle-ai-hpa-pathfinding-lab
```

E2E commands:

```bash
cargo run -p saddle-ai-hpa-pathfinding-lab --features e2e -- smoke_launch
cargo run -p saddle-ai-hpa-pathfinding-lab --features e2e -- hpa_pathfinding_smoke
cargo run -p saddle-ai-hpa-pathfinding-lab --features e2e -- hpa_pathfinding_dynamic
cargo run -p saddle-ai-hpa-pathfinding-lab --features e2e -- hpa_pathfinding_filters
cargo run -p saddle-ai-hpa-pathfinding-lab --features e2e -- hpa_pathfinding_large_grid
```

## BRP

Useful BRP commands against the lab:

```bash
HPA_PATHFINDING_LAB_BRP_PORT=15713 \
uv run --active --project .codex/skills/bevy-brp/script brp app launch saddle-ai-hpa-pathfinding-lab
uv run --active --project .codex/skills/bevy-brp/script brp world query bevy_ecs::name::Name
uv run --active --project .codex/skills/bevy-brp/script brp world query saddle_ai_hpa_pathfinding::components::PathfindingAgent
uv run --active --project .codex/skills/bevy-brp/script brp world query saddle_ai_hpa_pathfinding::components::ComputedPath
uv run --active --project .codex/skills/bevy-brp/script brp world resource saddle_ai_hpa_pathfinding::config::HpaPathfindingConfig
uv run --active --project .codex/skills/bevy-brp/script brp world resource saddle_ai_hpa_pathfinding::stats::PathfindingStats
uv run --active --project .codex/skills/bevy-brp/script brp extras screenshot /tmp/saddle_ai_hpa_pathfinding_lab.png
uv run --active --project .codex/skills/bevy-brp/script brp extras shutdown
```

If the local renderer is unavailable, launch the lab headlessly for BRP-only inspection:

```bash
HPA_PATHFINDING_LAB_HEADLESS=1 cargo run -p saddle-ai-hpa-pathfinding-lab
```

Headless mode keeps the ECS/runtime state inspectable but does not support screenshots or E2E capture.

## More Docs

- [Architecture](docs/architecture.md)
- [Configuration](docs/configuration.md)
- [Performance](docs/performance.md)
- [Debugging](docs/debugging.md)

## Benchmarks

Release-mode benchmark-style regression tests live in ignored unit tests:

```bash
cargo test -p saddle-ai-hpa-pathfinding --release -- --ignored
```
