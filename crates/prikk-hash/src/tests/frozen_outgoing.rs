//! The first-party SHA-256 implementation `prikk-hash` shipped before DC-55, frozen here
//! test-only as an independent differential reference.
//!
//! DC-50 decided to stop maintaining this implementation in production. DC-55 performed the swap
//! to `sha2` and used this frozen copy as the "outgoing" side of the equivalence campaign that
//! proved the swap changed no identity (see `tests::hash_differential`). Per DC-55 item 5, it is
//! kept afterward as the differential's permanent independent reference: genuinely independent of
//! `sha2`, already reviewed under DC-41, and immutable because nothing here is ever touched again.
//!
//! Do not "improve," refactor, optimize, or otherwise edit this module. Any change defeats the
//! reason it exists — it is a snapshot, not a maintained implementation.

const H0: [u32; 8] = [
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
];

const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/// Compute SHA-256 using the pre-DC-55 first-party implementation. Frozen; only call from
/// `tests::hash_differential`.
pub(super) fn sha256(input: &[u8]) -> [u8; 32] {
    let mut h = H0;
    let bit_len = (input.len() as u64).wrapping_mul(8);
    let mut chunks = Vec::with_capacity(input.len() + 72);
    chunks.extend_from_slice(input);
    chunks.push(0x80);
    while (chunks.len() % 64) != 56 {
        chunks.push(0);
    }
    chunks.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in chunks.chunks_exact(64) {
        compress(&mut h, chunk);
    }

    let mut out = [0_u8; 32];
    for (dst, word) in out.chunks_exact_mut(4).zip(h) {
        dst.copy_from_slice(&word.to_be_bytes());
    }
    out
}

fn compress(h: &mut [u32; 8], chunk: &[u8]) {
    let mut w = [0_u32; 64];
    for (slot, word_bytes) in w.iter_mut().take(16).zip(chunk.chunks_exact(4)) {
        let mut bytes = [0_u8; 4];
        bytes.copy_from_slice(word_bytes);
        *slot = u32::from_be_bytes(bytes);
    }

    for i in 16..64 {
        let s0 = small_sigma0(word_at(&w, i - 15));
        let s1 = small_sigma1(word_at(&w, i - 2));
        let value = word_at(&w, i - 16)
            .wrapping_add(s0)
            .wrapping_add(word_at(&w, i - 7))
            .wrapping_add(s1);
        if let Some(slot) = w.get_mut(i) {
            *slot = value;
        }
    }

    let [h0, h1, h2, h3, h4, h5, h6, h7] = *h;
    let mut a = h0;
    let mut b = h1;
    let mut c = h2;
    let mut d = h3;
    let mut e = h4;
    let mut f = h5;
    let mut g = h6;
    let mut hh = h7;

    for (constant, word) in K.iter().copied().zip(w.iter().copied()) {
        let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
        let ch = (e & f) ^ ((!e) & g);
        let temp1 = hh
            .wrapping_add(s1)
            .wrapping_add(ch)
            .wrapping_add(constant)
            .wrapping_add(word);
        let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
        let maj = (a & b) ^ (a & c) ^ (b & c);
        let temp2 = s0.wrapping_add(maj);

        hh = g;
        g = f;
        f = e;
        e = d.wrapping_add(temp1);
        d = c;
        c = b;
        b = a;
        a = temp1.wrapping_add(temp2);
    }

    *h = [
        h0.wrapping_add(a),
        h1.wrapping_add(b),
        h2.wrapping_add(c),
        h3.wrapping_add(d),
        h4.wrapping_add(e),
        h5.wrapping_add(f),
        h6.wrapping_add(g),
        h7.wrapping_add(hh),
    ];
}

fn word_at(words: &[u32; 64], index: usize) -> u32 {
    words.get(index).copied().unwrap_or(0)
}

fn small_sigma0(word: u32) -> u32 {
    word.rotate_right(7) ^ word.rotate_right(18) ^ (word >> 3)
}

fn small_sigma1(word: u32) -> u32 {
    word.rotate_right(17) ^ word.rotate_right(19) ^ (word >> 10)
}

#[cfg(test)]
mod tests {
    use super::sha256;

    /// Pin the frozen implementation against the same standard vectors `tests.rs` checks the
    /// current implementation against, so a slip while freezing this module would be caught here
    /// too rather than only by the differential.
    #[test]
    fn frozen_sha256_empty_matches_standard_vector() {
        let digest = sha256(b"");
        let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
        assert_eq!(
            hex,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
