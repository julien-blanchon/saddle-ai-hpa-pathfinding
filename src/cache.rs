use crate::{
    config::PathQueryMode,
    coord::GridCoord,
    filters::PathFilterId,
    hierarchy::{ClusterKey, ClusterVersionStamp},
    search::{PathVersion, ResolvedPath},
};
use bevy::prelude::*;
use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PathCacheKey {
    pub start: GridCoord,
    pub goal: GridCoord,
    pub filter: PathFilterId,
    pub mode: PathQueryMode,
    pub allow_partial: bool,
    pub overlay_signature: u64,
    pub version: PathVersion,
}

#[derive(Debug, Clone)]
pub struct PathCacheEntry {
    pub path: ResolvedPath,
    pub touched_clusters: Vec<ClusterVersionStamp>,
    pub last_touch_tick: u64,
}

#[derive(Debug, Clone, Default, Resource)]
pub struct PathCache {
    pub capacity: usize,
    pub ttl_frames: u64,
    entries: HashMap<PathCacheKey, PathCacheEntry>,
    order: VecDeque<PathCacheKey>,
}

impl PathCache {
    pub fn with_limits(capacity: usize, ttl_frames: u64) -> Self {
        Self {
            capacity,
            ttl_frames,
            ..default()
        }
    }

    pub fn apply_limits(&mut self, capacity: usize, ttl_frames: u64, tick: u64) -> usize {
        self.capacity = capacity;
        self.ttl_frames = ttl_frames;
        self.prune_expired(tick);
        self.evict_lru()
    }

    pub fn get(&mut self, key: &PathCacheKey, tick: u64) -> Option<ResolvedPath> {
        self.prune_expired(tick);
        let path = {
            let entry = self.entries.get_mut(key)?;
            entry.last_touch_tick = tick;
            entry.path.clone()
        };
        self.bump(key);
        Some(path)
    }

    pub fn insert(&mut self, key: PathCacheKey, entry: PathCacheEntry) -> usize {
        self.entries.insert(key, entry);
        self.bump(&key);
        self.evict_lru()
    }

    pub fn invalidate_clusters(&mut self, dirty_clusters: &[ClusterKey]) {
        let dirty = dirty_clusters.to_vec();
        self.entries.retain(|_, entry| {
            !entry
                .touched_clusters
                .iter()
                .any(|stamp| dirty.contains(&stamp.cluster))
        });
        self.order.retain(|key| self.entries.contains_key(key));
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn prune_expired(&mut self, tick: u64) {
        if self.ttl_frames == 0 {
            return;
        }
        self.entries
            .retain(|_, entry| tick.saturating_sub(entry.last_touch_tick) <= self.ttl_frames);
        self.order.retain(|key| self.entries.contains_key(key));
    }

    fn bump(&mut self, key: &PathCacheKey) {
        if let Some(index) = self.order.iter().position(|candidate| candidate == key) {
            self.order.remove(index);
        }
        self.order.push_back(*key);
    }

    fn evict_lru(&mut self) -> usize {
        let mut evicted = 0;
        while self.capacity > 0 && self.entries.len() > self.capacity {
            if let Some(key) = self.order.pop_front() {
                if self.entries.remove(&key).is_some() {
                    evicted += 1;
                }
            } else {
                break;
            }
        }
        evicted
    }
}

#[cfg(test)]
#[path = "cache_tests.rs"]
mod tests;
