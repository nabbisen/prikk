# RFC 116 stage 7 — two follow-ups: cross-platform artifact fixture, and the confidentiality notice

**Base:** current `main`. **Independent of RFC 117** — can be done in parallel.
**Origin:** the owner's questions on criterion 1's stated limits, 2026-08-22.

Two small, unrelated items. Neither needs a ruling; both were agreed in discussion.

---

## Part A — the cross-platform gap

### A1. What is untested

CI runs the full suite on Linux, macOS and Windows, so each platform exchanges **with itself**.
**Nothing tests an artifact produced on one platform being accepted on another** — which is the real
cross-host risk, since the mechanism is file-based and hosts differ by platform, not by being separate
machines.

Two VMs would cover it. **A committed byte fixture is cheaper and stricter**, and it is the pattern
RFC 114 already uses for migration coverage.

### A2. What to build

Commit a **`PEXCH001` artifact fixture** — real bytes from a real `build_sync_artifact` run — and a test
that **accepts it into a fresh repository on whatever platform CI is running**, asserting the patches
land and are sealable.

- **Generate it once, commit it, do not regenerate casually.** Treat it like `dc55_pre_swap_repo`: it is
  evidence, not a convenience. Document how it was produced, in the file that consumes it.
- **`PEXCH001` is representational** (RFC 114 §3), so this fixture is *not* a frozen-format promise —
  when the artifact format legitimately changes, the fixture is regenerated and the change reviewed.
  **Say that in the module doc** so nobody treats a needed regeneration as a stop-work finding, and
  nobody treats a casual one as fine.
- The fixture's own author key material must be self-contained, so the test needs no external state.

### A3. Control

Corrupt one byte of the committed fixture → accept must refuse. That proves the test exercises decoding
rather than merely reading a file.

---

## Part B — the confidentiality notice

### B1. Why

`sync build` writes a file containing source content, blobs and messages **in the clear**. prikk
guarantees integrity and authenticity, never secrecy — secrecy is whatever channel the user chooses.

Verified and worth stating in the notice's own wording: the artifact carries `key_id → public_key`
only. **No secret key material.** So the exposure is exactly the content the user chose to send, plus
public keys — not their identity.

prikk cannot enforce this without becoming a transport with recipient identity, which RFC 116 ruled
out. **The available fail-safe is to make the property impossible to misunderstand at the moment the
file is written.**

### B2. What to add

On a successful `sync build`, print a short notice alongside the existing report: the artifact contains
repository content in the clear, prikk does not encrypt it, and it should move over a channel the user
trusts.

- **Once, on the command that creates the exposure.** Not on `accept`, not on every subcommand.
- **A statement, not a warning to dismiss.** No prompt, no flag to silence it — a flag to silence it
  would defeat the purpose and add a surface.
- Mirror it in the `sync` documentation.

### B3. Control

Assert the notice is present in `sync build`'s output when an artifact is written, and **absent** on the
`AlreadyInSync` path where no file is created. Control: remove the emit → the presence test fails.

---

## Out of scope

- **Any encryption, transport, or recipient identity.** RFC 116 ruling 2.
- **Changing the artifact format.** Part A commits a fixture of the current one.
- **Cross-platform CI orchestration / VMs.** A2 is the cheaper substitute, deliberately.

## What to report

1. Control output for A3 and B3 — actual failure text, and the single line mutated.
2. **How the Part A fixture was produced**, and confirmation it is committed rather than generated at
   test time.
3. The **full gate set against the exact commit, after the last edit** (the standard nine).
   Cross-target clippy pair only if this diff contains `#[cfg(target_os)]` — **check this one carefully:
   a cross-platform fixture test is exactly the kind of diff that might acquire a `cfg`.**
4. Test counts before and after. **`snapshot.txt` must not change.**
5. Anything here that turned out to be wrong. **Say so plainly.**

**Stop and escalate, do not guess**, if: the committed fixture fails to accept on any CI platform —
**that is the gap this part exists to find, and it is a real finding, not a fixture problem to adjust
away.**
