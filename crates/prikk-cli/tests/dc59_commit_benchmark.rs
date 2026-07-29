//! DC-59 commit benchmark harness.
//!
//! Measures wall-clock cost of `prikk commit` across two axes, producing the evidence artifact
//! NFR-PERF-01 names. This increment **decides nothing** — DC-56 reads the report this test
//! produces and rules on compliance.
//!
//! `#[ignore]`d by default: it is a measurement instrument, not a correctness test, and its
//! dominant cost (generating up to 10,000-file repositories, several samples per point) does not
//! belong in the default suite. Run it deliberately with `--ignored` when a performance question is
//! open. Being excluded from routine runs means it can go stale between uses; that is accepted
//! deliberately rather than a maintenance oversight (see the RFC's Risks section).
//!
//! No new dev-dependency: `prikk-store` is already a normal dependency of this crate, reused here
//! only for `Ed25519MaintainerSigner` to derive the fixed benchmark maintainer key.
//!
//! `SplitMix64` below is a **deliberate duplicate** of the one in
//! `crates/prikk-hash/src/tests/hash_differential.rs`. That generator is a private struct in a
//! `#[cfg(test)]` module of a different crate and is not reachable from here; promoting it would
//! touch `prikk-hash` test material DC-55 froze on purpose. This is a second, independent copy of a
//! reviewed generator, not a new invented one.
//!
//! ## Why two commits per generated repository, not one
//!
//! A genesis commit (against an empty baseline) necessarily authors every file in the worktree —
//! there is no way to hold "repository size" and "changed-file count" independent on a first
//! commit, since every present file *is* the change. Both axes therefore need an established
//! baseline: generate the full repository, commit it (genesis), and seal it — all **untimed** setup
//! — then mutate exactly the files under test and time only the second `commit`, which must
//! reconcile the worktree against a real baseline of the target size. This is also why the fixed
//! maintainer key exists: only the untimed setup seal needs it, not the timed measurement itself.
//!
//! `commit` cannot run twice against one repository without a seal between attempts
//! (`node_authoring.rs`'s active-WAL guard). The one seal used here happens entirely inside setup,
//! before the timing window opens, and is not "sealing between trials" in the sense the design
//! forbids — there is exactly one timed trial per generated repository, never two.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::indexing_slicing)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

use prikk_store::{Ed25519MaintainerSigner, MaintainerSigner};

const FIXED_AUTHOR_KEY_ID: &str = "dc59-bench-author";
const FIXED_AUTHOR_SEED: [u8; 32] = [
    0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff,
    0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff,
];
const FIXED_MAINTAINER_KEY_ID: &str = "dc59-bench-maintainer";
const FIXED_MAINTAINER_SEED: [u8; 32] = [
    0x21, 0x21, 0x32, 0x32, 0x43, 0x43, 0x54, 0x54, 0x65, 0x65, 0x76, 0x76, 0x87, 0x87, 0x98, 0x98,
    0xa9, 0xa9, 0xba, 0xba, 0xcb, 0xcb, 0xdc, 0xdc, 0xed, 0xed, 0xfe, 0xfe, 0x0f, 0x0f, 0x10, 0x10,
];
/// Deterministic content seed. Distinct from DC-41/DC-55's constant — this generator produces
/// worktree file content, not hash-differential inputs, and the two should not be confused as the
/// same domain.
const CONTENT_SEED: u64 = 0x1234_5678_9abc_def0;

/// File sizes and tree shape held constant across both axes.
const FILE_SIZE_BYTES: usize = 256;
const TREE_BREADTH: usize = 8;
const TREE_DEPTH: usize = 3;

/// Axis A: repository size (file count), held at 1 changed file.
const AXIS_A_SIZES: [usize; 4] = [10, 100, 1_000, 10_000];
const AXIS_A_SAMPLES: [usize; 4] = [5, 5, 5, 3];

