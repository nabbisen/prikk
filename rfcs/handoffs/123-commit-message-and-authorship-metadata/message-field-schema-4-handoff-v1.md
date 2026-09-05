# RFC 123 — the commit message becomes evidence: `PatchPayload.message` at `Patch` schema 4

**RFC:** `rfcs/proposed/123-commit-message-and-authorship-metadata.md` — the ruling is §6/Status
(Option A, message-as-evidence, owner 2026-09-01); **the design is §8, and it is settled input, not a
starting point for re-derivation.**
**Base:** `main` at `10c9899`. **Release grouping authorized 2026-09-05: schema 4 ships alone.**

**The field is the easy part. The schema bump is a second one-way compatibility break within a week
of the first, on every commit — §4 is the part of this handoff that carries the real work.**

---

## 1. What to build

**`message: Option<String>` on `PatchPayload`, canonical tag 6, `WireType::String`, emitted only when
`Some`, only at `Patch` schema 4 and above.** Tags 1-5 are taken (2 permanently retired), the writer
emits in ascending tag order, so nothing existing moves.

Mint `PATCH_MESSAGE_SCHEMA: u32 = 4` beside `PATCH_PARENT_IDS_RETIRED_SCHEMA` and
`PATCH_TEXT_SPAN_V2_SCHEMA` in `payload/patch.rs`, with a doc comment in the same house style — those
two are the model for what a schema constant's doc must explain.

**`admitted_schemas(ObjectType::Patch)` becomes `[1, 2, 3, 4]`** (`prikk-store/src/format.rs:40-44`).

**Both authoring sites carry the message through:**

| Site | Command |
|---|---|
| `worktree_patch/node_authoring.rs:550` | `prikk commit --from-worktree` |
| `patch_inverse.rs:141` | `prikk rollback-draft --append-inverse` |

Both already mint `PATCH_TEXT_SPAN_V2_SCHEMA` unconditionally; both move to `PATCH_MESSAGE_SCHEMA`.
Both already receive a `-m` value that the CLI has validated.

## 2. Three design decisions that are settled — implement them, do not improve them

**2.1 Optional, not required (§8.2).** `-m` is mandatory at both sites, so "absent" will not occur in
practice today. **Keep the field optional anyway.** RFC 113's Git/Subversion/CVS import must be able
to represent a commit that genuinely had no message; Git permits an empty one. A required field would
force a fabricated message into a signed object.

**2.2 No length bound (§8.3).** The same object already carries `replacement_text` and `old_span_text`
(`payload/patch/operations.rs:179,185`), both unbounded user bytes; transport limits already bound
untrusted input; and a bound inside the identity surface is permanent. **Do not add one, and if you
think one is needed, report the reason rather than adding it** — a bound is a schema-5 decision, not
an implementation detail.

**2.3 `verify` gains nothing (§8.5).** The message is inside the id preimage, so tampering is caught
by the existing object-id check and malformedness by `validate()` at decode. **Add no message-specific
check, no new report line, no new verification path.** A redundant check reads as assurance and
provides none.

## 3. The one invariant that needs enforcing

**`PatchPayload::validate()` must reject `Some("")`**, so "absent" and "empty" never both mean *no
message*. It runs inside `encode_canonical`, so this is enforced on write; make sure the decode path
reaches it too, because a hostile or merely wrong object is exactly the case it exists for.

**The format rejects length-zero only.** The CLI keeps rejecting whitespace-only (`args.rs:463`,
already there, unchanged). That split is deliberate: the format rule is permanent and should be the
simplest thing that removes the ambiguity; `trim()`'s Unicode semantics stay an interface concern.

## 4. The compatibility break — demonstrated, never asserted

`PatchPayloadFieldCursor` **refuses unknown tags** (`payload/patch.rs:178-183`), so there is no
skip-unknown path. Every patch authored at schema 4 is unreadable by every earlier release, **on
every commit, not only ones that carry a message**.

**0.31.0 set the standard and it is binding here.** That release proved its own break against a
locally built `0.30.0` binary rather than reasoning about it, and captured the exact refusal text.
Do the same:

1. **Build `0.31.1` from its tag** in a scratch worktree.
2. **Author a commit with the candidate build**, then have the `0.31.1` binary read that repository.
   Capture the verbatim refusal.
