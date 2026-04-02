use crate::{coord::GridCoord, search::PathQueryId, validation::PathInvalidationReason};
use bevy::prelude::*;

#[derive(Message, Debug, Clone, Reflect)]
pub struct GridRegionChanged {
    pub min: GridCoord,
    pub max: GridCoord,
}

impl GridRegionChanged {
    pub fn new(min: GridCoord, max: GridCoord) -> Self {
        Self { min, max }
    }
}

#[derive(Message, Debug, Clone, Reflect)]
pub struct PathReady {
    pub entity: Entity,
    pub query_id: PathQueryId,
}

#[derive(Message, Debug, Clone, Reflect)]
pub struct PathInvalidated {
    pub entity: Entity,
    pub reason: PathInvalidationReason,
}
