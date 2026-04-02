use crate::{
    config::PathQueryMode,
    coord::GridCoord,
    filters::{AreaTypeId, PathCostOverlay, PathFilterId, overlay_signature},
    search::{PathQueryId, PathVersion},
};
use bevy::prelude::*;

#[derive(Component, Debug, Clone, Reflect)]
pub struct PathfindingAgent {
    pub filter: PathFilterId,
    pub clearance: u16,
    pub request_priority: i32,
}

impl Default for PathfindingAgent {
    fn default() -> Self {
        Self {
            filter: PathFilterId(0),
            clearance: 0,
            request_priority: 0,
        }
    }
}

#[derive(Component, Debug, Clone, Reflect)]
pub struct PathRequest {
    pub goal: GridCoord,
    pub mode: PathQueryMode,
    pub allow_partial: bool,
    pub overlays: Vec<PathCostOverlay>,
}

impl PathRequest {
    pub fn new(goal: GridCoord) -> Self {
        Self {
            goal,
            mode: PathQueryMode::Auto,
            allow_partial: false,
            overlays: Vec::new(),
        }
    }

    pub fn with_mode(mut self, mode: PathQueryMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn allow_partial(mut self) -> Self {
        self.allow_partial = true;
        self
    }

    pub fn with_overlays(mut self, overlays: Vec<PathCostOverlay>) -> Self {
        self.overlays = overlays;
        self
    }

    pub fn overlay_signature(&self) -> u64 {
        overlay_signature(&self.overlays)
    }
}

#[derive(Component, Debug, Clone, Reflect)]
pub struct ComputedPath {
    pub waypoints: Vec<Vec3>,
    pub corridor: Vec<GridCoord>,
    pub total_cost: f32,
    pub is_partial: bool,
    pub path_version: PathVersion,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub struct PendingPathQuery {
    pub id: PathQueryId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub enum ObstacleShape {
    Cell(GridCoord),
    Region { min: GridCoord, max: GridCoord },
}

#[derive(Component, Debug, Clone, PartialEq, Eq, Reflect)]
pub struct PathfindingObstacle {
    pub shape: ObstacleShape,
    pub area_override: Option<AreaTypeId>,
}

#[derive(Component, Debug, Clone, Reflect, Default)]
pub(crate) struct ComputedPathRuntime {
    pub traversed_cluster_keys: Vec<crate::hierarchy::ClusterVersionStamp>,
    pub query_id: Option<PathQueryId>,
}
