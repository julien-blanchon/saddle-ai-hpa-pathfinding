use crate::{
    components::{ObstacleShape, PathfindingObstacle},
    coord::{GridAabb, GridCoord},
    ecs_api::PathfindingGrid,
    filters::AreaTypeId,
    grid::CellData,
};
use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Default, Resource)]
pub(crate) struct ObstacleRuntimeState {
    records: HashMap<Entity, AppliedObstacle>,
    cells: HashMap<GridCoord, CellObstacleState>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ObstacleDescriptor {
    shape: ObstacleShape,
    area_override: Option<AreaTypeId>,
}

#[derive(Debug)]
struct AppliedObstacle {
    descriptor: ObstacleDescriptor,
    cells: Vec<GridCoord>,
}

#[derive(Debug)]
struct CellObstacleState {
    original: CellData,
    occupants: HashMap<Entity, Option<AreaTypeId>>,
}

pub(crate) fn sync_obstacles(
    mut grid: ResMut<PathfindingGrid>,
    mut state: ResMut<ObstacleRuntimeState>,
    obstacles: Query<(Entity, &PathfindingObstacle)>,
) {
    let current = obstacles
        .iter()
        .map(|(entity, obstacle)| {
            (
                entity,
                ObstacleDescriptor {
                    shape: obstacle.shape,
                    area_override: obstacle.area_override,
                },
            )
        })
        .collect::<HashMap<_, _>>();

    let mut removals = Vec::new();
    for (&entity, record) in &state.records {
        match current.get(&entity) {
            Some(descriptor) if *descriptor == record.descriptor => {}
            _ => removals.push(entity),
        }
    }
    for entity in removals {
        remove_obstacle(entity, &mut state, &mut grid);
    }

    for (entity, descriptor) in current {
        let needs_apply = state
            .records
            .get(&entity)
            .is_none_or(|record| record.descriptor != descriptor);
        if needs_apply {
            apply_obstacle(entity, descriptor, &mut state, &mut grid);
        }
    }
}

fn apply_obstacle(
    entity: Entity,
    descriptor: ObstacleDescriptor,
    state: &mut ObstacleRuntimeState,
    grid: &mut PathfindingGrid,
) {
    let cells = shape_cells(descriptor.shape, grid.grid().bounds());
    for coord in &cells {
        let original = grid.grid().cell(*coord).cloned().unwrap_or_default();
        let cell_state = state
            .cells
            .entry(*coord)
            .or_insert_with(|| CellObstacleState {
                original,
                occupants: HashMap::new(),
            });
        cell_state
            .occupants
            .insert(entity, descriptor.area_override);
        reapply_cell(*coord, cell_state, grid);
    }
    state
        .records
        .insert(entity, AppliedObstacle { descriptor, cells });
}

fn remove_obstacle(entity: Entity, state: &mut ObstacleRuntimeState, grid: &mut PathfindingGrid) {
    let Some(record) = state.records.remove(&entity) else {
        return;
    };

    let mut empty_cells = Vec::new();
    for coord in record.cells {
        let Some(cell_state) = state.cells.get_mut(&coord) else {
            continue;
        };
        cell_state.occupants.remove(&entity);
        if cell_state.occupants.is_empty() {
            grid.set_cell(coord, cell_state.original.clone());
            empty_cells.push(coord);
        } else {
            reapply_cell(coord, cell_state, grid);
        }
    }
    for coord in empty_cells {
        state.cells.remove(&coord);
    }
}

fn reapply_cell(coord: GridCoord, cell_state: &CellObstacleState, grid: &mut PathfindingGrid) {
    let mut cell = cell_state.original.clone();
    if !cell_state.occupants.is_empty() {
        cell.walkable = false;
        if let Some(area) = cell_state
            .occupants
            .iter()
            .filter_map(|(entity, area)| area.map(|area| (entity.to_bits(), area)))
            .min_by_key(|(bits, _)| *bits)
            .map(|(_, area)| area)
        {
            cell.area = area;
        }
    }
    grid.set_cell(coord, cell);
}

fn shape_cells(shape: ObstacleShape, bounds: GridAabb) -> Vec<GridCoord> {
    match shape {
        ObstacleShape::Cell(coord) => bounds
            .contains(coord)
            .then_some(coord)
            .into_iter()
            .collect(),
        ObstacleShape::Region { min, max } => GridAabb::new(min, max)
            .clamp_to(bounds)
            .map(|region| region.iter().collect())
            .unwrap_or_default(),
    }
}

#[cfg(test)]
#[path = "dynamic_tests.rs"]
mod tests;
