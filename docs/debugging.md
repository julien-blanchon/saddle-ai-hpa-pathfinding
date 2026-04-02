# Debugging

## BRP Workflows

Launch the crate-local lab:

```bash
HPA_PATHFINDING_LAB_BRP_PORT=15713 \
uv run --active --project .codex/skills/bevy-brp/script brp app launch saddle-ai-hpa-pathfinding-lab
```

Headless fallback for machines where Bevy rendering cannot boot:

```bash
HPA_PATHFINDING_LAB_HEADLESS=1 cargo run -p saddle-ai-hpa-pathfinding-lab
```

That mode keeps BRP resource/component inspection available but disables screenshot-oriented verification.

Inspect agents and paths:

```bash
uv run --active --project .codex/skills/bevy-brp/script brp world query bevy_ecs::name::Name
uv run --active --project .codex/skills/bevy-brp/script brp world query saddle_ai_hpa_pathfinding::components::PathfindingAgent
uv run --active --project .codex/skills/bevy-brp/script brp world query saddle_ai_hpa_pathfinding::components::ComputedPath
uv run --active --project .codex/skills/bevy-brp/script brp world query saddle_ai_hpa_pathfinding::components::PathfindingObstacle
uv run --active --project .codex/skills/bevy-brp/script brp resource get saddle_ai_hpa_pathfinding::stats::PathfindingStats
uv run --active --project .codex/skills/bevy-brp/script brp resource get saddle_ai_hpa_pathfinding::config::HpaPathfindingConfig
```

Toggle debug layers live:

```bash
uv run --active --project .codex/skills/bevy-brp/script brp resource mutate saddle_ai_hpa_pathfinding::config::HpaPathfindingConfig 'true' --path .debug_draw_clusters
uv run --active --project .codex/skills/bevy-brp/script brp resource mutate saddle_ai_hpa_pathfinding::config::HpaPathfindingConfig 'true' --path .debug_draw_portals
uv run --active --project .codex/skills/bevy-brp/script brp resource mutate saddle_ai_hpa_pathfinding::config::HpaPathfindingConfig 'true' --path .debug_draw_paths
```

Capture screenshots:

```bash
uv run --active --project .codex/skills/bevy-brp/script brp extras screenshot /tmp/saddle_ai_hpa_pathfinding_debug.png
```

## Failure Modes

### Path should exist but query returns none

Inspect:

- `PathfindingStats.last_failed_queries`
- `debug_draw_portals`
- `debug_draw_clusters`

Likely causes:

- grid cell marked blocked unexpectedly
- start or goal mapped to the wrong cell
- filter mask excludes the only corridor
- level-1 portal generation missed a border due to bad cluster sizing

### Long path hitches

Inspect:

- `PathfindingStats.queue_depth`
- `PathfindingStats.async_in_flight`
- `PathfindingStats.sliced_expansions`

Try:

- enable sliced mode
- raise hierarchy level count
- lower cluster size if direct repairs dominate

### Paths fail after an edit

Inspect:

- `PathfindingStats.dirty_cluster_count`
- `debug_draw_dirty_clusters`
- `PathInvalidated` messages

Likely causes:

- the edit was applied but `GridRegionChanged` was not emitted
- the `PathfindingObstacle` component was added or removed on the wrong entity
- rebuild budget is too small and the hierarchy is still dirty
- the path cache was not invalidated because the wrong region was marked dirty

### Same request changes across runs

Inspect:

- `HpaPathfindingConfig.deterministic`
- filter registration order
- any query-time overlays

If determinism matters, keep `deterministic = true` and use stable filter ids.

### Diagonal clipping or corner cutting bugs

Inspect:

- `HpaPathfindingConfig.allow_corner_cutting`
- `debug_draw_grid`
- direct A* unit tests around the failing cell pair

### Layered 2.5D routes fail to connect

Inspect:

- transition links on the intended stair or lift cells
- `debug_draw_portals`
- `raycast_line_of_sight` or direct search around the transition

## Debug Layers

Recommended layer combinations:

- topology debugging:
  `debug_draw_grid + debug_draw_clusters + debug_draw_portals`
- search debugging:
  `debug_draw_abstract_graph + debug_draw_paths`
- rebuild debugging:
  `debug_draw_dirty_clusters + debug_draw_clusters`
- cost debugging:
  `debug_draw_cost_heatmap + debug_draw_paths`

`debug_draw_cost_heatmap` shows base stored cell costs through the default filter profile. It is useful for persistent terrain costs, not for one-off request overlays.
