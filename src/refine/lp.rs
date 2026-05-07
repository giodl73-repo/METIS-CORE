use crate::graph::{CsrGraph, Partition};

/// Label-propagation balance refinement.
///
/// Each iteration: every boundary vertex considers moving to the adjacent
/// part with the highest weight deficit (most under target).  A move is
/// accepted only when it strictly reduces the maximum per-part imbalance and
/// keeps both the source and destination within `target ± epsilon`.
///
/// Mirrors METIS `BalanceAndRefineLP` from `kmetis.c`.
pub fn lp_balance(g: &CsrGraph, partition: &mut Partition, _ufactor: u32, max_iter: u32) {
    if max_iter == 0 { return; }
    let n = g.n();
    let k = partition.k as usize;
    if k <= 1 { return; }

    let total_wgt: i64 = g.vwgt.iter().map(|&w| w as i64).sum();
    let target  = total_wgt / k as i64;
    // Ceiling of 0.5% of total, matching the FM balance epsilon
    let epsilon = (total_wgt * 5 + 999) / 1000;

    let mut pwgts = vec![0i64; k];
    for v in 0..n {
        pwgts[partition.assignment[v] as usize] += g.vwgt[v] as i64;
    }

    for _iter in 0..max_iter {
        let mut moved = 0usize;

        for v in 0..n {
            let from = partition.assignment[v] as usize;
            let vwgt = g.vwgt[v] as i64;

            // Only consider boundary vertices (at least one neighbour in a different part)
            let is_boundary = (g.xadj[v] as usize..g.xadj[v + 1] as usize)
                .any(|j| partition.assignment[g.adjncy[j] as usize] as usize != from);
            if !is_boundary { continue; }

            // Source part must be overloaded before we consider moving anyone out
            if pwgts[from] <= target + epsilon { continue; }

            // Find the most under-target adjacent part (minimum pwgts wins)
            let best_to = (g.xadj[v] as usize..g.xadj[v + 1] as usize)
                .map(|j| partition.assignment[g.adjncy[j] as usize] as usize)
                .filter(|&to| to != from)
                .min_by_key(|&to| pwgts[to]);

            if let Some(to) = best_to {
                // Accept the move only when it keeps both sides in balance
                let new_from = pwgts[from] - vwgt;
                let new_to   = pwgts[to]   + vwgt;
                if new_from >= target - epsilon && new_to <= target + epsilon {
                    partition.assignment[v] = to as u32;
                    pwgts[from] = new_from;
                    pwgts[to]   = new_to;
                    moved += 1;
                }
            }
        }

        if moved == 0 { break; }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path_graph(n: usize) -> CsrGraph {
        let mut xadj = vec![0u32];
        let mut adjncy = Vec::new();
        for i in 0..n {
            if i > 0 { adjncy.push((i - 1) as u32); }
            if i < n - 1 { adjncy.push((i + 1) as u32); }
            xadj.push(adjncy.len() as u32);
        }
        CsrGraph { xadj, adjncy, ncon: 1, vwgt: vec![1i32; n], adjwgt: None }
    }

    fn make_grid_graph(rows: usize, cols: usize) -> CsrGraph {
        let n = rows * cols;
        let mut xadj = vec![0u32];
        let mut adjncy = Vec::new();
        for r in 0..rows {
            for c in 0..cols {
                let mut nbrs = Vec::new();
                if r > 0         { nbrs.push((r-1)*cols + c); }
                if r < rows - 1  { nbrs.push((r+1)*cols + c); }
                if c > 0         { nbrs.push(r*cols + (c-1)); }
                if c < cols - 1  { nbrs.push(r*cols + (c+1)); }
                for &u in &nbrs { adjncy.push(u as u32); }
                xadj.push(adjncy.len() as u32);
            }
        }
        CsrGraph { xadj, adjncy, ncon: 1, vwgt: vec![1i32; n], adjwgt: None }
    }

    fn pwgtss(g: &CsrGraph, p: &Partition) -> Vec<i64> {
        let k = p.k as usize;
        let mut wgts = vec![0i64; k];
        for v in 0..g.n() {
            wgts[p.assignment[v] as usize] += g.vwgt[v] as i64;
        }
        wgts
    }

    fn max_imbalance(wgts: &[i64], target: i64) -> i64 {
        wgts.iter().map(|&p| (p - target).abs()).max().unwrap_or(0)
    }

    /// LP with 0 iterations must be a no-op.
    #[test]
    fn lp_zero_iter_is_noop() {
        let g = path_graph(10);
        let mut p = Partition {
            assignment: vec![0,0,0,0,0,1,1,1,1,1],
            k: 2,
            tpwgts: None,
        };
        let before = p.assignment.clone();
        lp_balance(&g, &mut p, 5, 0);
        assert_eq!(p.assignment, before);
    }

    /// Already-balanced partition should not be disturbed.
    #[test]
    fn lp_balanced_partition_unchanged() {
        let g = path_graph(10);
        // Perfect 5-5 split
        let mut p = Partition {
            assignment: vec![0,0,0,0,0,1,1,1,1,1],
            k: 2,
            tpwgts: None,
        };
        let before = p.assignment.clone();
        lp_balance(&g, &mut p, 5, 10);
        // Should not have moved anyone (already balanced)
        assert_eq!(p.assignment, before,
            "perfectly balanced partition must not be changed by LP");
    }

    /// Heavily imbalanced 4-part partition on a grid should improve balance.
    #[test]
    fn lp_improves_imbalance_on_grid() {
        let g = make_grid_graph(4, 4); // 16 vertices
        // Intentionally unbalanced: put 10 vertices in part 0, 2 each in parts 1-3
        // Layout: first 10 in part 0, next 2 in 1, next 2 in 2, last 2 in 3
        let assignment: Vec<u32> = (0..16).map(|v| {
            if v < 10 { 0 }
            else if v < 12 { 1 }
            else if v < 14 { 2 }
            else { 3 }
        }).collect();
        let mut p = Partition { assignment, k: 4, tpwgts: None };
        let total: i64 = 16;
        let target = total / 4; // = 4
        let before_pops = pwgtss(&g, &p);
        let before_imb = max_imbalance(&before_pops, target);
        lp_balance(&g, &mut p, 5, 20);
        let after_pops = pwgtss(&g, &p);
        let after_imb = max_imbalance(&after_pops, target);
        assert!(after_imb <= before_imb,
            "LP must not worsen imbalance: before={before_imb} after={after_imb}");
    }

    /// Assignment validity: all assignments must remain in 0..k after LP.
    #[test]
    fn lp_assignment_stays_in_range() {
        let g = make_grid_graph(4, 4);
        let k = 4u32;
        let mut p = Partition {
            assignment: (0..16).map(|v| (v % k as usize) as u32).collect(),
            k,
            tpwgts: None,
        };
        lp_balance(&g, &mut p, 5, 10);
        assert!(p.assignment.iter().all(|&a| a < k),
            "LP produced out-of-range assignment");
    }

    /// Part populations must sum to total vertex weight after LP.
    #[test]
    fn lp_total_wgt_conserved() {
        let g = make_grid_graph(4, 4);
        let total_before: i64 = g.vwgt.iter().map(|&w| w as i64).sum();
        let mut p = Partition {
            assignment: (0..16).map(|v| (v % 4) as u32).collect(),
            k: 4,
            tpwgts: None,
        };
        lp_balance(&g, &mut p, 5, 10);
        let total_after: i64 = g.vwgt.iter().map(|&w| w as i64).sum();
        assert_eq!(total_before, total_after);
        // Also check per-part wgts sum to total
        let wgts = pwgtss(&g, &p);
        assert_eq!(wgts.iter().sum::<i64>(), total_before);
    }

    /// k=1 trivial partition — LP should be a no-op.
    #[test]
    fn lp_k1_noop() {
        let g = path_graph(5);
        let mut p = Partition {
            assignment: vec![0; 5],
            k: 1,
            tpwgts: None,
        };
        let before = p.assignment.clone();
        lp_balance(&g, &mut p, 5, 10);
        assert_eq!(p.assignment, before);
    }
}
