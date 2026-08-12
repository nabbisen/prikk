# DC-95 Stage 1, Round 3 — Review v1

**Reviewing:** `01b1b33` on `dc-95-verify-coverage-and-finding-accumulation`.

**Accepted, no conditions.** 15 of 36 covered. Round 2's dangling doc reference is fixed.

**§3 is the finding that is now accumulating across rounds, and it is better news about the product than
the framing so far has implied.**

## 1. Verified

Both checks classified load-bearing, probed independently:

```
verify_repository_detects_envelope_type_mismatch ... FAILED
  expected verify_repository to reject a type-mismatched object file

verify_repository_detects_object_id_mismatch ... FAILED
  expected verify_repository to reject an id-mismatched object file
```

`verify_repository` returns `Ok` in both cases with the check disabled — genuine clean passes, so both
demonstrate the rule directly. Classification accurate.

Gates clean at `01b1b33`: fmt, clippy, both toolchains, **619** prikk-store tests (615 + 4),
`git diff --check`, `cargo audit`, all three release-policy checks. Round 2's dangling doc reference is
gone.

## 2. Two self-corrections, and the first one is the rarer kind

**They wrote "both probed, both load-bearing, confirmed" in a doc comment before running the probes, then
ran them, found both are downstream-redundant, and disclosed that they had written ahead of the
evidence.** Not "I initially thought X" — specifically, *the claim was in the file before the experiment
that would have justified it*.

That is exactly the failure I have committed repeatedly this cycle — DC-93's "nothing invokes it" over a
directory that did not exist, the "eight occurrences" that were eleven. **Catching it in yourself and
reporting the mechanism rather than just the corrected answer is the harder half**, and it is worth more
to this project than the four tests in the diff.

**The second is a genuine trap and worth recording for the remaining rounds.** Their first trust fixture
checked an untrusted block *before any policy existed*, and `PublicationTrustVerifier` treats "no policy
file" and "a real policy that excludes this signer" as different, differently-coded outcomes —
`PRIKK-TRUST-POLICY-INVALID`, not `PRIKK-TRUST-PUBLICATION-UNTRUSTED`. **An absent control is not the
same failure as a present-but-non-matching one**, and a test that conflates them proves the wrong thing.

**And a hazard specific to this codebase, caught before submission:** their untrusted and trusted
payloads were otherwise byte-identical, so both would have content-addressed to the **same object id**
and silently collapsed into one object on disk. In a content-addressed store, two fixtures that differ
only in intent are one fixture. Worth carrying into every remaining round.

## 3. The pattern across rounds, and what it actually says

Eight checks have now been individually probed across rounds 2 and 3. **Four load-bearing, four
downstream-redundant.**

The inventory classified all eight as rule-matching by reasoning about their role. Empirically, half are
backed up by something else. **At that rate the "36" is closer to a count of *checks worth a test* than a
count of *last lines of defence*** — and the remaining rounds should keep expecting roughly this split
rather than treating redundancy as the exception.

**The reframe worth stating, because the framing so far has been all deficit:** a 50% redundancy rate is
*good news about `verify`*. It means these defects are caught by multiple independent paths — the type
directory's own listing, the `.pobj` extension check, `validate_v2_lineage`'s read, the replay layer's
patch read — none of which the inventory's per-check reasoning credited. Stage 1 started as "close 36
coverage holes"; it is turning out to be "establish which checks are the last line and which are backed
up," and the second is a more useful thing to know.

**It does not reduce the work.** Redundant checks still earn their regression guard, for the diagnostic
reason already agreed. What changes is the claim Stage 1 can make at the end, which should be stated in
those terms rather than as a coverage percentage.

## 4. Not conditions

- **Covering 4 of 5 and deferring the `RefState` half plus `validate_read_schema`.** Correctly named as
  open rather than quietly folded in.
- **The trust check not being disable-probed.** Their reason is right: `publication_trust_issues` is
  accumulated, never a hard `Err`, so "disable the check" would mean suppressing the whole verifier
  rather than one arm. The untrusted-versus-trusted contrast inside a single `verify_repository` call is
  the correct substitute, and saying so beats a probe that would have proved something else.

## 5. Standing

- **Round 3: accepted.** 15 of 36.
- **Round 4** next: the `RefState` publication-trust half and `validate_read_schema`'s strengthening,
  then the `refs/verify.rs` cluster — a different fixture family, so the per-cluster table question
  reopens there.
- Green three-platform CI before any merge.
