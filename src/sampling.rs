//! Index sampling policy. Legacy replay remains target-specific by default.
use rand::Rng;

pub(crate) fn index(rng: &mut impl Rng, upper: usize) -> usize {
    #[cfg(feature = "portable-seed-replay")]
    {
        rng.gen_range(0..upper as u64) as usize
    }
    #[cfg(not(feature = "portable-seed-replay"))]
    {
        rng.gen_range(0..upper)
    }
}

pub(crate) fn index_inclusive(rng: &mut impl Rng, upper: usize) -> usize {
    #[cfg(feature = "portable-seed-replay")]
    {
        rng.gen_range(0..=upper as u64) as usize
    }
    #[cfg(not(feature = "portable-seed-replay"))]
    {
        rng.gen_range(0..=upper)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_pcg::Pcg64;

    // Compare both values AND the remaining generator stream against the
    // original rand sampling expression; rejection sampling consumes draws.
    #[test]
    fn preserves_selected_sampling_stream() {
        for seed in [0, 1, 7, 42, u64::MAX] {
            let mut actual = Pcg64::seed_from_u64(seed);
            let mut reference = actual.clone();
            for upper in [1, 2, 3, 8, 18, 32, 2000, usize::MAX] {
                for _ in 0..64 {
                    #[cfg(all(feature = "portable-seed-replay", target_pointer_width = "32"))]
                    let expected = reference.gen_range(0..upper as u64) as usize;
                    #[cfg(any(not(feature = "portable-seed-replay"), target_pointer_width = "64"))]
                    let expected = reference.gen_range(0..upper);
                    assert_eq!(index(&mut actual, upper), expected);
                    #[cfg(all(feature = "portable-seed-replay", target_pointer_width = "32"))]
                    let expected = reference.gen_range(0..=upper as u64) as usize;
                    #[cfg(any(not(feature = "portable-seed-replay"), target_pointer_width = "64"))]
                    let expected = reference.gen_range(0..=upper);
                    assert_eq!(index_inclusive(&mut actual, upper), expected);
                }
            }
            assert_eq!(actual.gen::<u64>(), reference.gen::<u64>());
        }
    }
}
