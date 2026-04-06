use crate::{components::ComputedPath, ecs_api::PathfindingGrid, filters::PathFilterId};
use bevy::{
    ecs::system::SystemState,
    gizmos::{config::DefaultGizmoConfigGroup, gizmos::GizmoStorage, prelude::Gizmos},
    prelude::*,
};

/// Debug gizmos are drawn at fixed z-offsets so they layer correctly above
/// example grid tiles (typically z = −5) and below agent sprites (z = 5–8).
const Z_GRID: f32 = 0.5;
const Z_HEATMAP: f32 = 1.0;
const Z_CLUSTERS: f32 = 2.0;
const Z_DIRTY: f32 = 2.5;
const Z_PORTALS: f32 = 3.0;
const Z_GRAPH: f32 = 3.5;
const Z_CORRIDOR: f32 = 4.0;
const Z_PATH: f32 = 4.5;
const Z_WAYPOINTS: f32 = 4.8;

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

    if config.debug_draw_grid {
        draw_grid_overlay(&mut gizmos, grid_storage);
    }

    if config.debug_draw_cost_heatmap {
        draw_cost_heatmap(&mut gizmos, grid_storage, &grid);
    }

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
                    color.with_alpha(0.55),
                    Z_CLUSTERS,
                );
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
                    Color::srgb(0.95, 0.18, 0.22).with_alpha(0.35),
                    Z_DIRTY,
                );
            }
        }
    }

    if config.debug_draw_portals || config.debug_draw_abstract_graph {
        for node in &grid.snapshot.nodes {
            let pos = grid_storage.grid_to_world_center(node.coord);
            let pos = Vec3::new(pos.x, pos.y, Z_PORTALS);
            if config.debug_draw_portals {
                let half = grid_storage.space.cell_size * 0.28;
                let color = Color::srgb(0.98, 0.88, 0.32);
                gizmos.line(
                    pos + Vec3::new(-half, 0.0, 0.0),
                    pos + Vec3::new(half, 0.0, 0.0),
                    color,
                );
                gizmos.line(
                    pos + Vec3::new(0.0, -half, 0.0),
                    pos + Vec3::new(0.0, half, 0.0),
                    color,
                );
                // Draw a second pair slightly offset for thickness.
                let t = grid_storage.space.cell_size * 0.02;
                gizmos.line(
                    pos + Vec3::new(-half, t, 0.0),
                    pos + Vec3::new(half, t, 0.0),
                    color.with_alpha(0.5),
                );
                gizmos.line(
                    pos + Vec3::new(t, -half, 0.0),
                    pos + Vec3::new(t, half, 0.0),
                    color.with_alpha(0.5),
                );
            }
            if config.debug_draw_abstract_graph {
                for edge in &grid.snapshot.edges[node.id] {
                    if node.id < edge.to {
                        let to =
                            grid_storage.grid_to_world_center(grid.snapshot.nodes[edge.to].coord);
                        let to = Vec3::new(to.x, to.y, Z_GRAPH);
                        gizmos.line(pos, to, Color::srgb(0.34, 0.76, 0.95).with_alpha(0.7));
                    }
                }
            }
        }
    }

    if config.debug_draw_paths {
        draw_paths(&mut gizmos, grid_storage, &paths);
    }
}

