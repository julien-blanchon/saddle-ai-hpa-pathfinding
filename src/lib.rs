#![doc = include_str!("../README.md")]

mod async_runtime;
#[cfg(test)]
mod benchmarks;
mod cache;
mod components;
mod config;
mod coord;
mod debug;
mod dynamic;
mod ecs_api;
mod filters;
mod grid;
mod hierarchy;
mod messages;
mod search;
mod smoothing;
mod stats;
mod validation;

pub use crate::cache::{PathCache, PathCacheEntry, PathCacheKey};
pub use crate::components::{
    ComputedPath, ObstacleShape, PathRequest, PathfindingAgent, PathfindingObstacle,
    PendingPathQuery,
};
pub use crate::config::{HpaPathfindingConfig, NeighborhoodMode, PathQueryMode, PathSmoothingMode};
pub use crate::coord::{GridAabb, GridCoord, GridSpace, WorldRoundingPolicy};
pub use crate::ecs_api::PathfindingGrid;
pub use crate::filters::{
    AreaMask, AreaTypeId, PathCostOverlay, PathFilterId, PathFilterLibrary, PathFilterProfile,
};
pub use crate::grid::{CellData, GridStorage, TransitionKind, TransitionLink};
pub use crate::hierarchy::{
    ClusterInfo, ClusterKey, ClusterVersionStamp, EdgeKind, EdgeRoute, HierarchyLevel,
    PathfindingSnapshot, cluster_size_for_level,
};
pub use crate::messages::{GridRegionChanged, PathInvalidated, PathReady};
pub use crate::search::{
    CostEstimate, PathQueryId, PathVersion, ResolvedPath, SlicedGridSearch, estimate_cost,
    find_path, line_of_sight, nearest_walkable_cell,
};
pub use crate::stats::PathfindingStats;
pub use crate::validation::{PathInvalidationReason, PathValidationRecord};

use bevy::{
    app::PostStartup,
    ecs::{intern::Interned, schedule::ScheduleLabel},
    prelude::*,
};

#[derive(SystemSet, Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum HpaPathfindingSystems {
    DetectChanges,
    RebuildHierarchy,
    EnqueueQueries,
    ProcessQueries,
    ValidatePaths,
    PublishResults,
    DebugDraw,
}

pub struct HpaPathfindingPlugin {
    pub activate_schedule: Interned<dyn ScheduleLabel>,
    pub deactivate_schedule: Interned<dyn ScheduleLabel>,
    pub update_schedule: Interned<dyn ScheduleLabel>,
}

impl HpaPathfindingPlugin {
    pub fn new(
        activate_schedule: impl ScheduleLabel,
        deactivate_schedule: impl ScheduleLabel,
        update_schedule: impl ScheduleLabel,
    ) -> Self {
        Self {
            activate_schedule: activate_schedule.intern(),
            deactivate_schedule: deactivate_schedule.intern(),
            update_schedule: update_schedule.intern(),
        }
    }

    pub fn always_on(update_schedule: impl ScheduleLabel) -> Self {
        Self::new(
            PostStartup,
            async_runtime::NeverDeactivateSchedule,
            update_schedule,
        )
    }
}

impl Default for HpaPathfindingPlugin {
    fn default() -> Self {
        Self::always_on(Update)
    }
}

