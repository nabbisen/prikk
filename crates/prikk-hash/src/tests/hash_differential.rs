//! Randomized differential evidence that `prikk-hash::sha256` (backed by the `sha2` crate since
//! DC-55) still agrees with the first-party implementation it replaced, frozen test-only in
//! `tests::frozen_outgoing`.
//!
//! This module has served two purposes across two increments, over the same fixed-seed,
//! stated-distribution input set:
//!
//! - **DC-41 stage 3** first established that the (then-production) first-party implementation
//!   agreed with RustCrypto's `sha2` over 10,000 randomized cases — evidence for trusting the
//!   first-party code.
//! - **DC-55** repurposed the same generator and distribution to prove the opposite direction:
//!   that swapping `sha256`'s implementation *to* `sha2` produced no observable difference against
//!   the implementation it replaced. This is the equivalence campaign DC-55 item 1a requires, and
//!   the only run that demonstrates the swap preserved identity — comparing the new implementation
//!   against `sha2` directly would be a self-comparison and prove nothing, which is why the frozen
//!   module exists.
//!
//! Per DC-55 item 5, the frozen module is kept as this differential's **permanent** independent
//! reference rather than deleted or re-pointed at a third-party crate: it costs no new dependency,
//! remains genuinely independent of `sha2`, and was already reviewed under DC-41.
//!
//! A mismatch here is **not a bug to fix**. It would mean every ObjectId, state root, ref-name
//! path, and signature preimage computed from the disagreeing input is non-standard —
//! repository-format-invalidating, not an ordinary defect. Do not patch either side, narrow the
//! distribution, or adjust the seed in response to a failure; stop and escalate to an
//! architect/security review with the reproducing seed and case index.

use super::super::sha256;
use super::frozen_outgoing;

/// Fixed seed: the leading fractional bits of pi (a nothing-up-my-sleeve constant), so the
/// generated input set is reproducible by construction rather than by discipline.
const SEED: u64 = 0x243F_6A88_85A3_08D3;

/// Cases per run. The RFC's stage-3 bar is "at least 10,000 randomized cases per CI run."
const CASE_COUNT: usize = 10_000;

/// A minimal SplitMix64 generator. No dependency is added for this: `rand` is absent from the
/// lockfile and would introduce new packages for a single test module, and `rand_core` (already
/// locked) provides only traits, not a seedable generator.
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

    /// Fill `buf` with generator output, eight bytes per draw.
    fn fill_bytes(&mut self, buf: &mut [u8]) {
        let mut chunks = buf.chunks_exact_mut(8);
        for chunk in &mut chunks {
            chunk.copy_from_slice(&self.next_u64().to_le_bytes());
        }
        let remainder = chunks.into_remainder();
        if !remainder.is_empty() {
            let extra = self.next_u64().to_le_bytes();
            for (dst, byte) in remainder.iter_mut().zip(extra) {
                *dst = byte;
            }
        }
    }
}

#[test]
fn split_mix64_matches_published_self_check_sequence() {
    let mut rng = SplitMix64::new(SEED);
    let expected: [u64; 6] = [
        0x2cb0_f69f_4abe_a221,
        0x9417_0347_2314_8989,
        0xdd55_5950_609d_fe03,
        0xdbaf_b150_deb1_2800,
        0x7e78_9b2e_6c44_2cb6,
        0xf41e_5636_c7e4_f8c4,
    ];
    for value in expected {
        assert_eq!(
            rng.next_u64(),
            value,
            "SplitMix64 self-check diverged from the documented reference sequence"
        );
    }
}

/// Draw an input length for case `index`, from the stated distribution:
///
/// | Band | Lengths | Share |
/// |---|---|---|
/// | Empty | 0 | exactly 1 case (index 0), guaranteed rather than probabilistic |
/// | Sub-block | 1-54 | ~25% |
/// | First-boundary neighbourhood | 55-57, 63-65 | ~25% |
/// | Multi-block | 66-1024 | ~25% |
/// | Later-boundary neighbourhood | 119-121, 127-129, 183-185 | ~25% |
fn length_for_case(rng: &mut SplitMix64, index: usize) -> usize {
    if index == 0 {
        return 0;
    }
    match rng.next_u64() % 4 {
        0 => 1 + (rng.next_u64() as usize % 54),
        1 => {
            const FIRST_BOUNDARY: [usize; 6] = [55, 56, 57, 63, 64, 65];
            let index = rng.next_u64() as usize % FIRST_BOUNDARY.len();
            FIRST_BOUNDARY.get(index).copied().unwrap_or(56)
        }
        2 => 66 + (rng.next_u64() as usize % (1024 - 66 + 1)),
        _ => {
            const LATER_BOUNDARY: [usize; 9] = [119, 120, 121, 127, 128, 129, 183, 184, 185];
            let index = rng.next_u64() as usize % LATER_BOUNDARY.len();
            LATER_BOUNDARY.get(index).copied().unwrap_or(119)
        }
    }
}

#[test]
fn sha256_matches_frozen_pre_dc55_implementation_across_randomized_cases() {
    let mut rng = SplitMix64::new(SEED);
    for index in 0..CASE_COUNT {
        let length = length_for_case(&mut rng, index);
        let mut input = vec![0_u8; length];
        rng.fill_bytes(&mut input);

        let actual = sha256(&input);
        let expected = frozen_outgoing::sha256(&input);
        assert_eq!(
            actual, expected,
            "SHA-256 differential mismatch at case {index} (input length {length}, seed {SEED:#x}): \
             the sha2-backed implementation and the frozen pre-DC-55 first-party implementation \
             disagree. This is a stop-work finding per DC-41's escalation clause — do not patch \
             either side, narrow the distribution, or adjust the seed. Escalate to an \
             architect/security review with this seed and case index."
        );
    }
}
