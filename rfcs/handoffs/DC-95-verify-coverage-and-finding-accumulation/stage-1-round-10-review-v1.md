# DC-95 Stage 1, Round 10 — Review v1

**Reviewing:** `8b7867f` on `dc-95-verify-coverage-and-finding-accumulation`, and the resubmitted
classified inventory.

**Accepted, no conditions.** §2 closes. 28 resolved, 6 remain. The classification is
**downstream-redundant** and I verified it precisely rather than by the test's exit status.

## 1. Reproduced, and the line number is the evidence

Forced the format-1 branch off — `if false && format == RepositoryFormat::LegacyV1` — and ran the test:

```
verify_repository_detects_legacy_log_leads_under_format1 ... FAILED
  panicked at ref_cluster.rs:1044:5
```

**Line 1044 is the code-specific assertion. Line 1042 is
`assert!(report.has_blocking_ref_publication_issues())`, and it passed.** So with the format-1 label
gone, the repository is *still refused* — the format-2 sibling catches the same defect. Downstream-
redundant, exactly as reported.

**This is why round 8's assertion instruction was worth insisting on.** Had the test asserted only
`Err`/`Ok`, the probe would have shown a failure and told me nothing about which of the two claims was
true. The ordering of the two assertions is what makes the panic line itself the proof.

Gates re-run at `8b7867f`: fmt clean, clippy **0**, **633** prikk-store tests — matching 632 → 633.
Worktree removed, primary tree clean. Inventory arithmetic: 28 + 4 + 3 + 6 = 41, §2's remaining column
1 → 0.

## 2. The `require_retained_evidence` finding is the round's real contribution

Their first fixture produced `PRIKK-VERIFY-REF-DIVERGENCE` with *"pointer/log divergence is not proved by
matching retained active state and trust"* — not the expected code. Tracing it showed
`verify_repository` never returns `classify_ref_state`'s raw code for this arm: every
`POINTER-LEADS-LOG` / `LEGACY-LOG-LEADS` / `POINTER-MISSING` issue is piped through
`ref_publication::require_retained_evidence`, and `mark_unproved` **overwrites the issue in place —
code, message and all** — unless four independent conditions hold.

**They treated that as evidence rather than as an obstacle**, and it is the right reading: reaching
`LEGACY-LOG-LEADS` at all requires constructing the retained evidence that proves an interrupted
publication, not merely the bare pointer/log shape. The rebuilt fixture does that — real Patch and Blob,
`write_active_ref_metadata`, `Wal::append_patch`, and a Block whose `patch_ids` match the WAL.

**Two consequences worth recording beyond this row:**

- **A code emitted by `classify_ref_state` is not necessarily a code `verify` reports.** Any future
  inventory reasoning that maps codes to checks one-to-one across this arm is wrong, and §4's remaining
  WAL rows sit close to this machinery.
- **The first attempt failing was informative because they read the message instead of patching around
  it.** Round 7's fixture bug and round 9's bug 2 were both found the same way. That is now three
  rounds where the confusing result, not the clean one, carried the finding.

## 3. The classification contradicts the code's own doc comment, and they said so

They flag that downstream-redundant is *"the opposite of the doc comment's own working assumption."* A
distinct error code with its own message reads as a distinct defence; it is a diagnostic attribution —
which side of the format divide the divergence was found on.

**Flagging that unprompted is the behaviour this Stage exists to produce.** A reader who trusts the
distinct code would over-estimate what removing it costs. Consider putting the finding in
`refs/verify.rs`'s own documentation, per the round 7 ruling that classifications belong in the file
rather than the review archive.

## 4. Choosing to close §2 first was right

Given the choice between `LEGACY-LOG-LEADS` and §4's four, they closed the cluster — matching how
`verify/objects.rs` was closed across rounds 1–4 before this cluster opened. **Finishing a cluster
converts it into a settled fact; leaving one row open in each of three clusters converts none of them.**

## 5. Standing

- **Round 10: accepted.** §2 and §6 complete. **Six rows remain:** 4 in §4
  (`wal.rs`/`rollback_verify.rs`), 1 in §5, 1 in §7.
- **Round 11** next: §4's four as a block is the natural unit, and §2's precedent argues for it.
- Green three-platform CI before any merge.
- The `accepted/`→`done/` migration remains scheduled for Stage 1's close, unchanged.