/// Axis B: changed-file count, held at a fixed repository size.
const AXIS_B_REPO_SIZE: usize = 1_000;
const AXIS_B_CHANGE_COUNTS: [usize; 4] = [1, 10, 100, 1_000];
const AXIS_B_SAMPLES: usize = 5;

const SPAWN_FLOOR_SAMPLES: usize = 10;

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

fn prikk(repo: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_prikk"));
    cmd.current_dir(repo);
    cmd
}

fn ok(output: &Output, what: &str) {
    assert!(
        output.status.success(),
        "{what} failed (status {:?})\nstdout: {}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn maintainer_public_key_hex() -> String {
    let signer =
        Ed25519MaintainerSigner::from_seed(FIXED_MAINTAINER_KEY_ID, &FIXED_MAINTAINER_SEED)
            .expect("fixed maintainer seed derives a valid signer");
    hex(&signer.public_key_bytes())
}

fn unique_dir(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "prikk-dc59-bench-{tag}-{}-{nanos}",
        std::process::id()
    ));
    dir
}

/// Deterministically populate `root` with `file_count` files across a directory tree of the given
/// breadth and depth (files distributed round-robin across leaf directories, so the tree is
/// genuinely traversed rather than everything landing in one directory), each `file_size` bytes of
/// printable pseudo-random content so worktree edits later exercise the text-edit path. Returns the
/// created files' paths relative to `root`, in creation order.
fn generate_tree(
    root: &Path,
    file_count: usize,
    breadth: usize,
    depth: usize,
    file_size: usize,
    rng: &mut SplitMix64,
) -> Vec<PathBuf> {
    let mut leaf_dirs = Vec::new();
    build_leaf_dirs(PathBuf::new(), 0, depth, breadth, &mut leaf_dirs);
    if leaf_dirs.len() > file_count.max(1) {
        leaf_dirs.truncate(file_count.max(1));
    }
    for dir in &leaf_dirs {
        if !dir.as_os_str().is_empty() {
            std::fs::create_dir_all(root.join(dir)).unwrap();
        }
    }

    let mut paths = Vec::with_capacity(file_count);
    for index in 0..file_count {
        let dir = &leaf_dirs[index % leaf_dirs.len()];
        let relative = dir.join(format!("f{index}.txt"));
        let mut content = vec![0_u8; file_size];
        rng.fill_bytes(&mut content);
        for byte in &mut content {
            *byte = b'a' + (*byte % 26);
        }
        std::fs::write(root.join(&relative), &content).unwrap();
        paths.push(relative);
    }
    paths
}

fn build_leaf_dirs(
    prefix: PathBuf,
    level: usize,
    depth: usize,
    breadth: usize,
    out: &mut Vec<PathBuf>,
) {
    if level == depth {
        out.push(prefix);
        return;
    }
    for index in 0..breadth {
        build_leaf_dirs(
            prefix.join(format!("d{index}")),
            level + 1,
            depth,
            breadth,
            out,
        );
    }
}

/// Untimed setup: init, generate `file_count` files, genesis-commit them, trust a fixed maintainer
/// key, and seal — establishing a real baseline of the target size before the timing window opens.
/// Returns the generated files' paths for the caller to mutate.
fn setup_baseline_repository(root: &Path, file_count: usize, seed: u64) -> Vec<PathBuf> {
    std::fs::create_dir_all(root).unwrap();
    ok(&prikk(root).arg("init").output().unwrap(), "init");

    let mut rng = SplitMix64::new(seed);
    let files = generate_tree(
        root,
        file_count,
        TREE_BREADTH,
        TREE_DEPTH,
        FILE_SIZE_BYTES,
        &mut rng,
    );

    let out = prikk(root)
        .env("PRIKK_AUTHOR_KEY_ID", FIXED_AUTHOR_KEY_ID)
        .env("PRIKK_AUTHOR_SEED", hex(&FIXED_AUTHOR_SEED))
        .args(["commit", "-m", "dc59-bench: baseline"])
        .output()
        .unwrap();
    ok(&out, "baseline commit");

    let out = prikk(root)
        .args([
            "trust",
            "maintainer",
            "add",
            "--key-id",
            FIXED_MAINTAINER_KEY_ID,
            "--public-key",
            &maintainer_public_key_hex(),
        ])
        .output()
        .unwrap();
    ok(&out, "trust maintainer add");

    let out = prikk(root)
        .env("PRIKK_MAINTAINER_KEY_ID", FIXED_MAINTAINER_KEY_ID)
        .env("PRIKK_MAINTAINER_SEED", hex(&FIXED_MAINTAINER_SEED))
        .args(["seal", "--allow-no-audit"])
        .output()
        .unwrap();
    ok(&out, "baseline seal");

    files
}

