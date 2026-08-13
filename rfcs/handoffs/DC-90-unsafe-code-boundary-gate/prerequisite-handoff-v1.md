# DC-90 Unsafe Code Boundary and Gate — Prerequisite Handoff v1

**Cleared to answer §4's four questions only.** Accepted 2026-08-10,
`rfcs/done/DC-90-UNSAFE-CODE-BOUNDARY-GATE.md`. **Design follows the answers.**

## 1. Why this exists

The owner ruled on 2026-08-10: *"`unsafe` is allowed under control with safety and maintainability
preserved."* That settles permission. It does not describe the control, and an undescribed control
becomes whatever the first increment needing FFI happens to do.

Background, worth reading before §4 — `rfcs/handoffs/DC-87-windows-mutation/unsafe-surface-analysis-v1.md`:

- **prikk already runs unsafe code.** `rustix` does unsafe FFI internally on every Linux and macOS
  build. `forbid(unsafe_code)` is a property of the code prikk *writes*, not the code it *runs*. The
  architect's original escalation got this wrong and the correction is on the record.
- The choice between a bespoke FFI crate and `cap-std` is **still open** and is to be decided against
  measured numbers, not principle. **This increment constrains either outcome**, so it does not need
  that decision first.

## 2. A measurement I made, and the mistake I made making it

§4.2 asks what is already true. I measured it and got it wrong once, so you get both results:

I first grepped member manifests for `lints.workspace` and found **zero** matches, which reads as "the
workspace lint table is inert and `forbid` applies nowhere." **That conclusion was false.** The real
form is a `[lints]` section with `workspace = true` on the following line, and **all eight workspace
members carry it** — `prikk-cli`, `prikk-crypto`, `prikk-error`, `prikk-hash`, `prikk-object`,
`prikk-replay`, `prikk-store`, and `tools/release-policy`. `unsafe_code = "forbid"` does apply
everywhere.

I am telling you about the wrong answer rather than only the right one because **this specific check is
unusually easy to get wrong**, and the gate you build will be doing exactly it. Confirm it your own way
rather than inheriting my result.

**One real asymmetry the measurement did surface**, and it is a design input for §4.1: the *source-level*
marker is inconsistent. `prikk-crypto/src/lib.rs` carries no `#![forbid(unsafe_code)]` at all and is
covered purely by manifest inheritance; the other seven roots carry it explicitly as well. So "is this
crate forbidden?" has two possible sources of truth that currently disagree in presentation while
agreeing in effect. **Which one the gate treats as authoritative is a real decision** — checking only
source attributes would wrongly flag `prikk-crypto`; checking only manifests would miss a crate that
opts out in source.

## 3. Where to start

**§4.1 decides the increment's cost, so take it first.** The tool reads manifests today
(`boundary/placement.rs` parses TOML). Detecting a missing `forbid` is manifest work and looks cheap.
Detecting an `unsafe` block without a `SAFETY:` comment is *source* work, which the tool has never done.
**Report honestly whether that fits or is a new capability, and what it costs.**

**And take §4.3 seriously before building anything.** `cargo-geiger` and similar tools exist. A
dependency in `tools/` is far cheaper than one in a product crate. If something off the shelf does this
better, saying so is a better outcome than a bespoke check.

**The RFC's own §4.1 line is the one I mean most:** *a gate that is expensive to build badly is worse
than a documented convention honestly labelled as one.* If the source-level half is disproportionate,
propose the manifest-level gate plus a stated review obligation, and say plainly which half is
machine-checked and which is not. That is criterion 5, and it is not a consolation prize.

**§4.4 is where I expect the increment's real value.** The FFI-ABI risk this boundary exists around is
**not machine-checkable at this layer** — no manifest or source check can tell you an
`extern "system"` declaration matches the real Win32 ABI. Enumerate what the gate cannot see, plainly,
so nobody later reads a green check as evidence of a guarantee it never tested. DC-87's criterion-2
standard applies here to the gate itself.

## 4. Limits

- **No design in this pass.** Answers first.
- **Do not create the unsafe crate.** This increment defines and enforces the boundary; nothing goes
  through it yet.
- **Do not decide bespoke-FFI versus `cap-std`.** Explicitly out of scope (RFC §7).
- **`rustix`'s and other dependencies' internal unsafe is not in scope.** This governs code prikk
  writes.
- **Formal verification is not part of this.** It does not mitigate the FFI risk and coupling them would
  buy assurance in the wrong place.

## 5. Reporting

`.git-exclude/review-request/`, plain `.md`. Answer the four in order. Findings outside scope go in the
report; I register them in `FINDINGS.md`.

## 6. Sequencing

- **DC-88 is the priority** and is unaffected by this — different files, no collision.
- **This must land before the first `unsafe` line**, which today means before DC-87 Stage 2. Stage 2 is
  already blocked on DC-88, so there is room. If that ordering ever conflicts, it comes back to the
  owner rather than being resolved by whichever increment moves first.
- **DC-87 Stage 1's seam refactor** remains on hold behind DC-88.
