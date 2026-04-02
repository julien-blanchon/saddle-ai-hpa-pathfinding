use crate::{
    config::PathQueryMode,
    coord::GridCoord,
    filters::{PathCostOverlay, PathFilterProfile},
    hierarchy::{ClusterVersionStamp, EdgeRoute, NodeEdge, PathfindingSnapshot},
    smoothing::smooth_corridor,
};
use bevy::prelude::*;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Reflect, Default)]
pub struct PathQueryId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Reflect, Default)]
pub struct PathVersion(pub u64);

#[derive(Debug, Clone, Reflect)]
pub struct ResolvedPath {
    pub corridor: Vec<GridCoord>,
    pub waypoints: Vec<Vec3>,
    pub total_cost: f32,
    pub is_partial: bool,
    pub version: PathVersion,
    pub touched_clusters: Vec<ClusterVersionStamp>,
}

#[derive(Debug, Clone, Reflect)]
pub struct CostEstimate {
    pub estimated_cost: Option<f32>,
    pub used_hierarchy: bool,
}

#[derive(Debug, Clone, Reflect)]
pub struct PathQueryInput {
    pub start: GridCoord,
    pub goal: GridCoord,
    pub mode: PathQueryMode,
    pub allow_partial: bool,
    pub overlays: Vec<PathCostOverlay>,
}

#[derive(Debug, Clone)]
pub struct SlicedGridSearch {
    snapshot: PathfindingSnapshot,
    profile: PathFilterProfile,
    overlays: Vec<PathCostOverlay>,
    goal: GridCoord,
    allow_partial: bool,
    open: BinaryHeap<GridNodeState>,
    parents: HashMap<GridCoord, GridCoord>,
    g_score: HashMap<GridCoord, f32>,
    best_partial: Option<(GridCoord, f32, f32)>,
    exhausted: bool,
}

impl SlicedGridSearch {
    pub fn new(
        snapshot: PathfindingSnapshot,
        start: GridCoord,
        goal: GridCoord,
        profile: PathFilterProfile,
        allow_partial: bool,
        overlays: Vec<PathCostOverlay>,
    ) -> Option<Self> {
        if !snapshot.grid.contains(start) || !snapshot.grid.contains(goal) {
            return None;
        }
        if !snapshot.grid.is_passable(start, &profile, &overlays) {
            return None;
        }

        let mut open = BinaryHeap::new();
        open.push(GridNodeState::new(start, 0.0, heuristic(start, goal)));

        let mut g_score = HashMap::new();
        g_score.insert(start, 0.0);

        Some(Self {
            snapshot,
            profile,
            overlays,
            goal,
            allow_partial,
            open,
            parents: HashMap::new(),
            g_score,
            best_partial: Some((start, heuristic(start, goal), 0.0)),
            exhausted: false,
        })
    }

    pub fn advance(&mut self, budget: u32) -> Option<Option<ResolvedPath>> {
        if self.exhausted {
            return Some(None);
        }

        for _ in 0..budget.max(1) {
            let Some(current) = self.open.pop() else {
                self.exhausted = true;
                return Some(self.partial_result());
            };

            if current.coord == self.goal {
                self.exhausted = true;
                return Some(Some(finalize_corridor(
                    &self.snapshot,
                    &self.profile,
                    &self.overlays,
                    reconstruct_grid_path(&self.parents, current.coord),
                    current.g,
                    false,
                )));
            }

            update_best_partial(&mut self.best_partial, current.coord, current.h, current.g);
            for (neighbor, step_cost) in self.snapshot.grid.neighbor_cells(
                current.coord,
                self.snapshot.config.neighborhood,
                self.snapshot.config.allow_corner_cutting,
                &self.profile,
                &self.overlays,
            ) {
                let tentative = current.g + step_cost;
                if tentative + 0.0001
                    < self
                        .g_score
                        .get(&neighbor)
                        .copied()
                        .unwrap_or(f32::INFINITY)
                {
                    self.parents.insert(neighbor, current.coord);
                    self.g_score.insert(neighbor, tentative);
                    self.open.push(GridNodeState::new(
                        neighbor,
                        tentative,
                        heuristic(neighbor, self.goal),
                    ));
                }
            }
        }

        None
    }

    fn partial_result(&self) -> Option<ResolvedPath> {
        if !self.allow_partial {
            return None;
        }
        let (coord, _, cost) = self.best_partial?;
        Some(finalize_corridor(
            &self.snapshot,
            &self.profile,
            &self.overlays,
            reconstruct_grid_path(&self.parents, coord),
            cost,
            true,
        ))
    }
}

