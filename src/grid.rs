use crate::{
    config::NeighborhoodMode,
    coord::{GridAabb, GridCoord, GridSpace, WorldRoundingPolicy},
    filters::{AreaMask, AreaTypeId, PathCostOverlay, PathFilterProfile},
};
use bevy::prelude::*;
use std::collections::{BTreeMap, VecDeque};

#[derive(Debug, Clone, Reflect)]
pub struct CellData {
    pub walkable: bool,
    pub area: AreaTypeId,
    pub traversal_mask: AreaMask,
    pub base_cost: f32,
    pub clearance: u16,
}

impl Default for CellData {
    fn default() -> Self {
        Self {
            walkable: true,
            area: AreaTypeId(0),
            traversal_mask: AreaMask::from_bit(0),
            base_cost: 1.0,
            clearance: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub enum TransitionKind {
    Link,
    Ladder,
    Stair,
    Elevator,
    Teleport,
}

#[derive(Debug, Clone, Reflect)]
pub struct TransitionLink {
    pub target: GridCoord,
    pub cost: f32,
    pub required_mask: AreaMask,
    pub one_way: bool,
    pub kind: TransitionKind,
}

impl TransitionLink {
    pub fn new(target: GridCoord, cost: f32, kind: TransitionKind) -> Self {
        Self {
            target,
            cost,
            required_mask: AreaMask::EMPTY,
            one_way: false,
            kind,
        }
    }

    pub fn with_required_mask(mut self, required_mask: AreaMask) -> Self {
        self.required_mask = required_mask;
        self
    }

    pub fn one_way(mut self) -> Self {
        self.one_way = true;
        self
    }
}

#[derive(Debug, Clone, Reflect)]
pub struct GridStorage {
    pub dimensions: UVec3,
    pub space: GridSpace,
    pub cells: Vec<CellData>,
    pub transitions: BTreeMap<GridCoord, Vec<TransitionLink>>,
}

impl GridStorage {
    pub fn new(
        dimensions: UVec3,
        origin: Vec3,
        cell_size: f32,
        rounding: WorldRoundingPolicy,
    ) -> Self {
        let len = dimensions.x as usize * dimensions.y as usize * dimensions.z as usize;
        Self {
            dimensions,
            space: GridSpace {
                origin,
                cell_size,
                rounding,
            },
            cells: vec![CellData::default(); len],
            transitions: BTreeMap::new(),
        }
    }

    pub fn bounds(&self) -> GridAabb {
        GridAabb::from_min_size(GridCoord::ZERO, self.dimensions)
    }

    pub fn contains(&self, coord: GridCoord) -> bool {
        coord.x() >= 0
            && coord.y() >= 0
            && coord.z() >= 0
            && coord.x() < self.dimensions.x as i32
            && coord.y() < self.dimensions.y as i32
            && coord.z() < self.dimensions.z as i32
    }

    pub fn index(&self, coord: GridCoord) -> Option<usize> {
        if !self.contains(coord) {
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

    pub fn cell(&self, coord: GridCoord) -> Option<&CellData> {
        self.index(coord).map(|index| &self.cells[index])
    }

    pub fn cell_mut(&mut self, coord: GridCoord) -> Option<&mut CellData> {
        self.index(coord).map(move |index| &mut self.cells[index])
    }

    pub fn set_cell(&mut self, coord: GridCoord, cell: CellData) -> bool {
        if let Some(slot) = self.cell_mut(coord) {
            *slot = cell;
            true
        } else {
            false
        }
    }

    pub fn fill_region(&mut self, region: GridAabb, mut f: impl FnMut(GridCoord, &mut CellData)) {
        if let Some(region) = region.clamp_to(self.bounds()) {
            for coord in region.iter() {
                if let Some(cell) = self.cell_mut(coord) {
                    f(coord, cell);
                }
            }
        }
    }

    pub fn set_walkable(&mut self, coord: GridCoord, walkable: bool) -> bool {
        if let Some(cell) = self.cell_mut(coord) {
            cell.walkable = walkable;
            true
        } else {
            false
        }
    }

    pub fn add_transition(&mut self, from: GridCoord, transition: TransitionLink) {
        self.transitions
            .entry(from)
            .or_default()
            .push(transition.clone());
        if !transition.one_way {
            self.transitions
                .entry(transition.target)
                .or_default()
                .push(TransitionLink {
                    target: from,
                    cost: transition.cost,
                    required_mask: transition.required_mask,
                    one_way: false,
                    kind: transition.kind,
                });
        }
    }

    pub fn transitions_from(&self, coord: GridCoord) -> &[TransitionLink] {
        self.transitions
            .get(&coord)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn world_to_grid(&self, position: Vec3) -> GridCoord {
        self.space.to_grid(position)
    }

    pub fn grid_to_world_center(&self, coord: GridCoord) -> Vec3 {
        self.space.to_world_center(coord)
    }

    pub fn nearest_walkable(
        &self,
        start: GridCoord,
        profile: &PathFilterProfile,
    ) -> Option<GridCoord> {
        if self.is_passable(start, profile, &[]) {
            return Some(start);
        }

        let mut queue = VecDeque::from([start]);
        let mut visited = BTreeMap::from([(start, ())]);
        while let Some(current) = queue.pop_front() {
            for delta in cardinal_deltas_3d() {
                let neighbor = current.offset(delta);
                if visited.contains_key(&neighbor) || !self.contains(neighbor) {
                    continue;
                }
                if self.is_passable(neighbor, profile, &[]) {
                    return Some(neighbor);
                }
                visited.insert(neighbor, ());
                queue.push_back(neighbor);
            }
        }
        None
    }

    pub fn is_passable(
        &self,
        coord: GridCoord,
        profile: &PathFilterProfile,
        overlays: &[PathCostOverlay],
    ) -> bool {
        let Some(cell) = self.cell(coord) else {
            return false;
        };
        if !cell.walkable || cell.clearance < profile.clearance {
            return false;
        }
        if cell.traversal_mask.intersects(profile.blocked_mask) {
            return false;
        }
        if !profile.allowed_mask.contains(cell.traversal_mask) {
            return false;
        }
        if overlays
            .iter()
            .any(|overlay| overlay.region.contains(coord) && overlay.added_cost.is_infinite())
        {
            return false;
        }
        true
    }

    pub fn traversal_cost(
        &self,
        from: GridCoord,
        to: GridCoord,
        profile: &PathFilterProfile,
        overlays: &[PathCostOverlay],
    ) -> Option<f32> {
        let to_cell = self.cell(to)?;
        if !self.is_passable(to, profile, overlays) {
            return None;
        }
        let overlay_cost = overlays
            .iter()
            .filter(|overlay| overlay.region.contains(to))
            .map(|overlay| overlay.added_cost)
            .sum::<f32>();
        let movement = NeighborhoodMode::movement_cost(to.0 - from.0);
        Some(movement * (to_cell.base_cost * profile.multiplier_for(to_cell.area) + overlay_cost))
    }

    pub fn neighbor_cells(
        &self,
        coord: GridCoord,
        neighborhood: NeighborhoodMode,
        allow_corner_cutting: bool,
        profile: &PathFilterProfile,
        overlays: &[PathCostOverlay],
    ) -> Vec<(GridCoord, f32)> {
        let mut output = Vec::new();
        for delta in deltas_for_mode(neighborhood) {
            let next = coord.offset(delta);
            if !self.contains(next) {
                continue;
            }
            if !allow_corner_cutting && delta.x != 0 && delta.y != 0 {
                let side_a = GridCoord::new(coord.x() + delta.x, coord.y(), coord.z());
                let side_b = GridCoord::new(coord.x(), coord.y() + delta.y, coord.z());
                if !self.is_passable(side_a, profile, overlays)
                    || !self.is_passable(side_b, profile, overlays)
                {
                    continue;
                }
            }
            if let Some(cost) = self.traversal_cost(coord, next, profile, overlays) {
                output.push((next, cost));
            }
        }
        for transition in self.transitions_from(coord) {
            if transition.required_mask != AreaMask::EMPTY
                && !profile.allowed_mask.contains(transition.required_mask)
            {
                continue;
            }
            if self.is_passable(transition.target, profile, overlays) {
                output.push((transition.target, transition.cost));
            }
        }
        output
    }

    pub fn raycast_line_of_sight(
        &self,
        start: GridCoord,
        goal: GridCoord,
        profile: &PathFilterProfile,
        overlays: &[PathCostOverlay],
    ) -> bool {
        let start_world = self.grid_to_world_center(start);
        let goal_world = self.grid_to_world_center(goal);
        let delta = goal_world - start_world;
        let steps = delta.abs().max_element().max(1.0) / self.space.cell_size.max(0.001);
        let steps = steps.ceil() as i32;
        for index in 0..=steps {
            let t = index as f32 / steps as f32;
            let sample = start_world.lerp(goal_world, t);
            let coord = self.world_to_grid(sample);
            if !self.is_passable(coord, profile, overlays) {
                return false;
            }
        }
        true
    }
}

fn cardinal_deltas_3d() -> [IVec3; 6] {
    [
        IVec3::new(1, 0, 0),
        IVec3::new(-1, 0, 0),
        IVec3::new(0, 1, 0),
        IVec3::new(0, -1, 0),
        IVec3::new(0, 0, 1),
        IVec3::new(0, 0, -1),
    ]
}

pub fn deltas_for_mode(mode: NeighborhoodMode) -> Vec<IVec3> {
    let mut deltas = Vec::new();
    for z in -1_i32..=1 {
        for y in -1_i32..=1 {
            for x in -1_i32..=1 {
                if x == 0 && y == 0 && z == 0 {
                    continue;
                }
                let abs = x.abs() + y.abs() + z.abs();
                let delta = IVec3::new(x, y, z);
                match mode {
                    NeighborhoodMode::Cardinal2d if z == 0 && abs == 1 => deltas.push(delta),
                    NeighborhoodMode::Ordinal2d if z == 0 => deltas.push(delta),
                    NeighborhoodMode::Cardinal3d if abs == 1 => deltas.push(delta),
                    NeighborhoodMode::Ordinal18 if abs <= 2 => deltas.push(delta),
                    NeighborhoodMode::Ordinal26 => deltas.push(delta),
                    _ => {}
                }
            }
        }
    }
    deltas
}

#[cfg(test)]
#[path = "grid_tests.rs"]
mod tests;
