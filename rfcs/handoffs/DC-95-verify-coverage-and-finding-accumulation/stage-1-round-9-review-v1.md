# DC-95 Stage 1, Round 9 — Review v1

**Reviewing:** `0ec9dc0` on `dc-95-verify-coverage-and-finding-accumulation`, and the resubmitted
classified inventory.

**Accepted, with one required structural fix (§3) that is not about this round's code.** Four rows
resolved, 11 → 7. Reproduced independently.

## 1. Reproduced

Probed the catch-all in a detached worktree — replaced `refs/verify.rs`'s
`_ => Err(PrikkError::Integrity("unexplained pointer/log divergence…"))` with `_ => Ok(())`:

```
verify_repository_detects_unexplained_pointer_log_divergence ... FAILED
  expected verify_repository to reject an unexplained divergence
```

Restored, no residual diff. Gates re-run at `0ec9dc0`: fmt clean, clippy **0**, **632** prikk-store
tests — matching their 628 → 632 exactly.

Inventory arithmetic checks: 27 + 4 + 3 + 7 = 41, every section sums, §2 advances 7 → 11.

## 2. Round 8's assertion instruction does not apply here, and that is worth recording

Round 8's review required the blocking-predicate assertion — `has_publication_trust_issues()` rather
than a bare `Ok`/`Err` — and said *"adopt it for the remaining rows."* All four of this round's tests
use the `Ok(_) => panic!("expected verify_repository to reject …")` shape instead.

**That is correct, not a skipped instruction.** These four checks return `Err` — verified at
`refs/verify/scan.rs:188` for chain/sequence and `refs/verify.rs:161` for the catch-all. When they fire,
`verify_repository` returns `Err` and **no report exists to inspect**. There is no predicate to assert.

**The consequence matters more than the exemption.** For `Err`-shaped checks the test assertion cannot
distinguish load-bearing from downstream-redundant — every one produces the same panic string, which is
exactly what round 7 hit. So the *probe's* report inspection is the only evidence, and reporting
"every issue vector empty" is not a nicety here; it is the whole finding. They did it on all four.

Recording this so a later reader does not score round 9 as non-compliant against round 8.

## 3. Required: the fake-signed helper must stop being reachable by accident

**Bug 1 is a recurrence, and it is now the fourth.** `publish_ref_to_new_block`
(`verify/tests/ref_cluster.rs:161`) is a fake-signed helper. Reusing it "for speed" reintroduced the
trust confound that the classification pass already closed for rounds 1, 2 and 5 — every fixture
carrying a blocking `PRIKK-TRUST-POLICY-INVALID`, so no probe can answer the Stage 1 question.

**They caught it by probing, which is the system working.** But "remember not to use that helper" has
now failed four times across three months of rounds, and the fifth failure will look identical.

**Required, before Stage 1 closes:** either delete it — `write_trusted_block`/`write_trusted_ref_state`
have covered this need since round 6 — or rename it so the hazard is in the call site, e.g.
`publish_ref_to_new_block_fake_signed_confounds_probes`. **A name that reads as ordinary is the defect.**
This is the same reasoning as the round 6 ruling on the unreachable duplicate-identity checks: the
asymmetry favours the cheap structural guard over relying on care.

## 4. Bug 2 is the round 7 lesson applied

The sequence-gap fixture appended a chain-broken record without moving the pointer, so disabling the
chain check let `classify_ref_state`'s *"log leads pointer"* arm catch the resulting mismatch — real
evidence about a different defect. Caught by probing, fixed by advancing the pointer so only the chain
check's own internal-sequence validation is left to object.

**That is the round 7 hazard in a new guise** — a probe measuring the fixture's own defect rather than
the check under test — and recognising it from a confusing probe result rather than a clean one is the
harder version.

## 5. A gap the inventory's method cannot surface, found by RFC 101

RFC 101's §5.2 trace found that **`verify` never reads `refs/received/`** — absent from every
`verify*.rs` file. Imported ref pointers are outside the verification surface entirely, and are not
rebuildable once the source bundle is gone. It is now a `FINDINGS.md` row.

**The classified inventory does not carry it, and by construction cannot.** The inventory enumerates the
checks `verify` *has* and asks whether each is tested. A directory `verify` never looks at produces no
check, so no row. **That is a limit of the method, not an execution failure** — but a reader will take
the inventory as the map of `verify`'s coverage, and it is not.

**Required:** the inventory states this scope limit explicitly — that it enumerates *existing* checks,
not *required* ones — and names `refs/received/` as a known gap sitting outside its method. One
paragraph. Without it the artifact overstates what it proves, which is the failure Stage 1 exists to
prevent.

## 6. Standing

- **Round 9: accepted**, subject to §3 and §5.
- **Seven rows remain:** 1 in §2 (`LEGACY-LOG-LEADS`, needing the format-1 flip), 4 in §4, 1 in §5,
  1 in §7.
- **Round 10** next: `LEGACY-LOG-LEADS` closes §2, or take §4's four as a block — their call.
- Green three-platform CI before any merge.
