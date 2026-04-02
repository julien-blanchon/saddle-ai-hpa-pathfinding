use bevy::prelude::*;
use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub struct GridCoord(pub IVec3);

impl GridCoord {
    pub const ZERO: Self = Self(IVec3::ZERO);

    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self(IVec3::new(x, y, z))
    }

    pub const fn x(self) -> i32 {
        self.0.x
    }

    pub const fn y(self) -> i32 {
        self.0.y
    }

    pub const fn z(self) -> i32 {
        self.0.z
    }

    pub fn as_ivec3(self) -> IVec3 {
        self.0
    }

    pub fn offset(self, delta: IVec3) -> Self {
        Self(self.0 + delta)
    }
}

impl From<IVec3> for GridCoord {
    fn from(value: IVec3) -> Self {
        Self(value)
    }
}

impl From<GridCoord> for IVec3 {
    fn from(value: GridCoord) -> Self {
        value.0
    }
}

impl PartialOrd for GridCoord {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for GridCoord {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.z(), self.y(), self.x()).cmp(&(other.z(), other.y(), other.x()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub struct GridAabb {
    pub min: GridCoord,
    pub max: GridCoord,
}

impl GridAabb {
    pub fn new(min: GridCoord, max: GridCoord) -> Self {
        debug_assert!(min.x() <= max.x());
        debug_assert!(min.y() <= max.y());
        debug_assert!(min.z() <= max.z());
        Self { min, max }
    }

    pub fn from_min_size(min: GridCoord, size: UVec3) -> Self {
        let size = size.as_ivec3();
        Self::new(min, GridCoord::from(min.0 + size - IVec3::ONE))
    }

    pub fn contains(self, coord: GridCoord) -> bool {
        coord.x() >= self.min.x()
            && coord.x() <= self.max.x()
            && coord.y() >= self.min.y()
            && coord.y() <= self.max.y()
            && coord.z() >= self.min.z()
            && coord.z() <= self.max.z()
    }

    pub fn intersects(self, other: Self) -> bool {
        self.min.x() <= other.max.x()
            && self.max.x() >= other.min.x()
            && self.min.y() <= other.max.y()
            && self.max.y() >= other.min.y()
            && self.min.z() <= other.max.z()
            && self.max.z() >= other.min.z()
    }

    pub fn clamp_to(self, bounds: Self) -> Option<Self> {
        let min = GridCoord::from(IVec3::new(
            self.min.x().max(bounds.min.x()),
            self.min.y().max(bounds.min.y()),
            self.min.z().max(bounds.min.z()),
        ));
        let max = GridCoord::from(IVec3::new(
            self.max.x().min(bounds.max.x()),
            self.max.y().min(bounds.max.y()),
            self.max.z().min(bounds.max.z()),
        ));
        if min.x() > max.x() || min.y() > max.y() || min.z() > max.z() {
            None
        } else {
            Some(Self::new(min, max))
        }
    }

    pub fn size(self) -> UVec3 {
        let dims = self.max.0 - self.min.0 + IVec3::ONE;
        UVec3::new(dims.x as u32, dims.y as u32, dims.z as u32)
    }

    pub fn iter(self) -> impl Iterator<Item = GridCoord> {
        let min = self.min.0;
        let max = self.max.0;
        (min.z..=max.z).flat_map(move |z| {
            (min.y..=max.y).flat_map(move |y| (min.x..=max.x).map(move |x| GridCoord::new(x, y, z)))
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Default)]
pub enum WorldRoundingPolicy {
    #[default]
    Floor,
    Round,
    Ceil,
}

#[derive(Debug, Clone, Copy, PartialEq, Reflect)]
pub struct GridSpace {
    pub origin: Vec3,
    pub cell_size: f32,
    pub rounding: WorldRoundingPolicy,
}

impl GridSpace {
    pub fn to_world_center(self, coord: GridCoord) -> Vec3 {
        self.origin + (coord.0.as_vec3() + Vec3::splat(0.5)) * self.cell_size
    }

    pub fn to_world_corner(self, coord: GridCoord) -> Vec3 {
        self.origin + coord.0.as_vec3() * self.cell_size
    }

    pub fn to_grid(self, position: Vec3) -> GridCoord {
        let value = (position - self.origin) / self.cell_size;
        let rounded = match self.rounding {
            WorldRoundingPolicy::Floor => value.floor(),
            WorldRoundingPolicy::Round => value.round(),
            WorldRoundingPolicy::Ceil => value.ceil(),
        };
        GridCoord::from(rounded.as_ivec3())
    }
}

#[cfg(test)]
#[path = "coord_tests.rs"]
mod tests;