/// Mutate the content of the first `count` files in `files` (deterministically, via `rng`),
/// producing text edits against the sealed baseline.
fn mutate_files(root: &Path, files: &[PathBuf], count: usize, rng: &mut SplitMix64) {
    for path in files.iter().take(count) {
        let mut content = std::fs::read(root.join(path)).unwrap();
        content.push(b'\n');
        let mut extra = [0_u8; 16];
        rng.fill_bytes(&mut extra);
        for byte in &mut extra {
            *byte = b'a' + (*byte % 26);
        }
        content.extend_from_slice(&extra);
        std::fs::write(root.join(path), content).unwrap();
    }
}

/// Time the one measured `commit` against an already-baselined, already-mutated repository.
fn time_commit(root: &Path) -> Duration {
    let start = Instant::now();
    let out = prikk(root)
        .env("PRIKK_AUTHOR_KEY_ID", FIXED_AUTHOR_KEY_ID)
        .env("PRIKK_AUTHOR_SEED", hex(&FIXED_AUTHOR_SEED))
        .args(["commit", "-m", "dc59-bench: measured"])
        .output()
        .unwrap();
    let elapsed = start.elapsed();
    ok(&out, "measured commit");
    elapsed
}

fn spawn_floor_sample() -> Duration {
    let start = Instant::now();
    let out = Command::new(env!("CARGO_BIN_EXE_prikk"))
        .arg("--version")
        .output()
        .unwrap();
    let elapsed = start.elapsed();
    assert!(out.status.success());
    elapsed
}

struct Point {
    label: String,
    samples: Vec<Duration>,
}

impl Point {
    fn median(&self) -> Duration {
        let mut sorted = self.samples.clone();
        sorted.sort();
        sorted[sorted.len() / 2]
    }

    fn min(&self) -> Duration {
        *self.samples.iter().min().unwrap()
    }

    fn max(&self) -> Duration {
        *self.samples.iter().max().unwrap()
    }
}

fn fmt_ms(duration: Duration) -> String {
    format!("{:.2}", duration.as_secs_f64() * 1000.0)
}

fn filesystem_kind(path: &Path) -> String {
    let Ok(output) = Command::new("df").arg("-T").arg(path).output() else {
        return "unknown (df unavailable)".to_owned();
    };
    if !output.status.success() {
        return "unknown (df failed)".to_owned();
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .nth(1)
        .and_then(|line| line.split_whitespace().nth(1))
        .map(str::to_owned)
        .unwrap_or_else(|| "unknown (unparsed df output)".to_owned())
}

fn cpu_model() -> String {
    std::fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|text| {
            text.lines()
                .find(|line| line.starts_with("model name"))
                .and_then(|line| line.split(':').nth(1))
                .map(|value| value.trim().to_owned())
        })
        .unwrap_or_else(|| "unknown (non-Linux or /proc/cpuinfo unavailable)".to_owned())
}

