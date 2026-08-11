//! DC-92 lineage-replay benchmark harness.
//!
//! Measures wall-clock cost of `prikk verify` and `prikk seal` against **sealed-history length**
//! (block count), isolated from repository (tree) size — the axis DC-59/62/64's own harness never
//! varies, and DC-69's Axis D isolates for `commit` but not for `verify`/`seal`. This file answers
//! DC-92 handoff §2 (reproduce the O(N³) `verify` baseline) and §4.1 (measure `seal` against the same
//! axis to confirm or refute the O(N²)-per-seal hypothesis). It decides nothing about the fix; DC-92's
//! own report reads what this produces.
//!
//! `#[ignore]`d by default, matching `dc59_commit_benchmark.rs`'s precedent: a measurement
//! instrument, not a correctness test, and its dominant cost (growing a 160-block sealed history,
//! current-hypothesis O(N³) to do so if seal is itself O(N²) per step) does not belong in the default
//! suite.
//!
//! **One growing repository, not six independent ones.** DC-59's axes each build a fresh repository
//! per sample point. That would mean rebuilding history from scratch at N=5, again at N=10, ...,
//! again at N=160 — redundant work, since building to 160 already passes through every smaller
//! checkpoint. Instead: one repository is grown generation by generation to depth 160, `verify` is
//! timed non-destructively (read-only) at each checkpoint depth along the way, and **every** seal
//! call's duration is recorded as it happens, not only at checkpoints — giving the seal axis a full
//! curve at no extra cost.
//!
//! **Tree size held fixed via churn**, reusing DC-69 Axis D's exact technique (delete the oldest
//! tracked file, create one new file at a fresh path, each generation) rather than editing existing
//! files — editing the same text file across two separate commits is a pre-existing, independently
//! reported defect (`dc59_commit_benchmark.rs`'s own module doc), and churn avoids it while keeping
//! live tree size constant, which is what isolates history length from repository size.
//!
//! **Reduced sample count, stated rather than hidden.** Growing one repository to depth 160 is
//! expensive on its own (a `verify` call at depth 160 alone measured 34.2 s in the finding this
//! increment investigates, and this harness makes that many more calls building up to it). This file
//! uses `SAMPLES` independent full growth runs, deliberately fewer than DC-59's harness uses per
//! point — the report states the exact count and this module doc explains why, rather than silently
//! presenting a thin sample as if it were DC-59's depth of rigor.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::indexing_slicing)]

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

mod support;
use support::{init, seal, unique_repo, verify};

const REF_NAME: &str = "heads/main";

/// Independent full-growth-to-160 runs. See module doc for why this is small.
const SAMPLES: usize = 2;

/// Sealed-block-count checkpoints `verify` is timed at — the finding's own N values.
const N_VALUES: [usize; 6] = [5, 10, 20, 40, 80, 160];

/// Deepest checkpoint; the growth loop runs to this depth once per sample.
const MAX_DEPTH: usize = 160;

/// Live tree size held fixed throughout growth via churn (DC-69 Axis D's technique).
const TREE_SIZE: usize = 10;

const FILE_SIZE_BYTES: usize = 64;

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

fn random_content(rng: &mut SplitMix64) -> Vec<u8> {
    let mut content = vec![0_u8; FILE_SIZE_BYTES];
    rng.fill_bytes(&mut content);
    for byte in &mut content {
        *byte = b'a' + (*byte % 26);
    }
    content
}

/// Genesis: `TREE_SIZE` files, one commit, one seal -- sealed-block count 1 after this returns.
/// Untimed; this is setup, not a measured point (matching DC-59's own convention).
fn build_genesis(root: &Path, seed: u64) -> Vec<PathBuf> {
    init(root);
    let mut rng = SplitMix64::new(seed);
    let mut files = Vec::with_capacity(TREE_SIZE);
    for index in 0..TREE_SIZE {
        let path = PathBuf::from(format!("f{index}.txt"));
        std::fs::write(root.join(&path), random_content(&mut rng)).unwrap();
        files.push(path);
    }
    let commit_output = support::commit(root, REF_NAME, "dc92-bench: genesis");
    support::ok(&commit_output, "genesis commit");
    let seal_output = seal(root, REF_NAME);
    support::ok(&seal_output, "genesis seal");
    files
}

/// One churn generation: delete the oldest tracked file, create a new one at a fresh path, commit
/// (untimed), then **time** the seal that produces the next sealed block. Live tree size is
/// unchanged (still `TREE_SIZE`) before and after every call.
fn churn_generation_timed(
    root: &Path,
    files: &mut Vec<PathBuf>,
    next_index: &mut usize,
    rng: &mut SplitMix64,
) -> Duration {
    let victim = files.remove(0);
    std::fs::remove_file(root.join(&victim)).unwrap();
    let new_path = PathBuf::from(format!("churn-{next_index}.txt"));
    *next_index += 1;
    std::fs::write(root.join(&new_path), random_content(rng)).unwrap();
    files.push(new_path);

    let commit_output = support::commit(root, REF_NAME, "dc92-bench: churn");
    support::ok(&commit_output, "churn commit");

    let start = Instant::now();
    let seal_output = seal(root, REF_NAME);
    let elapsed = start.elapsed();
    support::ok(&seal_output, "churn seal");
    elapsed
}

