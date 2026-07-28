//! Randomized differential evidence: `prikk-hash`'s first-party SHA-256 against RustCrypto's
//! audited `sha2` crate, over a fixed-seed, stated-distribution input set (DC-41 stage 3).
//!
//! `sha2` is a dev-only dependency of this crate (`[dev-dependencies]` in `Cargo.toml`) and is
//! never reachable from `[dependencies]`. It is already present in the workspace's locked graph
//! as a transitive dependency of `ed25519-dalek` (for Ed25519's internal SHA-512), so this stage
//! adds a dependency *edge*, not a new package. The two implementations remain a genuine
//! independence check: `ed25519-dalek` uses `sha2`'s SHA-512, a different algorithm from the
//! SHA-256 exercised here, and their shared presence in one dependency graph does not correlate
//! their correctness. Adding this dev-dependency edge does not place `sha2` in `prikk-hash`'s
//! production dependency graph, which is what the RFC's discipline clause (differential
//! dependencies must not enter object identity or runtime trust paths) exists to prevent.
//!
//! A mismatch here is **not a bug to fix**. It would mean every ObjectId, state root, ref-name
//! path, and signature preimage computed from the disagreeing input is non-standard —
//! repository-format-invalidating, not an ordinary defect. Do not patch `sha256`, narrow the
//! distribution, or adjust the seed in response to a failure; stop and escalate to an
//! architect/security review with the reproducing seed and case index.

use sha2::{Digest, Sha256};

use super::super::sha256;

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

fn reference_sha256(input: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(input);
    hasher.finalize().into()
}

#[test]
fn sha256_matches_rustcrypto_reference_across_randomized_cases() {
    let mut rng = SplitMix64::new(SEED);
    for index in 0..CASE_COUNT {
        let length = length_for_case(&mut rng, index);
        let mut input = vec![0_u8; length];
        rng.fill_bytes(&mut input);

        let actual = sha256(&input);
        let expected = reference_sha256(&input);
        assert_eq!(
            actual, expected,
            "SHA-256 differential mismatch at case {index} (input length {length}, seed {SEED:#x}): \
             prikk-hash and RustCrypto sha2 disagree. This is a stop-work finding per DC-41's escalation \
             clause — do not patch sha256, narrow the distribution, or adjust the seed. Escalate to an \
             architect/security review with this seed and case index."
        );
    }
}
