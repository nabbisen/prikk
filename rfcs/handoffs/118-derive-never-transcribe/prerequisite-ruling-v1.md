# RFC 118 — prerequisite ruling v1

**RFC 118 §10 blocks design of any stage on four prerequisites.** This rules on **1, 2 and 4** from
source. **3 remains open and is the owner's.**

---

## Prerequisite 1 — is dispatch genuinely derivable? **YES, cleanly.**

**The dispatch is already almost uniform.** Of 22 command arms in `main.rs:86-108`:

- **20 are exactly `run_x(args.collect())`** — one shape.
- **`init`** is `run_init(args.next())` — `Option<String>`.
- **`status`** is `run_status()` — no arguments.

**And every `run_*` returns the same type**: `std::result::Result<(), String>`, uniform across all of
them (checked by extracting every signature, not sampled).

**So a registry is a plain table:**

```rust
struct Command { name: &'static str, run: fn(Vec<String>) -> Result<(), String>, /* help fields */ }
const COMMANDS: &[Command] = &[ /* … */ ];
```

**20 entries take their `run_*` directly; `init` and `status` take a one-line adapter closure each.**
No trait objects, no dynamic dispatch, no macro. **Per-command argument shapes are not lost** — each
`run_*` keeps parsing its own arguments exactly as today; the registry replaces only the `match`.

**Risk found: none.** `--help`/`--version` are meta-arms, not commands, and stay outside the table.

## Prerequisite 4 — does generation conflict with the zero-dependency CLI? **NO.**

`prikk-cli` has **no third-party dependencies** and this design adds none:

- **The registry** is a `const` array of plain data and function pointers — language only.
- **`--help`** becomes a loop of `println!` over that array — language only.
- **Documentation checking** lives in `release-policy`, which is `publish = false` and **already has
  `serde`**. Nothing enters the shipped binary.

**No build script, no code generation step, no new crate.** The "generation" in RFC 118 §7 is runtime
rendering from a table, not source generation — which is simpler than the RFC's own wording implies, and
the RFC's §7 should be read that way.

## Prerequisite 2 — the authority for "explained somewhere". **Generalizes, and improves on the precedent.**

`reference-check` today is: **a hand-maintained inventory JSON**
(`release/release-policy-command-inventory-v1.json`) + a scanner over docs/shell/YAML + a
`REQUIRED_LIVE_PATHS` assertion.

**The same shape works for prikk's commands, with the inventory *derived* rather than hand-written** —
which removes the one transcription `reference-check` still contains. **Under RFC 118's own principle,
the checker must not hold a second copy of the command list.**

**Bidirectional rule, per RFC 118 §8:**
- every registry entry is explained in at least one authored document;
- every `prikk <command>` mention in documentation names a real registry entry.

**Open sub-question, and it is a design choice, not a blocker:** how `release-policy` reads the registry.
Three candidates — **prikk emits its own inventory** (a derived command, cleanest, no parsing); a
build-time artifact; or `release-policy` parsing the Rust source (**worst — a second parser is a second
copy**). **Stage 1 should not settle this; the registry must exist first.**

## Prerequisite 3 — scope beyond commands. **OPEN — the owner's.**

RFC 118 §10.3 ranks candidates by demonstrated harm: **trust-gated operations** (hand-derived twice this
session, wrong once) and **`verify`'s stage inventory** (which turns a machine-readable `verify` result
into a derived view, dissolving the structured-output theme).

**Not ruled here.** Stage 1 is commands regardless, and nothing about it forecloses either.

## Conclusion

**Prerequisites 1, 2 and 4 are discharged. Stage 1 — the command registry — is designable now**, and its
shape is a plain `const` table with two adapter closures, no dependency, and no source generation.

**Prerequisite 3 does not block stage 1.**
