use crate::{
    config::PathSmoothingMode,
    coord::GridCoord,
    filters::{PathCostOverlay, PathFilterProfile},
    grid::GridStorage,
};
use bevy::prelude::*;

pub fn smooth_corridor(
    grid: &GridStorage,
    corridor: &[GridCoord],
    profile: &PathFilterProfile,
    overlays: &[PathCostOverlay],
    mode: PathSmoothingMode,
) -> Vec<Vec3> {
    if corridor.is_empty() {
        return Vec::new();
    }

    match mode {
        PathSmoothingMode::None => corridor
            .iter()
            .copied()
            .map(|coord| grid.grid_to_world_center(coord))
            .collect(),
        PathSmoothingMode::LineOfSight => {
            let mut output = Vec::new();
            let mut anchor = 0;
            output.push(grid.grid_to_world_center(corridor[0]));

            while anchor + 1 < corridor.len() {
                let mut furthest = anchor + 1;
                for candidate in anchor + 1..corridor.len() {
                    if grid.raycast_line_of_sight(
                        corridor[anchor],
                        corridor[candidate],
                        profile,
                        overlays,
                    ) {
                        furthest = candidate;
                    } else {
                        break;
                    }
                }
                output.push(grid.grid_to_world_center(corridor[furthest]));
                anchor = furthest;
            }

            output
        }
    }
}

#[cfg(test)]
#[path = "smoothing_tests.rs"]
mod tests;
