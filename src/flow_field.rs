use crate::{
    coord::{GridCoord, GridSpace},
    filters::{PathCostOverlay, PathFilterProfile},
    grid::deltas_for_mode,
    hierarchy::PathfindingSnapshot,
    search::nearest_walkable_cell,
};
use bevy::prelude::*;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

#[derive(Debug, Clone, Reflect, Default)]
pub struct FlowFieldCell {
    pub integration_cost: Option<f32>,
    pub next: Option<GridCoord>,
}

#[derive(Debug, Clone, Reflect)]
pub struct FlowField {
    pub goal: GridCoord,
    pub space: GridSpace,
    pub dimensions: UVec3,
    pub cells: Vec<FlowFieldCell>,
}

impl FlowField {
    pub fn index(&self, coord: GridCoord) -> Option<usize> {
        if coord.x() < 0
            || coord.y() < 0
            || coord.z() < 0
            || coord.x() >= self.dimensions.x as i32
            || coord.y() >= self.dimensions.y as i32
            || coord.z() >= self.dimensions.z as i32
        {
            return None;
        }

        let x = coord.x() as usize;
        let y = coord.y() as usize;
        let z = coord.z() as usize;
        Some(
            z * self.dimensions.x as usize * self.dimensions.y as usize
                + y * self.dimensions.x as usize
                + x,
        )
    }

    pub fn cell(&self, coord: GridCoord) -> Option<&FlowFieldCell> {
        self.index(coord).map(|index| &self.cells[index])
    }

    pub fn integration_cost(&self, coord: GridCoord) -> Option<f32> {
        self.cell(coord)?.integration_cost
    }

    pub fn next_step(&self, coord: GridCoord) -> Option<GridCoord> {
        self.cell(coord)?.next
    }

    pub fn direction_at(&self, coord: GridCoord) -> Option<Vec3> {
        let next = self.next_step(coord)?;
        let from_world = self.space.to_world_center(coord);
        let to_world = self.space.to_world_center(next);
        Some((to_world - from_world).normalize_or_zero())
    }
}

pub fn build_flow_field(
    snapshot: &PathfindingSnapshot,
    goal: GridCoord,
    profile: &PathFilterProfile,
    overlays: &[PathCostOverlay],
) -> Option<FlowField> {
    let goal = if snapshot.grid.is_passable(goal, profile, overlays) {
        goal
    } else {
        nearest_walkable_cell(snapshot, goal, profile)?
    };

    let len = snapshot.grid.cells.len();
    let mut cells = vec![FlowFieldCell::default(); len];
    let mut open = BinaryHeap::new();
    let mut incoming_transitions = HashMap::<GridCoord, Vec<(GridCoord, f32)>>::new();

    for (&from, transitions) in &snapshot.grid.transitions {
        for transition in transitions {
            incoming_transitions
                .entry(transition.target)
                .or_default()
                .push((from, transition.cost));
        }
    }

    let goal_index = snapshot.grid.index(goal)?;
    cells[goal_index].integration_cost = Some(0.0);
    open.push(FlowNodeState::new(goal, 0.0));

    while let Some(current) = open.pop() {
        let Some(current_index) = snapshot.grid.index(current.coord) else {
            continue;
        };
        let Some(best_known_cost) = cells[current_index].integration_cost else {
            continue;
        };
        if current.cost > best_known_cost + 0.0001 {
            continue;
        }

        for predecessor in predecessor_steps(snapshot, current.coord, profile, overlays) {
            let Some(index) = snapshot.grid.index(predecessor.coord) else {
                continue;
            };
            let tentative = current.cost + predecessor.cost;
            let should_update = match cells[index].integration_cost {
                Some(existing) => tentative + 0.0001 < existing,
                None => true,
            };
            if should_update {
                cells[index].integration_cost = Some(tentative);
                open.push(FlowNodeState::new(predecessor.coord, tentative));
            }
        }

        if let Some(predecessors) = incoming_transitions.get(&current.coord) {
            for &(predecessor, cost) in predecessors {
                if !snapshot.grid.is_passable(predecessor, profile, overlays) {
                    continue;
                }
                let Some(index) = snapshot.grid.index(predecessor) else {
                    continue;
                };
                let tentative = current.cost + cost;
                let should_update = match cells[index].integration_cost {
                    Some(existing) => tentative + 0.0001 < existing,
                    None => true,
                };
                if should_update {
                    cells[index].integration_cost = Some(tentative);
                    open.push(FlowNodeState::new(predecessor, tentative));
                }
            }
        }
    }

    for coord in snapshot.grid.bounds().iter() {
        let Some(index) = snapshot.grid.index(coord) else {
            continue;
        };
        if cells[index].integration_cost.is_none() || coord == goal {
            continue;
        }

        let mut best_step = None::<(GridCoord, f32)>;
        for (neighbor, step_cost) in snapshot.grid.neighbor_cells(
            coord,
            snapshot.config.neighborhood,
            snapshot.config.allow_corner_cutting,
            profile,
            overlays,
        ) {
            let Some(neighbor_index) = snapshot.grid.index(neighbor) else {
                continue;
            };
            let Some(neighbor_cost) = cells[neighbor_index].integration_cost else {
                continue;
            };
            let total = step_cost + neighbor_cost;
            let should_replace = match best_step {
                Some((_, best_total)) => total + 0.0001 < best_total,
                None => true,
            };
            if should_replace {
                best_step = Some((neighbor, total));
            }
        }
        cells[index].next = best_step.map(|(coord, _)| coord);
    }

    Some(FlowField {
        goal,
        space: snapshot.grid.space,
        dimensions: snapshot.grid.dimensions,
        cells,
    })
}

fn predecessor_steps(
    snapshot: &PathfindingSnapshot,
    coord: GridCoord,
    profile: &PathFilterProfile,
    overlays: &[PathCostOverlay],
) -> Vec<PredecessorStep> {
    let mut predecessors = Vec::new();
    for delta in deltas_for_mode(snapshot.config.neighborhood) {
        let predecessor = coord.offset(-delta);
        if !snapshot.grid.contains(predecessor) {
            continue;
        }
        if let Some((_, cost)) = snapshot
            .grid
            .neighbor_cells(
                predecessor,
                snapshot.config.neighborhood,
                snapshot.config.allow_corner_cutting,
                profile,
                overlays,
            )
            .into_iter()
            .find(|(neighbor, _)| *neighbor == coord)
        {
            predecessors.push(PredecessorStep {
                coord: predecessor,
                cost,
            });
        }
    }
    predecessors
}

#[derive(Debug, Clone, Copy)]
struct PredecessorStep {
    coord: GridCoord,
    cost: f32,
}

#[derive(Debug, Clone, Copy)]
struct FlowNodeState {
    coord: GridCoord,
    cost: f32,
}

impl FlowNodeState {
    fn new(coord: GridCoord, cost: f32) -> Self {
        Self { coord, cost }
    }
}

impl PartialEq for FlowNodeState {
    fn eq(&self, other: &Self) -> bool {
        self.coord == other.coord && self.cost.to_bits() == other.cost.to_bits()
    }
}

impl Eq for FlowNodeState {}

impl PartialOrd for FlowNodeState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FlowNodeState {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .cost
            .total_cmp(&self.cost)
            .then_with(|| self.coord.cmp(&other.coord))
    }
}

#[cfg(test)]
#[path = "flow_field_tests.rs"]
mod tests;
