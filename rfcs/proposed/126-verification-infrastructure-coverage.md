# RFC 126 — Four flanks the verification culture never reached

**Status.** **Proposed; §6 ruled.** Raised by the external architecture audit of 2026-08-31
(`audit-2026-08-31-task-2.md` §3 cross-cutting, `task-3.md` §2c/§2d, Top-10 #10). All four
independently confirmed at `3a8d730`.

**RULED by the project owner 2026-09-01: §6 option 4** — criterion enters the workspace in **its own
member, outside `default-members`**, the shape `tools/release-policy` already established. It appears
in no product crate's manifest and in no shipped dependency graph. The reversal of this RFC's first
recommendation, and the accessibility and drift arguments behind it, stand recorded in §6.

**Tracks.** Gates that do not exist. No product behaviour changes in this RFC.

---

## 1. The framing that makes this one RFC

This project verifies almost everything it does, unusually well: 76 failpoint sites each asserting
error *and* post-state *and* retry; decoder totality proptests; a frozen SHA-256 differential at
10,000 cases per run; cross-platform object-id diffing as a CI job; a policy binary that gates its own
boundaries. **The four gaps below are all the same shape — a place where that culture stopped —
and each is cheap relative to what already exists.**

## 2. The patch algebra has zero property tests

Confirmed: `grep -rl proptest crates/prikk-store/src/patch_algebra/` returns nothing.

**This is the subsystem where property testing pays most.** Commutation and confluence are
combinatorial; they are covered today by roughly 2,500 lines of hand-built fixtures which are
complete over the sealed/unsealed × malformed/missing matrix — but **nothing searches for a
non-commuting pair the fixtures did not imagine.**

And the harness is already built. `replay_oracle.rs` applies a pair in both orders over real
lifecycle state and real text bytes and compares final states. That is an executable specification,
which is exactly what a proptest needs:

> generate operation pairs and sequences; assert **classifier says `Commutes` ⇒ oracle states equal**.

**The failure this would catch is the only unsound one the design admits.** As the audit observes,
a gap in the pairwise theory can cause a spurious *refusal* safely — but a pair wrongly classified
`Commutes` is the one shape that produces an unsound merge, and it is precisely the shape a
fixture author is least likely to think of.

## 3. `cargo audit` runs in the gate set and not in CI

**The audit's framing is too broad and the gap is still real.** `cargo audit --no-fetch` is one of the
standing gate commands in `EXECUTION-ORDER.md` §6 rule 9 and runs on every gate pass. But
`grep -rn "cargo audit" .github/workflows/` returns nothing: **advisory monitoring depends on a human
running the gate set**, and it uses `--no-fetch`, so it is only as current as the local advisory
database.

A scheduled CI job that fetches is the missing half. It is also the only item in this RFC that can
find a problem *without anyone changing the code* — a new advisory against one of the 25 shipped
crates arrives on its own schedule, not on ours.

## 4. Documentation is never gated

Two distinct holes, confirmed:

- **`cargo doc` never runs in CI**, and `cargo doc --workspace --no-deps` currently emits **exactly 7
  `rustdoc::private_intra_doc_links` warnings** (`block_state.rs` ×5, `bundle.rs`, `worktree_patch.rs`).
  Each renders a public doc item's link as literal `[`Name`]` text. Gate: `-D rustdoc::private_intra_doc_links`.
- **`docs.yml` triggers only on push-to-`main`, filtered to `docs/**`.** So a code change that
  falsifies a documented claim triggers no docs build at all, and the book is never built on a PR.

**This matters more here than in most projects**, because RFC 118 ("derive, never transcribe") and the
§8 doc-coverage gate make documentation a checked artifact rather than prose — and those checks
themselves run only under `cargo test`, not against the rendered book.

**Separately: zero doctests.** `cargo test --workspace --doc` → `0 passed`. 566 public items are
documented and not one carries a compiler-verified example. Recommendation is deliberately narrow:
**the ten kernel entry points only** (`ObjectId::from_canonical_payload`, `validate_repo_path`,
`Ed25519KeyPair`, `verify_ed25519`, `CanonicalWriter`, `RefStore::publish`, and peers). A blanket
doctest campaign across 566 items would be a large ongoing cost for a small marginal gain.

## 5. Benchmarks exist and never run

`dc59_commit_benchmark.rs` (1,064 lines) is `#[ignore]`d; `dc92_lineage_replay_benchmark.rs` runs
`SAMPLES = 2` and says of itself that it "decides nothing". There is no `benches/`, no criterion, and
no CI job. **DC-62's peak-RSS work — genuinely good measurement — can regress invisibly**, and so can
the two performance walls tracked in `ROADMAP.md`'s corrective program.

## 6. The ruling this RFC needs: criterion as a dev-dependency

**The dependency gate permits it, and I verified why rather than taking the audit's word.**
`placement.rs:49-53` collects only `[dependencies]`, `[build-dependencies]`, and their `[target.*]`
forms — *"`[dev-dependencies]` is deliberately excluded everywhere… it is the sink this check
protects."* `proptest` already lives there in `prikk-object` and `prikk-store`.

**So the gate is not the question. The question is the workspace's own standard**, which is stricter
than its gate: exactly one third-party dev-dependency exists today, and criterion brings a
substantial tree into `cargo test` builds for developers and CI alike.

Four shapes:

1. **Criterion in a product crate's `[dev-dependencies]`.** Standard tooling, statistical rigor,
   baseline-relative regression detection. Puts a large tree into `prikk-store`'s own manifest.
2. **A fixed-threshold smoke job** on the existing harnesses in a scheduled workflow. Zero new
   dependencies; catches order-of-magnitude regressions only; thresholds are hand-maintained.
3. **Neither — record that performance is unmeasured** and stop implying otherwise.
4. **Criterion in a separate workspace member, outside `default-members`** — the shape
   `tools/release-policy` already established.

**The architect's recommendation is 4. An earlier draft of this RFC recommended 2, and that was
optimizing for the wrong thing.** Two properties decide it, and both were raised by the owner:

- **Accessibility.** Criterion is what a Rust contributor already knows: `cargo bench`, a standard
  report, a familiar statistical model. A bespoke threshold harness is one more thing that must be
  learned before anyone outside this project can evaluate a performance claim — and this project has
  to earn contributors against a novel model as it is. **On this axis option 2 is clearly worse, not
  marginally worse.**
- **Stability over a long-lived project.** Hand-maintained thresholds drift, and a threshold nobody
  recalibrates becomes a gate that passes vacuously — **the exact failure shape RFC 127 exists to
  correct**, reintroduced deliberately. Criterion compares against a stored baseline, so the
  maintenance is a by-product of running it rather than a separate discipline nobody owns.

**The dependency objection dissolves once placement is right, and that is why option 4 exists.**
`tools/release-policy` already carries 133 dependencies inside this workspace, excluded from
`default-members`, never shipped, with 13 duplicate-version crates confined there. **The project has
already solved "heavy infrastructure tooling without contaminating the product", and criterion is the
same problem.** A separate member keeps criterion out of every product crate's manifest and out of the
shipped dependency graph — the property `placement.rs` exists to protect. **Stated honestly: it does
not keep it out of build time**, since `cargo test --workspace` builds every member, exactly as it
already builds `tools/release-policy`.

**The owner's call remains**, because it accepts a real dependency tree into the workspace. But the
cost is one the project has already priced once and judged worth paying.

## 6a. Recorded 2026-09-02 — workflow permissions are not declared least-privilege

Found by the architect while reviewing §3/§4's increment. **Not introduced by it, and not a defect
in it** — recorded here because this RFC owns the CI surface and the observation would otherwise
live only in a git-excluded review.

Of five workflows, **two declare `permissions:` and three do not**:

| Workflow | trigger | `permissions:` |
|---|---|---|
| `docs.yml` | push to `main` | declared (`pages: write`, `id-token: write` — it deploys) |
| `release.yml` | release | declared |
| `ci.yml` | push, pull request | **absent** |
| `docs-pr.yml` | pull request | **absent** |
| `security-audit.yml` | schedule | **absent** |

A workflow with no `permissions:` block inherits the repository or organisation default, which may
be read-write. **The three that omit it are all check-only workflows that need nothing beyond
`contents: read`** — and two of them run `cargo install` from crates.io, one of those on a schedule
with no human watching.

**`security-audit.yml` is the sharpest case**: it is the workflow whose entire purpose is telling
the project whether its dependencies are trustworthy, and it runs third-party build scripts with
whatever the default grants.

**This predates the increment** — `ci.yml` has always been in this shape — so it is one item across
three files, not a correction to §3/§4. `permissions: contents: read` on each is the whole change.
**Whoever takes it should check the repository's actual default first**, since if it is already
read-only the change is documentation of an existing property rather than a hardening.

## 7. Scope

**In:** the oracle-backed property tests (§2); a scheduled fetching `cargo audit` CI job (§3);
`cargo doc -D rustdoc::private_intra_doc_links` in CI plus the 7 existing fixes, and `mdbook build`
on PRs touching `docs/**` or the CLI (§4); doctests for the ten kernel entry points (§4); whichever
benchmark shape §6 rules (§5).

**Out:** coverage measurement, mutation testing, sanitizers, Miri (moot — no unsafe outside the
Windows FFI crate, which CI already builds and tests on Windows). Differential testing against Git —
there is no compatibility claim to differentiate against.

## 8. Ordering

**§3 and §4 are the cheap half and should go first** — a CI job and a lint flag, each independently
landable, each closing a hole that lets everything else drift silently. §2 is the valuable half and
needs real design. §5 waits on §6's ruling.