/// Draw a full grid overlay: thin lines for every cell boundary plus X marks
/// on blocked cells.
fn draw_grid_overlay(
    gizmos: &mut Gizmos<'_, '_, DefaultGizmoConfigGroup>,
    grid: &crate::grid::GridStorage,
) {
    let cell = grid.space.cell_size;
    let dims = grid.dimensions;
    let o = grid.space.origin;
    let line_color = Color::srgba(0.45, 0.50, 0.55, 0.22);
    let blocked_color = Color::srgba(0.95, 0.22, 0.18, 0.65);

    let y_lo = o.y;
    let y_hi = o.y + dims.y as f32 * cell;
    for x in 0..=dims.x {
        let wx = o.x + x as f32 * cell;
        gizmos.line(
            Vec3::new(wx, y_lo, Z_GRID),
            Vec3::new(wx, y_hi, Z_GRID),
            line_color,
        );
    }
    let x_lo = o.x;
    let x_hi = o.x + dims.x as f32 * cell;
    for y in 0..=dims.y {
        let wy = o.y + y as f32 * cell;
        gizmos.line(
            Vec3::new(x_lo, wy, Z_GRID),
            Vec3::new(x_hi, wy, Z_GRID),
            line_color,
        );
    }

    for coord in grid.bounds().iter() {
        let Some(cell_data) = grid.cell(coord) else {
            continue;
        };
        if cell_data.walkable {
            continue;
        }
        let c = grid.grid_to_world_center(coord);
        let c = Vec3::new(c.x, c.y, Z_GRID + 0.1);
        let h = cell * 0.42;
        gizmos.line(
            c + Vec3::new(-h, -h, 0.0),
            c + Vec3::new(h, h, 0.0),
            blocked_color,
        );
        gizmos.line(
            c + Vec3::new(-h, h, 0.0),
            c + Vec3::new(h, -h, 0.0),
            blocked_color,
        );
    }
}

/// Draw a cost heatmap. Normalises to min/max across all walkable cells so
/// that grids with uniform costs still get a subtle base tint.
fn draw_cost_heatmap(
    gizmos: &mut Gizmos<'_, '_, DefaultGizmoConfigGroup>,
    grid_storage: &crate::grid::GridStorage,
    grid: &PathfindingGrid,
) {
    let profile = grid.filter(PathFilterId(0));
    let mut min_cost = f32::INFINITY;
    let mut max_cost = f32::NEG_INFINITY;

    for coord in grid_storage.bounds().iter() {
        let Some(cell) = grid_storage.cell(coord) else {
            continue;
        };
        if !cell.walkable {
            continue;
        }
        let c = cell.base_cost * profile.multiplier_for(cell.area);
        min_cost = min_cost.min(c);
        max_cost = max_cost.max(c);
    }

    if min_cost.is_infinite() {
        return;
    }

    let has_variation = (max_cost - min_cost) > 0.001;
    let range = (max_cost - min_cost).max(0.001);

    for coord in grid_storage.bounds().iter() {
        let Some(cell) = grid_storage.cell(coord) else {
            continue;
        };
        if !cell.walkable {
            continue;
        }

        let raw_cost = cell.base_cost * profile.multiplier_for(cell.area);
        let intensity = if has_variation {
            ((raw_cost - min_cost) / range).clamp(0.0, 1.0)
        } else {
            0.0
        };

        // Always draw a subtle base tint; brighter for higher costs.
        let alpha = 0.15 + intensity * 0.55;
        let color = heatmap_color(intensity).with_alpha(alpha);

        let center = grid_storage.grid_to_world_center(coord);
        let center = Vec3::new(center.x, center.y, Z_HEATMAP);
        let half = grid_storage.space.cell_size * 0.40;
        draw_cell_fill(gizmos, center, half, color);
    }
}

