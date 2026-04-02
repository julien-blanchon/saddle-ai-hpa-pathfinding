use bevy::prelude::*;

#[derive(Resource, Debug, Clone, Reflect, Default)]
#[reflect(Resource)]
pub struct PathfindingStats {
    pub total_queries_started: u64,
    pub total_queries_completed: u64,
    pub total_queries_failed: u64,
    pub total_queries_invalidated: u64,
    pub queue_depth: usize,
    pub cache_entries: usize,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_evictions: u64,
    pub dirty_cluster_count: usize,
    pub clusters_rebuilt: u64,
    pub async_in_flight: usize,
    pub sliced_expansions: u64,
    pub last_rebuild_micros: u64,
    pub last_query_process_micros: u64,
    pub last_publish_micros: u64,
    pub last_failed_queries: Vec<String>,
}

impl PathfindingStats {
    pub fn record_failure(&mut self, reason: impl Into<String>) {
        self.total_queries_failed += 1;
        self.last_failed_queries.push(reason.into());
        if self.last_failed_queries.len() > 8 {
            self.last_failed_queries.remove(0);
        }
    }
}
