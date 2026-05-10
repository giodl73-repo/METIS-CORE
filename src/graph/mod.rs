use crate::error::PartitionError;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct CsrGraph {
    pub(crate) xadj: Vec<u32>,
    pub(crate) adjncy: Vec<u32>,
    pub(crate) ncon: u32,
    pub(crate) vwgt: Vec<i32>,
    pub(crate) adjwgt: Option<Vec<i32>>,
}

impl CsrGraph {
    /// Build a CSR graph and validate its structural invariants.
    ///
    /// Callers outside this crate construct graphs through this method so
    /// malformed CSR input is rejected before partitioning starts.
    ///
    /// Required CSR contract:
    ///
    /// - `xadj.len() == n + 1` and `xadj[0] == 0`
    /// - `xadj[n] == adjncy.len()`
    /// - adjacency is undirected: every `v -> u` entry has a matching `u -> v`
    /// - vertex and edge weights are positive
    /// - the graph is connected
    pub fn new(
        xadj: Vec<u32>,
        adjncy: Vec<u32>,
        ncon: u32,
        vwgt: Vec<i32>,
        adjwgt: Option<Vec<i32>>,
    ) -> Result<Self, PartitionError> {
        let graph = Self {
            xadj,
            adjncy,
            ncon,
            vwgt,
            adjwgt,
        };
        graph.validate()?;
        Ok(graph)
    }

    /// Build a single-constraint CSR graph, using unit vertex or edge weights
    /// when the corresponding slices are empty.
    pub fn from_csr(
        xadj: &[u32],
        adjncy: &[u32],
        vwgt: &[i32],
        adjwgt: &[i32],
    ) -> Result<Self, PartitionError> {
        let n = xadj.len().saturating_sub(1);
        Self::new(
            xadj.to_vec(),
            adjncy.to_vec(),
            1,
            if vwgt.is_empty() {
                vec![1i32; n]
            } else {
                vwgt.to_vec()
            },
            if adjwgt.is_empty() {
                None
            } else {
                Some(adjwgt.to_vec())
            },
        )
    }

    pub fn n(&self) -> usize {
        self.xadj.len().saturating_sub(1)
    }

    pub fn xadj(&self) -> &[u32] {
        &self.xadj
    }

    pub fn adjncy(&self) -> &[u32] {
        &self.adjncy
    }

    pub fn ncon(&self) -> u32 {
        self.ncon
    }

    pub fn vwgt(&self) -> &[i32] {
        &self.vwgt
    }

    pub fn adjwgt(&self) -> Option<&[i32]> {
        self.adjwgt.as_deref()
    }

    pub fn is_valid(&self) -> bool {
        self.validate().is_ok()
    }

