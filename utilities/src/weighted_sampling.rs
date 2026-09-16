use std::cmp::Ordering;

/// A candidate in a validator election, carrying its voting power (stake).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Participant<Id> {
    pub id: Id,
    pub voting_power: u64,
}

impl<Id> Participant<Id> {
    pub fn new(id: Id, voting_power: u64) -> Self {
        Self { id, voting_power }
    }
}

/// Deterministic 64-bit PRNG (SplitMix64).
///
/// **Note:** For illustration/reproducibility only. A real consensus system
/// should seed a *cryptographically secure* stream cipher (e.g. ChaCha20)
/// from the VRF / block hash, because an adversary who can predict the stream
/// can grind the seed to bias the committee. See the "Production notes" below.
struct SplitMix64(u64);

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform sample in the *open* interval `(0, 1)` — never exactly 0 or 1.
    fn next_f64_open(&mut self) -> f64 {
        // 53 bits of entropy → x ∈ [1, 2^53 - 1], then divide by 2^53.
        loop {
            let x = self.next_u64() >> 11; // [0, 2^53)
            if x != 0 {
                return (x as f64) / ((1u64 << 53) as f64);
            }
        }
    }
}

/// Mix the full 32-byte hash/seed into a single u64 to seed the PRNG.
fn seed_to_u64(seed: &[u8; 32]) -> u64 {
    let mut h: u64 = 0x9E37_79B9_7F4A_7C15;
    for chunk in seed.chunks_exact(8) {
        let mut b = [0u8; 8];
        b.copy_from_slice(chunk);
        h = (h ^ u64::from_le_bytes(b)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        h ^= h >> 29;
    }
    h
}

/// Elect a committee of up to `committee_size` participants, weighted by
/// their `voting_power`, using `seed` (typically a block hash or VRF output).
///
/// * Participants with `voting_power == 0` are never elected.
/// * If `committee_size` exceeds the number of eligible participants, all
///   eligible participants are returned.
/// * The returned order is the natural E-S order: the first element is the
///   one most likely to have been chosen first — useful for leader election.
///   Sort the result yourself if you need a canonical order.
pub fn elect<Id: Clone>(
    participants: &[Participant<Id>],
    committee_size: usize,
    seed: &[u8; 32],
) -> Vec<Id> {
    if committee_size == 0 || participants.is_empty() {
        return Vec::new();
    }

    let mut rng = SplitMix64::new(seed_to_u64(seed));

    // Efraimidis–Spirakis A-ES: assign each participant a key
    //     key_i = u_i^(1 / w_i),   u_i ~ Uniform(0, 1)
    // and pick the committee_size largest keys. Working in log space:
    //     ln(key_i) = ln(u_i) / w_i
    // Since ln is monotonic, ranking by ln(key) is the same as ranking by key.
    //
    // Equivalently (via u = e^{-X}, X ~ Exp(1)): each participant draws
    // Y_i ~ Exp(w_i) and the smallest Y wins. This is why the FIRST slot
    // is chosen with probability *exactly* w_i / Σw.
    let mut keyed: Vec<(f64, &Id)> = participants
        .iter()
        .filter(|p| p.voting_power > 0)
        .map(|p| {
            let u = rng.next_f64_open();
            let w = p.voting_power as f64;
            (u.ln() / w, &p.id)
        })
        .collect();

    // Descending: largest key (closest to 0) wins.
    keyed.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(Ordering::Equal));

    keyed.into_iter().take(committee_size).map(|(_, id)| id.clone()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Deterministically derive a well-mixed 32-byte seed from a counter.
    fn seed_from(n: u64) -> [u8; 32] {
        let mut rng = SplitMix64::new(n ^ 0xDEAD_BEEF_CAFE_BABE);
        let mut seed = [0u8; 32];
        for chunk in seed.chunks_exact_mut(8) {
            chunk.copy_from_slice(&rng.next_u64().to_le_bytes());
        }
        seed
    }

    fn participants(powers: &[u64]) -> Vec<Participant<usize>> {
        powers.iter().enumerate().map(|(i, &w)| Participant::new(i, w)).collect()
    }

    // ---------- Determinism / basic correctness ----------

    #[test]
    fn same_seed_gives_same_committee() {
        let ps = participants(&[1, 2, 3, 4, 5]);
        let seed = seed_from(42);
        let a = elect(&ps, 3, &seed);
        let b = elect(&ps, 3, &seed);
        assert_eq!(a, b);
    }

    #[test]
    fn different_seeds_usually_differ() {
        let ps = participants(&[1, 1, 1, 1, 1, 1, 1, 1, 1, 1]);
        let mut seen = HashSet::new();
        for s in 0..50u64 {
            let c = elect(&ps, 3, &seed_from(s));
            seen.insert(c);
        }
        // With 10 candidates choose 3 there are 120 possible committees;
        // 50 random draws should give us well over a handful of distinct ones.
        assert!(seen.len() > 20, "only {} distinct committees", seen.len());
    }

    #[test]
    fn committee_size_is_respected() {
        let ps = participants(&[1, 2, 3, 4, 5]);
        for &k in &[0usize, 1, 2, 3, 4, 5] {
            let c = elect(&ps, k, &seed_from(7));
            assert_eq!(c.len(), k);
        }
    }

    #[test]
    fn no_duplicates_in_committee() {
        let ps = participants(&[5, 5, 5, 5, 5, 5]);
        for s in 0..100u64 {
            let c = elect(&ps, 4, &seed_from(s));
            let unique: HashSet<_> = c.iter().collect();
            assert_eq!(unique.len(), c.len());
        }
    }

    // ---------- Edge cases ----------

    #[test]
    fn empty_participants_returns_empty() {
        let ps: Vec<Participant<usize>> = vec![];
        assert!(elect(&ps, 5, &seed_from(0)).is_empty());
    }

    #[test]
    fn zero_committee_size_returns_empty() {
        let ps = participants(&[1, 2, 3]);
        assert!(elect(&ps, 0, &seed_from(0)).is_empty());
    }

    #[test]
    fn zero_power_participants_are_never_elected() {
        // Participant 2 has zero power and must never appear.
        let ps = vec![
            Participant::new(0usize, 10),
            Participant::new(1usize, 10),
            Participant::new(2usize, 0),
            Participant::new(3usize, 10),
        ];
        for s in 0..500u64 {
            let c = elect(&ps, 3, &seed_from(s));
            assert!(!c.contains(&2), "zero-power participant elected at seed {s}");
        }
    }

    #[test]
    fn committee_larger_than_eligible_returns_all_eligible() {
        let ps = vec![
            Participant::new(0usize, 5),
            Participant::new(1usize, 5),
            Participant::new(2usize, 0), // ineligible
        ];
        let c = elect(&ps, 100, &seed_from(0));
        assert_eq!(c.len(), 2);
        let set: HashSet<_> = c.iter().copied().collect();
        assert!(set.contains(&0) && set.contains(&1));
    }

    #[test]
    fn single_participant_always_elected() {
        let ps = vec![Participant::new(7usize, 1)];
        for s in 0..100u64 {
            assert_eq!(elect(&ps, 1, &seed_from(s)), vec![7]);
        }
    }

    #[test]
    fn committee_equal_to_population_is_a_permutation() {
        let ps = participants(&[1, 2, 3, 4, 5]);
        for s in 0..100u64 {
            let mut c = elect(&ps, 5, &seed_from(s));
            c.sort();
            assert_eq!(c, vec![0, 1, 2, 3, 4]);
        }
    }

    // ---------- Statistical fairness ----------

    /// The FIRST slot of the E-S output is chosen with probability *exactly*
    /// w_i / Σw. This test verifies that empirically over many seeds.
    #[test]
    fn first_slot_is_proportional_to_power() {
        let powers = [1u64, 2, 3, 4];
        let total: u64 = powers.iter().sum(); // 10
        let ps = participants(&powers);

        let trials: u64 = 50_000;
        let mut counts = [0usize; 4];

        for s in 0..trials {
            let c = elect(&ps, 1, &seed_from(s));
            counts[c[0]] += 1;
        }

        for (i, &w) in powers.iter().enumerate() {
            let expected = trials as f64 * (w as f64 / total as f64);
            let actual = counts[i] as f64;
            let rel_err = (actual - expected).abs() / expected;
            assert!(
                rel_err < 0.10,
                "participant {i} (w={w}): expected {expected:.0}, got {actual:.0}, rel_err {rel_err:.3}"
            );
        }
    }

    /// Sanity: with overwhelming stake, a participant should be elected
    /// almost every time when committee_size = 1.
    #[test]
    fn whale_dominates_first_slot() {
        let ps = vec![
            Participant::new("whale", 1_000_000u64),
            Participant::new("minnow_a", 1u64),
            Participant::new("minnow_b", 1u64),
            Participant::new("minnow_c", 1u64),
        ];
        let mut whale_wins = 0;
        let trials = 1_000u64;
        for s in 0..trials {
            let c = elect(&ps, 1, &seed_from(s));
            if c[0] == "whale" {
                whale_wins += 1;
            }
        }
        // Expected ≈ 1_000_000 / 1_000_003 ≈ 99.9997 %.
        assert!(whale_wins >= 999, "whale won only {whale_wins}/{trials}");
    }

    /// Over many draws, a higher-power participant should be elected to a
    /// multi-seat committee more often than a lower-power one.
    #[test]
    fn higher_power_elected_more_often_in_committee() {
        let ps = participants(&[1, 2, 3, 4, 5]); // total 15
        let trials = 20_000u64;
        let mut counts = [0usize; 5];

        for s in 0..trials {
            let c = elect(&ps, 3, &seed_from(s));
            for id in c {
                counts[id] += 1;
            }
        }

        // Monotonicity: counts should be non-decreasing with power.
        for i in 1..counts.len() {
            assert!(counts[i] >= counts[i - 1], "counts = {counts:?} is not monotonic");
        }
        // And meaningfully different between extremes.
        assert!(counts[4] > counts[0] * 2, "top vs bottom ratio too small: {counts:?}");
    }

    /// A zero-power whale must still be excluded even if all others are tiny.
    #[test]
    fn zero_power_whale_is_excluded() {
        let ps = vec![
            Participant::new("big_but_zero", 000_000u64),
            Participant::new("real_a", 1u64),
            Participant::new("real_b", 1u64),
        ];
        for s in 0..500u64 {
            let c = elect(&ps, 2, &seed_from(s));
            assert!(!c.contains(&"big_but_zero"));
        }
    }

    // ---------- PRNG sanity ----------

    #[test]
    fn next_f64_open_is_strictly_inside_unit_interval() {
        let mut rng = SplitMix64::new(12345);
        for _ in 0..100_000 {
            let x = rng.next_f64_open();
            assert!(x > 0.0 && x < 1.0, "x = {x}");
        }
    }
}
