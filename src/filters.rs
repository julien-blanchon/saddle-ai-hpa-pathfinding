use crate::coord::GridAabb;
use bevy::prelude::*;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Reflect, Default)]
pub struct AreaTypeId(pub u8);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Reflect, Default)]
pub struct PathFilterId(pub u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Default)]
pub struct AreaMask(pub u64);

impl AreaMask {
    pub const ALL: Self = Self(u64::MAX);
    pub const EMPTY: Self = Self(0);

    pub const fn from_bit(bit: u8) -> Self {
        Self(1_u64 << bit)
    }

    pub fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    pub fn intersects(self, other: Self) -> bool {
        (self.0 & other.0) != 0
    }
}

#[derive(Debug, Clone, Reflect)]
pub struct PathFilterProfile {
    pub id: PathFilterId,
    pub name: String,
    pub allowed_mask: AreaMask,
    pub blocked_mask: AreaMask,
    pub clearance: u16,
    pub area_cost_multipliers: BTreeMap<AreaTypeId, f32>,
}

impl Default for PathFilterProfile {
    fn default() -> Self {
        Self::named("default")
    }
}

impl PathFilterProfile {
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            id: PathFilterId(0),
            name: name.into(),
            allowed_mask: AreaMask::ALL,
            blocked_mask: AreaMask::EMPTY,
            clearance: 0,
            area_cost_multipliers: BTreeMap::new(),
        }
    }

    pub fn with_id(mut self, id: PathFilterId) -> Self {
        self.id = id;
        self
    }

    pub fn with_blocked_mask(mut self, blocked_mask: AreaMask) -> Self {
        self.blocked_mask = blocked_mask;
        self
    }

    pub fn with_allowed_mask(mut self, allowed_mask: AreaMask) -> Self {
        self.allowed_mask = allowed_mask;
        self
    }

    pub fn with_clearance(mut self, clearance: u16) -> Self {
        self.clearance = clearance;
        self
    }

    pub fn with_area_cost(mut self, area: AreaTypeId, multiplier: f32) -> Self {
        self.area_cost_multipliers.insert(area, multiplier);
        self
    }

    pub fn multiplier_for(&self, area: AreaTypeId) -> f32 {
        self.area_cost_multipliers
            .get(&area)
            .copied()
            .unwrap_or(1.0)
    }
}

#[derive(Debug, Clone, PartialEq, Reflect)]
pub struct PathCostOverlay {
    pub region: GridAabb,
    pub added_cost: f32,
}

impl PathCostOverlay {
    pub fn new(region: GridAabb, added_cost: f32) -> Self {
        Self { region, added_cost }
    }
}

pub fn overlay_signature(overlays: &[PathCostOverlay]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for overlay in overlays {
        for coord in [overlay.region.min, overlay.region.max] {
            for axis in [coord.x(), coord.y(), coord.z()] {
                hash ^= axis as i64 as u64;
                hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            }
        }
        hash ^= overlay.added_cost.to_bits() as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

#[derive(Debug, Clone, Default, Reflect)]
pub struct PathFilterLibrary {
    pub next_id: u16,
    pub profiles: BTreeMap<PathFilterId, PathFilterProfile>,
}

impl PathFilterLibrary {
    pub fn register(&mut self, mut profile: PathFilterProfile) -> PathFilterId {
        let id = if profile.id == PathFilterId(0) && self.profiles.contains_key(&PathFilterId(0)) {
            self.next_id = self.next_id.max(1);
            let id = PathFilterId(self.next_id);
            self.next_id += 1;
            id
        } else if profile.id == PathFilterId(0) && self.profiles.is_empty() {
            PathFilterId(0)
        } else {
            profile.id
        };
        profile.id = id;
        self.profiles.insert(id, profile);
        id
    }

    pub fn get(&self, id: PathFilterId) -> Option<&PathFilterProfile> {
        self.profiles.get(&id)
    }
}

#[cfg(test)]
#[path = "filters_tests.rs"]
mod tests;
