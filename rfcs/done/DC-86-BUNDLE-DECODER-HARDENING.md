# RFC (accepted) - DC-86 Bundle Decoder Hardening

**Status.** **ACCEPTED by the project owner 2026-08-09.** **Independence.** Author-reviewed — the
standing ceiling. **Arises from.** The DC-78 Stage 3 review: the bundle decoder is the newest
untrusted-input parser in the product and the only one consuming bytes from a party the operator does not
control, and it has neither fuzz coverage nor a resource bound.
**Target.** 0.20.0. **Test and hardening only** — no format change.

## 1. Why this surface, specifically

`EXECUTION-ORDER.md` §6 rule 3: *"randomized decoder input is where something will plausibly be found."*
DC-41 stage 4 acted on that for the **object** decoders. **The bundle decoder never received the same
treatment**, and it is strictly more exposed: object decoding reads bytes the repository already holds,
while bundle decoding reads bytes a stranger handed you.

Current coverage is a single `import_of_malformed_bytes_fails_closed` test.

## 2. Two defects, both fail-safe today, neither bounded

**No randomized coverage.** `bundle.rs` and `received.rs` contain zero `proptest`/fuzz references.

**No resource bound.** `import_bundle` caps neither object count nor total bytes. Content addressing
prevents overwriting anything held, and nothing imported is trusted before a key is adopted — **so there
is no integrity impact.** But a hostile bundle can write arbitrarily many arbitrarily large objects into
the store, and they then count in every subsequent scan.

## 3. Scope

1. **Property/fuzz coverage** over `decode_bundle` and the received-pointer decoder, in DC-41 stage 4's
   shape. **A panic on malformed input is an NFR-SEC-04 defect** and, per rule 3, opens its own
   corrective RFC with a minimized reproducer — **it is not a test expectation to encode here.**
2. **Explicit, configurable-with-a-default resource bound** on import: maximum object count and maximum
   total decoded bytes, refused **before** anything is written, in DC-57's shape (a hard block that fires
   ahead of any write, with the limit documented).
3. **A stated ceiling on what this increment claims.** Hardening a parser is not proving it correct; say
   what the fuzz campaign covered and for how long.

## 4. Acceptance criteria

1. Randomized input reaches `decode_bundle` and the received-pointer decoder; **no panic, no hang, no
   unbounded allocation** — every outcome is a typed error.
2. **Import refuses an over-limit bundle before writing any object.** Demonstrate with a
   before/after object count, not an error string alone.
3. **A negative control:** show the bound actually fires — a bundle just over the limit is refused, one
   just under is accepted.
4. All existing tests pass unchanged; **no bundle format change** — this increment hardens the reader,
   it does not alter what a valid bundle is.
5. Gate set per `EXECUTION-ORDER.md` §6 rule 9 as amended, **and a green macOS run before merge** — this
   touches filesystem-backed import.

## 5. Non-goals

Revocation, the received-ref audit trail, and the merge gap (DC-85) — each recorded separately in
`FINDINGS.md` and each its own question. Any change to the bundle format. Transport.
