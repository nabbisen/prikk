# `ROADMAP.md`'s Sync section reads as unstarted work that shipped

**Base:** current `main` (`452763b`). **Under `003-landing-work-on-main.md`.**
**Origin:** flagged three increments ago and deliberately left visible rather than bundled. **This is
the last known-stale documentation surface.**

---

## 1. Do not delete this section

Every other resolved theme this session was deleted. **This one stays**, because `ROADMAP.md` already
has a convention for shipped work that still carries residuals, and two sections in the same file
follow it:

```
### Merge execution — shipped in 0.19.0 (DC-74/DC-75); residuals remain
### Cross-platform mutation — shipped in 0.21.0; two narrower Windows guarantees remain
```

**Sync has real residuals** — prikk does not move the bytes itself, "two machines" is exercised as two
repositories with **no cross-host test**, there is **no discovery or remote-tracking**, and the
operator still copies the artifact by hand. **Those are the reason the section survives. Keep them.**

## 2. The heading

```
### Sync — recorded independently, prerequisite is a threat model
```

**It frames a threat model as a precondition for sync work to begin.** Sync is criterion 1, **MET
2026-08-22**, delivered across RFC 115/116/117 in ten increments — as the section's own first bullet
says. **Head it the way the two neighbours above are headed.**

## 3. The prerequisite paragraph is stale in its framing, not its facts

> **Prerequisite … a threat model before any sync code exists.** Sync is the first capability that
> gives prikk an attack surface it does not have today — verified: zero networking crates in
> `Cargo.lock`, no networked verb in the CLI.

**"Before any sync code exists" describes a future that arrived.** And **the prerequisite was
satisfied** — `docs/src/reference/trust-threat-model.md` covers sync substantively: Core Caveats
(*"Sync and tag adoption make no new trust decision of their own"*), the trust-gated operation list
(`sync build`, `sync seal`, `sync adopt-tag`), and the tag arrival/adoption rules.

**But the verification is still true and must survive**: zero networking crates, no networked verb.
**Rewrite the framing; keep the fact.** It is the evidence for the anonymity ruling and for why no
installer or transport question arises.

## 4. The dependency note is now moot as written

> An async runtime in `prikk-store` would need a DC-51 amendment … **That is part of the sync design,
> not a discovery to be made during it.**

**Sync shipped without an async runtime**, so "part of the sync design" describes a design decision
already made. **The DC-51 constraint itself remains true** (`placement.rs` permits only `getrandom`
and `rustix`). **Keep the constraint; drop the framing that treats it as pending.**

## 5. The lead paragraph

> M5 bundles "Sync and Quarantine." **They are separable, and sync alone is at least three distinct
> questions**

**Quarantine was dissolved this session** — nothing enters the store un-adopted, so there is no
halfway state to quarantine. And **all three questions are resolved**: sync MET, multi-parent shipped
in DC-75, transport settled by RFC 116.

**The separability argument was right and is now history.** Say so in past tense, or drop it — **but do
not leave it reading as a live decomposition of open work.**

## 6. Verify before relying on it

The section asserts *"Criterion 1's row carries the stated limits."* **Check that it still does**
before pointing readers there. If `MILESTONES.md`'s row no longer carries them, **the limits must stay
here in full** — losing them is the one outcome this increment must not produce.

## 7. Out of scope

- **`MILESTONES.md`.** If criterion 1's row is itself stale, **report it, do not fix it** — that file
  needs an explicit instruction naming it.
- **`trust-threat-model.md`**, which is current.
- **Any code change.**
- **The other two shipped sections.** They are the model here, not the target.

## 8. Controls

1. **No sentence in the section frames sync as unstarted, pending, or awaiting a prerequisite** — show
   it, do not assert it.
2. **The residuals survive** — all four named in §1 still appear. **Quote them from the final text.**
3. **The networking verification survives** and is still true — re-verify it against `Cargo.lock` and
   the CLI, do not carry it over on faith.
4. **Full gate set green, test count unmoved** — documentation only.

## 9. What to report

1. **The section, before and after, in full.**
2. **Your §6 finding** — does criterion 1's row still carry the limits?
3. **Anything else stale in the section** that I have not named.
4. All four controls (§8), quoted.
5. **Full gate set against the exact commit, after the last edit**, including `mdbook build` if any
   `docs/src/` file is touched (I expect none).
6. **Every numbered requirement's disposition, including ones that went without incident.**
7. Anything here was wrong.

**Stop and escalate, do not guess**, if: the networking verification is no longer true — **a networking
crate in `Cargo.lock` or a networked verb in the CLI would contradict the anonymity ruling and outranks
this increment.**
