# DC-85 — Implementation Review v1

**Reviewing:** `54d52a7` on `dc-85-merge-from-received-ref`, based on `af495c3`.
**Reviewer:** architect. **Independence:** I authored §3A's rulings, so this review is
author-adjacent on the design and independent only on the implementation. Compensated by
re-deriving every claim from source and by running my own negative control rather than
accepting the submission's evidence.

**Verdict: ACCEPT, conditional on two items in §5 — one green macOS CI run, and one
one-line amendment.** No finding disputes the increment's substance. The mandatory
criterion is met, and I proved it is met by breaking it.

---

## 1. The mandatory criterion (§3A.1) — verified by negative control

I did not take the submission's test at its word. In a detached worktree at `54d52a7` I
disabled the gate (`if false && from_is_received`) and re-ran the two new CLI tests:

```
merge_from_received_ref_with_untrusted_sealing_key_is_refused_and_writes_nothing ... FAILED
merge_from_received_ref_with_trusted_sealing_key_succeeds ....................... ok
```

The failure output is the important part. Without the gate the merge does not merely
proceed — it **completes**:

```
merged remotes/heads/main into heads/main
adopted target block: 146bea37…
adopted patches: 1
block id: b6838805…
heads/main RefState: 09ebeed2…
```

That is `heads/main` advanced to a Merge block adopting content sealed by a maintainer key
this repository was never told to trust. So three things are established at once: the gap
DC-78 Stage 3's review found was real and fully exploitable; the gate is load-bearing, not
decorative; and the positive control still passes with the gate disabled, which means the
negative test is discriminating on trust specifically and not on some incidental refusal.

The gate itself is right where it must be — `merge_execute.rs:115`, after
`verify_signer_trusted` and before the first `write_object` at :170. I confirmed the whole
evidence phase is genuinely read-only: `replay_derived_state` and `candidate_sequence`
contain no `write_file_atomically`, `write_object`, or directory creation, so "refused and
writes nothing" is a structural property, not a timing accident.

It reuses `verify_trusted_publication_envelope` unmodified — the same function `verify`
uses, which requires Ed25519, `SignerRole::Maintainer`, and a key id present in the loaded
policy, then performs one real signature verification. No parallel trust logic was
invented for this path. That was the shape §3A.1 asked for and it is what landed.

It also walks the same candidate set, not a second one: `candidate_blocks` now returns
`(ObjectId, BlockPayload)` and all three consumers — `candidate_sequence`,
`candidate_patch_ids`, `verify_candidate_blocks_trusted` — share it. §3A.1 named that as
"the shape to design toward"; it is the shape delivered.

## 2. The other three rulings

**§3A.3 (do not relax `validate_local_branch_ref`).** Honoured. `execute_merge` dispatches
on `from_ref.starts_with("remotes/")` into a new `MergeEvidenceTarget::ReceivedRef` arm;
`validate_local_branch_ref` is byte-for-byte unchanged and still the only gate on
`into_ref`. I checked the validator returns `Ok(ref_name.to_string())` — pure identity, no
normalization — so the dispatch cannot be confused by a canonicalization step.

**§3A.2 (naming asymmetry).** Carried forward exactly. The `ReceivedRef` arm omits the
local arm's `ref_state.ref_name != ref_name` equality check, with a comment stating why: a
received `RefState` embeds the origin's own name, never the local `remotes/` label.
Correct — the alternative would reject every genuine import.

**§3A.4 (nothing additional to record).** Confirmed by absence: `BlockPayload` is untouched,
and the merge block's `parent_block_ids` / `mainline_parent_id` /
`merge_baseline_block_id` are constructed exactly as DC-75 does. I verified the block the
gate checks and the block recorded as secondary parent are the same value —
`evidence.right_selector.target_block_id` feeds the gate, `candidate_patch_ids`, and
`adopted_target_block_id` alike. No TOCTOU between what was authorised and what was sealed.

## 3. The induction claim, independently checked

The gate fires only for received sources. The submission justifies exempting local sources
by induction: every block reachable from a local ref was created through this repository's
own seal or merge path, each gated at creation. That claim is only as good as the set of
ways a local ref can come to point at imported content. I enumerated them rather than
assuming:

- Every `RefStore::publish` call site: `branch.rs` (create, close), `seal.rs`, `tag.rs`,
  `merge_execute.rs`. Nothing else in production code publishes a ref.
- `branch create --from` resolves through `resolve_published_target`, which calls
  `RefStore::read_current_ref_state_id` — that reads `refs/by-id/` only, while received
  pointers live in `refs/received/`. A `--from remotes/heads/main` therefore finds no
  pointer and fails; `branch.rs` contains no `remotes/` handling at all.
- `branch close` reuses the current target and introduces no new content.
- Pointer paths are `sha256(ref_name)`, so the APFS case-folding class of bug that bit
  DC-78 Stage 1 does not apply here — `remotes/Foo` and `remotes/foo` hash to distinct
  keys and cannot collide on a case-insensitive filesystem.

