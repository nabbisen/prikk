# G1 — refresh the compatibility fixture to `0.27.1`

**Base:** current `main` (`d26746d`, `0.27.1` released and published).
**Under `003-landing-work-on-main.md`** — commit locally on `main`, do not push, do not tag.

**This is the fifth refresh, and it is the least interesting one yet — read §1 before deciding how
much care each control deserves.** §5 is the one genuinely new question.

---

## 1. This refresh buys alignment, not assurance — and that is worth saying plainly

**`0.27.1` shipped no code.** The only diff in `crates/` between the two tags, excluding tests and
fixtures, is `release_compatibility_gate.rs` — which is `#[cfg(test)]` and does not ship. **The
`0.27.1` binary is functionally identical to `0.27.0`.**

**So a `0.27.1`-built fixture is exactly as good as the one already committed, and no better.** This
refresh does not test anything the current fixture does not already test.

**Do it anyway, for one reason:** the gate's own contract is that the fixture *is the last release*,
and `DECLARED_BREAKS`'s version scoping keys off `LAST_RELEASE_FIXTURE_VERSION`. Leaving it at
`0.27.0` makes the constant stale and would make any future declared break reference the wrong
version. **Skipping when the delta looks small is how that invariant erodes.**

**What this means for your effort:** §2's provenance discipline still applies in full — but do not
manufacture significance for the result. **If the report says "this changed nothing except the
constant and the bytes," that is the correct report.**

## 2. Provenance, fourth consecutive time

Detached worktree at the **`0.27.1` tag**, `cargo build --locked -p prikk`, confirm `prikk --version`
prints `0.27.1`, construct with **that binary and no other**. **Quote the version output and the
worktree commit.**

**The fixtures still cannot be byte-identical** — `node_id_gen.rs` mints every `NodeId` from the OS
CSPRNG. **A `diff -r` against the outgoing fixture must differ; if it does not, the rebuild did not
happen.** Expect roughly ten of thirty-four files to differ, the same content-derived set as last
time.

## 3. The `*.log` hazard — six for six if it recurs

`.gitignore`'s `*.log` has silently excluded the same four empty generation logs from every fixture
before the one that caught them:

```
containers/generations.log
refs/containers/pointer-index-generation.log
refs/containers/received-index-generation.log
trust/policy-generation.log
```

**The outgoing `0.27.0` fixture is complete — 34 files on disk, 34 tracked.** Compare
`find <fixture> -type f | wc -l` against the tracked count before staging, `git add -f` the four, and
**report both numbers.**

## 4. The refresh

- **Replace, do not accumulate.** Delete the `0.27.0` fixture in the same commit. (Empty directory
  shells left behind by `git rm` are normal and not accumulation — `rfc119_g1_0_25_0_repo` and
  `_0_26_0_repo` are already such shells.)
- **Match the previous coverage**: six of seven persisted types, `Attestation` absent because no
  production path constructs one. **Report the coverage table.**
- **Re-derive the schema arrays from a probe** against the real new fixture. **Do not copy them from
  the committed expectations.**
- **No `DECLARED_BREAKS` entry is owed.** `0.27.1` changed no shipped code at all, so there is
  nothing that could constitute a decode break. Do not add one.

## 5. Control 3's site scarcity — the new question, and I think it resolves itself

The last review recorded: **spent are `Block`, `Patch`, `Tag`, `RefState`; remaining are `Blob` and
`RecognitionClaim`.** Use one of those two.

**But work out the endpoint rather than deferring it again.** The reasoning that forbids re-use is
that showing the gate fires for `Tag` does not show it fires for `Blob` — the decode dispatch is
per-type, so each refresh extends coverage across the type space. **`Attestation` is not in the
fixture and cannot be a site, so there are six usable types, four are spent, and this refresh plus
one more exhausts them.**

**That is not a crisis; it is a completion.** Once every fixture-present type has been shown to fire,
the claim *"the gate fires for any persisted type that fails to decode"* is fully established, and a
later refresh may re-use any site without weakening anything.

**Adjudicate and record it:** state in the report which site you used, which remain, and **whether
you agree the control reaches completion rather than exhaustion** — so refresh seven's author does
not rediscover this as a problem. **If you disagree, say why**, because I would rather be corrected
now than have a control quietly become ceremonial.

**One thing that is not a substitute, established during the `0.27.0` review:** corrupting fixture
*bytes* mid-container fires the **coverage** test, not the G1 compatibility test — the object
disappears from enumeration rather than failing to decode. **Useful, and it consumes no decode-arm
site, but it proves a different claim.** Do not use it for control 3.

## 6. What must not change

- **G1's shape** beyond the constant, the fixture, and its committed expectations.
- **The gate itself must not be modified to accommodate the new fixture.**
- **Any product behaviour.**

## 7. Controls

1. **The fixture is genuinely `0.27.1`-vintage** — `prikk --version` and the worktree commit quoted.
2. **Old-vs-new differ** (§2), with the count of differing files.
3. **The gate fires on an undeclared break** — §5's site. **Quote the failure**, then revert.
4. **Coverage remains load-bearing** — the committed expectations notice a shrunken or reschema'd
   fixture.
5. **The gate passes unmodified**, full suite green, count unmoved.
6. **Full gate set against the exact final commit.**

## 8. What to report

Provenance and the coverage table; both file counts from §3 and which four were force-added; your
re-derived schema arrays and whether they match; **§5's adjudication and endpoint reasoning**; all six
controls quoted; the full gate set; and **anything in this handoff that was wrong** — including §1's
claim that `0.27.1` shipped no code, which I derived from a diff and you should re-derive.

**Stop and escalate if the `0.27.1` fixture fails the gate immediately** — that would mean current
`main` cannot read what we published today.
