# DC-95 Stage 1, Round 4 — Review v1

**Reviewing:** `0450ad9` on `dc-95-verify-coverage-and-finding-accumulation`.

**Accepted, no conditions.** 17 of 36. Both of round 3's named-open items are closed.

**§2's third category is a real refinement of the rule, and §3 records what it depends on.**

## 1. Verified

Probed the signature-shape check independently:

```
verify_repository_rejects_malformed_signature_shape ... FAILED
  expected verify_repository to reject a malformed-shape signature
```

`verify_repository` returns `Ok` with `validate_strict` disabled. And I confirmed the mechanism they
rest the classification on: **none of the ten `has_*` predicates reads `signature_envelope_issues`** —
so the sibling finding that *does* observe the same defect changes nothing about `prikk verify`'s
verdict.

Gates clean at `0450ad9`: fmt, clippy, both toolchains, **621** prikk-store tests, `git diff --check`,
`cargo audit`, all three release-policy checks.

## 2. The third category is right, and it sharpens the rule

Rounds 2 and 3 produced a binary: load-bearing, or downstream-redundant. Round 4 found a case that is
neither — **a downstream sibling exists and independently observes the defect, but its finding type is
not wired into the blocking surface, so removing the real check still yields a clean `prikk verify`.**

They could have filed this under "downstream-redundant, relabel and move on" — the surface reading — and
instead traced whether the sibling actually changes the verdict. **It does not, so the check is
load-bearing.** That distinction is not obvious from the rule's original wording, and it is theirs.

**Adopted as the third classification** for the remaining rounds: *load-bearing* / *downstream-redundant
(something else rejects it)* / *downstream sibling observes it but the verdict is unchanged — still
load-bearing*.

## 3. What this classification rests on, recorded because it is contingent

**The same fact is now doing double duty in opposite directions**, and that is worth naming:

- In the prerequisite ruling I confirmed the four signature-envelope **findings** were correctly excluded
  from Stage 1's mandatory set *because* they are non-blocking.
- Round 4 now classifies the strict-shape **check** as load-bearing *because* its sibling finding is
  non-blocking.

Both are correct. Together they show that "`signature_envelope_issues` never fails verification" is
load-bearing for the classification itself — and it is a position recorded in `FINDINGS.md` as
*"Signature envelope canonicalization incomplete — Non-blocking — DC-39"* since the original architect
review, never revisited. **Reverse that decision and both conclusions flip**: the four excluded rows
become mandatory, and this round's check becomes downstream-redundant.

**To be precise about the product's actual state, since this could be read alarmingly:** a malformed
signature **is** rejected today, by `validate_strict` at read time. Nothing is currently getting through.
What round 4 establishes is that the sibling is a silent observer rather than a backstop — a
defence-in-depth gap, not a live hole. Registered in `FINDINGS.md` as its own question.

## 4. The construction note is worth keeping

Building the fixture needed `encode_envelope_file_structural`, because `verify/tests.rs`'s own
`encode_envelope_file` — used by rounds 2 and 3 — itself calls `validate_strict()` and would have
rejected the malformed signature before any bytes reached disk.

**That is a reassuring finding stated as an inconvenience.** The rule is enforced at every production
write path *and* at most test-support write paths, with exactly one loosely-validating helper existing
for this purpose. The difficulty of building the fixture is evidence the invariant holds.

## 5. Not conditions

- **The `RefState` test not being disable-probed.** Same accumulated, non-hard-`Err` mechanism as round
  3's `Block` test; the untrusted/trusted contrast in one `verify_repository` call is the right
  substitute, and the reason is stated rather than the omission being quiet.
- **Relying on `test_support::signed_ref_state_envelope`** rather than inventing a construction. Correct
  — and they checked that round 3's content-addressing collision hazard does not apply here, because two
  `RefState` payloads differ naturally once their `ref_name` does.

## 6. Standing

- **Round 4: accepted.** 17 of 36. The `verify/objects.rs` cluster is complete.
- **Round 5** next: the `refs/verify.rs` + `scan.rs` cluster, ~13 checks and a different fixture family
  (`RefStore`/pointer-file construction). The per-cluster table question reopens there — ask it again
  rather than assuming this file's shape carries.
- Green three-platform CI before any merge.
