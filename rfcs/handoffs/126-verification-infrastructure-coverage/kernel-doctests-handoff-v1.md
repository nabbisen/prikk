# RFC 126 §4 — the ten kernel doctests

**Authority:** `rfcs/proposed/126-verification-infrastructure-coverage.md` §4.
**Base:** current `main`. **Under `003-landing-work-on-main.md`.**

**This is RFC 126's last dev-team increment.** §2, §3, §4's CI half, §6a, §6b and §5 increment A have
all landed; §5's remaining question was ruled on 2026-09-03 (§5a) to be the owner's, not implementable
work. When this lands, RFC 126 is closed except that one decision.

---

## 1. The shape, and the thing this must not become

`cargo test --workspace --locked --doc` → **`0 passed`**. **566 public items are documented and not
one carries a compiler-verified example.**

**§4's recommendation is deliberately narrow: the ten kernel entry points only.** A blanket campaign
across 566 items was refused in the RFC itself as *"a large ongoing cost for a small marginal gain"*.
**Do not exceed ten.** If you find yourself adding an eleventh because it was easy, stop — the scope
is the deliverable.

## 2. No gate change is needed, and I verified that rather than assuming it

`cargo test --workspace --locked` **already runs doctests** — it emits a `Doc-tests <crate>` section
per library, each currently `0 passed`, and no manifest sets `doctest = false`. **So these run in the
standing gate set from the moment they exist**, at stable and at MSRV both.

**That is the whole point and also the constraint**: a doctest is a test that runs on every gate run
and in CI, on Linux, macOS and Windows. Write them accordingly — §4 below.

## 3. Which ten

**§4 names six**, and I verified every one is a real symbol at the current commit rather than a name
the RFC carried from an older shape:

| Entry point | Where |
|---|---|
| `ObjectId::from_canonical_payload` | `crates/prikk-object/src/envelope.rs` |
| `validate_repo_path` | `crates/prikk-object/src/lib.rs` |
| `Ed25519KeyPair` | `crates/prikk-crypto/src/lib.rs:19` |
| `verify_ed25519` | `crates/prikk-crypto/src/lib.rs:62` |
| `CanonicalWriter` | `crates/prikk-object/src/canonical.rs` |
| `RefStore::publish` | `prikk-store`'s refs surface — **find the definition; my grep hit a call site in `branch.rs`, not the definition, so confirm it** |

**§4 says "and peers" and names no others. Choosing the remaining four is yours**, against this
criterion: **the entry points an embedder meets first, or whose misuse is silent.** Something that
returns a wrong-but-plausible value when called incorrectly earns an example far more than something
that returns an obvious error.

**State your four and why in the report.** If you conclude fewer than ten are worth it, say that and
stop at the number you can justify — **ten is a ceiling from the RFC, not a quota to fill.**

## 4. What makes these doctests rather than decoration

**Every example must assert, not merely compile.** A doctest that constructs a value and ends is a
test that cannot fail — the exact shape this project refused in RFC 126 §2 (a tautological property)
and RFC 127 (a gate that could not fail). **If an example has no meaningful assertion, that entry
point was the wrong choice; pick another.**

**No `no_run` without a stated reason in the doc text itself.** An example the compiler checks but
never executes proves the signature and nothing about behaviour. Where a fixture makes running
genuinely impractical, `no_run` is acceptable **only** with one sentence in the surrounding prose
saying why — and report every instance.

**They run everywhere, so:**
- **No filesystem writes outside a temp directory**, and no reliance on `$TMPDIR`'s filesystem
  (see `tools/benchmarks/README.md` for what that assumption cost the benchmark).
- **No network, no clock, no randomness** that changes the asserted value. Deterministic or it will
  flake on one platform in CI and not the other two.
- **Fast.** These now run on every gate invocation; an example that builds a repository fixture is a
  cost every developer pays forever.
- **MSRV-clean.** `cargo +1.85.0 test --workspace --locked` runs them too.

**`missing_docs` already warns and these items are already documented.** You are adding *examples* to
existing prose, not writing the documentation. **Do not rewrite the surrounding doc comments** beyond
what an example needs, and re-verify any citation in a paragraph you do touch.

## 5. What I could not settle for you

**Whether `prikk-store` entry points can carry runnable examples at all.** Its public surface mostly
needs a `RepositoryLayout`, which means a temp directory and an `init` — cheap in isolation, not
obviously cheap when it runs on every gate run.

**Decide it on evidence: measure the wall-clock cost of the doctest suite before and after**, and
report both numbers. If a repository-backed example costs more than it teaches, prefer a
pure-function entry point from `prikk-object`/`prikk-crypto` and say so. **That measurement is a
deliverable of this increment**, not a nicety — it is what tells the next person whether this scope
can ever be widened.

## 6. Gates and reporting

Full set from `rfcs/EXECUTION-ORDER.md` §6 rule 9 against your final commit — **not reproduced
here**: `reference-check` rejects a policy-command line outside its registered sites. **Rule 9 gained
`cargo +1.85.0 check --workspace --all-targets --locked` on 2026-09-03.**

`mdbook build` does not apply unless you change a `docs/src/` page.

Local commits on `main`; **no push, no tag, no publish.** Report to `.git-exclude/review-request/`,
and state:

1. `cargo test --workspace --locked --doc` before and after — the before is `0 passed`.
2. Which four you chose beyond §4's six, and why; and if you stopped short of ten, why.
3. Every `no_run`, with its reason.
4. The doctest suite's wall-clock cost, measured (§5).
5. Whether `RefStore::publish`'s definition is where §4 implies.
6. Every place this handoff's claims proved wrong. **The six symbol locations are mine, one of them
   is admittedly a call site rather than a definition, and this project's handoffs have a consistent
   record of getting such details wrong.**