3. **Repeat through `bundle export` → `bundle import`** — a bundle written by the candidate, offered
   to the `0.31.1` binary. This is a separate path and 0.31.0's own evidence covered both.
4. **Confirm the reverse direction still works**: the candidate build reads a repository written by
   `0.31.1` — schemas 1/2/3 stay admitted, which is the half RFC 114 actually guarantees.

**Report all four verbatim.** If any refusal message is unclear about *why* the object was rejected,
say so — an accurate failure is the whole value of failing closed.

## 5. Conformance vectors — additions only

`crates/prikk-object/src/vectors/snapshot.txt` is a generated identity snapshot, 22 rows today,
regenerated with `PRIKK_REGEN=1`. `hard.rs` holds the hard FDD vectors and **is never regenerated**.

**Add a schema-4 `Patch` row.** Then:

> **Regenerate, and prove the diff is additions only.** `snapshot.rs`'s own doc warns that
> regenerating during an identity-preserving change destroys the only signal that the change was
> identity-preserving. This change is *not* identity-preserving for new objects — a new row is
> expected — **but no existing row's `object_id_hex` may move.** Show the diff in the report.

If an existing row changes, that is a stop-work finding: it means an existing object's bytes moved,
which RFC 114 forbids.

## 6. The CLI — and the note that becomes a lie

**`main.rs:178` prints** `note: the message is validated but not stored -- it will not appear in
`prikk log`; persisting it is a later increment`. **It becomes false the moment this lands. Remove it
in the same commit**, along with its test (`crates/prikk-cli/tests/rfc123_message_not_stored_note.rs`).

**Removing the note obliges showing the message.** `prikk log` is block-oriented today — it prints
`patches: <count>` (`output/worktree.rs:73`) and no per-patch detail at all, so a stored-but-invisible
message would leave the user exactly where the note said they were.

**Add per-patch lines under each block.** The exact formatting is yours (RFC 123 §8.8 says so). Two
things are not:

- **A patch at schema 1/2/3 shows no message line at all** — not `message: <none>`. Absence is the
  truth; printing a placeholder invents a distinction between "had none" and "could not have one".
- **Do not truncate silently.** If you truncate for width, mark it.

## 7. Controls

1. **Round-trip**: encode → decode → compare, for `Some(msg)` and for `None`, including a message with
   multibyte UTF-8 and one with a newline.
2. **`Some("")` is refused** at decode, not only at encode. Construct the bytes directly rather than
   going through the encoder, which will not produce them.
3. **Tag 6 at schema 3 is refused** — a schema-3 envelope carrying tag 6 must not decode.
4. **The id changes with the message**: two patches identical but for their message text have
   different ids. One line, and it is the whole claim of "message as evidence".
5. **§4's four demonstrations**, verbatim.
6. **§5's snapshot diff**, shown.

## 8. Gates

The full set from `rfcs/EXECUTION-ORDER.md` §6 rule 9, run as the last action against the exact final
commit: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features --locked
-- -D warnings`; `cargo test --workspace --locked`; `cargo +1.85.0 test --workspace --locked`;
`cargo +1.85.0 check --workspace --all-targets --locked`; `git diff --check`; `cargo audit
--no-fetch`; `RUSTDOCFLAGS="-D rustdoc::private_intra_doc_links" cargo doc --workspace --no-deps`;
release-policy `check`, `boundary-check`, `reference-check`.

**Report the test count before and after**, per crate that moves. Cross-target clippy only if your own
diff introduces `#[cfg(target_os)]` — check the diff rather than inferring from the change's shape.

## 9. Out of scope

Author display name (§5). `blame`, `show`, templates, trailers (§7). Any attempt to attach messages to
patches already sealed — impossible here by design, and it must not be implied to users as a future.
**Any other `Patch`-shape change**: the owner authorized schema 4 shipping alone. If you find another
change that wants this bump, **stop and report it** — batching is a release decision, not an
implementation one.

## 10. Reporting

Per `.git-exclude/tasks/dev-team/003-landing-work-on-main.md`: commit locally on `main`, do not push,
report to `.git-exclude/review-request/`. Include §7's six control results, §4's four verbatim
captures, §5's snapshot diff, and every departure.

**This increment needs a `CHANGELOG.md` entry led by the break**, in the shape 0.31.0's used. Write it
or say you have not — the architect will not assume either way.
