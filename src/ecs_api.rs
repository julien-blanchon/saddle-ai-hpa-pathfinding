use crate::{
    config::HpaPathfindingConfig,
    coord::{GridAabb, GridCoord},
    filters::{PathFilterId, PathFilterLibrary, PathFilterProfile},
    grid::{CellData, GridStorage, TransitionLink},
    hierarchy::{ClusterKey, PathfindingSnapshot},
    search::{
        CostEstimate, PathVersion, SlicedGridSearch, estimate_cost, find_path, line_of_sight,
        nearest_walkable_cell,
    },
};
use bevy::prelude::*;
use std::collections::{HashMap, VecDeque};

#[derive(Resource, Debug, Clone, Reflect)]
#[reflect(Resource)]
pub struct PathfindingGrid {
    pub snapshot: PathfindingSnapshot,
    pub filters: PathFilterLibrary,
    pub pending_dirty_regions: Vec<GridAabb>,
    pub pending_dirty_clusters: VecDeque<ClusterKey>,
    pub last_rebuilt_clusters: Vec<ClusterKey>,
    pub cluster_versions: HashMap<ClusterKey, u64>,
    pub tick: u64,
}

impl PathfindingGrid {
    pub fn from_config(config: HpaPathfindingConfig) -> Self {
        let grid = GridStorage::new(
            config.grid_dimensions,
            config.origin,
            config.cell_size,
            config.world_rounding,
        );
        Self::new(grid, config)
    }

    pub fn new(grid: GridStorage, config: HpaPathfindingConfig) -> Self {
        let snapshot = PathfindingSnapshot::build(grid, config, 1);
        let mut state = Self {
            snapshot,
            filters: PathFilterLibrary::default(),
            pending_dirty_regions: Vec::new(),
            pending_dirty_clusters: VecDeque::new(),
            last_rebuilt_clusters: Vec::new(),
            cluster_versions: HashMap::new(),
            tick: 0,
        };
        state.filters.register(PathFilterProfile::default());
        state.apply_cluster_versions(&[]);
        state
    }

    pub fn version(&self) -> PathVersion {
        PathVersion(self.snapshot.version)
    }

    pub fn snapshot(&self) -> &PathfindingSnapshot {
        &self.snapshot
    }

    pub fn grid(&self) -> &GridStorage {
        &self.snapshot.grid
    }

    pub fn filter(&self, id: PathFilterId) -> PathFilterProfile {
        self.filters
            .get(id)
            .cloned()
            .unwrap_or_else(PathFilterProfile::default)
    }

    pub fn query_path(
        &self,
        start: GridCoord,
        goal: GridCoord,
        filter: PathFilterId,
        mode: crate::config::PathQueryMode,
        allow_partial: bool,
        overlays: &[crate::filters::PathCostOverlay],
    ) -> Option<crate::search::ResolvedPath> {
        let profile = self.filter(filter);
        find_path(
            &self.snapshot,
            start,
            goal,
            &profile,
            mode,
            allow_partial,
            overlays,
        )
    }

    pub fn query_path_sliced(
        &self,
        start: GridCoord,
        goal: GridCoord,
        filter: PathFilterId,
        allow_partial: bool,
        overlays: Vec<crate::filters::PathCostOverlay>,
    ) -> Option<SlicedGridSearch> {
        let profile = self.filter(filter);
        SlicedGridSearch::new(
            self.snapshot.clone(),
            start,
            goal,
            profile,
            allow_partial,
            overlays,
        )
    }

    pub fn nearest_walkable(&self, start: GridCoord, filter: PathFilterId) -> Option<GridCoord> {
        let profile = self.filter(filter);
        nearest_walkable_cell(&self.snapshot, start, &profile)
    }

    pub fn raycast_line_of_sight(
        &self,
        start: GridCoord,
        goal: GridCoord,
        filter: PathFilterId,
        overlays: &[crate::filters::PathCostOverlay],
    ) -> bool {
        let profile = self.filter(filter);
        line_of_sight(&self.snapshot, start, goal, &profile, overlays)
    }

