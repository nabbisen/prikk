# RFC 126 — Four flanks the verification culture never reached

**Status.** **Proposed.** Raised by the external architecture audit of 2026-08-31
(`audit-2026-08-31-task-2.md` §3 cross-cutting, `task-3.md` §2c/§2d, Top-10 #10). All four
independently confirmed at `3a8d730`.

**Ruling required (§6).** Whether criterion enters the workspace as a dev-dependency.

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

Three shapes:

1. **Adopt criterion** as a dev-dependency. Statistical rigor, standard tooling, regression detection
   built in. Costs a large dev-only dependency tree.
2. **A fixed-threshold smoke job** on the existing harnesses in a scheduled workflow. Zero new
   dependencies; catches order-of-magnitude regressions only; the thresholds are hand-maintained and
   will drift.
3. **Neither — record that performance is unmeasured** and stop implying otherwise.

**The architect's recommendation is 2.** The regressions worth catching here are asymptotic (an O(n²)
lookup reappearing), not the few-percent changes criterion exists to resolve, and option 2 keeps the
one property this workspace has that almost nothing else does: a dependency tree small enough that a
person can read it. **The owner's call, because it trades a project value against a standard tool.**

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
