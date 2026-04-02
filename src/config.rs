use crate::coord::WorldRoundingPolicy;
use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Default)]
pub enum NeighborhoodMode {
    Cardinal2d,
    #[default]
    Ordinal2d,
    Cardinal3d,
    Ordinal18,
    Ordinal26,
}

impl NeighborhoodMode {
    pub fn movement_cost(delta: IVec3) -> f32 {
        let steps = delta.x.abs() + delta.y.abs() + delta.z.abs();
        match steps {
            0 | 1 => 1.0,
            2 => std::f32::consts::SQRT_2,
            _ => 3.0_f32.sqrt(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Default)]
pub enum PathQueryMode {
    DirectOnly,
    CoarseOnly,
    #[default]
    Auto,
    Sliced,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Default)]
pub enum PathSmoothingMode {
    None,
    #[default]
    LineOfSight,
}

#[derive(Resource, Debug, Clone, Reflect)]
#[reflect(Resource)]
pub struct HpaPathfindingConfig {
    pub grid_dimensions: UVec3,
    pub origin: Vec3,
    pub cell_size: f32,
    pub world_rounding: WorldRoundingPolicy,
    pub cluster_size: UVec3,
    pub hierarchy_levels: u8,
    pub neighborhood: NeighborhoodMode,
    pub allow_corner_cutting: bool,
    pub direct_search_distance: u32,
    pub rebuild_budget_per_frame: u32,
    pub max_queries_per_frame: u32,
    pub max_sliced_expansions_per_frame: u32,
    pub cache_capacity: usize,
    pub cache_ttl_frames: u64,
    pub smoothing_mode: PathSmoothingMode,
    pub deterministic: bool,
    pub debug_draw_grid: bool,
    pub debug_draw_clusters: bool,
    pub debug_draw_portals: bool,
    pub debug_draw_abstract_graph: bool,
    pub debug_draw_paths: bool,
    pub debug_draw_dirty_clusters: bool,
    pub debug_draw_cost_heatmap: bool,
}

impl Default for HpaPathfindingConfig {
    fn default() -> Self {
        Self {
            grid_dimensions: UVec3::new(64, 64, 1),
            origin: Vec3::ZERO,
            cell_size: 1.0,
            world_rounding: WorldRoundingPolicy::Floor,
            cluster_size: UVec3::new(16, 16, 1),
            hierarchy_levels: 2,
            neighborhood: NeighborhoodMode::Ordinal2d,
            allow_corner_cutting: false,
            direct_search_distance: 16,
            rebuild_budget_per_frame: 2,
            max_queries_per_frame: 8,
            max_sliced_expansions_per_frame: 96,
            cache_capacity: 128,
            cache_ttl_frames: 0,
            smoothing_mode: PathSmoothingMode::LineOfSight,
            deterministic: true,
            debug_draw_grid: false,
            debug_draw_clusters: false,
            debug_draw_portals: false,
            debug_draw_abstract_graph: false,
            debug_draw_paths: true,
            debug_draw_dirty_clusters: false,
            debug_draw_cost_heatmap: false,
        }
    }
}

impl HpaPathfindingConfig {
    pub fn clamped_hierarchy_levels(&self) -> u8 {
        self.hierarchy_levels.clamp(1, 3)
    }
}
