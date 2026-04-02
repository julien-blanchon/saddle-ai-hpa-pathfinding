# Architecture

## Overview

`saddle-ai-hpa-pathfinding` is split into two layers:

1. Pure pathfinding core
   - deterministic coordinate math
   - grid storage and transitions
   - hierarchy construction
   - direct and hierarchical search
   - cache and path validation
2. Bevy orchestration
   - plugin wiring
   - async and sliced request lifecycle
   - ECS publication and invalidation
   - diagnostics and debug gizmos

The runtime keeps one committed snapshot in the public `PathfindingGrid` resource.

- ECS and pure sync queries read the committed snapshot directly.
- Async jobs clone that snapshot before leaving the main thread.
- Grid edits mutate the owned `GridStorage`, mark dirty regions, and then rebuild a fresh committed snapshot.

The rebuild path reconstructs a fresh hierarchy snapshot from the current grid state after each dirty batch. The crate keeps the dirty surface explicit through `pending_dirty_clusters`, `cluster_versions`, and touched-cluster stamps on published paths.

## Coordinate Model

- `GridCoord` is an integer `IVec3` wrapper.
- `GridAabb` uses inclusive min/max coordinates.
- World-space mapping uses:
  - `origin`: world-space corner of cell `(0, 0, 0)`
  - `cell_size`: uniform cell edge length
  - `world_rounding`: explicit `Floor`, `Round`, or `Ceil`

Grid coordinates are deterministic because the rounding policy is part of the grid config and never inferred implicitly.

## Grid Model

Each cell stores:

- walkability
- area type
- traversal mask bits
- base movement cost
- clearance

Optional explicit transitions connect one cell to another with a cost and optional one-way behavior. These model stairs, ladders, teleports, elevators, or vertical links without hard-coding genre-specific semantics.

## Hierarchy Model

Level 1 is built directly from cells:

1. Partition the world into fixed-size clusters.
2. Scan every border face between adjacent clusters.
3. Flood-fill passable border cells into entrance groups.
4. Merge long contiguous groups into one or two representative portal pairs.
5. Create portal endpoint nodes on both sides of the border.
6. Precompute intra-cluster paths between portal endpoints.

Levels 2 and 3 are recursive shortcut layers:

- higher clusters are fixed 2x scale aggregates of the lower level
- higher-level portal nodes are anchored to lower-level portal endpoints
- projection edges connect higher-level nodes to their lower-level anchors with zero travel cost
- higher-level intra-cluster edges store lower-level node routes

This keeps the public hierarchy explicit and debugable while avoiding hidden graph mutation during query time.

## Query Pipeline

The default `query_path` flow is:

1. Convert world or cell start/goal to `GridCoord`.
2. Apply any request-scoped `PathCostOverlay` regions to the traversal-cost probe for that query only.
3. Use direct A* if the request is short-range or both points lie in the same level-1 cluster.
4. Otherwise connect temporary start/goal nodes to the level-1 portal nodes in their clusters.
5. Run A* over the union graph:
   - level-1 nodes
   - level-2 / level-3 shortcut nodes
   - temporary start/goal edges
6. Refine the chosen abstract route:
   - projection edges collapse away
   - higher-level shortcut edges expand into lower-level node routes
   - level-1 intra-cluster edges expand into cell corridors
7. Validate the corridor, then optionally smooth it.
8. Record touched cluster versions plus the overlay signature for invalidation and caching.

`estimate_cost` returns a lightweight cost probe using the current hierarchy path when available and falls back to direct search otherwise.

`query_path_sliced` keeps the direct-grid open list and parent map alive across frames and expands only a bounded number of low-level nodes per call. That keeps the runtime WASM-safe and frame-budgeted without blocking the main thread.

## Filter Model

Every request carries either a registered `PathFilterId` or an inline `PathFilterProfile`.

The filter can:

- exclude traversal-mask bits
- allow only specific traversal-mask bits
- override area costs
- enforce agent clearance
- add temporary cost overlays by region

Level-1 intra-cluster routes are computed against the requesting filter when temporary start/goal hookups are built. Higher-level shortcut edges are conservative in pass 1: they recursively expand into stored lower-level routes instead of running a fresh super-cluster repair.

## Dynamic Rebuild Flow

Edits are region-based:

1. A system mutates the `PathfindingGrid` directly or adds/removes a `PathfindingObstacle`.
2. Manual edits can emit `GridRegionChanged`; obstacle edits are synchronized into blocked cells by the crate before rebuilds.
3. Dirty level-1 clusters are computed from the region overlap.
4. Parent clusters on higher levels are marked dirty automatically.
5. `detect_grid_changes` consumes up to `rebuild_budget_per_frame` dirty cluster stamps.
6. After each consumed batch, the crate rebuilds a fresh committed hierarchy snapshot, reapplies the carried cluster versions, and invalidates any cache entry whose touched clusters overlap the rebuilt set.

Cached paths that touch dirty level-1 clusters are dropped immediately. Already published `ComputedPath` corridors are invalidated on the next validation pass if their touched cluster versions no longer match.

## Async and Sliced Queries

The ECS shell supports three execution modes:

- direct sync
- background async on `AsyncComputeTaskPool`
- sliced direct-grid search across frames

Async tasks clone the committed `PathfindingSnapshot`, never borrow ECS state directly, and return pure results. The ECS request component carries per-request overlays into those jobs, and publication compares the job's `PathVersion` with the current committed version.

Sliced mode keeps its own resumable search state resource so large requests can respect frame budgets even without worker threads or on WASM.

## Cache and Validation

The path cache is exact-keyed:

- start
- goal
- query mode
- filter id
- partial flag
- overlay signature
- committed snapshot version

Each entry stores:

- final corridor
- world-space waypoints
- total cost
- touched level-1 cluster version stamps
- last-touch tick for LRU and TTL

Dirty-region invalidation removes entries whose touched clusters overlap the dirty set. Runtime cache limits are pulled from `HpaPathfindingConfig` every frame so BRP edits to `cache_capacity` and `cache_ttl_frames` take effect immediately.

## Debug Layout

Debug layers are controlled through `HpaPathfindingConfig` flags:

- grid cells
- cluster bounds per level
- portal nodes
- abstract graph edges
- active paths
- dirty clusters
- cost heatmap

The heatmap renders base cell costs through the default filter profile (`PathFilterId(0)`). Query-time overlays remain request-local and are not merged into one global heatmap view.

The plugin only adds the gizmo draw system when the app already has Bevy gizmo resources, so logic-only tests and minimal apps can still use the runtime safely.
