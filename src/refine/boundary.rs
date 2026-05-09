use crate::graph::{CsrGraph, Partition};

/// Cache-friendly boundary set using a boolean array indexed by vertex ID.
/// O(1) contains/insert/remove with sequential memory access.
/// iter() is O(n) (scans full array) — acceptable since boundary is ~20% of n.
pub struct BoundarySet {
    inner: Vec<bool>,
}

impl BoundarySet {
    pub fn from_partition(g: &CsrGraph, p: &Partition) -> Self {
        let n = g.n();
        let mut inner = vec![false; n];
        for (v, is_boundary) in inner.iter_mut().enumerate().take(n) {
            for j in g.xadj[v] as usize..g.xadj[v + 1] as usize {
                let u = g.adjncy[j] as usize;
                if p.assignment[v] != p.assignment[u] {
                    *is_boundary = true;
                    break;
                }
            }
        }
        Self { inner }
    }

    pub fn contains(&self, v: u32) -> bool {
        self.inner[v as usize]
    }

    pub fn insert(&mut self, v: u32) {
        self.inner[v as usize] = true;
    }

    pub fn remove(&mut self, v: u32) {
        self.inner[v as usize] = false;
    }

    pub fn iter(&self) -> impl Iterator<Item = u32> + '_ {
        self.inner
            .iter()
            .enumerate()
            .filter_map(|(i, &b)| if b { Some(i as u32) } else { None })
    }

    pub fn len(&self) -> usize {
        self.inner.iter().filter(|&&b| b).count()
    }

    pub fn is_empty(&self) -> bool {
        !self.inner.iter().any(|&b| b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::Partition;

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

    #[test]
    fn boundary_contains_cross_part_vertices() {
        let g = path_graph(3);
        let p = Partition {
            assignment: vec![0, 0, 1],
            k: 2,
            tpwgts: None,
        };
        let b = BoundarySet::from_partition(&g, &p);
        assert!(b.contains(1), "vertex 1 should be on boundary");
        assert!(b.contains(2), "vertex 2 should be on boundary");
        assert!(!b.contains(0), "vertex 0 should NOT be on boundary");
    }

    #[test]
    fn boundary_all_same_part_empty() {
        let g = path_graph(5);
        let p = Partition {
            assignment: vec![0; 5],
            k: 1,
            tpwgts: None,
        };
        let b = BoundarySet::from_partition(&g, &p);
        for v in 0..5u32 {
            assert!(!b.contains(v));
        }
    }

    #[test]
    fn boundary_insert_and_remove() {
        let g = path_graph(4);
        let p = Partition {
            assignment: vec![0, 0, 1, 1],
            k: 2,
            tpwgts: None,
        };
        let mut b = BoundarySet::from_partition(&g, &p);
        // Vertex 0 is NOT on boundary initially
        assert!(!b.contains(0), "vertex 0 should not be on boundary");
        b.insert(0);
        assert!(b.contains(0), "after insert, vertex 0 must be on boundary");
        b.remove(0);
        assert!(
            !b.contains(0),
            "after remove, vertex 0 must not be on boundary"
        );
    }

    #[test]
    fn boundary_iter_yields_all_boundary_vertices() {
        let g = path_graph(4); // 0-1-2-3, bisected at middle
        let p = Partition {
            assignment: vec![0, 0, 1, 1],
            k: 2,
            tpwgts: None,
        };
        let b = BoundarySet::from_partition(&g, &p);
        let mut boundary_verts: Vec<u32> = b.iter().collect();
        boundary_verts.sort();
        // Vertices 1 and 2 are on the boundary (share edge across bisection)
        assert_eq!(
            boundary_verts,
            vec![1, 2],
            "only vertices 1 and 2 are on the boundary of a middle bisection"
        );
    }

    #[test]
    fn boundary_is_empty_and_len() {
        let g = path_graph(4);
        let p_full = Partition {
            assignment: vec![0, 0, 1, 1],
            k: 2,
            tpwgts: None,
        };
        let p_none = Partition {
            assignment: vec![0; 4],
            k: 1,
            tpwgts: None,
        };
        let b_full = BoundarySet::from_partition(&g, &p_full);
        let b_none = BoundarySet::from_partition(&g, &p_none);
        assert!(!b_full.is_empty(), "bisected graph has boundary vertices");
        assert_eq!(b_full.len(), 2, "exactly 2 boundary vertices");
        assert!(b_none.is_empty(), "all-same-part has no boundary");
        assert_eq!(b_none.len(), 0);
    }
}
