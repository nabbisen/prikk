# RFC 116 stage 2 — the negotiation artifacts and the delta: implementation handoff

**Design:** `rfcs/handoffs/116-sync-negotiation/design-v1.md` — **§1 (N1), §2 (N2), §4 (N4) and §5 (N5)
govern. Read §1.2's 2026-08-20 amendment: negotiation and exchange are strictly per ref, not a union.**
**RFC:** `rfcs/accepted/116-sync-negotiation-and-transport.md` (ACCEPTED, both rulings).
**Base:** current `main` (`346264e`). **Precedes stage 3, the sender side.**

**Nothing here touches the network.** `prikk-store` stays bytes-in, bytes-out (RFC 116 ruling 2). No new
dependency; the workspace's third-party runtime surface stays at five crates.

**And nothing here may construct a `RecognitionClaimPayload`** — see §6. Stage 3 is the first claim
producer, and creating one early closes a schema window two amendments have already needed.

---

## 1. What to build

Two artifacts and one computation.

| Artifact | Magic | Scope | Contents |
|---|---|---|---|
| **Sync summary** | `PSYNCSU1` | **all branch refs**, one message | per ref: name, `PatchSetDigest`, patch count |
| **Have-list** | `PSYNCHV1` | **exactly one ref** | ref name, `PatchSetDigest`, the full patch-id list |

**This refines the design's §1 table, which could be read as allowing many refs per have-list.** It does
not: the summary is repository-wide because its whole value is one cheap comparison; the have-list is
**one ref**, because §1.2's amendment makes exchange per-ref and a multi-ref have-list would invite a
union artifact back.

**Both are representational, not frozen** (RFC 114 §3) — they carry no identity of their own. Say so in
each module doc, and do not read it as licence for carelessness.

Model both on `patch_exchange/artifact.rs`: same framing primitives (`push_u64`, `push_bytes_u64`,
`ByteCursor`), same bounds-before-decode discipline. **Do not invent a second encoding style.**

## 2. Scope of refs — branches only

**Ruled: the summary covers `heads/*` only. Tags and `remotes/*` are excluded.**

- **`remotes/*`**: `compute_patch_set_digest_for_ref` already refuses it by name, before any lookup, for
  a stated reason (RFC 115 Stage 1). Do not work around that refusal.
- **Tags**: excluded deliberately, and this needs saying because it is a *capability* decision rather
  than an oversight. `seal_from_accepted_claim` calls `validate_local_branch_ref` — **a tag cannot be
  sealed onto.** Including tags in a sync summary would report differences nothing in stages 2 or 3 can
  act on, which reads as a broken feature rather than an absent one. **Tag sync is its own question,
  recorded, not answered here.** State the exclusion in the module doc with this reason.

## 3. The delta (N4)

```
delta(ref) = patch_ids_reachable_from_block(sender_tip(ref))  ∖  have_list.patch_ids
```

Both operands already exist: `patch_ids_reachable_from_block` is public and exported, and the have-list
supplies the subtrahend. **Return the ids, sorted.** Do not build an artifact here — that is stage 3.

**A ref present in one repository and absent in the other is not an error** (design §5 item 6). Ref
sets differ legitimately. Report the asymmetry in the comparison result; refuse nothing.

## 4. The self-consistency check (§1.3) — the one refusal that is genuinely new

`PSYNCHV1` carries **both** a digest and the list it summarises. **The reader recomputes the digest over
the list it was actually given and refuses on mismatch.**

The redundancy costs 32 bytes and converts a truncated or reordered list into a **refusal** rather than
a silently wrong delta. Reuse `compute_patch_set_digest` unchanged — it already refuses unsorted input,
so a list that is not sorted-and-unique fails there rather than needing its own check.

## 5. Security properties (N5), as tests with controls

Each needs a test **and** an observed-failing control. **A refusal nobody has seen fire is not evidence.**

| # | Property | Control |
|---|---|---|
| 1 | Reading a summary or have-list changes **no** state | Assert the repository is byte-identical across a read — object containers, ref state, trust policy |
| 2 | Declared counts are bounded **before** decoding | Declared count over the limit → rejected on the integer, not after allocation |
| 3 | A have-list whose digest disagrees with its own list is refused | Truncate the list, keep the digest → refusal |
| 4 | A summary omits `remotes/*` and tags | Build a repo with both → neither appears |
| 5 | A ref in one side only is reported, not refused | Asymmetric ref sets → both directions reported |
| 6 | The delta is exactly the set difference | Sender-only, receiver-only and shared patches → only sender-only ids returned |
| 7 | Total byte length is bounded before decoding starts | Oversized input → refused before parsing |

**Row 1 is the one to get right.** It is the property that makes "negotiation is safe to run against an
untrusted counterpart" true, and its failure would be invisible in normal use.

**On controls:** mutate **the narrowest line that should break the claim.** A control reverting two
things at once reports success while leaving one untested — the most repeated finding in this project's
reviews.

## 6. Out of scope — and one of these is load-bearing

- **Constructing any `RecognitionClaimPayload`.** Stage 2 needs none: the delta is patch ids, and claims
  enter only when stage 3 builds an artifact. **Confirm explicitly in your report that this increment
  introduces no claim producer**, per the same check the N3 amendment carried. Creating one here would
  close the free-schema-amendment window early.
- **Building or sending a `PEXCH001`.** Stage 3.
- **Any transport, protocol, socket, or new dependency.** RFC 116 ruling 2.
- **Tag sync** (§2). Its own question.
- **Set reconciliation** — Bloom/IBLT. RFC 116 §3(iv): revisit on measurement.
- **CLI wiring**, unless trivial. Stage 3 kept these surfaces at `prikk-store` level; follow that and
  say in your report if you think the boundary should move.

## 7. What to report

1. Control output for each row of §5 — actual failure text, and the single line mutated.
2. **For row 1 specifically:** what you compared, byte-for-byte, to prove nothing changed.
3. **Confirmation that no claim producer was introduced** (§6).
4. The **full gate set against the exact commit, after the last edit**: `cargo fmt --all -- --check`;
   `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`;
   `cargo test --workspace --locked`; `cargo +1.85.0 test --workspace --locked`; `git diff --check`;
   `cargo audit --no-fetch`; release-policy `check` / `boundary-check` / `reference-check`.
   Cross-target clippy pair only if this diff contains `#[cfg(target_os)]` — check this diff.
5. Test counts before and after, per crate. **`snapshot.txt` must not change** — no schema here.
6. Anything here that turned out to be wrong. **Say so plainly.** Several of my handoffs this month
   contained an error found by building against them, and each was worth more than the parts I got right.

**Stop and escalate, do not guess**, if: §2's branches-only rule blocks something that ought to work;
the per-ref shape in §1 turns out to make a real sync awkward in a way §1.2's amendment did not
anticipate; or the delta in §3 cannot be computed from the exported primitives without new traversal
code.
