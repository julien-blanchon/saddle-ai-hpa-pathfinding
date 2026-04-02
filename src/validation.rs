use crate::{coord::GridCoord, hierarchy::ClusterVersionStamp, search::PathVersion};
use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub enum PathInvalidationReason {
    DirtyCluster,
    SnapshotAdvanced,
    MissingCorridor,
    GoalBecameBlocked,
}

#[derive(Debug, Clone, Default, Reflect)]
pub struct PathValidationRecord {
    pub goal: Option<GridCoord>,
    pub version: PathVersion,
    pub traversed_clusters: Vec<ClusterVersionStamp>,
}

impl PathValidationRecord {
    pub fn is_valid_for(
        &self,
        current_version: PathVersion,
        cluster_versions: &[ClusterVersionStamp],
    ) -> bool {
        if self.version != current_version {
            return false;
        }
        self.traversed_clusters.iter().all(|expected| {
            cluster_versions
                .iter()
                .find(|actual| actual.cluster == expected.cluster)
                .is_some_and(|actual| actual.version == expected.version)
        })
    }
}

#[cfg(test)]
#[path = "validation_tests.rs"]
mod tests;
