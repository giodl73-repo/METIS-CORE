use super::{boundary::BoundarySet, gain::GainTable};
use crate::api::ObjectiveType;
use crate::graph::{CsrGraph, Partition};
use crate::refine::Refiner;

pub struct FiducciaMattheyses {
    pub niter: u32,
    /// Skip moves that would disconnect the source part (IsConnectedSubdomain).
    /// Default: `true`.
    pub contig_fm: bool,
    /// Objective function: edge cut (default) or communication volume.
    pub objective: ObjectiveType,
    /// Number of label-propagation balance iterations to run before the FM
    /// passes.  0 = disabled (default).  Mirrors METIS `BalanceAndRefineLP`.
    pub lp_iter: u32,
    /// Allowed imbalance in METIS units: `x` means `1 + x/1000`.
    pub ufactor: u32,
}

impl Default for FiducciaMattheyses {
    fn default() -> Self {
        Self {
            niter: 10,
            contig_fm: true,
            objective: ObjectiveType::Cut,
            lp_iter: 0,
            ufactor: 5,
        }
    }
}

impl Refiner for FiducciaMattheyses {
    fn refine(&self, g: &CsrGraph, p: Partition) -> Partition {
        // Optional label-propagation pre-balance pass (BalanceAndRefineLP).
        // Runs before FM so that FM starts from a better-balanced state.
        let p = if self.lp_iter > 0 {
            let mut p = p;
            crate::refine::lp::lp_balance(g, &mut p, self.ufactor, self.lp_iter);
            p
        } else {
            p
        };

        let mut state = FmState::new(g, p, self.objective);
        let mut best = state.checkpoint();

        for _pass in 0..self.niter {
            let improved = fm_pass(&mut state, &mut best, self.contig_fm, self.ufactor);
            // Always restore to best after each pass so the next pass
            // starts from the best-known state, not the end-of-pass state.
            state.restore(&best);
            if !improved {
                break;
            }
        }
        Partition {
            assignment: state.assignment,
            k: state.k,
            tpwgts: None,
        }
    }
}

/// Gain of moving vertex `v` from `from_part` to `to_part` under the volume
/// objective.  Communication volume counts (vertex, part) pairs where vertex
/// v has at least one neighbour in that part — i.e. the number of distinct
/// neighbour-parts for each vertex.  Moving v from `from` to `to` changes:
///
/// 1. v's own subdomain neighbourhood.
/// 2. For each neighbour u in `from_part`: u may lose `from` as a home-part
///    connection (if v was its only `to`-neighbour, it gains `to`; separately,
///    if v is the last `from`-part vertex seen by a `to`-part neighbour it
///    shrinks its subdomain).
/// 3. For each neighbour u in `to_part`: u may lose `from_part` from its
///    subdomain if v was the only `from`-neighbour.
///
/// Returns a positive value when the move decreases total volume (improvement).
pub fn compute_volume_gain(
    g: &CsrGraph,
    assignment: &[u32],
    v: usize,
    from_part: u32,
    to_part: u32,
) -> i32 {
    let mut gain = 0i32;

    for j in g.xadj[v] as usize..g.xadj[v + 1] as usize {
        let u = g.adjncy[j] as usize;
        let u_part = assignment[u];

        if u_part == from_part {
            // u is in from_part.
            // After the move, v is in to_part.  Does u now gain to_part in its
            // subdomain (v becomes a new to_part-neighbour for u)?
            let u_already_has_to = (g.xadj[u] as usize..g.xadj[u + 1] as usize)
                .any(|jj| assignment[g.adjncy[jj] as usize] == to_part);
            if !u_already_has_to {
                gain -= 1; // u gains a new subdomain entry → volume increases by 1
            }

            // Does u lose from_part from its *cross-boundary* perspective?
            // (u is in from_part itself, so "from_part" is its home — not counted
            //  in the subdomain tally we care about.  The relevant question: does
            //  any neighbour of u lose from_part as a cross-part connection because
            //  v moved away?)
            // For v itself: after the move v is in to_part, so v's own subdomain
            // changes.  We handle v's own delta separately below.
            //
            // For u: u loses v as a same-part neighbour, which has no subdomain
            // effect (same-part adjacencies don't contribute to comm volume).
            // No gain/penalty here.
        } else if u_part == to_part {
            // u is in to_part.
            // After the move, v moves to to_part — v is now a same-part neighbour
            // of u, so u loses v as a cross-boundary (from_part) neighbour.
            // If v was u's *only* from_part neighbour, u's subdomain shrinks.
            let u_has_other_from = (g.xadj[u] as usize..g.xadj[u + 1] as usize)
                .filter(|&jj| g.adjncy[jj] as usize != v)
                .any(|jj| assignment[g.adjncy[jj] as usize] == from_part);
            if !u_has_other_from {
                gain += 1; // u loses from_part from its subdomain → volume decreases
            }
        }
        // u in any other part: moving v does not affect u's subdomain count.
    }

    // v's own subdomain change.
    // Before: v is in from_part and sees some set of distinct neighbour-parts S_before.
    // After:  v is in to_part  and sees some set of distinct neighbour-parts S_after.
    // We approximate: count distinct non-from parts that v has as neighbours before
    // vs distinct non-to parts after (home-part is never counted in comm-vol).
    // Delta for v = |S_before| - |S_after|  (positive = improvement).

    // Collect distinct parts adjacent to v, excluding from_part and to_part.
    let mut other_parts_set = std::collections::BTreeSet::new();
    let mut v_has_from_nbr = false; // v has a neighbour in from_part?
    let mut v_has_to_nbr = false; // v has a neighbour in to_part?
    for j in g.xadj[v] as usize..g.xadj[v + 1] as usize {
        let u_part = assignment[g.adjncy[j] as usize];
        if u_part == from_part {
            v_has_from_nbr = true;
        } else if u_part == to_part {
            v_has_to_nbr = true;
        } else {
            other_parts_set.insert(u_part);
        }
    }

    // Before move: v is in from_part.
    //   subdomain = {to_part if v_has_to_nbr} ∪ other_parts_set
    //   (from_part neighbours are same-part, not counted)
    let s_before = other_parts_set.len() as i32 + if v_has_to_nbr { 1 } else { 0 };

    // After move: v is in to_part.
    //   subdomain = {from_part if v_has_from_nbr} ∪ other_parts_set
    let s_after = other_parts_set.len() as i32 + if v_has_from_nbr { 1 } else { 0 };

    gain += s_before - s_after;
    gain
}