impl Plugin for HpaPathfindingPlugin {
    fn build(&self, app: &mut App) {
        if self.deactivate_schedule == async_runtime::NeverDeactivateSchedule.intern() {
            app.init_schedule(async_runtime::NeverDeactivateSchedule);
        }

        app.init_resource::<HpaPathfindingConfig>()
            .init_resource::<PathfindingStats>()
            .init_resource::<PathCache>()
            .init_resource::<dynamic::ObstacleRuntimeState>()
            .init_resource::<async_runtime::PathfindingRuntimeState>()
            .add_message::<GridRegionChanged>()
            .add_message::<PathReady>()
            .add_message::<PathInvalidated>()
            .register_type::<AreaMask>()
            .register_type::<AreaTypeId>()
            .register_type::<CellData>()
            .register_type::<ClusterInfo>()
            .register_type::<ClusterKey>()
            .register_type::<ClusterVersionStamp>()
            .register_type::<ComputedPath>()
            .register_type::<EdgeKind>()
            .register_type::<EdgeRoute>()
            .register_type::<GridAabb>()
            .register_type::<GridCoord>()
            .register_type::<GridRegionChanged>()
            .register_type::<GridSpace>()
            .register_type::<GridStorage>()
            .register_type::<HierarchyLevel>()
            .register_type::<HpaPathfindingConfig>()
            .register_type::<NeighborhoodMode>()
            .register_type::<ObstacleShape>()
            .register_type::<PathFilterId>()
            .register_type::<PathFilterProfile>()
            .register_type::<PathCostOverlay>()
            .register_type::<PathInvalidated>()
            .register_type::<PathInvalidationReason>()
            .register_type::<PathQueryId>()
            .register_type::<PathQueryMode>()
            .register_type::<PathReady>()
            .register_type::<PathRequest>()
            .register_type::<PathSmoothingMode>()
            .register_type::<PathVersion>()
            .register_type::<PathfindingAgent>()
            .register_type::<PathfindingObstacle>()
            .register_type::<PathfindingStats>()
            .register_type::<PathfindingSnapshot>()
            .register_type::<PendingPathQuery>()
            .register_type::<TransitionKind>()
            .register_type::<TransitionLink>()
            .register_type::<WorldRoundingPolicy>()
            .configure_sets(
                self.update_schedule,
                (
                    HpaPathfindingSystems::DetectChanges,
                    HpaPathfindingSystems::RebuildHierarchy,
                    HpaPathfindingSystems::EnqueueQueries,
                    HpaPathfindingSystems::ProcessQueries,
                    HpaPathfindingSystems::ValidatePaths,
                    HpaPathfindingSystems::PublishResults,
                    HpaPathfindingSystems::DebugDraw,
                )
                    .chain(),
            )
            .add_systems(self.activate_schedule, async_runtime::setup_hpa_pathfinding)
            .add_systems(self.activate_schedule, async_runtime::activate_runtime)
            .add_systems(
                self.deactivate_schedule,
                async_runtime::cleanup_hpa_pathfinding,
            )
            .add_systems(
                self.update_schedule,
                async_runtime::ensure_runtime_initialized
                    .before(HpaPathfindingSystems::DetectChanges),
            )
            .add_systems(
                self.update_schedule,
                dynamic::sync_obstacles
                    .in_set(HpaPathfindingSystems::DetectChanges)
                    .before(async_runtime::detect_grid_changes)
                    .run_if(async_runtime::runtime_is_active),
            )
            .add_systems(
                self.update_schedule,
                async_runtime::detect_grid_changes
                    .in_set(HpaPathfindingSystems::DetectChanges)
                    .run_if(async_runtime::runtime_is_active),
            )
            .add_systems(
                self.update_schedule,
                async_runtime::enqueue_requests
                    .in_set(HpaPathfindingSystems::EnqueueQueries)
                    .run_if(async_runtime::runtime_is_active),
            )
            .add_systems(
                self.update_schedule,
                async_runtime::process_queries
                    .in_set(HpaPathfindingSystems::ProcessQueries)
                    .run_if(async_runtime::runtime_is_active),
            )
            .add_systems(
                self.update_schedule,
                async_runtime::validate_paths
                    .in_set(HpaPathfindingSystems::ValidatePaths)
                    .run_if(async_runtime::runtime_is_active),
            )
            .add_systems(
                self.update_schedule,
                async_runtime::publish_results
                    .in_set(HpaPathfindingSystems::PublishResults)
                    .run_if(async_runtime::runtime_is_active),
            )
            .add_systems(
                self.update_schedule,
                debug::draw_debug
                    .in_set(HpaPathfindingSystems::DebugDraw)
                    .run_if(async_runtime::runtime_is_active),
            );
    }
}
