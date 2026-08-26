# Five accumulated currency items — the queue I let build up

**Base:** current `main` (`8e0afa7`). **Under `003-landing-work-on-main.md`.**
**Owner-authorized, and that authorization names `MILESTONES.md` explicitly** — which this project's
standing rule requires before that file may be edited.

**All five are corrections to records I maintain, and three are my own errors.** Reviewing them
independently is the point.

---

## 1. `MILESTONES.md:463` — DC-49 is no longer blocked

```
8. DC-49 Portable-Logic Platform Matrix (blocked on the M1 portability-claim correction).
```

**Stale.** DC-49's own blocking condition — the "Linux-only exercised gates" wording in
`durability-recovery.md` and `concurrency-locking.md` — was corrected at **`873caa7`** (DC-87 Stage 2).
`git log -S` on the exact replacement sentence returns that commit alone for each file.

**One nuance that must survive the edit**: the correction shipped through **ordinary development**, not
through the formal M1 hold sequence DC-49's Trigger describes — and that sequence remains dormant
(`MILESTONES.md:410`). **Do not write "the gate fired."** Write what happened.

## 2. `MILESTONES.md:334` — the M5 row

```
| M5 | Sync and Quarantine | no | — |
```

**Both halves are wrong now.** Sync is criterion 1, **MET 2026-08-22**, recorded at line 159 of the
same file. **Quarantine was dissolved** — nothing enters the store un-adopted, so there is no halfway
state to quarantine, and its ROADMAP bullet was deleted this session.

**Adjudicate what the row should say.** A milestone whose first half shipped and whose second half no
longer exists does not have an obvious status. **If `M5` as defined is no longer a coherent milestone,
say so** rather than forcing a `yes`/`no`. **Do not invent a new milestone definition** — that is the
owner's.

## 3. `ROADMAP.md` — TASK-14 is done

Its row still reads `Open`. **The page landed at `7babdb4`** (`docs/src/reference/non-goals.md`), and
the row's own completion condition — *"Reviewed non-goals page is committed and links ROADMAP as the
planning authority"* — is satisfied.

**Mark it, following the table's existing convention for completed rows.**

## 4. `ROADMAP.md` — the transport bullet over-claims against RFC 116

```
**Transport — settled by RFC 116's accepted ruling, not open.** … by design, not a gap awaiting a
future increment.
```

**RFC 116 says the opposite of "not open":**

```
:121  prikk itself stays off the network **in this increment**
:122  **If a protocol is later wanted**, it belongs in its own crate or its own binary
:131  Transport **deferred** and kept outside the verification core
```

**This is mine.** I reviewed the section this bullet sits in, then cited it in a later handoff as a
permanent ruling. The dev team caught it while building the non-goals page and correctly refused to
carry it there.

**Correct the bullet to match its own source.** Transport is **deferred**, with a stated shape for if a
protocol is later wanted. **What remains true and must survive**: `prikk-store` stays
bytes-in/bytes-out, sync-over-any-channel satisfies criterion 1, and the operator copying the artifact
is a consequence of that, not an open question about ownership.

**Do not overcorrect into "transport is planned."** RFC 116 defers it; it does not schedule it.

## 5. `docs/src/reference/trust-threat-model.md` — two problems

**(a) Line 124 cites the wrong section.** It attributes repository anonymity to *"RFC 115 §2.4–§2.7."*
**Those sections are about block divergence and canonical sealing** — §2.4 *"The consequence that must
be accepted explicitly"*, §2.5 *"Is anything actually lost?"*, §2.6 *"canonical sealing would make
blocks converge."* **None of them establishes anonymity.**

**Cite the evidence, not a document.** The settlement's strength was that it is checkable:
`RecognitionClaimPayload` carries content ids only; `SyncSummaryRefEntry` has no originator field; tags
are adopted under the receiver's own key; trust is local and gated. **Those four facts are the
citation.**

**(b) The "Current non-goals" list at line 213 conflates refused with deferred.** It names global
identity trust, remote trust, hosted forge semantics, key lifecycle management, hardware signing,
multi-maintainer thresholds, production audit policy, plugin execution, and stable repository-format
migration — **several of which this page's own earlier text calls "capabilities not yet built."**

**Split them**, using the page's own words as evidence, and **cross-link
`docs/src/reference/non-goals.md`** for the refused set. **If an item's status is genuinely unclear
from the record, say so** — do not assign it to either column to make the list tidy.

## 6. Out of scope

- **Any other `MILESTONES.md` row** — the authorization names these two items, not the file.
- **Redefining `M5`** (§2).
- **The non-goals page itself**, which is current.
- **Any code change.**

## 7. Controls

1. **Every corrected claim cites its evidence** — file, line, or commit.
2. **No claim is corrected past what its source supports** — §4 is the case: RFC 116 defers, it does
   not plan. **Quote the source beside the correction.**
3. **`mdbook build` clean**, links resolve.
4. **The open-work index gate still passes** — `ROADMAP.md` is edited.
5. **Full gate set green, count unmoved** — documentation only.

## 8. What to report

1. **Each item's before and after.**
2. **Your §2 adjudication** on `M5`.
3. **Your §5(b) split**, and anything you left unassigned and why.
4. All five controls (§7), quoted.
5. **Full gate set against the exact commit, after the last edit.**
6. **Every numbered requirement's disposition, including ones that went without incident.**
7. Anything here was wrong — **including my §4 correction**, which is me fixing my own over-claim and
   could over-correct in the other direction.

**Stop and escalate, do not guess**, if: `M5` cannot be given an honest status without redefining the
milestone (§2); or another `MILESTONES.md` row turns out stale for the same reasons — **report it, do
not fix it**, since the authorization covers these two only.
