use std::collections::BTreeMap;

/// Bucket-sort gain table for O(1) max-gain lookup.
///
/// Gains ∈ `[-max_gain, +max_gain]`. Uses offset indexing so all array
/// indices are non-negative: `bucket_idx(gain) = gain + max_gain`.
pub(crate) struct GainTable {
    buckets: GainBuckets,
    position: Vec<Option<(i32, usize)>>,
    pub(crate) max_gain: i32,
    top_bucket: i32,
}

enum GainBuckets {
    Dense(Vec<Vec<u32>>),
    Sparse(BTreeMap<i32, Vec<u32>>),
}

const MAX_DENSE_BUCKETS: usize = 1 << 20;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gain_table_max_is_correct() {
        let mut gt = GainTable::new(5, 10);
        gt.insert(0, 3);
        gt.insert(1, -2);
        gt.insert(2, 7);
        let (v, g) = gt.peek_max().unwrap();
        assert_eq!((v, g), (2, 7));
    }

    #[test]
    fn gain_table_remove_max() {
        let mut gt = GainTable::new(5, 10);
        gt.insert(0, 5);
        gt.insert(1, 3);
        gt.insert(2, 8);
        let (v, g) = gt.pop_max().unwrap();
        assert_eq!((v, g), (2, 8));
        let (v2, g2) = gt.pop_max().unwrap();
        assert_eq!((v2, g2), (0, 5));
    }

    #[test]
    fn gain_table_update_gain() {
        let mut gt = GainTable::new(3, 5);
        gt.insert(0, 2);
        gt.update(0, 4);
        let (v, g) = gt.peek_max().unwrap();
        assert_eq!((v, g), (0, 4));
    }

    #[test]
    fn gain_table_negative_gain() {
        let mut gt = GainTable::new(3, 5);
        gt.insert(0, -3);
        gt.insert(1, -1);
        let (v, _) = gt.peek_max().unwrap();
        assert_eq!(v, 1, "vertex 1 has gain -1 which is > -3");
    }

    #[test]
    fn gain_table_empty_returns_none() {
        let gt = GainTable::new(4, 10);
        assert!(gt.peek_max().is_none());
        assert!(gt.is_empty());
    }

    #[test]
    fn gain_table_large_range_uses_sparse_storage() {
        let mut gt = GainTable::new(3, i32::MAX);
        gt.insert(0, i32::MAX);
        gt.insert(1, -i32::MAX);
        gt.insert(2, 7);

        assert_eq!(gt.peek_max(), Some((0, i32::MAX)));
        assert_eq!(gt.pop_max(), Some((0, i32::MAX)));
        assert_eq!(gt.peek_max(), Some((2, 7)));
    }
}

#[cfg(kani)]
mod kani_proofs {
    use super::*;

    /// Proves: GainTable::insert, pop_max, update never panic or overflow
    /// for all gain values in [-128, 128] and up to 8 vertices.
    #[kani::proof]
    #[kani::unwind(9)]
    fn verify_gain_table_no_overflow() {
        let max_gain: i32 = kani::any_where(|&g: &i32| g > 0 && g <= 128);
        let n: usize = kani::any_where(|&n: &usize| n > 0 && n <= 8);
        let mut gt = GainTable::new(n, max_gain);

        let v: u32 = kani::any_where(|&v: &u32| (v as usize) < n);
        let gain: i32 = kani::any_where(|&g: &i32| g >= -max_gain && g <= max_gain);

        // Insert must not panic
        gt.insert(v, gain);

        // peek_max must not panic
        let _ = gt.peek_max();

        // pop_max must not panic
        let _ = gt.pop_max();
    }

    /// Proves: GainTable::update never panics for valid vertex + gain.
    #[kani::proof]
    #[kani::unwind(9)]
    fn verify_gain_table_update_no_panic() {
        let max_gain: i32 = kani::any_where(|&g: &i32| g > 0 && g <= 64);
        let n: usize = kani::any_where(|&n: &usize| n >= 2 && n <= 8);
        let mut gt = GainTable::new(n, max_gain);

        let v: u32 = kani::any_where(|&v: &u32| (v as usize) < n);
        let g1: i32 = kani::any_where(|&g: &i32| g >= -max_gain && g <= max_gain);
        let g2: i32 = kani::any_where(|&g: &i32| g >= -max_gain && g <= max_gain);

        gt.insert(v, g1);
        // update (remove + insert) must not panic
        gt.update(v, g2);
        let _ = gt.peek_max();
    }
}

