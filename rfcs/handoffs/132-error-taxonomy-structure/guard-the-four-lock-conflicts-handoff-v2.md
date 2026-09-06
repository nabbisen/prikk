# RFC 132 round — required follow-up: guard the four that stay

**Follows:** `six-preconditions-and-the-broad-arm-handoff-v1.md`, **ACCEPTED and pushed**
(`b5c2c4d`, `a050725`) with one required follow-up.
**RFC:** `rfcs/done/132-error-taxonomy-structure.md` — increment 2 still deferred, still untouched.
**Base:** `main` at `a050725`.

**This is small and it is not optional.** The v1 round is otherwise accepted in full.

---

## 1. What was missed, and how I know

**v1 control 4:** *"The four are untouched, asserted the same way. A test that only checks the six
would pass if you moved all ten."*

The report answered: *"All four unchanged, all still passing under their existing coverage."* **That
is a different claim.** It says the four still work. It does not say anything would notice if they
changed.

**Measured, not inferred.** I moved `lock.rs:179` from `LockConflict` to `Precondition` and ran
`cargo test --workspace --locked`:

```
FAILED count: 0
```

**The whole suite passes with one of the four silently reclassified.** No test anywhere asserts the
`lock conflict:` prefix — the only occurrence of that string in any test file is a comment in
`rfc132_part2_precondition_prefix.rs`'s own module doc.

**So the round guards the six it changed and leaves the four it deliberately preserved unguarded.**
That is exactly the asymmetry control 4 exists to prevent: the next person sweeping this taxonomy has
nothing stopping them from taking all ten.

## 2. What to build

**Assertions that the four still classify as `LockConflict`**, in the same style you used for the six.

| Site | Message |
|---|---|
| `lock.rs:51` | `active lock belongs to a different repository authority` |
| `lock.rs:179` | `{kind} lock already exists: {path}` |
| `refs.rs:453` | `ref CAS mismatch for {ref}: expected …, got …` |
| `rollback_draft.rs:167` | `rollback-draft target ref changed during planning; retry rollback-draft` |

**Assert the `lock conflict:` prefix**, the way v1 asserted `precondition not met:` for the six.

**Use the same judgement v1 used about reachability, and state it the same way.** v1's report was
careful to distinguish CLI-reachable sites from white-box-only ones and to extend existing tests in
place rather than invent CLI paths that do not exist. Do that again — some of these four are likely
only reachable at store level, and a store-level unit test asserting the variant is the honest form.
**Do not manufacture a CLI path to reach one.**

## 3. Why the wording matters as much as the variant

For these four the class word is doing real work. `lock conflict:` tells a caller **wait and retry**;
`precondition not met:` tells them **change what you asked for**. The reporting front-end glosses one
as *"another writer is active"* — which is the correct gloss here and the wrong one for the six.

**So the assertion is not bookkeeping.** It pins the half of the distinction that is easy to lose
precisely because nothing changed about it this round.

## 4. Out of scope

- **Any change to the four sites themselves.** Their classification is correct; this adds guards, not
  edits. If while writing the tests you conclude one of the four *is* a precondition after all, **stop
  and report it** rather than changing it — that is a finding, and it would be the third independent
  reading of that line.
- **The six from v1.** Done and accepted.
- **RFC 132 increment 2.** Still deferred.
- **Any new test infrastructure.** If this needs more than assertions in existing or sibling test
  files, say so in the report rather than building it.

## 5. Controls

1. **Each of the four bites.** For each site: perturb it to `Precondition`, confirm your new
   assertion fails, revert. **Report that you did this per site** — this is the control v1's report
   did not run, and it is the whole reason this follow-up exists.
2. **The tree is clean after every perturbation.** `git status --short` empty. Revert by editing the
   value back, **not** by `git checkout -- <path>` — that has previously discarded an uncommitted fix
   sitting underneath a control mutation in this repository.
3. **The six still bite.** Re-run v1's own assertions unchanged; this follow-up must not disturb them.

## 6. Gates

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

## 7. No `CHANGELOG.md` entry

Test-only. Nothing a user can observe changes. **Ruled here rather than left unsaid**, per the
standing rule that every handoff either names the entry or rules it out.

## 8. Reporting

`.git-exclude/review-request/`, per the standing convention. Include **the per-site perturbation
results from control 1** — four perturbations, four failures, four reverts, and the tree clean after
each.
