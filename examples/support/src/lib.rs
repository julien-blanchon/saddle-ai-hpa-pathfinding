use bevy::prelude::*;
use saddle_ai_hpa_pathfinding::{
    AreaMask, AreaTypeId, CellData, GridCoord, GridStorage, HpaPathfindingConfig,
    PathFilterProfile, PathfindingGrid, WorldRoundingPolicy,
};

pub fn demo_config(dimensions: UVec3) -> HpaPathfindingConfig {
    HpaPathfindingConfig {
        grid_dimensions: dimensions,
        origin: Vec3::new(
            -(dimensions.x as f32) * 0.5,
            -(dimensions.y as f32) * 0.5,
            0.0,
        ),
        cell_size: 1.0,
        cluster_size: UVec3::new(16, 16, 1),
        hierarchy_levels: if dimensions.x >= 32 || dimensions.y >= 32 {
            2
        } else {
            1
        },
        debug_draw_paths: true,
        ..default()
    }
}

pub fn build_demo_grid(dimensions: UVec3) -> GridStorage {
    let mut grid = GridStorage::new(dimensions, Vec3::ZERO, 1.0, WorldRoundingPolicy::Floor);
    for x in 6..dimensions.x.saturating_sub(6) as i32 {
        grid.set_walkable(GridCoord::new(x, (dimensions.y / 2) as i32, 0), false);
    }
    let gate = GridCoord::new((dimensions.x / 2) as i32, (dimensions.y / 2) as i32, 0);
    grid.set_walkable(gate, true);
    grid
}

pub fn build_filter_grid() -> PathfindingGrid {
    let config = demo_config(UVec3::new(24, 18, 1));
    let mut grid = PathfindingGrid::new(build_demo_grid(config.grid_dimensions), config);
    grid.fill_region(
        saddle_ai_hpa_pathfinding::GridAabb::new(GridCoord::new(3, 12, 0), GridCoord::new(20, 15, 0)),
        |_coord, cell| {
            cell.area = AreaTypeId(1);
            cell.base_cost = 1.0;
            cell.traversal_mask = AreaMask::from_bit(1);
        },
    );
    grid.register_filter(
        PathFilterProfile::named("wheeled")
            .with_id(saddle_ai_hpa_pathfinding::PathFilterId(1))
            .with_area_cost(AreaTypeId(1), 4.0),
    );
    grid.register_filter(
        PathFilterProfile::named("utility")
            .with_id(saddle_ai_hpa_pathfinding::PathFilterId(2))
            .with_allowed_mask(AreaMask::ALL)
            .with_area_cost(AreaTypeId(1), 1.2),
    );
    grid
}

pub fn build_layered_grid() -> PathfindingGrid {
    let config = HpaPathfindingConfig {
        grid_dimensions: UVec3::new(16, 16, 2),
        cluster_size: UVec3::new(8, 8, 1),
        hierarchy_levels: 1,
        neighborhood: saddle_ai_hpa_pathfinding::NeighborhoodMode::Ordinal18,
        ..demo_config(UVec3::new(16, 16, 2))
    };
    let mut path_grid = PathfindingGrid::new(
        GridStorage::new(
            config.grid_dimensions,
            Vec3::ZERO,
            1.0,
            WorldRoundingPolicy::Floor,
        ),
        config,
    );
    path_grid.add_transition(
        GridCoord::new(7, 7, 0),
        saddle_ai_hpa_pathfinding::TransitionLink::new(
            GridCoord::new(7, 7, 1),
            1.5,
            saddle_ai_hpa_pathfinding::TransitionKind::Stair,
        ),
    );
    path_grid
}

pub fn make_blocked_cell() -> CellData {
    CellData {
        walkable: false,
        ..default()
    }
}