Separately, a narrowed-baseline attack does not work: `prepare_merge_evidence` runs
`candidate_sequence` for **both** sides, and `candidate_blocks` errors unless the baseline
is an ancestor of the target. The baseline must therefore be a common ancestor, and by the
induction above every common ancestor is already local — so no untrusted block can be
hidden below the baseline and skipped.

The induction holds. I am satisfied the exemption is sound today.

## 4. Gates

Re-run by me in a detached worktree at `54d52a7`, not read from the submission:

| Gate | Result |
| --- | --- |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | clean |
| `cargo test --workspace --locked` | all green (602 prikk-store lib tests) |
| `cargo +1.85.0 test --workspace --locked` | all green, 41 suites, zero failures |
| `git diff --check` | clean |
| `cargo audit --no-fetch` | 179 dependencies, nothing flagged |
| release-policy `check` | 154 oracle cases passed |
| release-policy `boundary-check` | `"valid": true` |
| release-policy `reference-check` | `"valid": true` |

Cross-target clippy was not required — no `#[cfg(target_os)]` code is touched — and the
submission ran it anyway. Noted, not credited as a requirement.

## 5. Conditions on acceptance

**5.1 — Green macOS CI run before merge.** Unchanged and non-negotiable. This increment
touches the merge and import paths, both filesystem-backed. The standing rule exists
because I broke it once on DC-78 Stage 1 and shipped a security regression.

**5.2 — Restore the self-merge guard's symmetry (one line).** In `execute_merge`:

```rust
let from_ref = from_ref.to_string();
if into_ref == from_ref { ... }
```

`from_ref` is rebound to the **raw** argument, so the guard now compares a validated name
against an unvalidated one. Behaviour is identical today only because
`validate_local_branch_ref` returns its input unchanged. But `refs.rs`'s own comment on
`validate_local_tag_ref` records that NFR-SEC-03's case-collision rule is unmet for both
namespaces and tracked for later — and that is precisely the change that would introduce
normalization and silently turn this guard into a no-op for `heads/Main` vs `heads/main`.
Carry the local arm's canonical string forward and compare against that. No test change
needed; this removes a trap rather than fixing a bug.

**5.3 — A short docs addition to `docs/src/guide/merge.md`.** Nothing there is now *false*
— `--from` is never described as local-only — but this increment closes the exchange loop
end to end, and it introduces an operator-facing refusal (`no trusted MAINTAINER
signature`) whose remedy is neither obvious nor safe to guess. An operator who meets it
without guidance will reach for `prikk trust maintainer add` on whatever key the bundle
happened to carry, which is exactly the decision the gate exists to force them to make
deliberately. Two short paragraphs: that `--from` accepts `remotes/<name>`, and that
adopting the origin's maintainer key is a deliberate act of trust, not a step to clear an
error. A line in `merge-evidence.md` / `merge-plan.md` noting the previews accept received
refs too would round it out.

## 6. Recorded, not required

**6.1 — Authenticate before parse.** The trust check runs *after* `prepare_merge_evidence`
has already decoded and replayed the received side's patch operations. §3A.1's criterion
("before `into_ref` advances") is met, but the stricter ordering — check trust immediately
after resolving the source, before any attacker-authored payload is decoded — is available
at near-zero cost, since the gate needs only the baseline and the received tip's block id.
I am **not** blocking on it, for a reason worth recording rather than leaving implicit: the
same pre-authentication decode surface is already reachable through `verify` and through
the new preview commands, so moving the gate would harden this one path without closing the
class. This belongs with whatever increment gives the patch decoder fuzzing attention — at
which point it should be done properly across all three entry points, not piecemeal here.

**6.2 — A constraint on the revocation design.** The gate checks *currently* trusted, which
is right. Local merges are exempted on trusted-*at-creation*. Those two are identical only
while trust is append-only. Once revocation exists — still the largest unowned design
question — they diverge: `verify` would flag a locally sealed block whose key was later
distrusted, while a local-to-local merge would keep adopting it silently. Not a defect now.
Recorded so the revocation design must either re-gate local merges or state plainly that it
accepts the divergence.

**6.3 — The preview extension goes further than asked.** `merge_target_from_arg` routes
both `--left-ref` and `--right-ref`, so `merge-evidence` can now compute a plan whose
*left* side is a received ref — a shape `execute_merge` will never accept, since `into_ref`
must be local. Read-only and harmless; the cost is that the preview can display a plan that
cannot be executed. Not worth a code change. Worth the doc line in 5.3.

**6.4 — No store-level unit test for the gate.** The submission declined one because
`test_support::maintainer_signature()` produces a non-cryptographic `vec![5; 64]`, which
would fail a real Ed25519 check regardless of policy content — proving nothing. I checked
that claim and it is correct. Declining a test that could only ever pass or fail for the
wrong reason is the right call, and saying so explicitly is better practice than adding a
green-looking assertion. The CLI test carries the guarantee, through real signing on both
sides, and my negative control confirms it discriminates.

## 7. What this does not deliver

Unchanged from the submission's own §5, and accurate: no transport, no automatic trust
adoption on import, no change to what `verify` checks. Merge-base discovery is still
manual. The received-ref audit trail is still unowned. None of these are DC-85's scope.