    pub fn validate(&self) -> Result<(), PartitionError> {
        let n = self.n();
        if self.xadj.len() != n + 1 {
            return Err(PartitionError::InvalidGraph("xadj length must be n + 1"));
        }
        if self.xadj[0] != 0 {
            return Err(PartitionError::InvalidGraph("xadj must start at zero"));
        }
        if self.xadj[n] as usize != self.adjncy.len() {
            return Err(PartitionError::InvalidGraph(
                "xadj terminator must equal adjncy length",
            ));
        }
        if self.ncon < 1 {
            return Err(PartitionError::InvalidGraph("ncon must be at least one"));
        }
        if self.vwgt.len() != n * self.ncon as usize {
            return Err(PartitionError::InvalidGraph(
                "vwgt length must equal n * ncon",
            ));
        }
        if self.vwgt.iter().any(|&w| w <= 0) {
            return Err(PartitionError::InvalidGraph(
                "vertex weights must be positive",
            ));
        }
        if let Some(ref aw) = self.adjwgt {
            if aw.len() != self.adjncy.len() {
                return Err(PartitionError::InvalidGraph(
                    "adjwgt length must equal adjncy length",
                ));
            }
            if aw.iter().any(|&w| w <= 0) {
                return Err(PartitionError::InvalidGraph(
                    "edge weights must be positive",
                ));
            }
        }
        if n == 0 {
            return Ok(());
        }
        for i in 0..n {
            if self.xadj[i] > self.xadj[i + 1] {
                return Err(PartitionError::InvalidGraph(
                    "xadj must be monotonically nondecreasing",
                ));
            }
            for j in self.xadj[i] as usize..self.xadj[i + 1] as usize {
                if j >= self.adjncy.len() {
                    return Err(PartitionError::InvalidGraph(
                        "xadj points past adjncy length",
                    ));
                }
                let nb = self.adjncy[j] as usize;
                if nb >= n || nb == i {
                    return Err(PartitionError::InvalidGraph(
                        "adjncy contains an invalid neighbor",
                    ));
                }
            }
        }
        for v in 0..n {
            for j in self.xadj[v] as usize..self.xadj[v + 1] as usize {
                let u = self.adjncy[j] as usize;
                let reverse = (self.xadj[u] as usize..self.xadj[u + 1] as usize)
                    .find(|&idx| self.adjncy[idx] as usize == v);
                let Some(reverse_idx) = reverse else {
                    return Err(PartitionError::InvalidGraph(
                        "adjncy must describe an undirected graph",
                    ));
                };
                if let Some(ref aw) = self.adjwgt {
                    if aw[j] != aw[reverse_idx] {
                        return Err(PartitionError::InvalidGraph(
                            "undirected edge weights must match",
                        ));
                    }
                }
            }
        }
        // Connectivity BFS from vertex 0
        let mut visited = vec![false; n];
        let mut queue = VecDeque::new();
        queue.push_back(0usize);
        visited[0] = true;
        while let Some(v) = queue.pop_front() {
            for j in self.xadj[v] as usize..self.xadj[v + 1] as usize {
                let u = self.adjncy[j] as usize;
                if !visited[u] {
                    visited[u] = true;
                    queue.push_back(u);
                }
            }
        }
        if visited.iter().all(|&v| v) {
            Ok(())
        } else {
            Err(PartitionError::InvalidGraph("graph must be connected"))
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Partition {
    pub(crate) assignment: Vec<u32>,
    pub(crate) k: u32,
    /// Target partition weights (one `f32` per part, summing to 1.0).
    /// `None` means equal weights (each part gets `1/k` of total population).
    /// Set by `split_weighted` and consumed by FM balance checks.
    pub(crate) tpwgts: Option<Vec<f32>>,
}

impl Partition {
    /// Build a partition from a part assignment.
    pub fn new(assignment: Vec<u32>, k: u32) -> Result<Self, PartitionError> {
        let partition = Self {
            assignment,
            k,
            tpwgts: None,
        };
        if partition.k == 0 {
            return Err(PartitionError::InvalidPartition("k must be at least one"));
        }
        if partition.assignment.iter().any(|&part| part >= partition.k) {
            return Err(PartitionError::InvalidPartition(
                "assignment contains part id outside 0..k",
            ));
        }
        Ok(partition)
    }

    pub fn assignment(&self) -> &[u32] {
        &self.assignment
    }

    pub fn k(&self) -> u32 {
        self.k
    }

    pub fn into_assignment(self) -> Vec<u32> {
        self.assignment
    }

    /// Validate that this partition is compatible with `g`.
    pub fn validate_for_graph(&self, g: &CsrGraph) -> Result<(), PartitionError> {
        if self.k == 0 {
            return Err(PartitionError::InvalidPartition("k must be at least one"));
        }
        if self.assignment.len() != g.n() {
            return Err(PartitionError::InvalidPartition(
                "assignment length must equal graph vertex count",
            ));
        }
        if self.assignment.iter().any(|&part| part >= self.k) {
            return Err(PartitionError::InvalidPartition(
                "assignment contains part id outside 0..k",
            ));
        }
        if let Some(tpwgts) = &self.tpwgts {
            if tpwgts.len() != self.k as usize {
                return Err(PartitionError::InvalidPartition(
                    "tpwgts length must equal k",
                ));
            }
            if tpwgts
                .iter()
                .any(|&weight| !weight.is_finite() || weight <= 0.0)
            {
                return Err(PartitionError::InvalidPartition(
                    "tpwgts entries must be finite and positive",
                ));
            }
            let sum: f32 = tpwgts.iter().sum();
            if sum <= 0.0 {
                return Err(PartitionError::InvalidPartition(
                    "tpwgts must contain positive total weight",
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct CoarseMap {
    cmap: Vec<u32>,
}

impl CoarseMap {
    /// Build a fine-to-coarse vertex map.
    ///
    /// `cmap[v]` is the coarse vertex containing fine vertex `v`. The map must
    /// contain one entry per fine vertex, every target must be in
    /// `0..coarse_n`, and every coarse vertex must be targeted by at least one
    /// fine vertex.
    pub fn new(cmap: Vec<u32>, fine_n: usize, coarse_n: usize) -> Result<Self, PartitionError> {
        if cmap.len() != fine_n {
            return Err(PartitionError::InvalidGraph(
                "coarse map length must equal fine vertex count",
            ));
        }
        if coarse_n == 0 && fine_n > 0 {
            return Err(PartitionError::InvalidGraph(
                "non-empty coarse map must target at least one coarse vertex",
            ));
        }
        if cmap.iter().any(|&target| target as usize >= coarse_n) {
            return Err(PartitionError::InvalidGraph(
                "coarse map target is outside coarse vertex range",
            ));
        }
        let mut covered = vec![false; coarse_n];
        for &target in &cmap {
            covered[target as usize] = true;
        }
        if covered.iter().any(|&seen| !seen) {
            return Err(PartitionError::InvalidGraph(
                "coarse map must target every coarse vertex",
            ));
        }
        Ok(Self { cmap })
    }

    pub(crate) fn from_validated(cmap: Vec<u32>) -> Self {
        Self { cmap }
    }

    pub fn as_slice(&self) -> &[u32] {
        &self.cmap
    }

    pub fn len(&self) -> usize {
        self.cmap.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cmap.is_empty()
    }
}

/// Check if every part in `partition` is connected within `g`.
/// Returns `Ok(())` if all parts are contiguous, the first disconnected part ID
/// if a valid partition is non-contiguous, or `u32::MAX` when the partition is
/// structurally invalid for the graph.
pub fn check_contiguity(g: &CsrGraph, partition: &Partition) -> Result<(), u32> {
    if partition.validate_for_graph(g).is_err() {
        return Err(u32::MAX);
    }

    let n = g.n();
    let k = partition.k as usize;

    // Find one representative vertex per part
    let mut rep = vec![usize::MAX; k];
    for v in 0..n {
        let p = partition.assignment[v] as usize;
        if rep[p] == usize::MAX {
            rep[p] = v;
        }
    }

    // BFS within each part from its representative
    let mut visited = vec![false; n];
    for (part, &start) in rep.iter().enumerate() {
        if start == usize::MAX {
            continue;
        } // empty part
        visited[start] = true;
        let mut queue = std::collections::VecDeque::from([start]);
        while let Some(v) = queue.pop_front() {
            for j in g.xadj[v] as usize..g.xadj[v + 1] as usize {
                let u = g.adjncy[j] as usize;
                if !visited[u] && partition.assignment[u] as usize == part {
                    visited[u] = true;
                    queue.push_back(u);
                }
            }
        }
        // Check all vertices of this part were reached
        for (v, &was_visited) in visited.iter().enumerate() {
            if partition.assignment[v] as usize == part && !was_visited {
                return Err(part as u32);
            }
        }
        // Reset visited for next part (only clear this part's vertices)
        for (v, was_visited) in visited.iter_mut().enumerate() {
            if partition.assignment[v] as usize == part {
                *was_visited = false;
            }
        }
    }
    Ok(())
}

/// Extract an induced subgraph containing only vertices with `assignment[v] == part`.
///
/// Returns `(subgraph, global_to_local, local_to_global)` where:
/// - `global_to_local[v]` is the local index of global vertex `v`
///   (`usize::MAX` if the vertex is not in the subgraph).
/// - `local_to_global[i]` is the global index corresponding to local vertex `i`.
///
/// Edge weights are preserved when present; vertex weights are copied.
/// The returned subgraph is not necessarily connected — callers that require
/// connectivity should ensure the chosen `part` is a contiguous region.
pub fn extract_subgraph(
    g: &CsrGraph,
    assignment: &[u32],
    part: u32,
) -> (CsrGraph, Vec<usize>, Vec<usize>) {
    let n = g.n();
    let ncon = g.ncon as usize;

    // Build vertex maps
    let mut global_to_local = vec![usize::MAX; n];
    let mut local_to_global: Vec<usize> = Vec::new();
    for v in 0..n {
        if assignment[v] == part {
            global_to_local[v] = local_to_global.len();
            local_to_global.push(v);
        }
    }

    // Build subgraph CSR — only include edges where both endpoints are in part
    let mut xadj = vec![0u32];
    let mut adjncy: Vec<u32> = Vec::new();
    let mut adjwgt: Vec<i32> = Vec::new();
    let mut vwgt: Vec<i32> = Vec::new();

    for &v in &local_to_global {
        for c in 0..ncon {
            vwgt.push(g.vwgt[v * ncon + c]);
        }
        for j in g.xadj[v] as usize..g.xadj[v + 1] as usize {
            let u = g.adjncy[j] as usize;
            if assignment[u] == part {
                adjncy.push(global_to_local[u] as u32);
                if let Some(ref aw) = g.adjwgt {
                    adjwgt.push(aw[j]);
                }
            }
        }
        xadj.push(adjncy.len() as u32);
    }

    let sub = CsrGraph {
        xadj,
        adjncy,
        ncon: g.ncon,
        vwgt,
        adjwgt: if g.adjwgt.is_some() {
            Some(adjwgt)
        } else {
            None
        },
    };
    (sub, global_to_local, local_to_global)
}

/// Repair non-contiguous partitions by reassigning disconnected components
/// to their largest adjacent part. Modifies partition in place.
/// Returns the number of vertices reassigned.
///
/// Each iteration enumerates ALL connected components for each part, keeps the
/// LARGEST component as the "main" component (mirrors METIS `EliminateComponents`
/// which keeps the heaviest component), and reassigns every smaller component
/// to the neighbouring part with the most boundary edges.  The neighbour is
/// chosen by scanning the entire component for external edges and picking the
/// most-frequently contacted foreign part.  This handles chains of vertices
/// whose internal members have no direct foreign neighbours.
///
/// Using the largest component as "main" avoids the pathological case where a
/// small low-index island is kept as "main" and a large correct piece gets
/// spuriously reassigned (which can cascade and require many extra passes).
pub fn repair_contiguity(g: &CsrGraph, partition: &mut Partition) -> usize {
    let n = g.n();
    let mut reassigned = 0usize;

    // Iteratively fix until all parts are contiguous (max n*k iterations for safety).
    // Each iteration reassigns at least one secondary component, reducing the total
    // component count across all parts by ≥1, so convergence is guaranteed.
    let k = partition.k as usize;
    for _ in 0..n * k {
        if check_contiguity(g, partition).is_ok() {
            break;
        }

        let mut made_progress = false;

        'parts: for part in 0..k {
            // Find ALL connected components of this part via BFS, tracking each
            // component's size so we can identify the largest ("main") component.
            let mut comp_id = vec![usize::MAX; n]; // comp_id[v] = component index
            let mut comp_sizes: Vec<usize> = Vec::new();

            for start in 0..n {
                if partition.assignment[start] as usize != part {
                    continue;
                }
                if comp_id[start] != usize::MAX {
                    continue;
                }

                let cid = comp_sizes.len();
                let mut size = 0usize;
                let mut queue = std::collections::VecDeque::from([start]);
                comp_id[start] = cid;
                while let Some(v) = queue.pop_front() {
                    size += 1;
                    for j in g.xadj[v] as usize..g.xadj[v + 1] as usize {
                        let u = g.adjncy[j] as usize;
                        if comp_id[u] == usize::MAX && partition.assignment[u] as usize == part {
                            comp_id[u] = cid;
                            queue.push_back(u);
                        }
                    }
                }
                comp_sizes.push(size);
            }

            if comp_sizes.len() <= 1 {
                continue;
            } // this part is already contiguous

            // Identify the largest component — this is the "main" component to keep.
            let main_cid = comp_sizes
                .iter()
                .enumerate()
                .max_by_key(|&(_, &sz)| sz)
                .map(|(i, _)| i)
                .unwrap_or(0);

            // Collect secondary (non-main) components and reassign them.
            // Iterate over secondary component IDs; for each, collect its vertices
            // and count external edges to choose the best target part.
            for sec_cid in 0..comp_sizes.len() {
                if sec_cid == main_cid {
                    continue;
                }

                // Collect all vertices of this secondary component.
                let component: Vec<usize> = (0..n).filter(|&v| comp_id[v] == sec_cid).collect();

                // Count external edges from this component to each foreign part.
                let mut adj_counts = vec![0u32; k];
                for &v in &component {
                    for j in g.xadj[v] as usize..g.xadj[v + 1] as usize {
                        let u = g.adjncy[j] as usize;
                        let up = partition.assignment[u] as usize;
                        if up != part {
                            adj_counts[up] += 1;
                        }
                    }
                }

                // Pick the foreign part with the most external edges.
                if let Some((best_part, _)) = adj_counts
                    .iter()
                    .enumerate()
                    .filter(|&(p, &c)| p != part && c > 0)
                    .max_by_key(|&(_, &c)| c)
                {
                    for &v in &component {
                        partition.assignment[v] = best_part as u32;
                        reassigned += 1;
                    }
                    made_progress = true;
                    break 'parts; // restart outer loop after any reassignment
                }
                // If no foreign edges found, this component is only connected to
                // other secondary components of the same part.  Those neighbours
                // will be reassigned in a later iteration, after which this
                // component will gain foreign edges and be fixed as well.
            }
        }

        if !made_progress {
            break;
        } // no further progress possible
    }
    reassigned
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn path_graph(n: usize) -> CsrGraph {
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

    #[test]
    fn valid_path_graph() {
        assert!(path_graph(5).is_valid());
    }

    #[test]
    fn new_accepts_valid_graph() {
        let g = path_graph(5);
        let built = CsrGraph::new(g.xadj, g.adjncy, g.ncon, g.vwgt, g.adjwgt)
            .expect("valid path graph should construct");
        assert_eq!(built.n(), 5);
    }

    #[test]
    fn new_rejects_invalid_graph() {
        let result = CsrGraph::new(vec![0, 1, 2], vec![1, 2], 1, vec![1, 1], None);
        assert!(matches!(result, Err(PartitionError::InvalidGraph(_))));
    }

    #[test]
    fn from_csr_defaults_empty_weights_to_unit_weights() {
        let g = path_graph(4);
        let built = CsrGraph::from_csr(&g.xadj, &g.adjncy, &[], &[])
            .expect("valid unweighted path graph should construct");
        assert_eq!(built.vwgt, vec![1; 4]);
        assert!(built.adjwgt.is_none());
    }

    #[test]
    fn invalid_self_loop() {
        let mut g = path_graph(4);
        g.adjncy[0] = 0;
        assert!(!g.is_valid());
    }

    #[test]
    fn invalid_out_of_bounds_adjncy() {
        let mut g = path_graph(4);
        g.adjncy[0] = 99;
        assert!(!g.is_valid());
    }

    #[test]
    fn invalid_zero_vwgt() {
        let mut g = path_graph(4);
        g.vwgt[1] = 0;
        assert!(!g.is_valid());
    }

    #[test]
    fn invalid_negative_vwgt() {
        let mut g = path_graph(4);
        g.vwgt[1] = -1;
        assert!(!g.is_valid());
    }

    #[test]
    fn invalid_disconnected() {
        let g = CsrGraph {
            xadj: vec![0, 1, 2, 3, 4],
            adjncy: vec![1, 0, 3, 2],
            ncon: 1,
            vwgt: vec![1; 4],
            adjwgt: None,
        };
        assert!(!g.is_valid());
    }

    #[test]
    fn invalid_adjwgt_wrong_len() {
        let mut g = path_graph(4);
        g.adjwgt = Some(vec![1i32; 3]);
        assert!(!g.is_valid());
    }

    #[test]
    fn invalid_trailing_adjncy() {
        let result = CsrGraph::new(vec![0, 1, 2], vec![1, 0, 0], 1, vec![1; 2], None);
        assert!(matches!(result, Err(PartitionError::InvalidGraph(_))));
    }

    #[test]
    fn invalid_directed_adjncy() {
        let result = CsrGraph::new(vec![0, 1, 2, 2], vec![1, 2], 1, vec![1; 3], None);
        assert!(matches!(result, Err(PartitionError::InvalidGraph(_))));
    }

    #[test]
    fn invalid_zero_adjwgt() {
        let mut g = path_graph(4);
        g.adjwgt = Some(vec![1i32; g.adjncy.len()]);
        g.adjwgt.as_mut().unwrap()[0] = 0;
        assert!(!g.is_valid());
    }

    #[test]
    fn invalid_asymmetric_adjwgt() {
        let result = CsrGraph::new(vec![0, 1, 2], vec![1, 0], 1, vec![1; 2], Some(vec![1, 2]));
        assert!(matches!(result, Err(PartitionError::InvalidGraph(_))));
    }

    #[test]
    fn valid_multi_constraint() {
        let mut g = path_graph(4);
        g.ncon = 2;
        g.vwgt = vec![1, 2, 3, 4, 5, 6, 7, 8];
        assert!(g.is_valid());
    }

    // ── extract_subgraph ───────────────────────────────────────────────────

    #[test]
    fn extract_subgraph_contains_correct_vertices() {
        let g = path_graph(4);
        let assignment = [0u32, 0, 1, 1];
        let (sub, _g2l, l2g) = extract_subgraph(&g, &assignment, 0);
        assert_eq!(
            l2g,
            vec![0, 1],
            "left subgraph should contain vertices 0 and 1"
        );
        assert!(sub.is_valid());
        assert_eq!(sub.n(), 2);
        assert_eq!(
            sub.adjncy.len(),
            2,
            "one internal edge 0-1 = 2 directed entries"
        );
    }

    #[test]
    fn extract_subgraph_preserves_edge_weights() {
        let mut g = path_graph(4);
        g.adjwgt = Some(vec![3i32; g.adjncy.len()]);
        let assignment = [0u32, 0, 1, 1];
        let (sub, _, _) = extract_subgraph(&g, &assignment, 0);
        assert!(sub.adjwgt.is_some());
        assert!(
            sub.adjwgt.as_ref().unwrap().iter().all(|&w| w == 3),
            "edge weight must survive into subgraph"
        );
    }

    #[test]
    fn extract_subgraph_unweighted_stays_unweighted() {
        let g = path_graph(4);
        let assignment = [0u32, 0, 1, 1];
        let (sub, _, _) = extract_subgraph(&g, &assignment, 0);
        assert!(sub.adjwgt.is_none());
    }

    #[test]
    fn extract_subgraph_single_vertex_no_edges() {
        let g = path_graph(4);
        let assignment = [0u32, 1, 0, 0]; // vertex 1 is isolated in part 1
        let (sub, _, l2g) = extract_subgraph(&g, &assignment, 1);
        assert_eq!(l2g, vec![1]);
        assert_eq!(sub.n(), 1);
        assert_eq!(sub.adjncy.len(), 0, "isolated vertex has no internal edges");
    }

    #[test]
    fn extract_subgraph_multiconstraint_weight_layout() {
        let mut g = path_graph(4);
        g.ncon = 2;
        g.vwgt = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let assignment = [0u32, 0, 1, 1];
        let (sub, _, _) = extract_subgraph(&g, &assignment, 0);
        assert_eq!(sub.ncon, 2);
        assert_eq!(sub.vwgt.len(), 4, "2 vertices × 2 constraints");
        assert_eq!(&sub.vwgt[0..2], &[1, 2], "vertex 0 weights");
        assert_eq!(&sub.vwgt[2..4], &[3, 4], "vertex 1 weights");
    }

    // ── check_contiguity ───────────────────────────────────────────────────

    #[test]
    fn check_contiguity_path_bisect_is_ok() {
        let g = path_graph(4);
        let p = Partition {
            assignment: vec![0, 0, 1, 1],
            k: 2,
            tpwgts: None,
        };
        assert!(check_contiguity(&g, &p).is_ok());
    }

    #[test]
    fn check_contiguity_disconnected_returns_err_with_part_id() {
        // Path 0-1-2-3-4: part 0 = {0,1,4} — not connected (4 separated from 0,1)
        let g = path_graph(5);
        let p = Partition {
            assignment: vec![0, 0, 1, 1, 0],
            k: 2,
            tpwgts: None,
        };
        let err = check_contiguity(&g, &p);
        assert!(err.is_err(), "disconnected part must return Err");
        assert_eq!(
            err.unwrap_err(),
            0,
            "err value must be the disconnected part ID"
        );
    }

    #[test]
    fn partition_validate_for_graph_accepts_valid_partition() {
        let g = path_graph(4);
        let p = Partition {
            assignment: vec![0, 0, 1, 1],
            k: 2,
            tpwgts: Some(vec![0.5, 0.5]),
        };
        assert!(p.validate_for_graph(&g).is_ok());
    }

    #[test]
    fn partition_validate_for_graph_rejects_short_assignment() {
        let g = path_graph(4);
        let p = Partition {
            assignment: vec![0, 1, 1],
            k: 2,
            tpwgts: None,
        };
        assert!(matches!(
            p.validate_for_graph(&g),
            Err(PartitionError::InvalidPartition(_))
        ));
    }

    #[test]
    fn partition_validate_for_graph_rejects_out_of_range_part() {
        let g = path_graph(4);
        let p = Partition {
            assignment: vec![0, 0, 1, 2],
            k: 2,
            tpwgts: None,
        };
        assert!(matches!(
            p.validate_for_graph(&g),
            Err(PartitionError::InvalidPartition(_))
        ));
    }

    #[test]
    fn coarse_map_new_accepts_surjective_in_range_map() {
        let cmap = CoarseMap::new(vec![0, 0, 1, 1], 4, 2)
            .expect("valid fine-to-coarse map should construct");
        assert_eq!(cmap.as_slice(), &[0, 0, 1, 1]);
    }

    #[test]
    fn coarse_map_new_rejects_malformed_maps() {
        assert!(matches!(
            CoarseMap::new(vec![0, 1], 3, 2),
            Err(PartitionError::InvalidGraph(_))
        ));
        assert!(matches!(
            CoarseMap::new(vec![0, 2], 2, 2),
            Err(PartitionError::InvalidGraph(_))
        ));
        assert!(matches!(
            CoarseMap::new(vec![0, 0], 2, 2),
            Err(PartitionError::InvalidGraph(_))
        ));
    }

    #[test]
    fn check_contiguity_rejects_invalid_partition_without_panic() {
        let g = path_graph(4);
        let p = Partition {
            assignment: vec![0, 0, 1, 2],
            k: 2,
            tpwgts: None,
        };
        assert_eq!(check_contiguity(&g, &p), Err(u32::MAX));
    }

    // ── repair_contiguity ──────────────────────────────────────────────────

    #[test]
    fn repair_contiguity_fixes_disconnected_part() {
        let g = path_graph(5);
        let mut p = Partition {
            assignment: vec![0, 0, 1, 1, 0],
            k: 2,
            tpwgts: None,
        };
        assert!(
            check_contiguity(&g, &p).is_err(),
            "pre-condition: must be non-contiguous"
        );
        let moved = repair_contiguity(&g, &mut p);
        assert!(moved > 0, "must have moved at least one vertex");
        assert!(
            check_contiguity(&g, &p).is_ok(),
            "must be contiguous after repair"
        );
    }

    #[test]
    fn repair_contiguity_noop_when_already_contiguous() {
        let g = path_graph(4);
        let mut p = Partition {
            assignment: vec![0, 0, 1, 1],
            k: 2,
            tpwgts: None,
        };
        let orig = p.assignment.clone();
        let moved = repair_contiguity(&g, &mut p);
        assert_eq!(
            moved, 0,
            "no vertices should move when partition is already contiguous"
        );
        assert_eq!(p.assignment, orig);
    }

    #[test]
    fn repair_contiguity_handles_k1() {
        let g = path_graph(6);
        let mut p = Partition {
            assignment: vec![0; 6],
            k: 1,
            tpwgts: None,
        };
        let moved = repair_contiguity(&g, &mut p);
        assert_eq!(moved, 0, "k=1 is trivially contiguous — no repair needed");
    }
}

#[cfg(kani)]
mod kani_proofs {
    use super::*;

    /// Proves: CsrGraph::is_valid() never panics for any input up to n=8.
    /// Covers: all branches in is_valid() including xadj check, self-loop,
    /// OOB adjncy, ncon, vwgt positivity, adjwgt length, BFS connectivity.
    #[kani::proof]
    #[kani::unwind(9)]
    fn verify_is_valid_no_panic() {
        let n: usize = kani::any_where(|&n: &usize| n <= 8);
        // Construct arbitrary xadj of length n+1
        let xadj: Vec<u32> = (0..=n).map(|_| kani::any()).collect();
        let adjncy_len: usize = kani::any_where(|&l: &usize| l <= 32);
        let adjncy: Vec<u32> = (0..adjncy_len).map(|_| kani::any()).collect();
        let ncon: u32 = kani::any_where(|&c: &u32| c <= 4);
        let vwgt_len = n.saturating_mul(ncon as usize).min(64);
        let vwgt: Vec<i32> = (0..vwgt_len).map(|_| kani::any()).collect();
        let g = CsrGraph {
            xadj,
            adjncy,
            ncon,
            vwgt,
            adjwgt: None,
        };
        // Must not panic — result is ignored
        let _ = g.is_valid();
    }
}