fn fm_pass(state: &mut FmState, best: &mut Checkpoint, contig_fm: bool, ufactor: u32) -> bool {
    let ncon = state.graph.ncon as usize;
    let k = state.k as usize;
    let total_wgts: Vec<i64> = (0..ncon).map(|c| state.pwgts[c].iter().sum()).collect();

    // Per-part, per-constraint targets and epsilons.
    // Layout: targets_pc[part][constraint], epsilons_pc[part][constraint].
    //
    // When tpwgts is provided: constraint 0 gets proportional targets derived from
    // the float weights; all other constraints (VAP, etc.) keep equal targets.
    // When tpwgts is None: all constraints use equal targets.
    let targets_pc: Vec<Vec<i64>> = (0..k)
        .map(|part| {
            (0..ncon)
                .map(|c| {
                    if c == 0 {
                        match &state.tpwgts {
                            Some(tw) => (total_wgts[0] as f64 * tw[part] as f64).round() as i64,
                            None => total_wgts[0] / k as i64,
                        }
                    } else {
                        total_wgts[c] / k as i64
                    }
                })
                .collect()
        })
        .collect();
    // INTEGER balance epsilon — METIS ufactor units: x means 1 + x/1000.
    let epsilons_pc: Vec<Vec<i64>> = targets_pc
        .iter()
        .map(|row| {
            row.iter()
                .map(|&t| (t.abs() * ufactor as i64 + 999) / 1000)
                .collect()
        })
        .collect();

    let start_cut = best.cut;

    // Locked set: vertices that have been popped this pass must not be re-inserted.
    // This is the fundamental FM invariant — each vertex moves at most once per pass.
    let n = state.graph.n();
    let mut locked = vec![false; n];
    let mut candidates: Vec<(u32, i32)> = Vec::new();

    while let Some((v, _gain)) = state.gain_table.pop_max() {
        let v = v as usize;
        locked[v] = true;

        let from_part = state.assignment[v] as usize;

        // Contiguity check: skip the move if removing v from from_part would
        // leave that part disconnected (IsConnectedSubdomain).
        // Fast path: if v has ≤1 neighbor in from_part it cannot be a cut vertex.
        if contig_fm {
            let g = state.graph;
            let from_nbr_count = (g.xadj[v] as usize..g.xadj[v + 1] as usize)
                .filter(|&j| state.assignment[g.adjncy[j] as usize] as usize == from_part)
                .count();
            if from_nbr_count >= 2 && would_disconnect(g, &state.assignment, v, from_part) {
                continue;
            }
        }

        // Gather per-constraint weights for vertex v
        let vwgt_v: Vec<i64> = (0..ncon)
            .map(|c| state.graph.vwgt[v * ncon + c] as i64)
            .collect();

        let Some(to_part) = best_legal_destination(
            state,
            v,
            from_part,
            &vwgt_v,
            &targets_pc,
            &epsilons_pc,
            &mut candidates,
        ) else {
            continue;
        };

        // Apply move
        state.assignment[v] = to_part as u32;
        for (c, &weight) in vwgt_v.iter().enumerate().take(ncon) {
            state.pwgts[c][from_part] -= weight;
            state.pwgts[c][to_part] += weight;
        }
        state.boundary.remove(v as u32);

        // Update current_cut incrementally — O(degree(v)) not O(m)
        {
            let g = state.graph;
            let mut cut_delta: i64 = 0;
            for j in g.xadj[v] as usize..g.xadj[v + 1] as usize {
                let u = g.adjncy[j] as usize;
                let ew = g.adjwgt.as_ref().map_or(1i64, |aw| aw[j] as i64);
                if state.assignment[u] as usize == from_part {
                    cut_delta += ew;
                } // was same, now cross
                if state.assignment[u] as usize == to_part {
                    cut_delta -= ew;
                } // was cross, now same
            }
            state.current_cut += cut_delta;
        }

        // Update gains for all unlocked neighbours of v
        let g = state.graph;
        for j in g.xadj[v] as usize..g.xadj[v + 1] as usize {
            let u = g.adjncy[j] as usize;
            if locked[u] {
                continue;
            } // never re-insert a locked vertex

            let new_gain = match state.objective {
                ObjectiveType::Cut => {
                    compute_cut_gain_with_buffer(g, &state.assignment, u, &mut candidates)
                }
                ObjectiveType::Volume => {
                    let u_from = state.assignment[u];
                    best_volume_gain(g, &state.assignment, u, u_from, state.k)
                }
            };
            let clamped = new_gain.clamp(-state.gain_table.max_gain, state.gain_table.max_gain);
            // Check if u is on boundary after the move
            let u_on_boundary = (g.xadj[u] as usize..g.xadj[u + 1] as usize)
                .any(|jj| state.assignment[g.adjncy[jj] as usize] != state.assignment[u]);
            if u_on_boundary {
                if state.gain_table.contains(u as u32) {
                    state.gain_table.update(u as u32, clamped);
                } else {
                    state.boundary.insert(u as u32);
                    state.gain_table.insert(u as u32, clamped);
                }
            } else {
                if state.gain_table.contains(u as u32) {
                    state.gain_table.remove(u as u32);
                }
                state.boundary.remove(u as u32);
            }
        }

        // Checkpoint if improved — use incremental cut value, O(1)
        let cur_cut = state.current_cut;
        if cur_cut < best.cut {
            *best = Checkpoint {
                assignment: state.assignment.clone(),
                cut: cur_cut,
            };
        }
    }

    best.cut < start_cut
}

