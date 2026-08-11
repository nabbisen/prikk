# DC-91 — Evaluation Ruling: the Answer, and What It Means for DC-87 Stage 2

**Reviewing:** `.git-exclude/review-request/prikk-dc-91-prerequisite-questions-v1.md`.

**Investigation accepted. DC-91's question is answered: partial — real value, narrowly scoped.** Not
the clean "no" that would end the RFC, and not a "yes" either.

**§4.3 contains a finding sharper than anything in my RFC, and it changes the DC-87 Stage 2 picture
materially.** §3 is that. The decision it feeds is the owner's, and §5 states my recommendation.

## 1. Verified

- **`open_append_regular` is exclusive-create-then-fall-back** (`regular.rs:110-118`): `open_new_regular`
  first, `EXIST` → `open_existing_regular` with `APPEND`. So a ref log's create-if-absent branch fires
  **only on that ref's first-ever record**. This is what §4.3's whole split rests on, and it holds.
- **DC-41 Stage 1's coverage claim is accurate** — its own document states 24/24 variants covered, 0
  unexercisable, at the audit's snapshot.
- **Their §4.1 self-correction is right to prefer honesty over a tidy number.** Recording "all 24 are
  reachable, and the earlier 21-of-24 framing undercounted two ubiquitous variants I had not traced
  fully" is worth more than a precise-looking figure reached by exclusion.

**One number I could not reproduce, recorded rather than endorsed.** They measure the ref-publication
test surface at 1,387 lines / 24 tests. Counting `refs/tests/publication_recovery.rs` plus every `.rs`
under `publication_recovery/`, I get **1,335 lines / 23 `#[test]`**. Either their set includes a file
mine does not, or tests were counted differently (table-driven cases, perhaps). The discrepancy is ~4%
and changes no conclusion — the figure is an order-of-magnitude cost estimate and works as one — but I
would rather put my own count beside theirs than silently endorse one I could not reproduce.

## 2. The answer to §4.2, as asked

**Real reduction, genuinely scoped:**

- Two on-disk names in flight collapse to one, and with them the entire
  `PRIKK-VERIFY-REF-CANDIDATE-DEBRIS` state class — a class that exists *because* there are two names to
  reconcile, so a one-name design cannot produce it regardless of its own failpoint count. Declining to
  assert a replacement failpoint count, on the grounds that the primitive is an unmade implementation
  choice, is the right restraint.
- **Detectability genuinely improves** for that class: a torn slot is caught by its own checksum, from
  the record's own bytes, with no `refs/tmp/` scan. This is the one place the RFC's "self-describing
  recovery" hypothesis survives contact with the code.

**And what does not move:**

- `PRIKK-VERIFY-REF-POINTER-LEADS-LOG` is untouched, because it is a fact about the *joint* relationship
  between two records advancing in sequence, not about the pointer's storage format. **That is arguably
  DC-38's central concern**, and a pointer-only change cannot reach it.
- Object persistence, directory creation, log append, WAL cleanup: unchanged.
- **Recoverability cannot improve.** Today's ceiling is 24/24 reachable states with independently
  audited recovery. A new design can at best equal that, and only after re-earning the audit. Stating
  it as "currently unproven, not merely unequal" is precisely right.

## 3. The decisive finding, and it is theirs

**A pointer-only slot record makes routine seals to existing refs fully Windows-achievable, and does
not unblock new branch or tag creation at all.**

For an existing ref, both halves of a seal are existing-file operations — the pointer update by
construction, and the log append because `open_append_regular` only creates on the first record. Both
are achievable on Windows per DC-87 Stage 2's own §3.2 verdict.

For a **new** ref, the pointer's first write and the log's first record are required in the same
transaction. Migrating the pointer removes one first-appearance requirement; the log's fires at the same
moment and is completely untouched. **DC-38's invariant stays exactly as unenforceable on Windows for
new-ref creation as it is today.** And branch and tag creation are ordinary recurring operations here —
DC-60, DC-61 and DC-63 made them so — not an `init`-only event.

They also showed the log cannot simply adopt the same shape: a two-slot record represents *the current
value of one thing*, and a ref log is an append-only sequence — DC-38's audit trail. Making it
slot-shaped means either discarding history beyond two entries (a guarantee change, out of scope) or
keeping an unbounded structure anyway, which needs first-appearance durability for itself.

**The generalisation, which neither report states and which I think is the real conclusion:** this was
never a fact about the pointer's shape. **Any design that keeps per-ref files has a first-appearance
problem at ref creation.** So the question for Stage 2 is not "does the pointer's shape avoid it" — it
is whether *any* shape does, and that is a much larger increment than DC-91 modelled.

## 4. What this does to DC-87 Stage 2's options

The owner's three options were: (1) restructure publication, (2) ship Windows with a documented weaker
invariant, (3) do not ship Windows mutation.