pub fn nearest_walkable_cell(
    snapshot: &PathfindingSnapshot,
    start: GridCoord,
    profile: &PathFilterProfile,
) -> Option<GridCoord> {
    snapshot.grid.nearest_walkable(start, profile)
}

pub fn line_of_sight(
    snapshot: &PathfindingSnapshot,
    start: GridCoord,
    goal: GridCoord,
    profile: &PathFilterProfile,
    overlays: &[PathCostOverlay],
) -> bool {
    snapshot
        .grid
        .raycast_line_of_sight(start, goal, profile, overlays)
}

pub fn estimate_cost(
    snapshot: &PathfindingSnapshot,
    start: GridCoord,
    goal: GridCoord,
    profile: &PathFilterProfile,
    overlays: &[PathCostOverlay],
) -> CostEstimate {
    let cluster_match =
        snapshot.cluster_key_for_coord(1, start) == snapshot.cluster_key_for_coord(1, goal);
    if cluster_match {
        return CostEstimate {
            estimated_cost: direct_search(snapshot, start, goal, profile, false, overlays)
                .map(|path| path.total_cost),
            used_hierarchy: false,
        };
    }

    CostEstimate {
        estimated_cost: hierarchical_search(snapshot, start, goal, profile, false, overlays)
            .map(|path| path.total_cost)
            .or_else(|| {
                direct_search(snapshot, start, goal, profile, false, overlays)
                    .map(|path| path.total_cost)
            }),
        used_hierarchy: true,
    }
}

pub fn find_path(
    snapshot: &PathfindingSnapshot,
    start: GridCoord,
    goal: GridCoord,
    profile: &PathFilterProfile,
    mode: PathQueryMode,
    allow_partial: bool,
    overlays: &[PathCostOverlay],
) -> Option<ResolvedPath> {
    let start = if snapshot.grid.is_passable(start, profile, overlays) {
        start
    } else {
        nearest_walkable_cell(snapshot, start, profile)?
    };

    match mode {
        PathQueryMode::DirectOnly | PathQueryMode::Sliced => {
            direct_search(snapshot, start, goal, profile, allow_partial, overlays)
        }
        PathQueryMode::CoarseOnly => {
            hierarchical_search(snapshot, start, goal, profile, allow_partial, overlays)
                .or_else(|| direct_search(snapshot, start, goal, profile, allow_partial, overlays))
        }
        PathQueryMode::Auto => {
            let same_cluster =
                snapshot.cluster_key_for_coord(1, start) == snapshot.cluster_key_for_coord(1, goal);
            let chebyshev = (goal.0 - start.0).abs().max_element() as u32;
            if same_cluster || chebyshev <= snapshot.config.direct_search_distance {
                direct_search(snapshot, start, goal, profile, allow_partial, overlays).or_else(
                    || hierarchical_search(snapshot, start, goal, profile, allow_partial, overlays),
                )
            } else {
                hierarchical_search(snapshot, start, goal, profile, allow_partial, overlays)
                    .or_else(|| {
                        direct_search(snapshot, start, goal, profile, allow_partial, overlays)
                    })
            }
        }
    }
}

fn direct_search(
    snapshot: &PathfindingSnapshot,
    start: GridCoord,
    goal: GridCoord,
    profile: &PathFilterProfile,
    allow_partial: bool,
    overlays: &[PathCostOverlay],
) -> Option<ResolvedPath> {
    if !snapshot.grid.contains(start) || !snapshot.grid.contains(goal) {
        return None;
    }
    if !snapshot.grid.is_passable(start, profile, overlays) {
        return None;
    }

    let mut open = BinaryHeap::new();
    let mut parents = HashMap::<GridCoord, GridCoord>::new();
    let mut g_score = HashMap::<GridCoord, f32>::new();
    let mut best_partial = Some((start, heuristic(start, goal), 0.0));
    open.push(GridNodeState::new(start, 0.0, heuristic(start, goal)));
    g_score.insert(start, 0.0);

    while let Some(current) = open.pop() {
        if current.coord == goal {
            return Some(finalize_corridor(
                snapshot,
                profile,
                overlays,
                reconstruct_grid_path(&parents, goal),
                current.g,
                false,
            ));
        }

        update_best_partial(&mut best_partial, current.coord, current.h, current.g);
        for (neighbor, step_cost) in snapshot.grid.neighbor_cells(
            current.coord,
            snapshot.config.neighborhood,
            snapshot.config.allow_corner_cutting,
            profile,
            overlays,
        ) {
            let tentative = current.g + step_cost;
            if tentative + 0.0001 < g_score.get(&neighbor).copied().unwrap_or(f32::INFINITY) {
                parents.insert(neighbor, current.coord);
                g_score.insert(neighbor, tentative);
                open.push(GridNodeState::new(
                    neighbor,
                    tentative,
                    heuristic(neighbor, goal),
                ));
            }
        }
    }

    if allow_partial {
        let (coord, _, cost) = best_partial?;
        Some(finalize_corridor(
            snapshot,
            profile,
            overlays,
            reconstruct_grid_path(&parents, coord),
            cost,
            true,
        ))
    } else {
        None
    }
}

