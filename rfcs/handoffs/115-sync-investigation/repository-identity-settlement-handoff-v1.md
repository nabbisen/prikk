# Repository identity — settled: repositories are anonymous

**Base:** current `main` (`080346d`, CI green on all 12 jobs). **Under `003-landing-work-on-main.md`.**

**This records a ruling the shipped design already made; it does not make a new one.** No new RFC:
RFC 115 §2.4–§2.7 decided this, and `seal_from_accepted.rs`, `tag_travel.rs`, and the sync wire have
implemented it consistently ever since. What is missing is that it is **written down as settled**, so
two ROADMAP themes stop being treated as open and one type code stops implying otherwise.

**I had this wrong.** I told the owner repository identity was an open question gating peer trust and
quarantine. It is not, and reading the sync implementation rather than the theme descriptions would
have told me so. The correction is the point of this increment.

---

## 1. The ruling

**Repositories are anonymous. Identity lives in signer keys and in patch ids — never in a repository.**

**prikk has no peers. It has artifacts.** `crates/prikk-cli/src/sync.rs` opens with *"No network. No
socket. No new dependency."* Every sync subcommand reads and writes files. There is no session, no
remote party, and nothing to authenticate as a repository.

**An artifact asserts nothing binding.** From `seal_from_accepted.rs`'s own doc: *"A claim **never
gates admission and never confers trust**… the receiver applies the claimed order and either it
produces a valid state — which the receiver then seals under their **own** key, `verify_signer_trusted`
unchanged and still gating — or it does not… A hostile or simply wrong order **cannot forge a
state**."*

**The evidence, checked rather than assumed:**

- `RecognitionClaimPayload` carries `block_id`, `patch_ids`, `parent_block_ids` — **content ids only.**
- `SyncSummaryRefEntry` carries `ref_name`, `digest`, `patch_count` — **no originator field.**
- Tags travel and are **adopted under the receiver's own key** (RFC 117).
- Trust is local (`trust maintainer add`), gated through `GatedOperation`.

**So "what is a remote permitted to assert?" has an answer: nothing that binds the receiver.** The
receiver is the sole authority over its own store.

## 2. Record it

Write the ruling into **`docs/src/reference/trust-threat-model.md`**, which already holds the trust-gate
and tag-adoption positions. **State the four evidence points above as the reason**, not as assertion —
a reader must be able to check it the way this handoff did.

**Say explicitly what it forecloses**: there is no repository identifier to spoof, no peer to
impersonate, and no origin field a receiver could be fooled by, **because none exists**. That is a
security property worth stating, not an omission to apologize for.

## 3. Delete `ProjectGenesis` — the last artifact implying otherwise

Type code `0x0A`. **No payload module** (`crates/prikk-object/src/payload/` has none), **no admitted
schema** (`admitted_schemas` returns `None`), refused on every path, and **nothing can construct one.**
Its only effect today is to imply a repository-level genesis object the design has rejected.

Known sites — **re-derive and report anything I missed**: `id.rs` (enum, `0x0A` decode arm, label),
`vectors.rs`, `vectors/hard.rs`, `layout.rs:870` (`"genesis"` directory name), `format.rs:43`,
`file_codec/tests.rs`, `format/tests.rs`, `signature_contract_tests/vectors.rs`.

**Retire the code, do not free it.** Record `0x0A` as permanently unavailable for reuse, in whatever
surface allocates type codes. **There are 245 codes free; the benefit of reuse is zero and the cost of
a collision is unbounded.** One line, and it is the difference between a deletion and a hazard.

**`vectors/hard.rs` pins `(ObjectType::ProjectGenesis, 0x0A)` and is a frozen RFC 114 surface.**
**Report how removing a code from it interacts with Gate A and the format-stability gate.** **If it
turns out the code genuinely cannot be removed from a frozen vector set — stop and report that**, and
I will settle for renaming it to something that does not imply a design we rejected. **Do not force it
through.**

## 4. Dissolve the two themes

- **Quarantine policy — delete the theme.** It presupposes untrusted objects entering the store and
  needing to be held. **Nothing enters un-adopted**: content the receiver does not seal under its own
  key is simply not in its history. `.prikk/quarantine` was already removed from the layout, and
  nothing writes to it. **There is no halfway state to quarantine.**
- **Peer trust — delete the theme.** Its open question was *"a peer claiming a ref advanced is a new
  authority question."* It is not a new authority question: the claim is a hint, the receiver
  re-verifies and re-seals locally, and no authority transfers. **If you find a residual this ruling
  does not answer, say so and leave the theme with only that residual** rather than deleting a live
  question.

Full section deletion, per the `10a2a13` / `3717220` precedent.

## 5. Out of scope

- **Any change to sync, trust, or sealing behaviour.** This increment records and removes; it must not
  alter what anything decides.
- **RFC 115's ruling itself.** Not reopened.
- **Adding a repository identifier of any kind**, including "just for diagnostics."
- **Conflict arbitration**, which is a separate theme.

## 6. Controls

1. **Nothing could construct a `ProjectGenesis`** — show it, do not assert it. The absence of a payload
   module is evidence; a search for construction sites is proof.
2. **The type-code retirement is load-bearing** — show that an attempt to reuse `0x0A` is refused or
   flagged, and quote it. If your chosen surface cannot express that, say so.
3. **No behaviour changed** — full suite green, and say whether the count moved and why.
4. **Gate A and the format-stability gate still pass**, and say what they now cover that they did not.

**Quote every failure.**

## 7. What to report

1. **Your re-derived `ProjectGenesis` site list**, including anything I missed.
2. **How the frozen-vector surface handled the removal** (§3) — the part most likely to force a stop.
3. **Where the type-code retirement lives**, and control 2's evidence.
4. **Whether the peer-trust theme has a residual** (§4), and your reasoning either way.
5. All four controls (§6), quoted.
6. **Full gate set against the exact commit, after the last edit**, including `mdbook build` — this
   touches `docs/src/`.
7. **Every numbered requirement's disposition, including ones that went without incident.**
8. Anything here that was wrong. **§1 is a ruling, but its four evidence points are checkable — if any
   is wrong, the ruling is wrong, and I would rather learn that now.**

**Stop and escalate, do not guess**, if: `0x0A` cannot be removed from the frozen vectors (§3); any
path can construct a `ProjectGenesis`; or you find something on the sync wire that **does** identify a
repository — **that last one would refute §1 outright and everything else here should stop.**