/// Time one `verify` invocation against the repository's current state. Read-only, so this does not
/// disturb the growing history -- the same repository continues to the next checkpoint afterward.
fn time_verify(root: &Path) -> Duration {
    let start = Instant::now();
    let output = verify(root);
    let elapsed = start.elapsed();
    support::ok(&output, "measured verify");
    elapsed
}

struct SampleRun {
    /// `seal_by_depth[i]` is the duration of the seal that produced sealed block `i + 2` (block 1 is
    /// genesis, untimed; the first timed seal produces block 2).
    seal_by_depth: Vec<Duration>,
    /// `verify_by_n[k]` is the duration of the `verify` call at sealed-block count `N_VALUES[k]`.
    verify_by_n: Vec<Duration>,
}

fn run_one_sample(sample_index: usize) -> SampleRun {
    let root = unique_repo(&format!("dc92-lineage-{sample_index}"));
    let seed = 0xDC92_0000_0000_0000_u64
        .wrapping_add(sample_index as u64)
        .rotate_left(7);
    let mut files = build_genesis(&root, seed);
    let mut rng = SplitMix64::new(seed ^ 0xFFFF_FFFF_0000_0000);
    let mut next_index = TREE_SIZE;

    let mut seal_by_depth = Vec::with_capacity(MAX_DEPTH - 1);
    let mut verify_by_n = Vec::with_capacity(N_VALUES.len());

    // Depth 1 (genesis) already exists, untimed. Grow to MAX_DEPTH, timing every seal and checking
    // verify at each N_VALUES checkpoint as depth reaches it.
    for depth in 2..=MAX_DEPTH {
        let elapsed = churn_generation_timed(&root, &mut files, &mut next_index, &mut rng);
        seal_by_depth.push(elapsed);
        if N_VALUES.contains(&depth) {
            verify_by_n.push(time_verify(&root));
        }
    }

    let _ = std::fs::remove_dir_all(&root);
    SampleRun {
        seal_by_depth,
        verify_by_n,
    }
}

fn fmt_ms(duration: Duration) -> String {
    format!("{:.2}", duration.as_secs_f64() * 1000.0)
}

fn median(durations: &[Duration]) -> Duration {
    let mut sorted = durations.to_vec();
    sorted.sort();
    sorted[sorted.len() / 2]
}

#[test]
#[ignore = "long-running measurement instrument; run deliberately, see module docs"]
fn lineage_replay_benchmark() {
    let mut runs = Vec::with_capacity(SAMPLES);
    for sample_index in 0..SAMPLES {
        eprintln!(
            "sample {}/{SAMPLES}: growing to depth {MAX_DEPTH}...",
            sample_index + 1
        );
        runs.push(run_one_sample(sample_index));
    }

    let mut out = String::new();
    out.push_str("# DC-92 Lineage Replay Benchmark Report v1\n\n");
    out.push_str(&format!(
        "Generated by `cargo test -p prikk --locked --test dc92_lineage_replay_benchmark -- --ignored --nocapture lineage_replay_benchmark`. {SAMPLES} independent full-growth-to-{MAX_DEPTH} runs (see module doc for why this is fewer than DC-59's per-point sample counts). Tree size held fixed at {TREE_SIZE} files throughout via churn.\n\n"
    ));

    out.push_str("## verify — cost against sealed-block count\n\n");
    out.push_str("| Sealed blocks (N) | Samples | Median (ms) |\n");
    out.push_str("|---:|---:|---:|\n");
    for (index, &n) in N_VALUES.iter().enumerate() {
        let samples: Vec<Duration> = runs.iter().map(|run| run.verify_by_n[index]).collect();
        out.push_str(&format!(
            "| {n} | {} | {} |\n",
            samples.len(),
            fmt_ms(median(&samples))
        ));
    }
    out.push('\n');

    out.push_str("## seal — cost against sealed-block count already present\n\n");
    out.push_str(
        "Each row is the seal that produced the block at the stated position, i.e. the ancestor \
         chain already sealed *before* this call. `N=2` means the seal producing the second block, \
         with 1 ancestor already sealed.\n\n",
    );
    out.push_str("| Sealed blocks (N, after this seal) | Samples | Median (ms) |\n");
    out.push_str("|---:|---:|---:|\n");
    for &n in &N_VALUES {
        if n < 2 {
            continue;
        }
        let index = n - 2;
        let samples: Vec<Duration> = runs
            .iter()
            .filter_map(|run| run.seal_by_depth.get(index).copied())
            .collect();
        if samples.is_empty() {
            continue;
        }
        out.push_str(&format!(
            "| {n} | {} | {} |\n",
            samples.len(),
            fmt_ms(median(&samples))
        ));
    }
    out.push('\n');
    out.push_str(
        "**Full per-depth seal curve** (every generation, not only checkpoints), first sample run \
         only, for inspecting the shape between checkpoints:\n\n",
    );
    out.push_str("| Sealed blocks (N) | Duration (ms) |\n");
    out.push_str("|---:|---:|\n");
    if let Some(first) = runs.first() {
        for (index, duration) in first.seal_by_depth.iter().enumerate() {
            let n = index + 2;
            if n % 10 == 0 || n == 2 || n == MAX_DEPTH {
                out.push_str(&format!("| {n} | {} |\n", fmt_ms(*duration)));
            }
        }
    }
    out.push('\n');

    let report_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../rfcs/handoffs/DC-92-lineage-replay-memoization/benchmark-report-v1.md"
    );
    std::fs::write(report_path, &out).unwrap();
    eprintln!("report written to {report_path}");
}