#[test]
#[ignore = "long-running measurement instrument; run deliberately, see module docs"]
fn commit_benchmark() {
    let temp_root = std::env::temp_dir();
    eprintln!(
        "filesystem under {}: {}",
        temp_root.display(),
        filesystem_kind(&temp_root)
    );

    let mut spawn_floor = Vec::with_capacity(SPAWN_FLOOR_SAMPLES);
    for _ in 0..SPAWN_FLOOR_SAMPLES {
        spawn_floor.push(spawn_floor_sample());
    }
    let spawn_floor_point = Point {
        label: "spawn floor".to_owned(),
        samples: spawn_floor,
    };

    let mut axis_a = Vec::new();
    for (size, sample_count) in AXIS_A_SIZES.into_iter().zip(AXIS_A_SAMPLES) {
        let mut samples = Vec::with_capacity(sample_count);
        for sample_index in 0..sample_count {
            let root = unique_dir(&format!("axis-a-{size}-{sample_index}"));
            let seed = CONTENT_SEED
                .wrapping_add(size as u64)
                .wrapping_add(sample_index as u64);
            let files = setup_baseline_repository(&root, size, seed);
            let mut rng = SplitMix64::new(seed ^ 0xFFFF_FFFF_0000_0000);
            mutate_files(&root, &files, 1, &mut rng);
            samples.push(time_commit(&root));
            let _ = std::fs::remove_dir_all(&root);
        }
        axis_a.push(Point {
            label: format!("{size} files"),
            samples,
        });
    }

    let mut axis_b = Vec::new();
    for change_count in AXIS_B_CHANGE_COUNTS {
        let mut samples = Vec::with_capacity(AXIS_B_SAMPLES);
        for sample_index in 0..AXIS_B_SAMPLES {
            let root = unique_dir(&format!("axis-b-{change_count}-{sample_index}"));
            let seed = CONTENT_SEED
                .wrapping_add(0x1000_0000)
                .wrapping_add(change_count as u64)
                .wrapping_add(sample_index as u64);
            let files = setup_baseline_repository(&root, AXIS_B_REPO_SIZE, seed);
            let mut rng = SplitMix64::new(seed ^ 0xFFFF_FFFF_0000_0000);
            mutate_files(&root, &files, change_count, &mut rng);
            samples.push(time_commit(&root));
            let _ = std::fs::remove_dir_all(&root);
        }
        axis_b.push(Point {
            label: format!("{change_count} changed"),
            samples,
        });
    }

    let report = render_report(&spawn_floor_point, &axis_a, &axis_b);
    let report_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../rfcs/handoffs/DC-59-commit-benchmark-harness/benchmark-report-v1.md"
    );
    std::fs::write(report_path, report).unwrap();
    eprintln!("report written to {report_path}");
}

