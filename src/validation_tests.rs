use super::{PathInvalidationReason, PathValidationRecord};
use crate::{
    coord::GridCoord,
    hierarchy::{ClusterKey, ClusterVersionStamp},
    search::PathVersion,
};

#[test]
fn validation_rejects_version_mismatch() {
    let record = PathValidationRecord {
        goal: Some(GridCoord::new(3, 3, 0)),
        version: PathVersion(1),
        traversed_clusters: vec![ClusterVersionStamp {
            cluster: ClusterKey::new(1, GridCoord::new(0, 0, 0)),
            version: 4,
        }],
    };
    let actual = vec![ClusterVersionStamp {
        cluster: ClusterKey::new(1, GridCoord::new(0, 0, 0)),
        version: 4,
    }];

    assert!(!record.is_valid_for(PathVersion(2), &actual));
    assert!(matches!(
        PathInvalidationReason::SnapshotAdvanced,
        PathInvalidationReason::SnapshotAdvanced
    ));
}

#[test]
fn validation_rejects_cluster_version_change() {
    let record = PathValidationRecord {
        goal: None,
        version: PathVersion(7),
        traversed_clusters: vec![ClusterVersionStamp {
            cluster: ClusterKey::new(1, GridCoord::new(1, 0, 0)),
            version: 2,
        }],
    };
    let actual = vec![ClusterVersionStamp {
        cluster: ClusterKey::new(1, GridCoord::new(1, 0, 0)),
        version: 3,
    }];

    assert!(!record.is_valid_for(PathVersion(7), &actual));
}