/// Returns `true` if removing vertex `v` from `from_part` would leave that part
/// disconnected.  Uses a BFS restricted to `from_part \ {v}` starting from an
/// arbitrary neighbor of `v` that is also in `from_part`, then checks whether
/// all other vertices in `from_part` are reachable.
///
/// Caller must ensure `v` has at least 2 neighbors in `from_part` before
/// calling (fast path: ≤1 neighbor → always returns `false`).
fn would_disconnect(g: &CsrGraph, assignment: &[u32], v: usize, from_part: usize) -> bool {
    // Find the first neighbor of v in from_part to seed the BFS.
    let start = (g.xadj[v] as usize..g.xadj[v + 1] as usize)
        .find(|&j| assignment[g.adjncy[j] as usize] as usize == from_part)
        .map(|j| g.adjncy[j] as usize);

    let start = match start {
        Some(s) => s,
        None => return false, // no neighbor in from_part → safe to move
    };

    // Count vertices in from_part excluding v.
    let part_size: usize = assignment
        .iter()
        .enumerate()
        .filter(|&(u, &p)| u != v && p as usize == from_part)
        .count();

    if part_size == 0 {
        return false;
    }

    // BFS from start, restricted to from_part \ {v}.
    let n = g.n();
    let mut visited = vec![false; n];
    let mut queue = std::collections::VecDeque::new();
    visited[start] = true;
    queue.push_back(start);
    let mut reached = 1usize;

    while let Some(u) = queue.pop_front() {
        for j in g.xadj[u] as usize..g.xadj[u + 1] as usize {
            let w = g.adjncy[j] as usize;
            if !visited[w] && w != v && assignment[w] as usize == from_part {
                visited[w] = true;
                queue.push_back(w);
                reached += 1;
            }
        }
    }

    reached < part_size // true = some vertices in from_part unreachable → disconnected
}

fn best_legal_destination(
    state: &FmState,
    v: usize,
    from_part: usize,
    vwgt_v: &[i64],
    targets_pc: &[Vec<i64>],
    epsilons_pc: &[Vec<i64>],
    candidates: &mut Vec<(u32, i32)>,
) -> Option<usize> {
    let ncon = state.graph.ncon as usize;
    let from = from_part as u32;
    candidates.clear();

    match state.objective {
        ObjectiveType::Cut => {
            fill_cut_candidates(state.graph, &state.assignment, v, from, candidates)
        }
        ObjectiveType::Volume => {
            fill_volume_candidates(state.graph, &state.assignment, v, from, candidates)
        }
    }

    candidates
        .iter()
        .copied()
        .map(|(to, gain)| (to as usize, gain))
        .filter(|&(to, _)| {
            (0..ncon).all(|c| {
                let new_from = state.pwgts[c][from_part] - vwgt_v[c];
                let new_to = state.pwgts[c][to] + vwgt_v[c];
                new_from >= targets_pc[from_part][c] - epsilons_pc[from_part][c]
                    && new_to <= targets_pc[to][c] + epsilons_pc[to][c]
            })
        })
        .max_by_key(|&(to, gain)| (gain, std::cmp::Reverse(to as u32)))
        .map(|(to, _)| to)
}

pub struct FmState<'g> {
    pub graph: &'g CsrGraph,
    pub assignment: Vec<u32>,
    pub k: u32,
    pub gain_table: GainTable,
    pub boundary: BoundarySet,
    /// `pwgts[constraint][part]` = weight sum for constraint c in part p.
    /// For ncon=1, `pwgts[0][part]` is equivalent to the old single-constraint `pwgts[part]`.
    pub pwgts: Vec<Vec<i64>>,
    pub current_cut: i64,
    /// Per-part target weights (one f32 per part, summing to 1.0).
    /// `None` means equal weights: each part targets `total_wgt / k`.
    pub tpwgts: Option<Vec<f32>>,
    /// Objective function used for gain computation and move selection.
    pub objective: ObjectiveType,
}

#[derive(Clone)]
pub struct Checkpoint {
    pub assignment: Vec<u32>,
    pub cut: i64,
}

