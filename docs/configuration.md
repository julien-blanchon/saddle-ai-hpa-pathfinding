# Configuration

## `HpaPathfindingConfig`

| Field | Type | Default | Effect |
| --- | --- | --- | --- |
| `grid_dimensions` | `UVec3` | `(64, 64, 1)` | Logical cell dimensions of the grid. |
| `origin` | `Vec3` | `Vec3::ZERO` | World-space origin of cell `(0, 0, 0)`. |
| `cell_size` | `f32` | `1.0` | Uniform cell edge length used by world/grid conversion helpers. |
| `world_rounding` | `WorldRoundingPolicy` | `Floor` | Explicit world-to-grid conversion policy. |
| `cluster_size` | `UVec3` | `(16, 16, 1)` | Level-1 cluster size in cells. |
| `hierarchy_levels` | `u8` | `2` | Number of shortcut levels to build. Valid range is `1..=3`. |
| `neighborhood` | `NeighborhoodMode` | `Ordinal2d` | Base neighborhood used for low-level search. |
| `allow_corner_cutting` | `bool` | `false` | Whether diagonal movement may clip corners through blocked side cells. |
| `direct_search_distance` | `u32` | `16` | Requests shorter than this use direct A* first. |
| `rebuild_budget_per_frame` | `u32` | `2` | Maximum dirty clusters rebuilt per update. |
| `max_queries_per_frame` | `u32` | `8` | Maximum queued requests started per frame. |
| `max_sliced_expansions_per_frame` | `u32` | `96` | Maximum low-level node expansions for sliced direct-grid queries each frame. |
| `cache_capacity` | `usize` | `128` | Maximum cached path entries before LRU eviction. |
| `cache_ttl_frames` | `u64` | `0` | Optional cache expiry in update ticks. `0` disables TTL expiry. |
| `smoothing_mode` | `PathSmoothingMode` | `LineOfSight` | Post-process applied after corridor validation. |
| `deterministic` | `bool` | `true` | Keeps queue aging and publication ordering stable for reproducible ECS request handling. |
| `debug_draw_grid` | `bool` | `false` | Draw cell centers and blocked cells. |
| `debug_draw_clusters` | `bool` | `false` | Draw level bounds and labels by color. |
| `debug_draw_portals` | `bool` | `false` | Draw portal endpoints and crossings. |
| `debug_draw_abstract_graph` | `bool` | `false` | Draw abstract graph edges. |
| `debug_draw_paths` | `bool` | `true` | Draw active computed corridors and waypoints. |
| `debug_draw_dirty_clusters` | `bool` | `false` | Highlight clusters awaiting rebuild. |
| `debug_draw_cost_heatmap` | `bool` | `false` | Draw base cell costs through the default filter profile. Query-time overlays stay request-local. |

## Tradeoffs

### `cluster_size`

- Smaller clusters:
  - more portals
  - more detailed abstract graph
  - more rebuild work per map area
  - better short-range fidelity
- Larger clusters:
  - fewer portals
  - faster high-level search
  - more expensive intra-cluster repairs

`16x16x1` is a strong default for broad 2D grids. Dense mazes often prefer `8x8x1`. Sparse large worlds often prefer `16x16x2` or `16x16x4`.

### `hierarchy_levels`

- `1`: lowest memory cost, simplest rebuilds, good for medium grids
- `2`: strong default for large colony/factory maps
- `3`: only worth it for very large worlds or repeated long-distance requests

### `direct_search_distance`

Use a higher value when:

- most requests are room-sized
- exact per-cell costs matter
- the hierarchy would be overhead for nearby targets

Use a lower value when:

- most requests are map-scale
- the abstract graph is much smaller than the grid

### `deterministic`

When `true`, the ECS runtime uses stable ordering for:

- queue aging sort
- cache invalidation order
- publication order for completed requests

Disable only if you do not care about reproducible request ordering in the ECS wrapper.

## Filter Profiles

`PathFilterProfile` supports:

- `allowed_mask`
- `blocked_mask`
- `clearance`
- area-specific cost multipliers

Profiles are registered on the grid with a stable `PathFilterId` so ECS agents can refer to them cheaply.

## Agent Clearance

`PathfindingAgent.clearance` and the `*_with_clearance` query helpers enforce a minimum free square around each traversed cell.

- `0`: agent uses the raw filter profile only
- `1+`: requires at least that many contiguous walkable cells of clearance

Use clearance when:

- the same grid serves small scouts and large vehicles
- agents should reject narrow hallways without creating duplicate grids
- nearest-walkable queries must find a cell that actually fits the agent body

## Temporary Overlays

`PathCostOverlay` adds a cost to all cells inside a `GridAabb` for one request. Overlays are best for:

- danger zones
- congestion fields
- temporary soft reservations

Overlays do not mutate the stored grid. They only influence that request, including ECS `PathRequest` components.

## Flow Fields

`PathfindingGrid::build_flow_field` and `build_flow_field_with_clearance` generate a reusable one-goal field from the currently committed snapshot.

Flow fields are best for:

- RTS groups converging on one rally point
- evacuation or lane-routing visualizations
- combining long-range HPA planning with local movement systems that only need the next best step
