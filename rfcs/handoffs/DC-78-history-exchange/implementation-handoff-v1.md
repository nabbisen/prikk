# DC-78 History Exchange — Handoff v1

**Cleared for §4's investigation only.** Accepted by the project owner 2026-08-09, at
`rfcs/done/DC-78-HISTORY-EXCHANGE.md`. **Authored by** the architect.
**No design, no production code, until §4 is answered and reported.**

## 1. Sequencing

**DC-76 has priority** — it is in design and it is the increment the owner's cross-platform goal depends
on. §4 here is read-only analysis and cannot collide with it, so **you may start whenever you judge you
have the capacity**; you have called sequencing well twice, and this is yours to call.

## 2. What this increment is, in one line

**A distributed VCS that cannot distribute.** Nothing in the tree exchanges history between
repositories. This is status-claim criterion 1 (`MILESTONES.md`) — the only one of six with no increment
behind it, and on the architect's reading the largest single thing between prikk and dropping the
"early implementation" badge.

## 3. Two things already ruled, so you do not re-litigate them

- **§2 accepted: scope to exchange, not transport.** Exchange is *what moves and what the receiver
  verifies*; transport is *how bytes travel*. A verifiable subset written to a file needs **no network
  and no new dependency** — and `prikk-store` may depend on exactly `getrandom` and `rustix`, so any
  transport is an `ALLOWED_THIRD_PARTY` decision. **Do not design transport.** If the investigation shows
  exchange is incoherent without it, **report that** — it is a finding, not scope to absorb.
- **§3.1 is a starting position, not a ruling.** It exists so you attack a stated proposition instead of
  a blank page. **Test it. If it is wrong, saying so is the deliverable** — the same standing rule that
  has already corrected the architect five times this cycle.

## 4. Answer question 2 first, and question §3.2 early

**Q2 — against whose keys does a receiver verify?** If distributed trust has no acceptable answer, every
other question here is wasted work. §3.1's proposition to test: reception needs no trust because objects
are content-addressed; **authority is the only thing needing a decision**, asked once per key at
adoption rather than per object at reception; first contact handled by trust-on-first-use **recorded and
thereafter enforced**.

**The line §3.1 draws, which is not up for testing:** received history must never be trusted by default
for convenience, and must be **distinguishable at the object level, permanently and non-strippably**.
That is the RΔ5 Git-import ruling of 2026-08-02 applied to the same defect shape. A design that loses
that distinction is wrong regardless of its other merits.

**§3.2 — is DC-53 a prerequisite?** `verify` performs **no** cryptographic verification of author
signatures; the product's only crypto verification call site is a policy signature
(`crates/prikk-store/src/trust.rs:215`). A receiver may be unable to meaningfully check what it was sent
until that exists. **Settle this early** — it decides whether this increment can proceed at all right
now, and two of the six status-claim criteria may be one chain rather than two items.

## 5. Also worth resolving, and easy to miss

`verify` re-derives state roots by walking lineage to genesis. **A receiver holding only a suffix of
history cannot do that.** Either exchange is always genesis-complete, or lineage horizons acquire a
meaning they do not have today. §3's question 5.

## 6. Report, in §4's discipline

Write `prerequisite-questions-v1.md` beside this file. **Answered from the code and the requirements,
before any design** — the pattern that has widened the recorded scope in five consecutive increments.
State plainly whether criterion 1 is satisfied by exchange alone or needs transport, because that
determines whether this increment moves the badge.