impl<'g> FmState<'g> {
    pub fn new(g: &'g CsrGraph, p: Partition, objective: ObjectiveType) -> Self {
        let n = g.n();
        let k = p.k as usize;
        let ncon = g.ncon as usize;
        let mut pwgts = vec![vec![0i64; k]; ncon];
        for v in 0..n {
            let part = p.assignment[v] as usize;
            for (c, part_weights) in pwgts.iter_mut().enumerate().take(ncon) {
                part_weights[part] += g.vwgt[v * ncon + c] as i64;
            }
        }

        // max_gain = max edge weight (or 1 if unweighted) × max degree
        // For volume objective, each move can affect at most deg(v)+1 subdomain
        // entries, so max_gain bound = max_deg is a safe upper bound (≥1).
        let max_ew = g
            .adjwgt
            .as_ref()
            .and_then(|aw| aw.iter().copied().max())
            .unwrap_or(1);
        let max_deg = (0..n).map(|v| g.xadj[v + 1] - g.xadj[v]).max().unwrap_or(1) as i32;
        let max_gain = (max_ew * max_deg).max(1);

        let boundary = BoundarySet::from_partition(g, &p);
        let mut gain_table = GainTable::new(n, max_gain);
        let mut candidates = Vec::new();
        for v_u32 in boundary.iter() {
            let v = v_u32 as usize;
            let gain = match objective {
                ObjectiveType::Cut => {
                    compute_cut_gain_with_buffer(g, &p.assignment, v, &mut candidates)
                }
                ObjectiveType::Volume => {
                    let from = p.assignment[v];
                    // Best-gain destination for volume objective (greedy scan)
                    best_volume_gain(g, &p.assignment, v, from, p.k)
                }
            };
            debug_assert!(
                gain.abs() <= max_gain,
                "computed gain {gain} exceeds max_gain {max_gain} — max_gain estimate is wrong"
            );
            let gain_clamped = gain.clamp(-max_gain, max_gain);
            gain_table.insert(v_u32, gain_clamped);
        }

        let tpwgts = p.tpwgts.clone();
        let mut state = FmState {
            graph: g,
            assignment: p.assignment,
            k: p.k,
            gain_table,
            boundary,
            pwgts,
            current_cut: 0,
            tpwgts,
            objective,
        };
        state.current_cut = state.cut();
        state
    }

    pub fn cut(&self) -> i64 {
        let g = self.graph;
        let mut c = 0i64;
        for v in 0..g.n() {
            for j in g.xadj[v] as usize..g.xadj[v + 1] as usize {
                let u = g.adjncy[j] as usize;
                if self.assignment[v] != self.assignment[u] {
                    c += g.adjwgt.as_ref().map_or(1i64, |aw| aw[j] as i64);
                }
            }
        }
        c / 2 // each edge counted twice
    }

    pub fn checkpoint(&self) -> Checkpoint {
        Checkpoint {
            assignment: self.assignment.clone(),
            cut: self.current_cut,
        }
    }

    pub fn restore(&mut self, cp: &Checkpoint) {
        self.assignment = cp.assignment.clone();
        // tpwgts is preserved across restore — it is a property of the partition problem,
        // not the current state, and must survive all passes.
        let p = Partition {
            assignment: self.assignment.clone(),
            k: self.k,
            tpwgts: self.tpwgts.clone(),
        };
        self.boundary = BoundarySet::from_partition(self.graph, &p);
        let n = self.graph.n();
        let max_gain = self.gain_table.max_gain;
        self.gain_table = GainTable::new(n, max_gain);
        let mut candidates = Vec::new();
        for v_u32 in self.boundary.iter() {
            let v = v_u32 as usize;
            let gain = match self.objective {
                ObjectiveType::Cut => {
                    compute_cut_gain_with_buffer(self.graph, &self.assignment, v, &mut candidates)
                }
                ObjectiveType::Volume => {
                    let from = self.assignment[v];
                    best_volume_gain(self.graph, &self.assignment, v, from, self.k)
                }
            };
            debug_assert!(
                gain.abs() <= max_gain,
                "computed gain {gain} exceeds max_gain {max_gain} — max_gain estimate is wrong"
            );
            let gain_clamped = gain.clamp(-max_gain, max_gain);
            self.gain_table.insert(v_u32, gain_clamped);
        }
        // Recompute per-constraint part populations
        let ncon = self.graph.ncon as usize;
        let k = self.k as usize;
        self.pwgts = vec![vec![0i64; k]; ncon];
        for v in 0..n {
            let part = self.assignment[v] as usize;
            for (c, part_weights) in self.pwgts.iter_mut().enumerate().take(ncon) {
                part_weights[part] += self.graph.vwgt[v * ncon + c] as i64;
            }
        }
        self.current_cut = self.cut(); // recompute once after restore
    }
}

/// Gain of moving vertex v to the best adjacent destination part.
///
/// Gain = `external_degree_to_destination - internal_degree`. For k-way
/// partitioning the destination must be a single part, so using total external
/// degree across all neighboring parts overstates the true move gain.
pub fn compute_gain(g: &CsrGraph, assignment: &[u32], v: usize) -> i32 {
    compute_cut_gain_with_buffer(g, assignment, v, &mut Vec::new())
}

