# `trust-threat-model.md` — tag-adoption trust semantics: implementation handoff

**Base:** current `main` (`93c0b53`, CI + Docs green). **Under `003-landing-work-on-main.md`.**
**Origin:** reported out-of-scope by the reference-page increment (`93c0b53`), correctly not fixed there.

**One file.** But **not the same kind of work as the last five increments.** Those corrected sentences
that had gone wrong. **Nothing on this page is false** (with one item to adjudicate, §3). The defect is
**absence** — and on a threat model, absence is the more dangerous failure, because a reader takes the
page's silence as "there is no boundary here."

---

## 1. The confirmed omission

**RFC 117 shipped a trust act, and the threat model does not mention it.** `sync adopt-tag` turns a
received tag into a local one **under the receiver's own maintainer key**, as a **separate, explicit
act** — a trust decision, made by a person, with refusals attached.

**Verified, not inherited:** the page contains five occurrences of *"adopt"* — **all five are
MAINTAINER key adoption** (`prikk trust maintainer add`, lines 86-89, 151). **None is tag adoption.**
The `tag` substring hits are `mutation`/`stage`. **The topic is genuinely absent.**

## 2. Semantics to document — starting points, derive each from source

`crates/prikk-store/src/tag_travel.rs`. **These are where I looked, not a specification to copy:**

- **Adoption is receiver-signed.** `adopt_tag` takes a `MaintainerSigner` and creates a *local* tag. The
  module doc states outright that **the sender's tag and the receiver's tag are different objects.**
- **Arrival is not adoption.** A received tag sits in the received namespace and advances nothing until
  someone runs the command. **Trust does not expand on receipt.**
- **It refuses on ambiguity.** *"N received tags are named X, refusing to pick"* — the tool declines to
  choose rather than guessing. **A refusal is a threat boundary and belongs on this page.**
- **It refuses to name history you do not hold** — *"patch set is not held locally yet."*
- **A tag names a patch set, not a block** (RFC 117), because blocks diverge across repositories. **This
  is what makes cross-repository tag identity meaningful at all**, and it has trust consequences worth
  stating.

**Read the functions. If any of the above is wrong, say so** — §7 of every handoff I have written in
this arc has asked that, and four of them needed it.

## 3. Adjudicate, do not assume — line 14-15

> *"there is no key rotation, hardware signing, remote trust, **sync trust**, or stable migration policy
> yet"*

**Two readings, and I am not ruling between them:**

- **"No sync trust semantics"** — **false.** They exist and are enforced: verified on arrival, trust
  never expanding on receipt, receiver sealing under its own key.
- **"No sync trust *policy*"** — **true.** There is no configurable statement of what a peer may assert;
  `ROADMAP.md`'s **Peer trust** item is explicitly open.

The list it sits in is all *policy* absences, so the second reading is probably intended. **But the
ambiguity is only harmful because §1's positive semantics are missing** — a reader meets *"no sync
trust"* with nothing to weigh it against. **Fixing §1 may resolve this on its own. Decide, and say
which.**

**If you conclude it is false rather than ambiguous, that is a finding — report it plainly.**

## 4. Where it goes

The page's existing structure decides this; **do not invent a new top-level section if an existing one
fits.** `## Trust Roots and Roles` and `## Threat Boundaries` are the obvious candidates — **read both
and place it where it belongs.**

**`## Claim-to-Source Anchors` is a page convention: every substantive claim carries a row citing its
source.** New claims need new rows. **Match the existing row style exactly.**

## 5. Do not overclaim — the constraint that matters most here

This is a **threat model**. Anything written here is a security claim a reader may rely on.

- **Do not state a guarantee no code enforces.** If adoption's protection is "a person had to run a
  command," **say that** — do not dress it as a cryptographic property.
- **Carry the standing limits across**: criterion 5's **trust-on-first-use** (continuity of authorship,
  not authenticity of first contact) applies to transported tag material as it does everywhere else.
- **Adoption does not verify who the sender is.** It verifies what arrived and re-signs it locally. **If
  that is right, the page must say it** — it is exactly what a reader would otherwise assume.

## 6. Out of scope

- **Every other file**, including the other 11 reference pages — just adjudicated at `93c0b53`.
  **Report contradictions; do not edit.**
- **No code.** If the page should say something the code does not do, **report it** — do not implement.
- **`MILESTONES.md`, `ROADMAP.md`, `README.md`, the badge.**
- **`path-safety.md:207`'s stale anchor citation** — the other open finding from `93c0b53`; **not this
  increment.**

## 7. What to report

1. **What you added, where, and the source for each claim** — function and file, not an RFC's prose. An
   RFC says what was intended; this page must say what is enforced.
2. **Your §3 decision**, with reasoning, and whether §1's fix resolved it.
3. **Anything in §2 I got wrong.**
4. **Every anchor row added** (§4).
5. **Anything you deliberately did not claim** because no code enforces it (§5). **That list is a real
   part of the deliverable on a threat model.**
6. **Full gate set against the exact commit, after the last edit**, plus `mdbook build`.
7. Test counts — **expected unchanged**.

**Stop and escalate, do not guess**, if: documenting a boundary would require asserting a guarantee you
cannot find enforced; §3 turns out to be a false claim needing a wider correction than one clause; or
you find **other** shipped trust-relevant behaviour this page omits — **that would make this a pattern
and a larger increment**, and it is the finding I would most want to know about.
