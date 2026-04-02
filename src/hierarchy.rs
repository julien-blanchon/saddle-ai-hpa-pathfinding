use crate::{
    config::{HpaPathfindingConfig, NeighborhoodMode},
    coord::{GridAabb, GridCoord},
    filters::PathFilterProfile,
    grid::GridStorage,
};
use bevy::prelude::*;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub struct ClusterKey {
    pub level: u8,
    pub coord: GridCoord,
}

impl ClusterKey {
    pub fn new(level: u8, coord: GridCoord) -> Self {
        Self { level, coord }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub struct ClusterVersionStamp {
    pub cluster: ClusterKey,
    pub version: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub enum EdgeKind {
    Projection,
    InterCluster,
    IntraCluster,
}

#[derive(Debug, Clone, Reflect)]
pub enum EdgeRoute {
    Cells(Vec<GridCoord>),
    Nodes(Vec<usize>),
    Projection,
}

#[derive(Debug, Clone, Reflect)]
pub struct NodeEdge {
    pub to: usize,
    pub cost: f32,
    pub kind: EdgeKind,
    pub route: EdgeRoute,
}

#[derive(Debug, Clone, Reflect)]
pub struct AbstractNode {
    pub id: usize,
    pub level: u8,
    pub cluster: ClusterKey,
    pub coord: GridCoord,
    pub anchor_lower: Option<usize>,
}

#[derive(Debug, Clone, Reflect)]
pub struct ClusterInfo {
    pub key: ClusterKey,
    pub bounds: GridAabb,
    pub node_ids: Vec<usize>,
    pub version: u64,
    pub dirty: bool,
}

#[derive(Debug, Clone, Reflect)]
pub struct HierarchyLevel {
    pub level: u8,
    pub cluster_size: UVec3,
    pub clusters: HashMap<ClusterKey, ClusterInfo>,
}

#[derive(Debug, Clone, Reflect)]
pub struct PathfindingSnapshot {
    pub version: u64,
    pub config: HpaPathfindingConfig,
    pub grid: GridStorage,
    pub levels: Vec<HierarchyLevel>,
    pub nodes: Vec<AbstractNode>,
    pub edges: Vec<Vec<NodeEdge>>,
}

impl PathfindingSnapshot {
    pub fn build(grid: GridStorage, config: HpaPathfindingConfig, version: u64) -> Self {
        let mut snapshot = Self {
            version,
            config,
            grid,
            levels: Vec::new(),
            nodes: Vec::new(),
            edges: Vec::new(),
        };

        let max_levels = snapshot.config.clamped_hierarchy_levels();
        for level in 1..=max_levels {
            let cluster_size = cluster_size_for_level(snapshot.config.cluster_size, level);
            let clusters = build_cluster_index(&snapshot.grid, level, cluster_size);
            snapshot.levels.push(HierarchyLevel {
                level,
                cluster_size,
                clusters,
            });
            if level == 1 {
                build_level_one(&mut snapshot, level);
            } else {
                build_higher_level(&mut snapshot, level);
            }
        }

        snapshot
    }

    pub fn level(&self, level: u8) -> Option<&HierarchyLevel> {
        self.levels
            .iter()
            .find(|candidate| candidate.level == level)
    }

    pub fn level_mut(&mut self, level: u8) -> Option<&mut HierarchyLevel> {
        self.levels
            .iter_mut()
            .find(|candidate| candidate.level == level)
    }

    pub fn cluster_key_for_coord(&self, level: u8, coord: GridCoord) -> ClusterKey {
        let size = cluster_size_for_level(self.config.cluster_size, level).as_ivec3();
        ClusterKey::new(
            level,
            GridCoord::new(coord.x() / size.x, coord.y() / size.y, coord.z() / size.z),
        )
    }

    pub fn cluster_versions_for_corridor(
        &self,
        corridor: &[GridCoord],
    ) -> Vec<ClusterVersionStamp> {
        let mut versions = HashMap::<ClusterKey, u64>::new();
        for coord in corridor {
            let key = self.cluster_key_for_coord(1, *coord);
            if let Some(cluster) = self.level(1).and_then(|level| level.clusters.get(&key)) {
                versions.entry(key).or_insert(cluster.version);
            }
        }
        let mut output = versions
            .into_iter()
            .map(|(cluster, version)| ClusterVersionStamp { cluster, version })
            .collect::<Vec<_>>();
        output.sort_by_key(|stamp| {
            (
                stamp.cluster.level,
                stamp.cluster.coord.x(),
                stamp.cluster.coord.y(),
                stamp.cluster.coord.z(),
            )
        });
        output
    }

    pub fn dirty_clusters(&self) -> Vec<ClusterKey> {
        let mut output = Vec::new();
        for level in &self.levels {
            for (key, cluster) in &level.clusters {
                if cluster.dirty {
                    output.push(*key);
                }
            }
        }
        output.sort_by_key(|key| (key.level, key.coord.x(), key.coord.y(), key.coord.z()));
        output
    }
}

pub fn cluster_size_for_level(base: UVec3, level: u8) -> UVec3 {
    let scale = 1_u32 << level.saturating_sub(1);
    UVec3::new(base.x * scale, base.y * scale, base.z * scale)
}

fn build_cluster_index(
    grid: &GridStorage,
    level: u8,
    cluster_size: UVec3,
) -> HashMap<ClusterKey, ClusterInfo> {
    let counts = UVec3::new(
        grid.dimensions.x.div_ceil(cluster_size.x.max(1)),
        grid.dimensions.y.div_ceil(cluster_size.y.max(1)),
        grid.dimensions.z.div_ceil(cluster_size.z.max(1)),
    );
    let mut clusters = HashMap::new();
    for z in 0..counts.z as i32 {
        for y in 0..counts.y as i32 {
            for x in 0..counts.x as i32 {
                let key = ClusterKey::new(level, GridCoord::new(x, y, z));
                let min = GridCoord::new(
                    x * cluster_size.x as i32,
                    y * cluster_size.y as i32,
                    z * cluster_size.z as i32,
                );
                let max = GridCoord::new(
                    ((x + 1) * cluster_size.x as i32 - 1).min(grid.dimensions.x as i32 - 1),
                    ((y + 1) * cluster_size.y as i32 - 1).min(grid.dimensions.y as i32 - 1),
                    ((z + 1) * cluster_size.z as i32 - 1).min(grid.dimensions.z as i32 - 1),
                );
                clusters.insert(
                    key,
                    ClusterInfo {
                        key,
                        bounds: GridAabb::new(min, max),
                        node_ids: Vec::new(),
                        version: 0,
                        dirty: false,
                    },
                );
            }
        }
    }
    clusters
}

fn add_node(snapshot: &mut PathfindingSnapshot, node: AbstractNode) -> usize {
    let id = node.id;
    snapshot.nodes.push(node);
    snapshot.edges.push(Vec::new());
    id
}

fn add_undirected_edge(
    snapshot: &mut PathfindingSnapshot,
    from: usize,
    to: usize,
    kind: EdgeKind,
    route: EdgeRoute,
    cost: f32,
) {
    snapshot.edges[from].push(NodeEdge {
        to,
        cost,
        kind,
        route: route.clone(),
    });
    snapshot.edges[to].push(NodeEdge {
        to: from,
        cost,
        kind,
        route,
    });
}

#[derive(Debug, Clone, Copy)]
struct BorderPair {
    left: GridCoord,
    right: GridCoord,
    u: i32,
    v: i32,
}

fn build_level_one(snapshot: &mut PathfindingSnapshot, level: u8) {
    let cluster_keys = sorted_cluster_keys(snapshot.level(level).unwrap());
    for key in cluster_keys {
        for axis in active_axes(snapshot.config.neighborhood, snapshot.grid.dimensions.z) {
            let neighbor_coord = key.coord.offset(axis.as_ivec3());
            let neighbor_key = ClusterKey::new(level, neighbor_coord);
            if !snapshot
                .level(level)
                .unwrap()
                .clusters
                .contains_key(&neighbor_key)
            {
                continue;
            }
            if compare_cluster_key(key, neighbor_key) != Ordering::Less {
                continue;
            }
            let Some(pairs) = border_pairs_for_level_one(snapshot, key, neighbor_key, axis) else {
                continue;
            };
            for pair in select_representative_pairs(&pairs) {
                let left_id = add_node(
                    snapshot,
                    AbstractNode {
                        id: snapshot.nodes.len(),
                        level,
                        cluster: key,
                        coord: pair.left,
                        anchor_lower: None,
                    },
                );
                let right_id = add_node(
                    snapshot,
                    AbstractNode {
                        id: snapshot.nodes.len(),
                        level,
                        cluster: neighbor_key,
                        coord: pair.right,
                        anchor_lower: None,
                    },
                );
                snapshot
                    .level_mut(level)
                    .unwrap()
                    .clusters
                    .get_mut(&key)
                    .unwrap()
                    .node_ids
                    .push(left_id);
                snapshot
                    .level_mut(level)
                    .unwrap()
                    .clusters
                    .get_mut(&neighbor_key)
                    .unwrap()
                    .node_ids
                    .push(right_id);
                add_undirected_edge(
                    snapshot,
                    left_id,
                    right_id,
                    EdgeKind::InterCluster,
                    EdgeRoute::Cells(vec![pair.left, pair.right]),
                    base_transition_cost(snapshot, pair.left, pair.right),
                );
            }
        }
    }
    connect_intra_cluster_edges(snapshot, level);
}

fn build_higher_level(snapshot: &mut PathfindingSnapshot, level: u8) {
    let lower_level = level - 1;
    let cluster_keys = sorted_cluster_keys(snapshot.level(level).unwrap());
    for key in cluster_keys {
        for axis in active_axes(snapshot.config.neighborhood, snapshot.grid.dimensions.z) {
            let neighbor_coord = key.coord.offset(axis.as_ivec3());
            let neighbor_key = ClusterKey::new(level, neighbor_coord);
            if !snapshot
                .level(level)
                .unwrap()
                .clusters
                .contains_key(&neighbor_key)
            {
                continue;
            }
            if compare_cluster_key(key, neighbor_key) != Ordering::Less {
                continue;
            }

            let candidate_pairs =
                lower_level_crossings(snapshot, lower_level, key, neighbor_key, axis);
            for (left_lower, right_lower) in select_representative_lower_pairs(&candidate_pairs) {
                let left_coord = snapshot.nodes[left_lower].coord;
                let right_coord = snapshot.nodes[right_lower].coord;
                let left_id = add_node(
                    snapshot,
                    AbstractNode {
                        id: snapshot.nodes.len(),
                        level,
                        cluster: key,
                        coord: left_coord,
                        anchor_lower: Some(left_lower),
                    },
                );
                let right_id = add_node(
                    snapshot,
                    AbstractNode {
                        id: snapshot.nodes.len(),
                        level,
                        cluster: neighbor_key,
                        coord: right_coord,
                        anchor_lower: Some(right_lower),
                    },
                );
                snapshot
                    .level_mut(level)
                    .unwrap()
                    .clusters
                    .get_mut(&key)
                    .unwrap()
                    .node_ids
                    .push(left_id);
                snapshot
                    .level_mut(level)
                    .unwrap()
                    .clusters
                    .get_mut(&neighbor_key)
                    .unwrap()
                    .node_ids
                    .push(right_id);
                add_undirected_edge(
                    snapshot,
                    left_id,
                    right_id,
                    EdgeKind::InterCluster,
                    EdgeRoute::Nodes(vec![left_lower, right_lower]),
                    direct_edge_cost(snapshot, left_lower, right_lower).unwrap_or(1.0),
                );
                add_undirected_edge(
                    snapshot,
                    left_id,
                    left_lower,
                    EdgeKind::Projection,
                    EdgeRoute::Projection,
                    0.0,
                );
                add_undirected_edge(
                    snapshot,
                    right_id,
                    right_lower,
                    EdgeKind::Projection,
                    EdgeRoute::Projection,
                    0.0,
                );
            }
        }
    }
    connect_intra_cluster_edges(snapshot, level);
}

fn connect_intra_cluster_edges(snapshot: &mut PathfindingSnapshot, level: u8) {
    let cluster_keys = sorted_cluster_keys(snapshot.level(level).unwrap());
    for key in cluster_keys {
        let (bounds, node_ids) = {
            let cluster = snapshot.level(level).unwrap().clusters.get(&key).unwrap();
            (cluster.bounds, cluster.node_ids.clone())
        };
        for i in 0..node_ids.len() {
            for j in i + 1..node_ids.len() {
                let start = node_ids[i];
                let goal = node_ids[j];
                let result = if level == 1 {
                    low_level_path(
                        snapshot,
                        snapshot.nodes[start].coord,
                        snapshot.nodes[goal].coord,
                        bounds,
                    )
                    .map(|(corridor, cost)| (EdgeRoute::Cells(corridor), cost))
                } else {
                    let lower_level = level - 1;
                    let start_anchor = snapshot.nodes[start].anchor_lower.unwrap();
                    let goal_anchor = snapshot.nodes[goal].anchor_lower.unwrap();
                    search_same_level_nodes(
                        snapshot,
                        lower_level,
                        start_anchor,
                        goal_anchor,
                        bounds,
                    )
                    .map(|(route, cost)| (EdgeRoute::Nodes(route), cost))
                };
                if let Some((route, cost)) = result {
                    add_undirected_edge(snapshot, start, goal, EdgeKind::IntraCluster, route, cost);
                }
            }
        }
    }
}

fn border_pairs_for_level_one(
    snapshot: &PathfindingSnapshot,
    left_key: ClusterKey,
    right_key: ClusterKey,
    axis: FaceAxis,
) -> Option<Vec<BorderPair>> {
    let left = snapshot.level(1)?.clusters.get(&left_key)?;
    let right = snapshot.level(1)?.clusters.get(&right_key)?;
    let mut pairs = Vec::new();
    let left_face = left.bounds;
    let right_face = right.bounds;
    match axis {
        FaceAxis::X => {
            let x_left = left_face.max.x();
            let x_right = right_face.min.x();
            for z in left_face.min.z()..=left_face.max.z() {
                for y in left_face.min.y()..=left_face.max.y() {
                    let a = GridCoord::new(x_left, y, z);
                    let b = GridCoord::new(x_right, y, z);
                    if snapshot.grid.cell(a).is_some_and(|cell| cell.walkable)
                        && snapshot.grid.cell(b).is_some_and(|cell| cell.walkable)
                    {
                        pairs.push(BorderPair {
                            left: a,
                            right: b,
                            u: y,
                            v: z,
                        });
                    }
                }
            }
        }
        FaceAxis::Y => {
            let y_left = left_face.max.y();
            let y_right = right_face.min.y();
            for z in left_face.min.z()..=left_face.max.z() {
                for x in left_face.min.x()..=left_face.max.x() {
                    let a = GridCoord::new(x, y_left, z);
                    let b = GridCoord::new(x, y_right, z);
                    if snapshot.grid.cell(a).is_some_and(|cell| cell.walkable)
                        && snapshot.grid.cell(b).is_some_and(|cell| cell.walkable)
                    {
                        pairs.push(BorderPair {
                            left: a,
                            right: b,
                            u: x,
                            v: z,
                        });
                    }
                }
            }
        }
        FaceAxis::Z => {
            let z_left = left_face.max.z();
            let z_right = right_face.min.z();
            for y in left_face.min.y()..=left_face.max.y() {
                for x in left_face.min.x()..=left_face.max.x() {
                    let a = GridCoord::new(x, y, z_left);
                    let b = GridCoord::new(x, y, z_right);
                    if snapshot.grid.cell(a).is_some_and(|cell| cell.walkable)
                        && snapshot.grid.cell(b).is_some_and(|cell| cell.walkable)
                    {
                        pairs.push(BorderPair {
                            left: a,
                            right: b,
                            u: x,
                            v: y,
                        });
                    }
                }
            }
        }
    }
    if pairs.is_empty() { None } else { Some(pairs) }
}

fn lower_level_crossings(
    snapshot: &PathfindingSnapshot,
    lower_level: u8,
    left_parent: ClusterKey,
    right_parent: ClusterKey,
    axis: FaceAxis,
) -> Vec<(usize, usize)> {
    let mut pairs = Vec::new();
    for node in snapshot
        .nodes
        .iter()
        .filter(|node| node.level == lower_level)
    {
        for edge in &snapshot.edges[node.id] {
            let other = &snapshot.nodes[edge.to];
            if other.level != lower_level
                || edge.kind != EdgeKind::InterCluster
                || node.id > other.id
            {
                continue;
            }
            let parent_a = snapshot.cluster_key_for_coord(left_parent.level, node.coord);
            let parent_b = snapshot.cluster_key_for_coord(right_parent.level, other.coord);
            if ((parent_a == left_parent && parent_b == right_parent)
                || (parent_a == right_parent && parent_b == left_parent))
                && axis.matches_pair(node.coord, other.coord)
            {
                pairs.push((node.id, other.id));
            }
        }
    }
    pairs
}

fn select_representative_pairs(pairs: &[BorderPair]) -> Vec<BorderPair> {
    let groups = group_border_pairs(pairs);
    let mut selected = Vec::new();
    for group in groups {
        if group.len() <= 2 {
            selected.extend(group);
            continue;
        }
        let min_u = group.iter().min_by_key(|pair| pair.u).copied().unwrap();
        let max_u = group.iter().max_by_key(|pair| pair.u).copied().unwrap();
        let min_v = group.iter().min_by_key(|pair| pair.v).copied().unwrap();
        let max_v = group.iter().max_by_key(|pair| pair.v).copied().unwrap();
        let spread_u = max_u.u - min_u.u;
        let spread_v = max_v.v - min_v.v;
        if spread_u >= spread_v {
            selected.push(min_u);
            if min_u.left != max_u.left {
                selected.push(max_u);
            }
        } else {
            selected.push(min_v);
            if min_v.left != max_v.left {
                selected.push(max_v);
            }
        }
    }
    selected
}

fn select_representative_lower_pairs(pairs: &[(usize, usize)]) -> Vec<(usize, usize)> {
    if pairs.len() <= 2 {
        return pairs.to_vec();
    }
    vec![pairs[0], *pairs.last().unwrap()]
}

fn group_border_pairs(pairs: &[BorderPair]) -> Vec<Vec<BorderPair>> {
    let mut groups = Vec::new();
    let mut visited = vec![false; pairs.len()];
    for index in 0..pairs.len() {
        if visited[index] {
            continue;
        }
        let mut queue = VecDeque::from([index]);
        let mut group = Vec::new();
        visited[index] = true;
        while let Some(current) = queue.pop_front() {
            let seed = pairs[current];
            group.push(seed);
            for (candidate_index, candidate) in pairs.iter().enumerate() {
                if visited[candidate_index] {
                    continue;
                }
                if (seed.u - candidate.u).abs() + (seed.v - candidate.v).abs() == 1 {
                    visited[candidate_index] = true;
                    queue.push_back(candidate_index);
                }
            }
        }
        groups.push(group);
    }
    groups
}

fn base_transition_cost(snapshot: &PathfindingSnapshot, from: GridCoord, to: GridCoord) -> f32 {
    let movement = NeighborhoodMode::movement_cost(to.0 - from.0);
    let destination_cost = snapshot
        .grid
        .cell(to)
        .map(|cell| cell.base_cost)
        .unwrap_or(1.0);
    movement * destination_cost
}

fn direct_edge_cost(snapshot: &PathfindingSnapshot, from: usize, to: usize) -> Option<f32> {
    snapshot
        .edges
        .get(from)?
        .iter()
        .find(|edge| edge.to == to)
        .map(|edge| edge.cost)
}

fn low_level_path(
    snapshot: &PathfindingSnapshot,
    start: GridCoord,
    goal: GridCoord,
    bounds: GridAabb,
) -> Option<(Vec<GridCoord>, f32)> {
    let profile = PathFilterProfile::default();
    let mut open = BinaryHeap::new();
    let mut parents = HashMap::<GridCoord, GridCoord>::new();
    let mut g_score = HashMap::<GridCoord, f32>::new();
    open.push(CostNode::new(start, 0.0, heuristic(start, goal)));
    g_score.insert(start, 0.0);

    while let Some(current) = open.pop() {
        if current.coord == goal {
            let mut corridor = vec![goal];
            let mut cursor = goal;
            while let Some(parent) = parents.get(&cursor).copied() {
                corridor.push(parent);
                cursor = parent;
            }
            corridor.reverse();
            return Some((corridor, *g_score.get(&goal).unwrap()));
        }
        for (neighbor, step_cost) in snapshot.grid.neighbor_cells(
            current.coord,
            snapshot.config.neighborhood,
            snapshot.config.allow_corner_cutting,
            &profile,
            &[],
        ) {
            if !bounds.contains(neighbor) {
                continue;
            }
            let tentative = g_score
                .get(&current.coord)
                .copied()
                .unwrap_or(f32::INFINITY)
                + step_cost;
            if tentative + 0.0001 < g_score.get(&neighbor).copied().unwrap_or(f32::INFINITY) {
                parents.insert(neighbor, current.coord);
                g_score.insert(neighbor, tentative);
                open.push(CostNode::new(
                    neighbor,
                    tentative,
                    heuristic(neighbor, goal),
                ));
            }
        }
    }
    None
}

fn search_same_level_nodes(
    snapshot: &PathfindingSnapshot,
    level: u8,
    start: usize,
    goal: usize,
    bounds: GridAabb,
) -> Option<(Vec<usize>, f32)> {
    let mut open = BinaryHeap::new();
    let mut parents = HashMap::<usize, usize>::new();
    let mut g_score = HashMap::<usize, f32>::new();
    open.push(NodeState::new(
        start,
        0.0,
        heuristic(snapshot.nodes[start].coord, snapshot.nodes[goal].coord),
    ));
    g_score.insert(start, 0.0);

    while let Some(current) = open.pop() {
        if current.id == goal {
            let mut route = vec![goal];
            let mut cursor = goal;
            while let Some(parent) = parents.get(&cursor).copied() {
                route.push(parent);
                cursor = parent;
            }
            route.reverse();
            return Some((route, *g_score.get(&goal).unwrap()));
        }

        for edge in snapshot.edges[current.id].iter().filter(|edge| {
            let node = &snapshot.nodes[edge.to];
            node.level == level && bounds.contains(node.coord)
        }) {
            let tentative = g_score.get(&current.id).copied().unwrap_or(f32::INFINITY) + edge.cost;
            if tentative + 0.0001 < g_score.get(&edge.to).copied().unwrap_or(f32::INFINITY) {
                parents.insert(edge.to, current.id);
                g_score.insert(edge.to, tentative);
                open.push(NodeState::new(
                    edge.to,
                    tentative,
                    heuristic(snapshot.nodes[edge.to].coord, snapshot.nodes[goal].coord),
                ));
            }
        }
    }
    None
}

fn heuristic(a: GridCoord, b: GridCoord) -> f32 {
    let delta = (a.0 - b.0).abs();
    (delta.x + delta.y + delta.z) as f32
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FaceAxis {
    X,
    Y,
    Z,
}

impl FaceAxis {
    fn as_ivec3(self) -> IVec3 {
        match self {
            Self::X => IVec3::X,
            Self::Y => IVec3::Y,
            Self::Z => IVec3::Z,
        }
    }

    fn matches_pair(self, a: GridCoord, b: GridCoord) -> bool {
        match self {
            Self::X => (a.x() - b.x()).abs() == 1,
            Self::Y => (a.y() - b.y()).abs() == 1,
            Self::Z => (a.z() - b.z()).abs() == 1,
        }
    }
}

fn active_axes(mode: NeighborhoodMode, depth: u32) -> Vec<FaceAxis> {
    let mut axes = vec![FaceAxis::X, FaceAxis::Y];
    if depth > 1
        && !matches!(
            mode,
            NeighborhoodMode::Cardinal2d | NeighborhoodMode::Ordinal2d
        )
    {
        axes.push(FaceAxis::Z);
    }
    axes
}

fn sorted_cluster_keys(level: &HierarchyLevel) -> Vec<ClusterKey> {
    let mut keys = level.clusters.keys().copied().collect::<Vec<_>>();
    keys.sort_by(|left, right| compare_cluster_key(*left, *right));
    keys
}

fn compare_cluster_key(a: ClusterKey, b: ClusterKey) -> Ordering {
    (a.coord.z(), a.coord.y(), a.coord.x()).cmp(&(b.coord.z(), b.coord.y(), b.coord.x()))
}

#[derive(Debug, Clone, Copy)]
struct CostNode {
    coord: GridCoord,
    g: f32,
    h: f32,
}

impl CostNode {
    fn new(coord: GridCoord, g: f32, h: f32) -> Self {
        Self { coord, g, h }
    }
}

impl PartialEq for CostNode {
    fn eq(&self, other: &Self) -> bool {
        self.coord == other.coord
    }
}

impl Eq for CostNode {}

impl PartialOrd for CostNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CostNode {
    fn cmp(&self, other: &Self) -> Ordering {
        let self_f = self.g + self.h;
        let other_f = other.g + other.h;
        other_f
            .total_cmp(&self_f)
            .then_with(|| other.h.total_cmp(&self.h))
            .then_with(|| self.coord.x().cmp(&other.coord.x()))
            .then_with(|| self.coord.y().cmp(&other.coord.y()))
            .then_with(|| self.coord.z().cmp(&other.coord.z()))
    }
}

#[derive(Debug, Clone, Copy)]
struct NodeState {
    id: usize,
    g: f32,
    h: f32,
}

impl NodeState {
    fn new(id: usize, g: f32, h: f32) -> Self {
        Self { id, g, h }
    }
}

impl PartialEq for NodeState {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for NodeState {}

impl PartialOrd for NodeState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for NodeState {
    fn cmp(&self, other: &Self) -> Ordering {
        let self_f = self.g + self.h;
        let other_f = other.g + other.h;
        other_f
            .total_cmp(&self_f)
            .then_with(|| other.id.cmp(&self.id))
    }
}

#[cfg(test)]
#[path = "hierarchy_tests.rs"]
mod tests;