fn render_report(spawn_floor: &Point, axis_a: &[Point], axis_b: &[Point]) -> String {
    let mut out = String::new();
    out.push_str("# DC-59 Commit Benchmark Report v1\n\n");
    out.push_str("Generated by `cargo test -p prikk --locked --test dc59_commit_benchmark -- --ignored --nocapture commit_benchmark`.\n");
    out.push_str("Re-running that exact command regenerates this file. The *numbers* are hardware-dependent; the **shape** of Axis A is the claim under test.\n\n");

    out.push_str("## Scope\n\n");
    out.push_str("This report states what was measured. It does not conclude whether `prikk commit` complies with NFR-PERF-01 — that determination belongs to DC-56.\n\n");

    out.push_str("## Machine and filesystem context\n\n");
    out.push_str(&format!("- CPU: {}\n", cpu_model()));
    out.push_str(&format!(
        "- Temp directory filesystem: {}\n",
        filesystem_kind(&std::env::temp_dir())
    ));
    out.push_str("- Commit includes fsync; NFR-PERF-01 names fsync in its bound, so the filesystem above is part of what these numbers claim.\n\n");

    out.push_str("## Generator parameters\n\n");
    out.push_str(&format!(
        "- File size: {FILE_SIZE_BYTES} bytes, printable pseudo-random content\n"
    ));
    out.push_str(&format!("- Tree shape: breadth {TREE_BREADTH}, depth {TREE_DEPTH} (files distributed round-robin across leaf directories, not concentrated in one directory)\n"));
    out.push_str(&format!("- Content seed: `{CONTENT_SEED:#x}` (SplitMix64, a deliberate duplicate of the generator in `crates/prikk-hash/src/tests/hash_differential.rs`, documented in this file's module docs)\n"));
    out.push_str(&format!("- Fixed author key id: `{FIXED_AUTHOR_KEY_ID}`, seed: `{}` (benchmark material, not a credential)\n", hex(&FIXED_AUTHOR_SEED)));
    out.push_str(&format!("- Fixed maintainer key id: `{FIXED_MAINTAINER_KEY_ID}`, seed: `{}` (benchmark material, not a credential; used only by the untimed setup seal, never by the timed commit)\n\n", hex(&FIXED_MAINTAINER_SEED)));

    out.push_str("## Methodology\n\n");
    out.push_str("Each sample generates a fresh repository, commits its full file set (genesis, untimed), seals it (untimed), mutates the target number of files, then times exactly one subsequent `commit`. Generation, the baseline commit, and the seal are all outside the timing window; only the measured `commit` invocation is timed. A repository is used for exactly one timed trial and then discarded — `commit` cannot run twice against one repository without an intervening seal, and repeating that cycle within a trial would let seal cost contaminate the measurement, so variance instead comes from sampling independently generated repositories per point.\n\n");
    out.push_str("Signing cost: Ed25519 author signing happens inside every timed `commit` and scales with the change set, which is exactly what NFR-PERF-01 permits. It therefore contributes to Axis B's growth, not Axis A's — Axis A holds the change set at 1 file throughout, so any growth there is not attributable to signing.\n\n");

    out.push_str("## Process-spawn floor\n\n");
    out.push_str(&format!(
        "`prikk --version`, {} samples: median {} ms, range {}-{} ms.\n",
        spawn_floor.samples.len(),
        fmt_ms(spawn_floor.median()),
        fmt_ms(spawn_floor.min()),
        fmt_ms(spawn_floor.max()),
    ));
    out.push_str("Every measurement below drives the binary through `Command`, so this floor is included in every figure below as a roughly constant additive offset. It does not hide Axis A's shape but may dominate at the smallest repository size.\n\n");

    out.push_str("## Axis A — cost against repository size, 1 file changed\n\n");
    out.push_str("Repository size varies; exactly 1 of the baseline's files is modified before each timed commit.\n\n");
    out.push_str("| Repository size | Samples | Median (ms) | Min (ms) | Max (ms) |\n");
    out.push_str("|---:|---:|---:|---:|---:|\n");
    for point in axis_a {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            point.label,
            point.samples.len(),
            fmt_ms(point.median()),
            fmt_ms(point.min()),
            fmt_ms(point.max()),
        ));
    }
    out.push('\n');
    out.push_str("If cost grows with repository size here despite the change set staying fixed at 1 file, that growth is not explained by patch construction or signing — both scale with the change set, not the baseline — and points at a full-tree scan.\n\n");

    out.push_str(&format!(
        "## Axis B — cost against changed-file count, fixed {AXIS_B_REPO_SIZE}-file repository\n\n"
    ));
    out.push_str("Repository size is held fixed; the number of modified files varies before each timed commit.\n\n");
    out.push_str("| Changed files | Samples | Median (ms) | Min (ms) | Max (ms) |\n");
    out.push_str("|---:|---:|---:|---:|---:|\n");
    for point in axis_b {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            point.label,
            point.samples.len(),
            fmt_ms(point.median()),
            fmt_ms(point.min()),
            fmt_ms(point.max()),
        ));
    }
    out.push('\n');
    out.push_str("This is the cost NFR-PERF-01 permits: patch construction and signing scale with the change set.\n\n");

    out.push_str("## Reproduction\n\n");
    out.push_str("```\ncargo test -p prikk --locked --test dc59_commit_benchmark -- --ignored --nocapture commit_benchmark\n```\n");
    out
}