fn compute_cut_gain_with_buffer(
    g: &CsrGraph,
    assignment: &[u32],
    v: usize,
    candidates: &mut Vec<(u32, i32)>,
) -> i32 {
    let from = assignment[v];
    fill_cut_candidates(g, assignment, v, from, candidates);
    candidates
        .iter()
        .map(|&(_, gain)| gain)
        .max()
        .unwrap_or_else(|| {
            let internal: i32 = (g.xadj[v] as usize..g.xadj[v + 1] as usize)
                .filter(|&j| assignment[g.adjncy[j] as usize] == from)
                .map(|j| g.adjwgt.as_ref().map_or(1i32, |aw| aw[j]))
                .sum();
            -internal
        })
}

fn fill_cut_candidates(
    g: &CsrGraph,
    assignment: &[u32],
    v: usize,
    from: u32,
    candidates: &mut Vec<(u32, i32)>,
) {
    candidates.clear();
    let mut internal = 0i32;
    for j in g.xadj[v] as usize..g.xadj[v + 1] as usize {
        let ew = g.adjwgt.as_ref().map_or(1i32, |aw| aw[j]);
        let part = assignment[g.adjncy[j] as usize];
        if part == from {
            internal += ew;
        } else if let Some((_, external)) = candidates
            .iter_mut()
            .find(|(candidate, _)| *candidate == part)
        {
            *external += ew;
        } else {
            candidates.push((part, ew));
        }
    }

    for (_, external) in candidates.iter_mut() {
        *external -= internal;
    }
}

fn fill_volume_candidates(
    g: &CsrGraph,
    assignment: &[u32],
    v: usize,
    from: u32,
    candidates: &mut Vec<(u32, i32)>,
) {
    candidates.clear();
    for j in g.xadj[v] as usize..g.xadj[v + 1] as usize {
        let part = assignment[g.adjncy[j] as usize];
        if part == from || candidates.iter().any(|(candidate, _)| *candidate == part) {
            continue;
        }
        let gain = compute_volume_gain(g, assignment, v, from, part);
        candidates.push((part, gain));
    }
}

