use crate::{components::ComputedPath, ecs_api::PathfindingGrid, filters::PathFilterId};
use bevy::{
    ecs::system::SystemState,
    gizmos::{config::DefaultGizmoConfigGroup, gizmos::GizmoStorage, prelude::Gizmos},
    prelude::*,
};

type DebugState<'w, 's> = SystemState<(
    Res<'w, crate::config::HpaPathfindingConfig>,
    Res<'w, PathfindingGrid>,
    Gizmos<'w, 's, DefaultGizmoConfigGroup>,
    Query<'w, 's, &'static ComputedPath>,
)>;

pub(crate) fn draw_debug(world: &mut World) {
    if !world.contains_resource::<GizmoStorage<DefaultGizmoConfigGroup, ()>>() {
        return;
    }

    let mut state: DebugState<'_, '_> = SystemState::new(world);
    let (config, grid, mut gizmos, paths) = state.get_mut(world);
    let grid_storage = &grid.snapshot.grid;

    if config.debug_draw_clusters {
        let colors = [
            Color::srgb(0.95, 0.48, 0.21),
            Color::srgb(0.23, 0.72, 0.96),
            Color::srgb(0.78, 0.46, 0.95),
        ];
        for level in &grid.snapshot.levels {
            let color = colors[(level.level as usize - 1).min(colors.len() - 1)];
            for cluster in level.clusters.values() {
                draw_region_outline(
                    &mut gizmos,
                    grid_storage,
                    cluster.bounds,
                    color.with_alpha(0.35),
                );
            }
        }
    }

    if config.debug_draw_cost_heatmap {
        let profile = grid.filter(PathFilterId(0));
        let max_cost = grid_storage
            .bounds()
            .iter()
            .filter_map(|coord| {
                let cell = grid_storage.cell(coord)?;
                cell.walkable
                    .then_some(cell.base_cost * profile.multiplier_for(cell.area))
            })
            .fold(1.0, f32::max);

        for coord in grid_storage.bounds().iter() {
            let Some(cell) = grid_storage.cell(coord) else {
                continue;
            };
            if !cell.walkable {
                continue;
            }

            let raw_cost = cell.base_cost * profile.multiplier_for(cell.area);
            let intensity = ((raw_cost - 1.0) / (max_cost - 1.0).max(1.0)).clamp(0.0, 1.0);
            if intensity <= 0.001 {
                continue;
            }

            let color = heatmap_color(intensity);
            draw_region_outline(
                &mut gizmos,
                grid_storage,
                crate::coord::GridAabb::new(coord, coord),
                color.with_alpha(0.2 + intensity * 0.45),
            );

            let center = grid_storage.grid_to_world_center(coord);
            let half = grid_storage.space.cell_size * 0.18;
            gizmos.line(
                center + Vec3::new(-half, -half, 0.1),
                center + Vec3::new(half, half, 0.1),
                color.with_alpha(0.25 + intensity * 0.55),
            );
            gizmos.line(
                center + Vec3::new(-half, half, 0.1),
                center + Vec3::new(half, -half, 0.1),
                color.with_alpha(0.25 + intensity * 0.55),
            );
        }
    }

    if config.debug_draw_portals || config.debug_draw_abstract_graph {
        for node in &grid.snapshot.nodes {
            let pos = grid_storage.grid_to_world_center(node.coord);
            if config.debug_draw_portals {
                gizmos.cross(
                    pos,
                    grid_storage.space.cell_size * 0.18,
                    Color::srgb(0.98, 0.88, 0.32),
                );
            }
            if config.debug_draw_abstract_graph {
                for edge in &grid.snapshot.edges[node.id] {
                    if node.id < edge.to {
                        let to =
                            grid_storage.grid_to_world_center(grid.snapshot.nodes[edge.to].coord);
                        gizmos.line(pos, to, Color::srgb(0.34, 0.76, 0.95));
                    }
                }
            }
        }
    }

    if config.debug_draw_paths {
        for path in &paths {
            for pair in path.waypoints.windows(2) {
                gizmos.line(pair[0], pair[1], Color::srgb(0.34, 0.93, 0.48));
            }
        }
    }

    if config.debug_draw_dirty_clusters {
        for key in grid.snapshot.dirty_clusters() {
            if let Some(cluster) = grid
                .snapshot
                .level(key.level)
                .and_then(|level| level.clusters.get(&key))
            {
                draw_region_outline(
                    &mut gizmos,
                    grid_storage,
                    cluster.bounds,
                    Color::srgb(0.95, 0.18, 0.22).with_alpha(0.2),
                );
            }
        }
    }

    if config.debug_draw_grid {
        for coord in grid_storage.bounds().iter() {
            let Some(cell) = grid_storage.cell(coord) else {
                continue;
            };
            if cell.walkable {
                continue;
            }
            draw_region_outline(
                &mut gizmos,
                grid_storage,
                crate::coord::GridAabb::new(coord, coord),
                Color::srgb(0.42, 0.12, 0.10).with_alpha(0.28),
            );
        }
    }
}

fn draw_region_outline(
    gizmos: &mut Gizmos<'_, '_, DefaultGizmoConfigGroup>,
    grid: &crate::grid::GridStorage,
    region: crate::coord::GridAabb,
    color: Color,
) {
    let min = grid.space.to_world_corner(region.min);
    let max = grid.space.to_world_corner(region.max.offset(IVec3::ONE));
    let z = min.z;
    let a = Vec3::new(min.x, min.y, z);
    let b = Vec3::new(max.x, min.y, z);
    let c = Vec3::new(max.x, max.y, z);
    let d = Vec3::new(min.x, max.y, z);
    gizmos.line(a, b, color);
    gizmos.line(b, c, color);
    gizmos.line(c, d, color);
    gizmos.line(d, a, color);
}

fn heatmap_color(intensity: f32) -> Color {
    let t = intensity.clamp(0.0, 1.0);
    let cold = Vec3::new(0.18, 0.70, 0.42);
    let warm = Vec3::new(0.96, 0.82, 0.28);
    let hot = Vec3::new(0.93, 0.26, 0.20);
    let rgb = if t < 0.5 {
        cold.lerp(warm, t * 2.0)
    } else {
        warm.lerp(hot, (t - 0.5) * 2.0)
    };
    Color::srgb(rgb.x, rgb.y, rgb.z)
}
