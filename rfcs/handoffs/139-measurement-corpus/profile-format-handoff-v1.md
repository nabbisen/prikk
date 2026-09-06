# RFC 139 increment 1 — the profile format, and prikk's own profile

**RFC:** `rfcs/accepted/139-measurement-corpus.md` — **accepted in full by the project owner
2026-09-06.** §3 (profile-plus-builder, never a stored repository) and §4 (provenance) are settled
input and are not reopened here.
**Base:** `main` at `c821835`.

**This increment builds no repository and measures no cost.** It produces the *description* that
increment 2's builder will read, and the extractor that derives one. **§2 is the part to read twice:
the extractor must be a pure text-to-profile transform, and the reason is testability, not taste.**

---

## 1. What to build

Three things, in a new workspace member `tools/corpus`:

1. **The profile format** — a TOML document, specified in §3 below.
2. **The extractor** — turns a recorded `git log` output into a profile.
3. **prikk's own profile** — the first one, derived from this repository at a stated revision.

**Increment 2 adds the builder to this same crate.** Create it with that in mind — a library with a
thin binary over it (RFC 139 §7), not logic inside a `main`.

## 2. The extractor is a pure text-to-profile transform, and does not run `git`

**The extractor takes text on stdin (or a file path) and writes a profile. It never spawns `git`.**

Three reasons, and the third is the one that matters:

1. No `git` dependency enters the workspace, and none is available to a build anyway.
2. RFC 139 §4 requires the profile to record its extraction command **verbatim** so a reader can
   re-derive rather than trust. A command recorded in a data file and run by a human is checkable; a
   command buried in Rust that shells out is one more thing to read.
3. **It makes the extractor testable without a repository.** A pure function from text to profile can
   be tested against a small committed fixture with a hand-checked expected result. An extractor that
   runs `git` can only be tested against whatever history the machine happens to have, which is the
   incomparability defect RFC 139 §2 exists to retire, reproduced inside the tool built to retire it.

**The intermediate text is not committed** (it is large and re-derivable). **The fixture used to test
the extractor is committed** — it is small, hand-written, and is test material, not a corpus.

## 3. The profile format

TOML. **Not JSON:** this project's TOML files are its human-editable inputs (`release-signers.toml`,
every manifest) and its JSON is machine-readable *output* (`verify --format json`, every
`release-policy` report). A profile is an input a human must be able to read in a diff. `toml` is
already in this workspace's dependency graph via `tools/release-policy`, so this adds no new
third-party surface to the build.

**`schema_version` is an integer field, following `release-signers.toml`'s own idiom**
(`schema_version = 1`), not a string like the JSON reports. This format will change; increment 2 will
probably be the first to change it.

**Required sections.** Field names below are indicative; the shape is what is specified.

**Provenance** — RFC 139 §4, all four mandatory:

- source repository identification, and the exact revision or range extracted;
- **every extraction command, verbatim**, as a list of strings;
- extraction date;
- whether git rename detection was in effect (see §4).

**Shape** — what the builder needs to synthesize history:

- commit count in range;
- **files-changed-per-commit as a histogram**, not a mean. RFC 136 §9.1 reported mean 3.37 *and*
  median *and* p90 because the mean alone hides the shape; a histogram is the honest form and the
  builder needs it to sample from;
- **operation-kind mix** — see §4, this is the part §9.1 never captured;
- **distinct paths touched over the range, and the touches-per-path histogram.** This is the
  concentration property RFC 139 §4's second-profile requirement turns on: one project touches many
  paths once each, another touches few paths many times. Without it the builder cannot tell them
  apart;
- **file-size distribution** at the extracted revision, as a histogram. RFC 133 §2 measured that
  commit cost follows **bytes, not paths** — a profile that omits sizes would produce a corpus whose
  dominant cost driver was invented.

**Builder inputs** — fixed here so a corpus is reproducible (RFC 139 §5):

- the generator seed;
- anything else a seal commits to that would otherwise vary per run. **You will find these; I have not
  enumerated them.** RFC 123 put an optional `message` inside object identity, so a message that
  varies per run makes the corpus nondeterministic. Add fields as you find them and say in the report
  what you found — increment 2's determinism test is what proves the list complete, and this
  increment's job is to leave room for it.

**One prohibition, and it is checkable.** RFC 139 §4: a profile stores **aggregate distributions
only — never file contents, and never paths from the source project.** Make this a test, not a
convention: assert that a profile extracted from a fixture containing distinctive path strings
contains none of them.

## 4. The extraction command changes from RFC 136 §9.1's, deliberately

§9.1 used, verbatim:

```
git log --pretty=format:'@@%H' --name-only --no-merges -n 600
```

**Use `--name-status` instead of `--name-only`**, keeping the rest of the shape.

**Why.** §9.1 needed only *how many* files a commit touched, to compute a collapse ratio.
A builder needs to know **what kind of change to synthesize**, because prikk's operation kinds do not
cost the same: `OperationKind` (`crates/prikk-object/src/payload/patch.rs:354-369`) is `CreateFile`,
`DeleteNode`, `EditText`, `RenamePath`, `ChangePerm`, `CreateSymlink`, `ReplaceBinary`.
`--name-status` gives `A`/`M`/`D`/`R` per path; `--name-only` gives none of it, and a builder reading
a `--name-only` profile would have to **guess** the mix.

**On rename detection: leave git's default on, and record that you did.** I had intended to specify
`--no-renames` on the reasoning that a rename is a delete plus a create — **that reasoning is wrong,
and I checked before writing it down: prikk has a first-class `RenamePath` operation** (`patch.rs:362`).
Git's rename detection therefore *matches* prikk's model rather than distorting it. It must still be
recorded in the provenance, because it materially changes the distinct-path and touches-per-path
numbers, and a reader comparing two profiles needs to know both were extracted the same way.