fn hierarchical_search(
    snapshot: &PathfindingSnapshot,
    start: GridCoord,
    goal: GridCoord,
    profile: &PathFilterProfile,
    allow_partial: bool,
    overlays: &[PathCostOverlay],
) -> Option<ResolvedPath> {
    let level = snapshot.level(1)?;
    let start_cluster = snapshot.cluster_key_for_coord(1, start);
    let goal_cluster = snapshot.cluster_key_for_coord(1, goal);
    let start_info = level.clusters.get(&start_cluster)?;
    let goal_info = level.clusters.get(&goal_cluster)?;

    let start_links = connect_endpoint(
        snapshot,
        start,
        start_info.node_ids.as_slice(),
        start_info.bounds,
        profile,
        overlays,
    );
    let goal_links = connect_endpoint(
        snapshot,
        goal,
        goal_info.node_ids.as_slice(),
        goal_info.bounds,
        profile,
        overlays,
    );
    if start_links.is_empty() || goal_links.is_empty() {
        return None;
    }

    let temp_start = snapshot.nodes.len();
    let temp_goal = temp_start + 1;
    let mut goal_edge_map = HashMap::<usize, NodeEdge>::new();
    for edge in goal_links {
        goal_edge_map.insert(
            edge.to,
            NodeEdge {
                to: temp_goal,
                cost: edge.cost,
                kind: edge.kind,
                route: reverse_route(&edge.route),
            },
        );
    }

    let mut open = BinaryHeap::new();
    let mut parents = HashMap::<usize, usize>::new();
    let mut g_score = HashMap::<usize, f32>::new();
    let mut best_partial = Some((temp_start, heuristic(start, goal), 0.0));

    open.push(AbstractNodeState::new(
        temp_start,
        0.0,
        heuristic(start, goal),
    ));
    g_score.insert(temp_start, 0.0);

    while let Some(current) = open.pop() {
        if current.id == temp_goal {
            let route = reconstruct_node_path(&parents, temp_goal);
            let corridor = flatten_node_route(
                snapshot,
                temp_start,
                temp_goal,
                &route,
                &start_links,
                &goal_edge_map,
            )?;
            return Some(finalize_corridor(
                snapshot, profile, overlays, corridor, current.g, false,
            ));
        }

        let current_coord = if current.id == temp_start {
            start
        } else if current.id == temp_goal {
            goal
        } else {
            snapshot.nodes[current.id].coord
        };
        update_best_partial(
            &mut best_partial,
            current.id,
            heuristic(current_coord, goal),
            current.g,
        );

        for edge in abstract_neighbors(snapshot, current.id, &start_links, &goal_edge_map) {
            let tentative = current.g + edge.cost;
            if tentative + 0.0001 < g_score.get(&edge.to).copied().unwrap_or(f32::INFINITY) {
                parents.insert(edge.to, current.id);
                g_score.insert(edge.to, tentative);
                let neighbor_coord = if edge.to == temp_goal {
                    goal
                } else {
                    snapshot.nodes[edge.to].coord
                };
                open.push(AbstractNodeState::new(
                    edge.to,
                    tentative,
                    heuristic(neighbor_coord, goal),
                ));
            }
        }
    }

    if allow_partial {
        let (best_id, _, cost) = best_partial?;
        if best_id == temp_start {
            return None;
        }
        let route = reconstruct_node_path(&parents, best_id);
        let corridor = flatten_node_route(
            snapshot,
            temp_start,
            best_id,
            &route,
            &start_links,
            &HashMap::new(),
        )?;
        Some(finalize_corridor(
            snapshot, profile, overlays, corridor, cost, true,
        ))
    } else {
        None
    }
}