impl GainTable {
    pub(crate) fn new(n_vertices: usize, max_gain: i32) -> Self {
        let max_gain = max_gain.max(1);
        let size = i64::from(max_gain)
            .checked_mul(2)
            .and_then(|size| size.checked_add(1))
            .unwrap_or(i64::MAX);
        let buckets = if size <= MAX_DENSE_BUCKETS as i64 {
            GainBuckets::Dense(vec![Vec::new(); size as usize])
        } else {
            GainBuckets::Sparse(BTreeMap::new())
        };
        Self {
            buckets,
            position: vec![None; n_vertices],
            max_gain,
            top_bucket: i32::MIN,
        }
    }

    fn bucket_idx(max_gain: i32, gain: i32) -> usize {
        (i64::from(gain) + i64::from(max_gain)) as usize
    }

    pub(crate) fn insert(&mut self, vertex: u32, gain: i32) {
        let gain = gain.clamp(-self.max_gain, self.max_gain);
        let pos = match &mut self.buckets {
            GainBuckets::Dense(buckets) => {
                let bi = Self::bucket_idx(self.max_gain, gain);
                let pos = buckets[bi].len();
                buckets[bi].push(vertex);
                pos
            }
            GainBuckets::Sparse(buckets) => {
                let bucket = buckets.entry(gain).or_default();
                let pos = bucket.len();
                bucket.push(vertex);
                pos
            }
        };
        self.position[vertex as usize] = Some((gain, pos));
        if gain > self.top_bucket {
            self.top_bucket = gain;
        }
    }

    pub(crate) fn remove(&mut self, vertex: u32) {
        if let Some((gain, pos)) = self.position[vertex as usize].take() {
            match &mut self.buckets {
                GainBuckets::Dense(buckets) => {
                    let bi = Self::bucket_idx(self.max_gain, gain);
                    let last = buckets[bi].len() - 1;
                    if pos < last {
                        let swap_v = buckets[bi][last];
                        buckets[bi][pos] = swap_v;
                        self.position[swap_v as usize] = Some((gain, pos));
                    }
                    buckets[bi].pop();
                }
                GainBuckets::Sparse(buckets) => {
                    if let Some(bucket) = buckets.get_mut(&gain) {
                        let last = bucket.len() - 1;
                        if pos < last {
                            let swap_v = bucket[last];
                            bucket[pos] = swap_v;
                            self.position[swap_v as usize] = Some((gain, pos));
                        }
                        bucket.pop();
                        if bucket.is_empty() {
                            buckets.remove(&gain);
                        }
                    }
                }
            }
        }
    }

    pub(crate) fn update(&mut self, vertex: u32, new_gain: i32) {
        self.remove(vertex);
        self.insert(vertex, new_gain);
    }

    pub(crate) fn peek_max(&self) -> Option<(u32, i32)> {
        match &self.buckets {
            GainBuckets::Dense(buckets) => {
                let mut g = i64::from(self.top_bucket);
                let min_gain = -i64::from(self.max_gain);
                while g >= min_gain {
                    let gain = g as i32;
                    let bi = Self::bucket_idx(self.max_gain, gain);
                    if let Some(&v) = buckets[bi].last() {
                        return Some((v, gain));
                    }
                    g -= 1;
                }
                None
            }
            GainBuckets::Sparse(buckets) => buckets
                .range(..=self.top_bucket)
                .next_back()
                .and_then(|(&gain, bucket)| bucket.last().copied().map(|v| (v, gain))),
        }
    }

    pub(crate) fn pop_max(&mut self) -> Option<(u32, i32)> {
        let (v, g) = self.peek_max()?;
        self.remove(v);
        self.top_bucket = g;
        Some((v, g))
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.peek_max().is_none()
    }

    pub(crate) fn contains(&self, vertex: u32) -> bool {
        self.position[vertex as usize].is_some()
    }
}