    pub fn estimate_cost(
        &self,
        start: GridCoord,
        goal: GridCoord,
        filter: PathFilterId,
        overlays: &[crate::filters::PathCostOverlay],
    ) -> CostEstimate {
        let profile = self.filter(filter);
        estimate_cost(&self.snapshot, start, goal, &profile, overlays)
    }

    pub fn register_filter(&mut self, profile: PathFilterProfile) -> PathFilterId {
        self.filters.register(profile)
    }

    pub fn set_cell(&mut self, coord: GridCoord, cell: CellData) -> bool {
        let changed = self.snapshot.grid.set_cell(coord, cell);
        if changed {
            self.mark_dirty_region(GridAabb::new(coord, coord));
        }
        changed
    }

    pub fn set_walkable(&mut self, coord: GridCoord, walkable: bool) -> bool {
        let changed = self.snapshot.grid.set_walkable(coord, walkable);
        if changed {
            self.mark_dirty_region(GridAabb::new(coord, coord));
        }
        changed
    }

    pub fn fill_region(&mut self, region: GridAabb, f: impl FnMut(GridCoord, &mut CellData)) {
        self.snapshot.grid.fill_region(region, f);
        self.mark_dirty_region(region);
    }

    pub fn add_transition(&mut self, from: GridCoord, transition: TransitionLink) {
        let target = transition.target;
        self.snapshot.grid.add_transition(from, transition);
        self.mark_dirty_region(GridAabb::new(
            GridCoord::new(
                from.x().min(target.x()),
                from.y().min(target.y()),
                from.z().min(target.z()),
            ),
            GridCoord::new(
                from.x().max(target.x()),
                from.y().max(target.y()),
                from.z().max(target.z()),
            ),
        ));
    }

    pub fn mark_dirty_region(&mut self, region: GridAabb) {
        let Some(region) = region.clamp_to(self.snapshot.grid.bounds()) else {
            return;
        };
        self.pending_dirty_regions.push(region);
        for level in 1..=self.snapshot.config.clamped_hierarchy_levels() {
            let min_key = self.snapshot.cluster_key_for_coord(level, region.min);
            let max_key = self.snapshot.cluster_key_for_coord(level, region.max);
            for z in min_key.coord.z()..=max_key.coord.z() {
                for y in min_key.coord.y()..=max_key.coord.y() {
                    for x in min_key.coord.x()..=max_key.coord.x() {
                        let key = ClusterKey::new(level, GridCoord::new(x, y, z));
                        if !self.pending_dirty_clusters.contains(&key) {
                            self.pending_dirty_clusters.push_back(key);
                        }
                    }
                }
            }
        }
    }

    pub fn advance_tick(&mut self) {
        self.tick += 1;
    }

    pub fn rebuild_budgeted(
        &mut self,
        config: &HpaPathfindingConfig,
        budget: u32,
    ) -> Vec<ClusterKey> {
        let mut rebuilt = Vec::new();
        for _ in 0..budget.max(1) {
            let Some(cluster) = self.pending_dirty_clusters.pop_front() else {
                break;
            };
            let next = self.cluster_versions.get(&cluster).copied().unwrap_or(0) + 1;
            self.cluster_versions.insert(cluster, next);
            rebuilt.push(cluster);
        }

        self.last_rebuilt_clusters = rebuilt.clone();
        if rebuilt.is_empty() {
            self.pending_dirty_regions.clear();
            self.apply_cluster_versions(&[]);
            return rebuilt;
        }

        let next_version = self.snapshot.version + 1;
        let grid = self.snapshot.grid.clone();
        self.snapshot = PathfindingSnapshot::build(grid, config.clone(), next_version);
        self.apply_cluster_versions(&rebuilt);
        self.pending_dirty_regions.clear();
        rebuilt
    }

    fn apply_cluster_versions(&mut self, dirty_clusters: &[ClusterKey]) {
        for level in &mut self.snapshot.levels {
            for (key, cluster) in &mut level.clusters {
                cluster.version = self.cluster_versions.get(key).copied().unwrap_or(0);
                cluster.dirty =
                    dirty_clusters.contains(key) || self.pending_dirty_clusters.contains(key);
            }
        }
    }
}