fn connect_endpoint(
    snapshot: &PathfindingSnapshot,
    endpoint: GridCoord,
    node_ids: &[usize],
    bounds: crate::coord::GridAabb,
    profile: &PathFilterProfile,
    overlays: &[PathCostOverlay],
) -> Vec<NodeEdge> {
    let mut output = Vec::new();
    for node_id in node_ids.iter().copied() {
        if let Some((cells, cost)) = bounded_direct_search(
            snapshot,
            endpoint,
            snapshot.nodes[node_id].coord,
            bounds,
            profile,
            overlays,
        ) {
            output.push(NodeEdge {
                to: node_id,
                cost,
                kind: crate::hierarchy::EdgeKind::Projection,
                route: EdgeRoute::Cells(cells),
            });
        }
    }
    output.sort_by_key(|edge| edge.to);
    output
}

fn bounded_direct_search(
    snapshot: &PathfindingSnapshot,
    start: GridCoord,
    goal: GridCoord,
    bounds: crate::coord::GridAabb,
    profile: &PathFilterProfile,
    overlays: &[PathCostOverlay],
) -> Option<(Vec<GridCoord>, f32)> {
    let mut open = BinaryHeap::new();
    let mut parents = HashMap::<GridCoord, GridCoord>::new();
    let mut g_score = HashMap::<GridCoord, f32>::new();

    open.push(GridNodeState::new(start, 0.0, heuristic(start, goal)));
    g_score.insert(start, 0.0);

    while let Some(current) = open.pop() {
        if current.coord == goal {
            return Some((reconstruct_grid_path(&parents, goal), current.g));
        }

        for (neighbor, step_cost) in snapshot.grid.neighbor_cells(
            current.coord,
            snapshot.config.neighborhood,
            snapshot.config.allow_corner_cutting,
            profile,
            overlays,
        ) {
            if !bounds.contains(neighbor) {
                continue;
            }
            let tentative = current.g + step_cost;
            if tentative + 0.0001 < g_score.get(&neighbor).copied().unwrap_or(f32::INFINITY) {
                parents.insert(neighbor, current.coord);
                g_score.insert(neighbor, tentative);
                open.push(GridNodeState::new(
                    neighbor,
                    tentative,
                    heuristic(neighbor, goal),
                ));
            }
        }
    }

    None
}

fn flatten_node_route(
    snapshot: &PathfindingSnapshot,
    temp_start: usize,
    _temp_goal: usize,
    route: &[usize],
    start_links: &[NodeEdge],
    goal_links: &HashMap<usize, NodeEdge>,
) -> Option<Vec<GridCoord>> {
    if route.is_empty() {
        return None;
    }

    let mut corridor = Vec::new();
    for pair in route.windows(2) {
        let from = pair[0];
        let to = pair[1];
        let segment = if from == temp_start {
            start_links.iter().find(|edge| edge.to == to)?.route.clone()
        } else if let Some(edge) = goal_links.get(&from).filter(|edge| edge.to == to) {
            edge.route.clone()
        } else {
            find_edge(snapshot, from, to)?.route.clone()
        };
        append_route(snapshot, &mut corridor, &segment)?;
    }
    Some(corridor)
}

fn append_route(
    snapshot: &PathfindingSnapshot,
    corridor: &mut Vec<GridCoord>,
    route: &EdgeRoute,
) -> Option<()> {
    match route {
        EdgeRoute::Cells(cells) => append_cells(corridor, cells),
        EdgeRoute::Projection => {}
        EdgeRoute::Nodes(nodes) => {
            if nodes.len() == 1 {
                append_cells(corridor, &[snapshot.nodes[nodes[0]].coord]);
            } else {
                for pair in nodes.windows(2) {
                    let edge = find_edge(snapshot, pair[0], pair[1])?;
                    append_route(snapshot, corridor, &edge.route)?;
                }
            }
        }
    }
    Some(())
}

fn append_cells(corridor: &mut Vec<GridCoord>, cells: &[GridCoord]) {
    for cell in cells.iter().copied() {
        if corridor.last().copied() != Some(cell) {
            corridor.push(cell);
        }
    }
}

fn find_edge(snapshot: &PathfindingSnapshot, from: usize, to: usize) -> Option<&NodeEdge> {
    let mut edges = snapshot.edges.get(from)?.iter().collect::<Vec<_>>();
    edges.sort_by_key(|edge| edge.to);
    edges.into_iter().find(|edge| edge.to == to)
}