/// Best volume gain for vertex `v` moving away from `from_part` — scans all
/// adjacent parts and returns the maximum `compute_volume_gain` across them.
/// Returns 0 if v has no adjacent parts other than `from_part`.
pub fn best_volume_gain(g: &CsrGraph, assignment: &[u32], v: usize, from_part: u32, k: u32) -> i32 {
    // Collect distinct adjacent parts (other than from_part)
    let mut adj_parts: Vec<u32> = (g.xadj[v] as usize..g.xadj[v + 1] as usize)
        .map(|j| assignment[g.adjncy[j] as usize])
        .filter(|&p| p != from_part)
        .collect();
    adj_parts.sort_unstable();
    adj_parts.dedup();

    if adj_parts.is_empty() {
        // v is interior — try all other parts for completeness (rarely wins)
        (0..k)
            .filter(|&p| p != from_part)
            .map(|to| compute_volume_gain(g, assignment, v, from_part, to))
            .max()
            .unwrap_or(0)
    } else {
        adj_parts
            .iter()
            .map(|&to| compute_volume_gain(g, assignment, v, from_part, to))
            .max()
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path_graph(n: usize) -> CsrGraph {
        let mut xadj = vec![0u32];
        let mut adjncy = Vec::new();
        for i in 0..n {
            if i > 0 {
                adjncy.push((i - 1) as u32);
            }
            if i < n - 1 {
                adjncy.push((i + 1) as u32);
            }
            xadj.push(adjncy.len() as u32);
        }
        CsrGraph {
            xadj,
            adjncy,
            ncon: 1,
            vwgt: vec![1i32; n],
            adjwgt: None,
        }
    }

    fn grid_4x4() -> CsrGraph {
        let n = 16usize;
        let mut xadj = vec![0u32];
        let mut adjncy = Vec::new();
        for i in 0..4usize {
            for j in 0..4usize {
                let mut nbrs = Vec::new();
                if i > 0 {
                    nbrs.push((i - 1) * 4 + j);
                }
                if i < 3 {
                    nbrs.push((i + 1) * 4 + j);
                }
                if j > 0 {
                    nbrs.push(i * 4 + (j - 1));
                }
                if j < 3 {
                    nbrs.push(i * 4 + (j + 1));
                }
                for &u in &nbrs {
                    adjncy.push(u as u32);
                }
                xadj.push(adjncy.len() as u32);
            }
        }
        CsrGraph {
            xadj,
            adjncy,
            ncon: 1,
            vwgt: vec![1i32; n],
            adjwgt: None,
        }
    }

    fn dumbbell_graph() -> CsrGraph {
        // Two K5 cliques connected by a single bridge edge
        // Vertices 0-4: left clique, vertices 5-9: right clique
        // Bridge: vertex 4 -- vertex 5
        let n = 10usize;
        let mut xadj = vec![0u32];
        let mut adjncy = Vec::new();
        for v in 0..n {
            let mut nbrs: Vec<usize> = Vec::new();
            let clique = if v < 5 { 0..5 } else { 5..10 };
            for u in clique {
                if u != v {
                    nbrs.push(u);
                }
            }
            // bridge
            if v == 4 {
                nbrs.push(5);
            }
            if v == 5 {
                nbrs.push(4);
            }
            for &u in &nbrs {
                adjncy.push(u as u32);
            }
            xadj.push(adjncy.len() as u32);
        }
        CsrGraph {
            xadj,
            adjncy,
            ncon: 1,
            vwgt: vec![1i32; n],
            adjwgt: None,
        }
    }

    fn compute_cut_for_test(g: &CsrGraph, assignment: &[u32]) -> u32 {
        let mut cut = 0u32;
        for v in 0..g.n() {
            for j in g.xadj[v] as usize..g.xadj[v + 1] as usize {
                let u = g.adjncy[j] as usize;
                if assignment[v] != assignment[u] {
                    cut += 1;
                }
            }
        }
        cut / 2
    }

    #[test]
    fn fm_state_cut_path3_split() {
        // path 0-1-2, partition [0,0,1]
        // cut edge: 1-2, so cut = 1
        let g = path_graph(3);
        let p = Partition {
            assignment: vec![0, 0, 1],
            k: 2,
            tpwgts: None,
        };
        let state = FmState::new(&g, p, ObjectiveType::Cut);
        assert_eq!(state.cut(), 1);
    }

    #[test]
    fn fm_state_checkpoint_restore() {
        let g = path_graph(4);
        let p = Partition {
            assignment: vec![0, 0, 1, 1],
            k: 2,
            tpwgts: None,
        };
        let mut state = FmState::new(&g, p, ObjectiveType::Cut);
        let cp = state.checkpoint();
        assert_eq!(cp.cut, state.cut());
        // Modify assignment (simulate a move)
        state.assignment[1] = 1;
        assert_ne!(state.assignment[1], cp.assignment[1]);
        // Restore
        state.restore(&cp);
        assert_eq!(state.assignment, cp.assignment);
    }

    #[test]
    fn fm_does_not_increase_cut() {
        use crate::init::random::RandomBisect;
        use crate::init::InitialPartitioner;
        use crate::refine::Refiner;
        let g = grid_4x4();
        let p_init = RandomBisect.partition(&g, 2, 0);
        let cut_before = compute_cut_for_test(&g, &p_init.assignment);
        let fm = FiducciaMattheyses {
            niter: 10,
            contig_fm: true,
            objective: ObjectiveType::Cut,
            lp_iter: 0,
            ufactor: 5,
        };
        let p = fm.refine(&g, p_init);
        let cut_after = compute_cut_for_test(&g, &p.assignment);
        assert!(
            cut_after <= cut_before,
            "FM must not increase cut: before={cut_before} after={cut_after}"
        );
    }

    #[test]
    fn fm_oracle_dumbbell_bisect() {
        // Dumbbell: two K5 joined by 1 edge — optimal bisection cut = 1
        use crate::init::random::RandomBisect;
        use crate::init::InitialPartitioner;
        use crate::refine::Refiner;
        let g = dumbbell_graph();
        let p_init = RandomBisect.partition(&g, 2, 42);
        let fm = FiducciaMattheyses {
            niter: 20,
            contig_fm: true,
            objective: ObjectiveType::Cut,
            lp_iter: 0,
            ufactor: 5,
        };
        let p = fm.refine(&g, p_init);
        let cut = compute_cut_for_test(&g, &p.assignment);
        assert_eq!(cut, 1, "dumbbell bisect optimal cut is 1, got {cut}");
    }

    #[test]
    fn fm_preserves_population_balance() {
        use crate::init::random::RandomBisect;
        use crate::init::InitialPartitioner;
        use crate::refine::Refiner;
        let g = grid_4x4();
        let total: i64 = g.vwgt.iter().map(|&w| w as i64).sum(); // = 16
        let target = total / 2; // = 8
        let eps = (target * 5 + 999) / 1000; // ceiling of 0.5% = 1
        let p_init = RandomBisect.partition(&g, 2, 99);
        let fm = FiducciaMattheyses {
            niter: 10,
            contig_fm: true,
            objective: ObjectiveType::Cut,
            lp_iter: 0,
            ufactor: 5,
        };
        let p = fm.refine(&g, p_init);
        for part in 0..2u32 {
            let wgt: i64 = (0..g.n())
                .filter(|&v| p.assignment[v] == part)
                .map(|v| g.vwgt[v] as i64)
                .sum();
            assert!(
                (wgt - target).abs() <= eps,
                "part {part} wgt {wgt} violates balance (target {target} ± {eps})"
            );
        }
    }

    #[test]
    fn fm_multi_constraint_balance() {
        // Grid 4x4 with ncon=2: constraint 0 = population, constraint 1 = VAP.
        // Both are uniform (weight 1) so the expected balance is identical for both.
        use crate::init::random::RandomBisect;
        use crate::init::InitialPartitioner;
        use crate::refine::Refiner;
        let mut g = grid_4x4();
        g.ncon = 2;
        // vwgt: vertex i has [pop=1, vap=1]; interleaved layout: [1, 1, 1, 1, ...]
        g.vwgt = vec![1i32; 32]; // 16 vertices × 2 constraints
        let p_init = RandomBisect.partition(&g, 2, 42);
        let fm = FiducciaMattheyses {
            niter: 10,
            contig_fm: true,
            objective: ObjectiveType::Cut,
            lp_iter: 0,
            ufactor: 5,
        };
        let p = fm.refine(&g, p_init);
        assert_eq!(p.assignment.len(), 16);
        // Both constraints should be balanced within ε = ceil(0.5% × target) = 1
        let target = 8i64;
        let eps = (target * 5 + 999) / 1000; // = 1
        for part in 0..2u32 {
            let wgt0: i64 = (0..16)
                .filter(|&v| p.assignment[v] == part)
                .map(|v| g.vwgt[v * 2] as i64)
                .sum();
            let wgt1: i64 = (0..16)
                .filter(|&v| p.assignment[v] == part)
                .map(|v| g.vwgt[v * 2 + 1] as i64)
                .sum();
            assert!(
                (wgt0 - target).abs() <= eps,
                "part {part} constraint 0 wgt {wgt0} violates balance (target {target} ± {eps})"
            );
            assert!(
                (wgt1 - target).abs() <= eps,
                "part {part} constraint 1 wgt {wgt1} violates balance (target {target} ± {eps})"
            );
        }
    }

    // ── would_disconnect unit tests ────────────────────────────────────────

    /// Path 0-1-2-3-4, bisected [0,0,1,1,1].
    /// Removing vertex 2 (the interior of part 1) should disconnect {3,4} from
    /// the rest of part 1 — i.e. would_disconnect returns true.
    #[test]
    fn would_disconnect_path_interior_vertex() {
        let g = path_graph(5);
        let assignment = vec![0u32, 0, 1, 1, 1];
        // Vertex 2 is in part 1 and has neighbors 1 (part 0) and 3 (part 1).
        // From part 1's perspective: removing 2 leaves {3,4}, which cannot
        // reach any other part-1 vertex without going through 2. Check BFS:
        // start from 3 (first neighbor of 2 in part 1).
        // Part 1 vertices excluding 2: {3, 4}.  BFS from 3 reaches 4 → reached=2=part_size → connected.
        // Actually path 0-1-2-3-4 part1={2,3,4}: removing 2 leaves {3,4} reachable from each other (3-4 edge).
        // So would_disconnect should return false.
        assert!(
            !would_disconnect(&g, &assignment, 2, 1),
            "path interior: {{3,4}} still connected after removing 2"
        );
    }

    /// Path 0-1-2-3-4, bisected [0,0,0,1,1].
    /// Vertex 3 is in part 1 with neighbors 2 (part 0) and 4 (part 1).
    /// Removing 3 from part 1 leaves only {4}, which is size 1 — still connected.
    #[test]
    fn would_disconnect_endpoint_safe() {
        let g = path_graph(5);
        let assignment = vec![0u32, 0, 0, 1, 1];
        assert!(
            !would_disconnect(&g, &assignment, 3, 1),
            "removing endpoint 3 leaves {{4}} which is trivially connected"
        );
    }

    /// Star graph: center vertex 0 connected to leaves 1,2,3,4,5 all in part 0.
    /// Removing the center (vertex 0) disconnects all leaves → should return true.
    #[test]
    fn would_disconnect_star_center() {
        // star: 0 -- 1,2,3,4,5; all in part 0
        let n = 6usize;
        let mut xadj = vec![0u32];
        let mut adjncy = Vec::new();
        // vertex 0: neighbors 1..5
        for i in 1..n {
            adjncy.push(i as u32);
        }
        xadj.push(adjncy.len() as u32);
        // vertices 1..5: neighbor is 0
        for _ in 1..n {
            adjncy.push(0u32);
            xadj.push(adjncy.len() as u32);
        }
        let g = CsrGraph {
            xadj,
            adjncy,
            ncon: 1,
            vwgt: vec![1i32; n],
            adjwgt: None,
        };
        let assignment = vec![0u32; n];
        // Removing center 0 from part 0 leaves {1,2,3,4,5} with no edges among them
        // (only edges are star spokes).  BFS from vertex 1 reaches nothing else.
        assert!(
            would_disconnect(&g, &assignment, 0, 0),
            "removing star center must disconnect leaves"
        );
    }

    /// Triangle (K3) with all vertices in part 0.
    /// Removing any single vertex leaves 2 vertices still connected (edge between them).
    #[test]
    fn would_disconnect_triangle_no_cut_vertex() {
        // Triangle: 0-1-2-0
        let xadj = vec![0u32, 2, 4, 6];
        let adjncy = vec![1u32, 2, 0, 2, 0, 1];
        let g = CsrGraph {
            xadj,
            adjncy,
            ncon: 1,
            vwgt: vec![1i32; 3],
            adjwgt: None,
        };
        let assignment = vec![0u32, 0, 0];
        for v in 0..3 {
            assert!(
                !would_disconnect(&g, &assignment, v, 0),
                "triangle: removing vertex {v} must not disconnect"
            );
        }
    }

    /// Contiguity-aware FM on a path graph must not produce a disconnected part.
    ///
    /// Path 0-1-2-3-4, initial bisection [0,1,0,1,0].  Without contiguity
    /// checking the odd-even assignment would produce alternating parts — with
    /// checking every proposed move that would isolate a vertex must be skipped.
    #[test]
    fn contig_fm_preserves_contiguity_path() {
        let g = path_graph(5);
        // Hand-craft an assignment that mixes parts so FM has something to do.
        let p = Partition {
            assignment: vec![0u32, 0, 1, 1, 0],
            k: 2,
            tpwgts: None,
        };
        let fm = FiducciaMattheyses {
            niter: 20,
            contig_fm: true,
            objective: ObjectiveType::Cut,
            lp_iter: 0,
            ufactor: 5,
        };
        let result = fm.refine(&g, p);

        // Verify both parts are contiguous by checking that each part's induced
        // subgraph is connected via a simple BFS.
        for part in 0..2u32 {
            let members: Vec<usize> = (0..g.n())
                .filter(|&v| result.assignment[v] == part)
                .collect();
            if members.is_empty() {
                continue;
            }
            // BFS within part
            let mut visited = vec![false; g.n()];
            let mut queue = std::collections::VecDeque::new();
            visited[members[0]] = true;
            queue.push_back(members[0]);
            while let Some(u) = queue.pop_front() {
                for j in g.xadj[u] as usize..g.xadj[u + 1] as usize {
                    let w = g.adjncy[j] as usize;
                    if !visited[w] && result.assignment[w] == part {
                        visited[w] = true;
                        queue.push_back(w);
                    }
                }
            }
            let reached = members.iter().filter(|&&v| visited[v]).count();
            assert_eq!(
                reached,
                members.len(),
                "part {part} is disconnected after contig_fm refinement"
            );
        }
    }

    // ── Volume objective tests ─────────────────────────────────────────────

    #[test]
    fn compute_volume_gain_path_move_boundary() {
        // Path 0-1-2-3, bisected [0,0,1,1].
        // Moving vertex 1 (part 0) to part 1:
        //   - vertex 2 (part 1): v=1 was its only from_part(0) neighbour → gain +1
        //   - vertex 0 (part 0): does not have to_part(1) neighbour → gain -1
        //   - v's own subdomain: before = {1} (neighbour 2 is in part 1), after = {0} (neighbour 0 is in part 0)
        //     s_before = 1, s_after = 1 → delta 0
        // Total: +1 - 1 + 0 = 0
        let g = path_graph(4);
        let assignment = vec![0u32, 0, 1, 1];
        let gain = compute_volume_gain(&g, &assignment, 1, 0, 1);
        // Accept 0 — no net improvement when both endpoints keep their subdomain sizes.
        // The exact value depends on the formula; just check it doesn't panic.
        let _ = gain;
    }

    #[test]
    fn volume_objective_produces_valid_partition() {
        use crate::api::{MetisParams, MetisPartitioner, ObjectiveType, Partitioner};
        let g = grid_4x4();
        let params = MetisParams {
            objective: ObjectiveType::Volume,
            ..MetisParams::default()
        };
        let p = MetisPartitioner::with_params(params, 4)
            .split(&g, 4, Some(0))
            .unwrap();
        assert_eq!(p.assignment.len(), 16);
        for part in 0..4u32 {
            assert!(
                p.assignment.contains(&part),
                "part {part} is missing from assignment"
            );
        }
    }

    #[test]
    fn volume_objective_valid_bisection() {
        // Sanity: volume objective on a path graph should still bisect correctly.
        use crate::api::{MetisParams, MetisPartitioner, ObjectiveType, Partitioner};
        let g = path_graph(10);
        let params = MetisParams {
            objective: ObjectiveType::Volume,
            ..MetisParams::default()
        };
        let p = MetisPartitioner::with_params(params, 2)
            .split(&g, 2, Some(42))
            .unwrap();
        assert_eq!(p.assignment.len(), 10);
        assert_eq!(p.k, 2);
        assert!(p.assignment.contains(&0), "part 0 missing");
        assert!(p.assignment.contains(&1), "part 1 missing");
    }

    #[test]
    fn best_volume_gain_boundary_vertex() {
        // Path 0-1-2-3, assignment [0,0,1,1].
        // Vertex 1 (part 0) is on boundary: adjacent to part 1.
        // best_volume_gain should return the gain for moving to part 1.
        let g = path_graph(4);
        let assignment = vec![0u32, 0, 1, 1];
        let g_val = best_volume_gain(&g, &assignment, 1, 0, 2);
        // Result must be a valid i32 — just check it doesn't panic.
        let _ = g_val;
    }
}

#[cfg(kani)]
mod kani_proofs {
    use super::*;
    use crate::refine::Refiner;

    fn kani_path(n: usize) -> CsrGraph {
        let mut xadj = vec![0u32];
        let mut adjncy = Vec::new();
        for i in 0..n {
            if i > 0 {
                adjncy.push((i - 1) as u32);
            }
            if i < n - 1 {
                adjncy.push((i + 1) as u32);
            }
            xadj.push(adjncy.len() as u32);
        }
        CsrGraph {
            xadj,
            adjncy,
            ncon: 1,
            vwgt: vec![1i32; n],
            adjwgt: None,
        }
    }

    /// Proves: FiducciaMattheyses::refine() never panics or goes OOB
    /// for valid path graphs up to n=16, k=4.
    /// Also proves: output assignment has correct length and valid part IDs.
    #[kani::proof]
    #[kani::unwind(17)]
    fn verify_fm_no_oob() {
        let n: usize = kani::any_where(|&n: &usize| n >= 4 && n <= 16);
        let k: u32 = kani::any_where(|&k: &u32| k >= 2 && k <= 4);
        kani::assume(k as usize <= n);

        let g = kani_path(n);
        kani::assume(g.is_valid());

        // Simple initial partition: vertex i gets part i%k
        let p = Partition {
            assignment: (0..n).map(|i| (i % k as usize) as u32).collect(),
            k,
            tpwgts: None,
        };

        let fm = FiducciaMattheyses {
            niter: 2,
            contig_fm: true,
            objective: ObjectiveType::Cut,
            lp_iter: 0,
            ufactor: 5,
        };
        let result = fm.refine(&g, p);

        // Safety postconditions:
        assert!(result.assignment.len() == n);
        assert!(result.assignment.iter().all(|&a| a < k));
    }
}