**Option 1's payoff is now measured, and it is partial while its cost is full.** It buys Windows-viable
seals to existing refs and one genuinely better-detected state class; it does not buy new-ref creation,
does not touch pointer-log joint consistency, and cannot improve recoverability past an already-audited
ceiling — while costing the re-earning of ~1,335 lines and 23 tests of DC-41-grade proof on the most
safety-critical machinery in the product, plus `doctor`/`verify` state-machine re-derivation and
documentation.

## 5. My recommendation, and the decision that is the owner's

**I recommend against restructuring ref publication for Windows' sake.** The payoff arrives partially
and the cost arrives in full, and "Windows can seal to existing branches but cannot create one" is a
hard product story to stand behind.

**This is not the minimality argument the owner overruled.** That argument was *do not disturb
proved-safe machinery*. This is different and survives the criterion: measured on the owner's own three
axes, the more robust design is **not clearly more robust** — it improves detectability in one state
class, leaves the central joint-consistency state untouched, and is strictly worse on recoverability
until it re-earns an audit today's design already has. Robustness outranking minimality does not decide
a comparison that comes back mixed.

**What I would keep from this.** The slot record's detectability gain is real and stands on its own
POSIX merits, independent of Windows. If it is wanted, it is a small increment on that basis alone — not
a Windows unblock, and it should never be sold as one.

**Two things remain the owner's**, and this ruling does not presume either:

1. Whether DC-87 Stage 2 takes option 2 or option 3 — or commissions the much larger "solve
   first-appearance generally" increment §3 implies.
2. Whether "branch and tag creation stay Windows-blocked while ordinary seals work" would ever be an
   acceptable interim state. They flagged it as a product judgment and declined to settle it. Correct.

## 5a. Addendum — the recommendation's scope, after the owner's maintain-and-verify question

The owner asked whether §5's recommendation holds "for the future as well as now," stating a preference
for **ease of maintenance and verifiability of security**. It is worth being precise, because §5 read
more absolutely than it was meant.

**What §5 was scoped to, and what still holds on any horizon:** do not restructure publication *for
Windows' sake*. That does not weaken with time — the first-appearance problem at ref creation is not
fixed by any pointer-shaped change, so the Windows payoff stays partial however long one waits.

**What the owner's criterion raises that DC-91 did not measure.** The evaluation scored crash-state
count, detectability, recoverability, and cost-to-re-prove. **It did not measure maintainability**, and
on a long horizon the arithmetic shifts in a way §5's "cost is full" framing understated: re-earning the
audit is paid **once**; a simpler verification story pays back for the product's life.

On the owner's two goals, from what DC-91 did establish:

- **Verifiability improves, narrowly and genuinely.** One state class becomes self-describing from a
  single artifact's checksum rather than requiring a `refs/tmp/` scan plus knowledge of the protocol
  shape. That is on-thesis for a product whose central claim is history that can be verified.
- **Maintenance surface shrinks** in one identifiable place: two on-disk names become one,
  `doctor`/`verify` lose the `CANDIDATE-DEBRIS` classification entirely, and dependence on
  directory-sync ordering — the hardest part of DC-38 to test, and much of why DC-41's matrix is as
  large as it is — is reduced.
- **The hard part is unchanged.** Pointer-log joint consistency stays exactly as complex.
- **And it is worse before it is better.** The riskiest moment for security-critical machinery is when
  it is changed. DC-34/DC-38 exist because of a real split-brain bug; "easier to verify eventually" is
  bought with "less verified right now."

**Revised recommendation: not now, but not never — and never as a Windows increment.** If it is done it
should be justified on POSIX maintain-and-verify merits alone, scoped as its own increment with the
full DC-41-grade proof budgeted up front rather than discovered mid-flight, and sequenced when the
verification path is otherwise stable, which it is not while DC-92 is open.

**A redirect worth more for the stated goal than this increment is.** If the objective is to verify
security easily, two already-registered findings do more for it, cost far less, and do not touch
safety-critical machinery:

1. **`verify` reports only the first hard error**, so a damaged repository takes N runs to enumerate N
   defects — the difference between triage and guesswork for the command that *is* the security
   assurance.
2. **Nothing proved `verify` state-checks blocks end to end.** DC-92 closes that for one path; what else
   `verify` does that no end-to-end test would notice remains open.

Both are directly about verifiability. The architect would take those before touching publication.

## 6. Standing

- **DC-91: complete.** Its question is answered; §5 criterion 3 held — no design was proposed and no
  publication code changed.
- **DC-87 Stage 2:** still deferred, now with its unblocking condition *answered* rather than
  outstanding. The deferral stops being "pending an investigation" and becomes a live decision.
- Registered in `FINDINGS.md`: the new-ref-creation split, since it constrains every future Windows
  design and is not obvious from any single document.
- **DC-92** remains blocked on its cross-target CI fix.