fn abstract_neighbors<'a>(
    snapshot: &'a PathfindingSnapshot,
    id: usize,
    start_links: &'a [NodeEdge],
    goal_links: &'a HashMap<usize, NodeEdge>,
) -> Vec<&'a NodeEdge> {
    if id == snapshot.nodes.len() {
        return start_links.iter().collect();
    }

    let mut output = snapshot.edges[id].iter().collect::<Vec<_>>();
    if let Some(goal_edge) = goal_links.get(&id) {
        output.push(goal_edge);
    }
    output.sort_by_key(|edge| edge.to);
    output
}

fn reverse_route(route: &EdgeRoute) -> EdgeRoute {
    match route {
        EdgeRoute::Cells(cells) => {
            let mut reversed = cells.clone();
            reversed.reverse();
            EdgeRoute::Cells(reversed)
        }
        EdgeRoute::Nodes(nodes) => {
            let mut reversed = nodes.clone();
            reversed.reverse();
            EdgeRoute::Nodes(reversed)
        }
        EdgeRoute::Projection => EdgeRoute::Projection,
    }
}

fn finalize_corridor(
    snapshot: &PathfindingSnapshot,
    profile: &PathFilterProfile,
    overlays: &[PathCostOverlay],
    corridor: Vec<GridCoord>,
    total_cost: f32,
    is_partial: bool,
) -> ResolvedPath {
    let touched_clusters = snapshot.cluster_versions_for_corridor(&corridor);
    let waypoints = smooth_corridor(
        &snapshot.grid,
        &corridor,
        profile,
        overlays,
        snapshot.config.smoothing_mode,
    );
    ResolvedPath {
        corridor,
        waypoints,
        total_cost,
        is_partial,
        version: PathVersion(snapshot.version),
        touched_clusters,
    }
}

fn reconstruct_grid_path(
    parents: &HashMap<GridCoord, GridCoord>,
    goal: GridCoord,
) -> Vec<GridCoord> {
    let mut path = vec![goal];
    let mut cursor = goal;
    while let Some(parent) = parents.get(&cursor).copied() {
        path.push(parent);
        cursor = parent;
    }
    path.reverse();
    path
}

fn reconstruct_node_path(parents: &HashMap<usize, usize>, goal: usize) -> Vec<usize> {
    let mut path = vec![goal];
    let mut cursor = goal;
    while let Some(parent) = parents.get(&cursor).copied() {
        path.push(parent);
        cursor = parent;
    }
    path.reverse();
    path
}

fn update_best_partial<T: Copy>(
    best: &mut Option<(T, f32, f32)>,
    node: T,
    heuristic_value: f32,
    g: f32,
) {
    match best {
        Some((_, best_h, best_g)) if heuristic_value > *best_h + 0.0001 => {}
        Some((_, best_h, best_g))
            if (heuristic_value - *best_h).abs() <= 0.0001 && g >= *best_g => {}
        _ => *best = Some((node, heuristic_value, g)),
    }
}

fn heuristic(a: GridCoord, b: GridCoord) -> f32 {
    let delta = (a.0 - b.0).abs();
    delta.max_element() as f32
}

#[derive(Debug, Clone, Copy)]
struct GridNodeState {
    coord: GridCoord,
    g: f32,
    h: f32,
}

impl GridNodeState {
    fn new(coord: GridCoord, g: f32, h: f32) -> Self {
        Self { coord, g, h }
    }
}

impl PartialEq for GridNodeState {
    fn eq(&self, other: &Self) -> bool {
        self.coord == other.coord
    }
}

impl Eq for GridNodeState {}

impl PartialOrd for GridNodeState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for GridNodeState {
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
struct AbstractNodeState {
    id: usize,
    g: f32,
    h: f32,
}

impl AbstractNodeState {
    fn new(id: usize, g: f32, h: f32) -> Self {
        Self { id, g, h }
    }
}

impl PartialEq for AbstractNodeState {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for AbstractNodeState {}

impl PartialOrd for AbstractNodeState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AbstractNodeState {
    fn cmp(&self, other: &Self) -> Ordering {
        let self_f = self.g + self.h;
        let other_f = other.g + other.h;
        other_f
            .total_cmp(&self_f)
            .then_with(|| other.id.cmp(&self.id))
    }
}

#[cfg(test)]
#[path = "search_tests.rs"]
mod tests;
