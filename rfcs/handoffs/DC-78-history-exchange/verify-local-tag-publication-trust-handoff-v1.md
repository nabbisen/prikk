# `verify` — publication trust for locally-published tags: implementation handoff

**Base:** current `main` (`053e442`, CI green). **Under `003-landing-work-on-main.md`.**
**Origin:** `.git-exclude/reviewed/tag-create-trust-gate-investigation-v1.md`, Part 2 as revised (§11-§14).

**Owner's standing requirement, 2026-08-24:** *"finally clean, safe, robust and sophisticated design.
Not 'so-so safe for now'."* **This handoff exists because the cheap option — document the gap and move
on — does not meet that bar.** The gap is real and it is closeable.

---

## 1. What is wrong, and what is not

**`Tag` is the only publication-signed object type `verify` does not trust-check.** `Block` and
`RefState` are checked at `verify/objects.rs:299`; `RefUpdate` has its own stage at `verify.rs:1029`.

**But the obvious fix is wrong, and this is the load-bearing part of this handoff.** Adding `Tag` to
`matches!(object_type, Block | RefState)` **would break sync**:

- A **received** tag lives in the **same Tag container** as a local one (`received_tag_ids`).
- Its signature is the **sender's**, under a key the receiver has deliberately **not adopted** — the
  threat model states this as designed: *"an `Unverifiable` tag is stored and reported exactly like a
  `Sound` one."*
- An unadopted signer records a `PublicationTrustIssue` (`verify/trust.rs:55`), and those **fail
  `verify`** (`main.rs:597`).

**So a blanket type-based check makes every repository that has received a tag fail `verify`** —
contradicting criterion 1's load-bearing clause, *"and both verify it afterward."* **Do not do it.**

## 2. The correct shape: trust follows provenance, not type

**A tag reachable from a local `tags/*` ref must have been published or adopted locally** — since
`053e442`, both paths gate on `verify_signer_trusted`. **Its publication trust is therefore
re-derivable offline, which is exactly what `verify` exists to do.**

**A tag reachable only from the received namespace is awaiting adoption** and is *expected* to carry a
signature this repository cannot verify.

**So: check the Tag envelope where the local ref scan already resolves it. Leave the object scan
alone.** The principle worth stating in the code: **publication-trust expectation follows an object's
provenance, not its type.** `Block`/`RefState`/`RefUpdate` never need this distinction because they are
only ever held under a key the holder is expected to have adopted; `Tag` is the one type that is not.

## 3. The trap — `ensure_ref_target_valid` is shared

`refs/verify/scan.rs:414`'s `ensure_ref_target_valid` resolves `RefKind::Tag` and looks like the natural
insertion point. **It is not. It has four callers:**

| Caller | Should trust-check? |
|---|---|
| `refs/verify/scan.rs:171` and `:376` — local ref verification | **yes** |
| `verify.rs:1350` — the **ReceivedRefs** stage | **NO** — this is the received namespace |
| `bundle.rs:445` — bundle export | **NO** — a different operation entirely |

**Putting the check inside the shared function re-creates §1's bug** by a different route, and the
`ReceivedRefs` stage would be the thing that breaks.

**Do the check at the local call sites, outside the shared function.** Do not add a boolean parameter to
a shared validator to carry a caller-specific concern — **if that looks like the only way, stop and
report; the placement is wrong.**

## 4. Tests — and one that must exist regardless

**`crates/prikk-cli/tests/rfc117_stage3_tag_travel_cli.rs` never runs `verify` — zero occurrences.**
`rfc116_sync_cli.rs` runs it on both repositories and is what criterion 1 cites as evidence, **but it
does not transport a tag.** The tag path is not covered by criterion 1's own load-bearing assertion.

**Required:**

1. **Add a `verify` assertion to the tag-travel test** — the receiver must pass `verify` **after
   receiving a tag it has not adopted**. **This is the control for §1**: write it first, confirm it
   passes on today's code, and it will catch the blanket-check mistake if anyone tries it later. **This
   test is worth having even if the rest of the increment were abandoned.**
2. **A local tag with an untrusted signer must fail `verify`.** Constructing one now requires bypassing
   `053e442`'s gate — **say how you did it**; if it cannot be constructed through public surfaces, that
   is itself worth reporting, and a store-level test is acceptable.
3. **A received, unadopted tag must not fail `verify`** — the same property as (1) at whichever level
   you can assert it precisely.
4. **Negative control**: each new assertion observed failing before the fix. **Report the output.**

## 5. Also document the reasoning (§2)

The exclusion currently reads as an oversight — **it read that way to me, and I recommended the wrong
fix because of it.** Once the local check exists, record in `verify.rs`'s own reasoning **why received
tags are exempt**, so the next reader does not re-derive it or "fix" it.

**Report whether `trust-threat-model.md` needs a matching sentence** — do not edit it here.

## 6. Out of scope

- **The object scan's `matches!`** — leave it exactly as it is (§1).
- **`ensure_ref_target_valid`'s signature** (§3).
- **`main.rs`'s error text** — the existing publication-trust message covers this.
- **`MILESTONES.md`, `ROADMAP.md`, the badge.**

## 7. What to report

1. **Where you put the check**, and why that site is local-only.
2. **Each of §3's four callers**, and confirmation the three that must not check, do not.
3. **All four tests** (§4), with the negative-control output.
4. **Whether a locally-published untrusted tag is constructible through public surfaces at all** (§4.2).
5. Your verdict on `trust-threat-model.md` (§5) — report only.
6. **Full gate set against the exact commit, after the last edit.** **Test counts will change.**
7. Anything here that was wrong. **§1 and §2 are my analysis after I had already recommended the wrong
   fix once** — I withdrew "add `Tag` to the match" only after tracing it. **Re-derive §1's five-step
   chain yourself before trusting §2's conclusion.**

**Stop and escalate, do not guess**, if: the local-only insertion point does not exist as cleanly as §2
assumes; §4.2 turns out to require weakening `053e442`'s gate to test; or the received-tag exemption
turns out to have a case §2 does not cover — **that last one would mean the provenance principle itself
needs refining, and it is the finding I would most want.**
