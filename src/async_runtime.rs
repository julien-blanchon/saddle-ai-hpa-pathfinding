use crate::{
    cache::{PathCache, PathCacheEntry, PathCacheKey},
    components::{
        ComputedPath, ComputedPathRuntime, PathRequest, PathfindingAgent, PendingPathQuery,
    },
    config::{HpaPathfindingConfig, PathQueryMode},
    coord::GridCoord,
    ecs_api::PathfindingGrid,
    messages::{GridRegionChanged, PathInvalidated, PathReady},
    search::{PathQueryId, ResolvedPath, SlicedGridSearch, find_path},
    stats::PathfindingStats,
    validation::{PathInvalidationReason, PathValidationRecord},
};
use bevy::platform::time::Instant;
use bevy::{
    ecs::schedule::ScheduleLabel,
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
};
use futures_lite::future;
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct NeverDeactivateSchedule;

#[derive(Debug, Clone)]
struct QueuedQuery {
    entity: Entity,
    start: GridCoord,
    goal: GridCoord,
    filter: crate::filters::PathFilterId,
    priority: i32,
    mode: PathQueryMode,
    allow_partial: bool,
    overlay_signature: u64,
    overlays: Vec<crate::filters::PathCostOverlay>,
    enqueued_tick: u64,
    version: crate::search::PathVersion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct DedupKey {
    entity: Entity,
    start: GridCoord,
    goal: GridCoord,
    filter: crate::filters::PathFilterId,
    mode: PathQueryMode,
    allow_partial: bool,
    overlay_signature: u64,
}

#[derive(Debug)]
struct AsyncQueryOutput {
    entity: Entity,
    query_id: PathQueryId,
    cache_key: PathCacheKey,
    result: Option<ResolvedPath>,
}

#[derive(Debug)]
struct ActiveSlicedQuery {
    entity: Entity,
    dedup_key: DedupKey,
    runner: SlicedGridSearch,
}

#[derive(Default, Resource)]
pub(crate) struct PathfindingRuntimeState {
    pub active: bool,
    pub next_query_id: u64,
    queue: Vec<QueuedQuery>,
    queued_keys: HashSet<DedupKey>,
    inflight: HashMap<PathQueryId, (DedupKey, Task<AsyncQueryOutput>)>,
    sliced: HashMap<PathQueryId, ActiveSlicedQuery>,
    completed: VecDeque<AsyncQueryOutput>,
}

pub(crate) fn setup_hpa_pathfinding(
    mut commands: Commands,
    config: Res<HpaPathfindingConfig>,
    maybe_grid: Option<Res<PathfindingGrid>>,
) {
    if maybe_grid.is_none() {
        commands.insert_resource(PathfindingGrid::from_config(config.clone()));
    }
}

pub(crate) fn cleanup_hpa_pathfinding(mut runtime: ResMut<PathfindingRuntimeState>) {
    runtime.active = false;
    runtime.queue.clear();
    runtime.queued_keys.clear();
    runtime.inflight.clear();
    runtime.sliced.clear();
    runtime.completed.clear();
}

pub(crate) fn activate_runtime(mut runtime: ResMut<PathfindingRuntimeState>) {
    runtime.active = true;
}

pub(crate) fn ensure_runtime_initialized(
    mut commands: Commands,
    config: Res<HpaPathfindingConfig>,
    maybe_grid: Option<Res<PathfindingGrid>>,
    mut runtime: ResMut<PathfindingRuntimeState>,
) {
    if maybe_grid.is_none() {
        commands.insert_resource(PathfindingGrid::from_config(config.clone()));
    }
    runtime.active = true;
}

pub(crate) fn runtime_is_active(runtime: Res<PathfindingRuntimeState>) -> bool {
    runtime.active
}

pub(crate) fn detect_grid_changes(
    mut grid: ResMut<PathfindingGrid>,
    config: Res<HpaPathfindingConfig>,
    mut stats: ResMut<PathfindingStats>,
    mut messages: MessageReader<GridRegionChanged>,
    mut cache: ResMut<PathCache>,
) {
    let started = Instant::now();
    grid.advance_tick();
    stats.cache_evictions +=
        cache.apply_limits(config.cache_capacity, config.cache_ttl_frames, grid.tick) as u64;
    for message in messages.read() {
        grid.mark_dirty_region(crate::coord::GridAabb::new(message.min, message.max));
    }

    let dirty = grid.rebuild_budgeted(&config, config.rebuild_budget_per_frame);
    if !dirty.is_empty() {
        cache.invalidate_clusters(&dirty);
    }
    stats.dirty_cluster_count = grid.pending_dirty_clusters.len();
    stats.clusters_rebuilt += dirty.len() as u64;
    stats.cache_entries = cache.len();
    stats.last_rebuild_micros = started.elapsed().as_micros() as u64;
}

pub(crate) fn enqueue_requests(
    config: Res<HpaPathfindingConfig>,
    grid: Res<PathfindingGrid>,
    mut runtime: ResMut<PathfindingRuntimeState>,
    mut stats: ResMut<PathfindingStats>,
    query: Query<(
        Entity,
        Option<&PathfindingAgent>,
        &PathRequest,
        Option<&GlobalTransform>,
        Option<&Transform>,
        Option<&PendingPathQuery>,
        Option<&ComputedPath>,
    )>,
) {
    for (entity, agent, request, global, local, pending, computed) in &query {
        if pending.is_some() {
            continue;
        }
        if computed
            .map(|path| {
                path.path_version == grid.version()
                    && path.corridor.last().copied() == Some(request.goal)
            })
            .unwrap_or(false)
        {
            continue;
        }

        let position = global
            .map(|transform| transform.translation())
            .or_else(|| local.map(|transform| transform.translation))
            .unwrap_or(Vec3::ZERO);
        let start = grid.snapshot.grid.world_to_grid(position);
        let dedup_key = DedupKey {
            entity,
            start,
            goal: request.goal,
            filter: agent.map_or(crate::filters::PathFilterId(0), |agent| agent.filter),
            mode: request.mode,
            allow_partial: request.allow_partial,
            overlay_signature: request.overlay_signature(),
        };
        if runtime.queued_keys.contains(&dedup_key) {
            continue;
        }
        runtime.queue.push(QueuedQuery {
            entity,
            start,
            goal: request.goal,
            filter: dedup_key.filter,
            priority: agent.map_or(0, |agent| agent.request_priority),
            mode: request.mode,
            allow_partial: request.allow_partial,
            overlay_signature: dedup_key.overlay_signature,
            overlays: request.overlays.clone(),
            enqueued_tick: grid.tick,
            version: grid.version(),
        });
        runtime.queued_keys.insert(dedup_key);
    }

    if config.deterministic {
        runtime.queue.sort_by(|left, right| {
            effective_priority(right, grid.tick)
                .cmp(&effective_priority(left, grid.tick))
                .then_with(|| left.enqueued_tick.cmp(&right.enqueued_tick))
                .then_with(|| left.entity.index().cmp(&right.entity.index()))
        });
    } else {
        runtime.queue.sort_unstable_by(|left, right| {
            effective_priority(right, grid.tick).cmp(&effective_priority(left, grid.tick))
        });
    }
    stats.queue_depth = runtime.queue.len();
}

pub(crate) fn process_queries(
    mut commands: Commands,
    grid: Res<PathfindingGrid>,
    config: Res<HpaPathfindingConfig>,
    mut runtime: ResMut<PathfindingRuntimeState>,
    mut stats: ResMut<PathfindingStats>,
    mut cache: ResMut<PathCache>,
) {
    if !runtime.active {
        return;
    }

    let started = Instant::now();
    let pool = AsyncComputeTaskPool::get();
    let mut launched = 0_u32;
    while launched < config.max_queries_per_frame {
        let Some(query) = runtime.queue.pop() else {
            break;
        };
        let dedup_key = DedupKey {
            entity: query.entity,
            start: query.start,
            goal: query.goal,
            filter: query.filter,
            mode: query.mode,
            allow_partial: query.allow_partial,
            overlay_signature: query.overlay_signature,
        };

        let cache_key = PathCacheKey {
            start: query.start,
            goal: query.goal,
            filter: query.filter,
            mode: query.mode,
            allow_partial: query.allow_partial,
            overlay_signature: query.overlay_signature,
            version: query.version,
        };
        if let Some(path) = cache.get(&cache_key, grid.tick) {
            let query_id = next_query_id(&mut runtime);
            runtime.completed.push_back(AsyncQueryOutput {
                entity: query.entity,
                query_id,
                cache_key,
                result: Some(path),
            });
            stats.cache_hits += 1;
            runtime.queued_keys.remove(&dedup_key);
            stats.total_queries_started += 1;
            launched += 1;
            continue;
        }

        stats.cache_misses += 1;
        let query_id = next_query_id(&mut runtime);
        stats.total_queries_started += 1;
        commands
            .entity(query.entity)
            .insert(PendingPathQuery { id: query_id });
        let snapshot = grid.snapshot.clone();
        let profile = grid.filter(query.filter);
        if matches!(query.mode, PathQueryMode::Sliced) {
            if let Some(runner) = SlicedGridSearch::new(
                snapshot,
                query.start,
                query.goal,
                profile,
                query.allow_partial,
                query.overlays.clone(),
            ) {
                runtime.sliced.insert(
                    query_id,
                    ActiveSlicedQuery {
                        entity: query.entity,
                        dedup_key,
                        runner,
                    },
                );
            } else {
                runtime.completed.push_back(AsyncQueryOutput {
                    entity: query.entity,
                    query_id,
                    cache_key,
                    result: None,
                });
                runtime.queued_keys.remove(&dedup_key);
            }
        } else {
            let entity = query.entity;
            let start = query.start;
            let goal = query.goal;
            let mode = query.mode;
            let allow_partial = query.allow_partial;
            let overlays = query.overlays.clone();
            let cache_key_for_task = cache_key;
            let task = pool.spawn(async move {
                let result = find_path(
                    &snapshot,
                    start,
                    goal,
                    &profile,
                    mode,
                    allow_partial,
                    &overlays,
                );
                AsyncQueryOutput {
                    entity,
                    query_id,
                    cache_key: cache_key_for_task,
                    result,
                }
            });
            runtime.inflight.insert(query_id, (dedup_key, task));
        }
        launched += 1;
    }

    let sliced_budget = config.max_sliced_expansions_per_frame.max(1);
    let mut completed_sliced = Vec::new();
    for (&query_id, active) in &mut runtime.sliced {
        if let Some(result) = active.runner.advance(sliced_budget) {
            completed_sliced.push((query_id, active.entity, active.dedup_key, result));
        }
        stats.sliced_expansions += sliced_budget as u64;
    }
    for (query_id, entity, dedup_key, result) in completed_sliced {
        runtime.sliced.remove(&query_id);
        runtime.queued_keys.remove(&dedup_key);
        runtime.completed.push_back(AsyncQueryOutput {
            entity,
            query_id,
            cache_key: PathCacheKey {
                start: dedup_key.start,
                goal: dedup_key.goal,
                filter: dedup_key.filter,
                mode: dedup_key.mode,
                allow_partial: dedup_key.allow_partial,
                overlay_signature: dedup_key.overlay_signature,
                version: grid.version(),
            },
            result,
        });
    }

    let mut finished = Vec::new();
    for (&query_id, (_dedup, task)) in &mut runtime.inflight {
        if let Some(output) = future::block_on(future::poll_once(task)) {
            finished.push((query_id, output));
        }
    }
    for (query_id, output) in finished {
        if let Some((dedup_key, _)) = runtime.inflight.remove(&query_id) {
            runtime.queued_keys.remove(&dedup_key);
        }
        runtime.completed.push_back(output);
    }

    stats.async_in_flight = runtime.inflight.len();
    stats.queue_depth = runtime.queue.len();
    stats.cache_entries = cache.len();
    stats.last_query_process_micros = started.elapsed().as_micros() as u64;
}

pub(crate) fn validate_paths(
    mut commands: Commands,
    grid: Res<PathfindingGrid>,
    mut invalidated: MessageWriter<PathInvalidated>,
    mut stats: ResMut<PathfindingStats>,
    query: Query<(Entity, &ComputedPath, &ComputedPathRuntime, &PathRequest)>,
) {
    for (entity, path, runtime, request) in &query {
        let current_clusters = grid.snapshot.cluster_versions_for_corridor(&path.corridor);
        let record = PathValidationRecord {
            goal: Some(request.goal),
            version: path.path_version,
            traversed_clusters: runtime.traversed_cluster_keys.clone(),
        };
        if !record.is_valid_for(grid.version(), &current_clusters) {
            commands
                .entity(entity)
                .remove::<ComputedPath>()
                .remove::<ComputedPathRuntime>();
            invalidated.write(PathInvalidated {
                entity,
                reason: if path.path_version != grid.version() {
                    PathInvalidationReason::SnapshotAdvanced
                } else {
                    PathInvalidationReason::DirtyCluster
                },
            });
            stats.total_queries_invalidated += 1;
        }
    }
}

pub(crate) fn publish_results(
    mut commands: Commands,
    grid: Res<PathfindingGrid>,
    mut runtime: ResMut<PathfindingRuntimeState>,
    mut ready: MessageWriter<PathReady>,
    mut stats: ResMut<PathfindingStats>,
    mut cache: ResMut<PathCache>,
) {
    let started = Instant::now();
    while let Some(output) = runtime.completed.pop_front() {
        commands.entity(output.entity).remove::<PendingPathQuery>();
        match output.result {
            Some(path) => {
                let touched_clusters = path.touched_clusters.clone();
                let evicted = cache.insert(
                    output.cache_key,
                    PathCacheEntry {
                        path: path.clone(),
                        touched_clusters: touched_clusters.clone(),
                        last_touch_tick: grid.tick,
                    },
                );
                stats.cache_evictions += evicted as u64;
                commands.entity(output.entity).insert((
                    ComputedPath {
                        waypoints: path.waypoints.clone(),
                        corridor: path.corridor.clone(),
                        total_cost: path.total_cost,
                        is_partial: path.is_partial,
                        path_version: path.version,
                    },
                    ComputedPathRuntime {
                        traversed_cluster_keys: touched_clusters,
                        query_id: Some(output.query_id),
                    },
                ));
                ready.write(PathReady {
                    entity: output.entity,
                    query_id: output.query_id,
                });
                stats.total_queries_completed += 1;
            }
            None => {
                stats.record_failure(format!("query {:?}", output.query_id));
            }
        }
    }
    stats.cache_entries = cache.len();
    stats.last_publish_micros = started.elapsed().as_micros() as u64;
}

fn effective_priority(query: &QueuedQuery, tick: u64) -> i32 {
    query.priority + (tick.saturating_sub(query.enqueued_tick) / 60) as i32
}

fn next_query_id(runtime: &mut PathfindingRuntimeState) -> PathQueryId {
    runtime.next_query_id += 1;
    PathQueryId(runtime.next_query_id)
}

#[cfg(test)]
#[path = "async_runtime_tests.rs"]
mod tests;
