# Performance

## Direct A* vs HPA*

The runtime uses direct A* for:

- same-cluster requests
- short-range requests under `direct_search_distance`
- direct-only query mode

Hierarchical search pays for itself when:

- the grid is large
- requests are long-distance
- the same world is queried repeatedly
- dirty rebuilds are cheaper than repeated whole-grid searches

## Cluster Size Guidance

Rough heuristics:

- dense indoor maze: `8x8x1`
- mixed colony sim: `16x16x1`
- layered building: `16x16x2`
- voxel terrain: `16x16x4` or `32x32x4`

Too-small clusters create too many portal nodes and heavy rebuild churn. Too-large clusters reduce hierarchy quality and make filter repairs expensive.

## Cache Guidance

The cache helps most when:

- many agents share route templates
- requests are repeated between hubs
- dirty edits are localized

Set `cache_capacity` to at least the number of frequently repeated route pairs you expect in-flight. Use `cache_ttl_frames` only when the route distribution changes faster than dirty invalidation captures.

## Async vs Sliced

- Async mode:
  - best on native builds
  - highest throughput
  - ideal for many independent queries
- Sliced mode:
  - best for deterministic or WASM-friendly budgeting
  - spreads large direct-grid searches over frames in pass 1
  - avoids background task management

## Budget Tuning

- Increase `rebuild_budget_per_frame` if dirty edits backlog.
- Increase `max_queries_per_frame` if queue latency grows but frame time is still healthy.
- Lower `max_sliced_expansions_per_frame` if large-path spikes still hitch.

`PathfindingStats` exposes:

- queue depth
- cache entries
- cache hits and misses
- dirty cluster counts
- rebuild counts
- async in-flight counts
- cumulative query totals
- last rebuild/query/publication timings in microseconds

## Benchmarks

Benchmark-style regression coverage lives in ignored tests so it can run in release mode without slowing the default test suite:

```bash
cargo test -p saddle-ai-hpa-pathfinding --release -- --ignored
```

The ignored tests cover:

- direct A* versus hierarchical search on large queries
- budgeted rebuild batches on a dirty large grid

Use the richer examples and the crate-local lab to compare visual/runtime behavior:

- `cargo run -p saddle-ai-hpa-pathfinding-example-large-grid`
- `cargo run -p saddle-ai-hpa-pathfinding-example-async-queries`
- `cargo run -p saddle-ai-hpa-pathfinding-lab --features e2e -- hpa_pathfinding_large_grid`

Those runs plus the ignored release-mode tests are enough to catch obvious hierarchy crossover regressions, queue starvation, and dirty-update stalls without adding a separate Criterion harness yet.
