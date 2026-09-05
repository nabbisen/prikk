# RFC 134 shape 1 — state the invariant, and stop calling a consistent refusal "malformed evidence"

**Authority:** `rfcs/done/134-text-span-identity-under-composition.md` **§7.4 item 1**, ruled
2026-09-04 and accepted with the RFC.
**Base:** current `main`. **Under `003-landing-work-on-main.md`.**

**Small and self-contained.** No format change, no schema, no identity change. **§8's v2 work does not
retire this** — see §1.

---

## 1. Why v2 did not close it

**v1 operations rely on the invariant permanently.** Schema 3 changes what *new* operations record;
every `EditText` already written still resolves through the positional `dup_index` path
(`locate_text_span`, `text_span.rs:215`), which is correct **only while each operation was authored
against the state its predecessors produced.**

**Nothing states that anywhere**, and nothing checks it — confirmed: `text_span.rs` contains no
mention of the invariant today.

**And the misleading diagnosis outlives v2 too.** A v2 sequence authored against a shared baseline
fails as well — the first edit changes the second's anchor context, so `locate_text_span_v2`
(`:264`) refuses on anchors instead of on a duplicate index. **Different cause, same wrong label.**

## 2. What to write down

In **`text_span.rs`'s module doc**, beside where span identity is defined:

> Every `EditText` is authored against the state its predecessors produced — never against a shared
> baseline. v1 identity depends on this because `dup_index` is recomputed against the buffer at
> lookup; v2 depends on it because anchors are.

**Say what upholds it**, so the next reader can check whether it still does: `plan_edit_text`
(`node_authoring.rs`) emits one operation per file per commit, and `current_text_for_node` resolves
through the queued-patch cache, then the stored blob, then replay.

**And say what breaks it**, because it is not hypothetical: a sequence built programmatically against
one baseline — **RFC 113's Git/Subversion/CVS import is the named case** — or a crafted or
externally-produced patch.

## 3. The diagnosis

`commutation.rs`'s `replay_sequence_order` maps `OracleFailure::Replay` to:

```rust
reason: "composed replay failed after confluence proof".to_string(),
```

**That says "your evidence is broken" when the truth is "these operations are mutually
inconsistent"** — and the wording *"after confluence proof"* asserts the state is unreachable, which
RFC 134 established it is not.

**Replace the reason with what the condition actually is.** The pairwise verdicts are sound; what
failed is that the sequence's own operations do not compose. **Keep it a refusal — do not make the
sequence replay.** That was shape 4, refused in §7.4/§7.5.

**`OracleFailure::Unknown`'s neighbouring arm is not in scope.** Leave it.

## 4. The trap that will make `main` red

**`ALLOWLISTED_EVIDENCE_ERROR_REASONS` (`algebra_properties.rs:735-736`) pins that exact string:**

```rust
&["composed replay failed after confluence proof"];
```

**Change the reason without changing the allowlist and Property B hard-fails**, because an unlisted
reason is exactly what it is built to refuse. **Both edits belong in one commit**, and the allowlist
entry's comment should be updated to name the new wording and still point at RFC 134.

**The persisted seed stays.** It reproduces the same condition regardless of how the reason reads, and
it retires only when the generator moves to v2 and the case passes for the right reason — not because
the label improved.

## 5. Gates

Full set from `rfcs/EXECUTION-ORDER.md` §6 rule 9 against your final commit — **not reproduced here**.

**Run both cross-target clippy commands** (`x86_64-pc-windows-gnu`, `x86_64-apple-darwin`) **whether or
not this diff contains `#[cfg(target_os)]`.** The previous increment carried none and still broke both
non-Linux jobs, because a helper it added had consumers only behind a pre-existing gate. **The
question is not "does my diff contain a cfg" but "does anything I added have consumers only behind
one".**

Local commit on `main`; **no push.** Report to `.git-exclude/review-request/`, stating the new reason
string, that the allowlist moved with it in the same commit, and both cross-target results.