/// Draw computed paths: corridor cells, smoothed waypoint lines, and markers
/// at each waypoint.
fn draw_paths(
    gizmos: &mut Gizmos<'_, '_, DefaultGizmoConfigGroup>,
    grid: &crate::grid::GridStorage,
    paths: &Query<'_, '_, &ComputedPath>,
) {
    let corridor_color = Color::srgba(0.30, 0.75, 0.95, 0.45);
    let line_color = Color::srgb(0.15, 1.0, 0.45);
    let wp_color = Color::srgb(1.0, 0.75, 0.15);
    let cell = grid.space.cell_size;

    for path in paths {
        // 1. Draw corridor cells as highlighted rectangles.
        for &coord in &path.corridor {
            let c = grid.grid_to_world_center(coord);
            let c = Vec3::new(c.x, c.y, Z_CORRIDOR);
            let h = cell * 0.44;
            draw_cell_fill(gizmos, c, h, corridor_color);
        }

        // 2. Draw smoothed waypoint path as bright lines.
        for pair in path.waypoints.windows(2) {
            let a = Vec3::new(pair[0].x, pair[0].y, Z_PATH);
            let b = Vec3::new(pair[1].x, pair[1].y, Z_PATH);
            gizmos.line(a, b, line_color);
            // Draw parallel offset lines for visible thickness.
            let dir = (b - a).normalize_or_zero();
            let perp = Vec3::new(-dir.y, dir.x, 0.0);
            let offset1 = perp * (cell * 0.06);
            let offset2 = perp * (cell * 0.12);
            gizmos.line(a + offset1, b + offset1, line_color.with_alpha(0.7));
            gizmos.line(a - offset1, b - offset1, line_color.with_alpha(0.7));
            gizmos.line(a + offset2, b + offset2, line_color.with_alpha(0.35));
            gizmos.line(a - offset2, b - offset2, line_color.with_alpha(0.35));
        }

        // 3. Draw diamond markers at each waypoint.
        let marker_size = cell * 0.26;
        for wp in &path.waypoints {
            let p = Vec3::new(wp.x, wp.y, Z_WAYPOINTS);
            draw_diamond(gizmos, p, marker_size, wp_color);
        }

        // 4. Start marker (green cross) and end marker (red cross).
        if let Some(first) = path.waypoints.first() {
            let p = Vec3::new(first.x, first.y, Z_WAYPOINTS);
            draw_cross(gizmos, p, marker_size * 1.8, Color::srgb(0.18, 0.92, 0.40));
        }
        if path.waypoints.len() > 1
            && let Some(last) = path.waypoints.last()
        {
            let p = Vec3::new(last.x, last.y, Z_WAYPOINTS);
            draw_cross(gizmos, p, marker_size * 1.8, Color::srgb(0.93, 0.26, 0.20));
        }
    }
}

// ---- helpers ----

fn draw_region_outline(
    gizmos: &mut Gizmos<'_, '_, DefaultGizmoConfigGroup>,
    grid: &crate::grid::GridStorage,
    region: crate::coord::GridAabb,
    color: Color,
    z: f32,
) {
    let min = grid.space.to_world_corner(region.min);
    let max = grid.space.to_world_corner(region.max.offset(IVec3::ONE));
    let a = Vec3::new(min.x, min.y, z);
    let b = Vec3::new(max.x, min.y, z);
    let c = Vec3::new(max.x, max.y, z);
    let d = Vec3::new(min.x, max.y, z);
    gizmos.line(a, b, color);
    gizmos.line(b, c, color);
    gizmos.line(c, d, color);
    gizmos.line(d, a, color);
}

/// Draw a cell outline plus two diagonals to give a "filled" appearance.
fn draw_cell_fill(
    gizmos: &mut Gizmos<'_, '_, DefaultGizmoConfigGroup>,
    center: Vec3,
    half: f32,
    color: Color,
) {
    let a = center + Vec3::new(-half, -half, 0.0);
    let b = center + Vec3::new(half, -half, 0.0);
    let c = center + Vec3::new(half, half, 0.0);
    let d = center + Vec3::new(-half, half, 0.0);
    gizmos.line(a, b, color);
    gizmos.line(b, c, color);
    gizmos.line(c, d, color);
    gizmos.line(d, a, color);
    gizmos.line(a, c, color);
    gizmos.line(b, d, color);
}

/// Draw a diamond shape (4 edges rotated 45°).
fn draw_diamond(
    gizmos: &mut Gizmos<'_, '_, DefaultGizmoConfigGroup>,
    center: Vec3,
    size: f32,
    color: Color,
) {
    let n = Vec3::new(0.0, size, 0.0);
    let e = Vec3::new(size, 0.0, 0.0);
    gizmos.line(center + n, center + e, color);
    gizmos.line(center + e, center - n, color);
    gizmos.line(center - n, center - e, color);
    gizmos.line(center - e, center + n, color);
}

/// Draw a plus-sign cross.
fn draw_cross(
    gizmos: &mut Gizmos<'_, '_, DefaultGizmoConfigGroup>,
    center: Vec3,
    size: f32,
    color: Color,
) {
    gizmos.line(
        center + Vec3::new(-size, 0.0, 0.0),
        center + Vec3::new(size, 0.0, 0.0),
        color,
    );
    gizmos.line(
        center + Vec3::new(0.0, -size, 0.0),
        center + Vec3::new(0.0, size, 0.0),
        color,
    );
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
