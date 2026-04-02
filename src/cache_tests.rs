use super::{PathCache, PathCacheEntry, PathCacheKey};
use crate::{
    config::PathQueryMode,
    coord::GridCoord,
    filters::PathFilterId,
    hierarchy::{ClusterKey, ClusterVersionStamp},
    search::{PathVersion, ResolvedPath},
};

fn cache_key(index: i32) -> PathCacheKey {
    PathCacheKey {
        start: GridCoord::new(index, 0, 0),
        goal: GridCoord::new(index + 1, 0, 0),
        filter: PathFilterId(0),
        mode: PathQueryMode::Auto,
        allow_partial: false,
        overlay_signature: 0,
        version: PathVersion(1),
    }
}

fn cache_entry(index: i32) -> PathCacheEntry {
    PathCacheEntry {
        path: ResolvedPath {
            corridor: vec![GridCoord::new(index, 0, 0), GridCoord::new(index + 1, 0, 0)],
            waypoints: Vec::new(),
            total_cost: 1.0,
            is_partial: false,
            version: PathVersion(1),
            touched_clusters: vec![ClusterVersionStamp {
                cluster: ClusterKey::new(1, GridCoord::new(index / 2, 0, 0)),
                version: 1,
            }],
        },
        touched_clusters: vec![ClusterVersionStamp {
            cluster: ClusterKey::new(1, GridCoord::new(index / 2, 0, 0)),
            version: 1,
        }],
        last_touch_tick: 0,
    }
}

#[test]
fn cache_hit_and_miss_behavior() {
    let mut cache = PathCache::with_limits(2, 0);
    let key = cache_key(0);
    assert!(cache.get(&key, 0).is_none());
    cache.insert(key, cache_entry(0));
    assert!(cache.get(&key, 1).is_some());
}

#[test]
fn cache_ttl_expires_entries() {
    let mut cache = PathCache::with_limits(4, 3);
    let key = cache_key(0);
    cache.insert(key, cache_entry(0));
    assert!(cache.get(&key, 2).is_some());
    assert!(cache.get(&key, 10).is_none());
}

#[test]
fn cache_evicts_lru_entries() {
    let mut cache = PathCache::with_limits(2, 0);
    cache.insert(cache_key(0), cache_entry(0));
    cache.insert(cache_key(1), cache_entry(1));
    cache.get(&cache_key(0), 1);
    cache.insert(cache_key(2), cache_entry(2));

    assert!(cache.get(&cache_key(0), 2).is_some());
    assert!(cache.get(&cache_key(1), 2).is_none());
}

#[test]
fn cache_invalidates_dirty_clusters() {
    let mut cache = PathCache::with_limits(4, 0);
    cache.insert(cache_key(0), cache_entry(0));
    cache.insert(cache_key(2), cache_entry(2));

    cache.invalidate_clusters(&[ClusterKey::new(1, GridCoord::new(0, 0, 0))]);

    assert!(cache.get(&cache_key(0), 0).is_none());
    assert!(cache.get(&cache_key(2), 0).is_some());
}

#[test]
fn cache_reconfigures_limits_from_runtime_settings() {
    let mut cache = PathCache::with_limits(4, 0);
    cache.insert(cache_key(0), cache_entry(0));
    cache.insert(cache_key(1), cache_entry(1));
    cache.insert(cache_key(2), cache_entry(2));

    let evicted = cache.apply_limits(2, 5, 0);

    assert_eq!(evicted, 1);
    assert_eq!(cache.capacity, 2);
    assert_eq!(cache.ttl_frames, 5);
    assert_eq!(cache.len(), 2);
}