**File sizes need a second command**, since no `git log` form carries them:

```
git ls-tree -r -l <revision>
```

Its fourth column is the byte size. Record this command in the provenance too, and state the revision
it was run at.

**A consequence to state in the report rather than let a reader trip over: this profile's numbers will
not match RFC 136 §9.1's.** Different command, and §9.1 counted differently. §9.1 stands as its own
measurement of its own question; this is not a discrepancy and must not be presented as a correction
of it.

## 5. Where it lives, and the dependency convention that differs from the product crates

`tools/corpus`, a new workspace member. Follow `tools/benchmarks`' manifest exactly in kind:
`publish = false`, `version.workspace = true`, `[lints] workspace = true`, added to `members` but
**not** to `default-members`.

**Dependencies in `tools/` are declared with literal versions in the tool's own manifest, not through
`[workspace.dependencies]`.** That is the opposite of the product-crate convention, and it is what
both existing tools already do — `tools/release-policy` declares `toml = "1.1"`, `serde`, `regex`
literally; `tools/benchmarks` declares `criterion = "0.7"`, `tempfile = "3"`. **Match the tools, not
the crates.** The `[workspace.dependencies]` table describes the shipped dependency graph, and
`boundary-check`'s `ALLOWED_THIRD_PARTY` scopes to the eight product crates only
(`tools/release-policy/src/boundary/placement.rs:5-16`) — a tool is outside it by construction.

**Use the same `toml` version `tools/release-policy` already pins** rather than introducing a second
resolution of it. Check `Cargo.lock` after adding, and say in the report whether the dependency count
moved.

## 6. Controls

Six. Each must be a test that could fail, and you should be able to say how you saw it fail.

1. **The extractor is deterministic.** Same input text twice → byte-identical profile. Trivially true
   if the implementation is pure, which is the point — it also catches a `HashMap` iteration order
   reaching the output, which is the realistic way this breaks.
2. **The prohibition of §3 holds.** A fixture containing distinctive path strings and file contents
   produces a profile containing none of them. Assert on the actual strings.
3. **The histograms are histograms.** A hand-built fixture with a known, uneven distribution produces
   the counts you computed by hand — not just a plausible mean. Include at least one commit touching
   many files and several touching one, so a mean-only bug is visible.
4. **The operation-kind mix is real.** A fixture containing all of `A`, `M`, `D` and `R` lines
   produces four non-zero categories. This is the field §9.1 never had; an extractor that silently
   dropped `R` would look correct against an `A`/`M`-only fixture.
5. **Malformed input is refused, not absorbed.** Truncated output, a commit header with no file lines,
   an unrecognized status letter. **Refuse with an error naming the line** — do not skip and continue.
   A profile silently derived from half its input is worse than no profile, because it is comparable
   to nothing and looks fine.
6. **The committed prikk profile round-trips.** Parse the profile this increment produces and confirm
   the parse agrees with the file. This is the check that the format is actually readable by the
   thing that will read it, before increment 2 depends on it.

## 7. Out of scope

- **The builder.** Increment 2. Do not start it, and do not shape the format around a builder design
  you have not been given — if the format is missing something the builder needs, that is a finding
  for the report and a schema bump in increment 2, which is why `schema_version` exists.
- **The second profile.** Increment 4, and it needs a source project chosen deliberately for the
  opposite concentration property.
- **Any measurement.** No cost figure is produced by this increment.
- **Any change to `crates/`.** Nothing shipped is touched.

## 8. Gates

The full set, verbatim from `rfcs/EXECUTION-ORDER.md` §6 rule 9:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- `cargo test --workspace --locked`
- `cargo +1.85.0 test --workspace --locked`
- `cargo +1.85.0 check --workspace --all-targets --locked`
- `git diff --check`
- `cargo audit --no-fetch`
- `RUSTDOCFLAGS="-D rustdoc::private_intra_doc_links" cargo doc --workspace --no-deps`
- release-policy `check`, `boundary-check`, `reference-check`

**A new workspace member is exactly the kind of change that moves `boundary-check`.** Run it early,
not only at the end.

**Cross-target clippy only if your own diff introduces `#[cfg(target_os)]`/`#[cfg(unix)]`/
`#[cfg(windows)]`.** It should introduce none — a text transform has no platform surface. If you find
yourself adding one, that is a finding worth reporting rather than a detail.

## 9. No `CHANGELOG.md` entry, and this is a ruling rather than an omission

`tools/corpus` is `publish = false`, outside `default-members`, and ships to nobody. There is no
user-facing surface, so there is no changelog entry.

**This is stated explicitly because the opposite error has happened twice** — `.prikkignore` in
0.29.0 and `prikk key`/`prikk setup` in 0.33.0 both shipped undocumented, and in both cases the cause
was a handoff that simply did not mention the changelog. **Every handoff now either names the entry as
a deliverable or rules it out in writing. This one rules it out.**

## 10. Reporting

`.git-exclude/review-request/`, per the standing convention. Include:

- the profile you produced, and how to re-derive it — the reader must be able to run your recorded
  commands and get your numbers;
- **the fields you added to "builder inputs" that this handoff did not name**, and how you found them;
- how you saw each of §6's six controls fail before it passed;
- whether `Cargo.lock`'s dependency count moved, and what by;
- **anything the format cannot express that you think a builder will need.** You are the first person
  to look at this format with an implementation in hand; that finding is worth more than a clean
  report.
