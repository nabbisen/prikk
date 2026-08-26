# CI — exchange sealed history between two hosts

**Base:** current `main` (`de14d59`, CI green on all 12 jobs). **Under `003-landing-work-on-main.md`.**
**Owner-authorized.** Second of two; the realistic fixture landed first so this exercises real history
rather than a genesis commit.

**This closes a limit criterion 1 states about itself. Read §6 before assuming what may be claimed.**

---

## 1. What this proves that nothing else does

`rfc116_sync_cli.rs` (842 lines) exercises sync end to end — **in one process, on one machine.**
No `prikk sync` verb runs anywhere in CI.

Criterion 1's own row says it:

> file-based and channel-agnostic, so nothing in it is host-local, **but no cross-host test exists**

**The principle from the fixture increment applies unchanged**: a CI job earns its place only by
proving a claim that cannot be proven in one process on one machine. **This is such a claim.**

## 2. The shape — the existing fixture is already the sender

`rfc116_sync_cli.rs`'s own doc comment gives the design:

> **1. Repo A seals a patch. Repo B is empty.**

**First contact. No shared genesis required.** So:

- **Sender = the existing `fixture` job's repository**, authored on Linux, already sealed, now
  carrying real history. **Reuse the artifact; do not author a second one.**
- **Receiver = a fresh `prikk init` on Windows or macOS.**

**Pick one non-Linux host and say why.** Windows is the higher-value target — it is where path and
filesystem behaviour differs most — but macOS is defensible if something makes Windows impractical.

## 3. The flow, and the hop it forces

From the test, in order: `sync summary` → `sync compare` → `sync have` → `sync build` →
`sync accept` → `sync pending` → `sync seal --claim <id>`.

**`sync have` runs on the receiver; `sync build` runs on the sender.** So the receiver's have-list
must reach the sender, and the artifact must come back — **and the receiver's own repository must
survive between its two turns.**

**Three jobs, three artifacts:**

1. **receiver-prepare** (non-Linux): `init`, `sync have <ref> --output` → upload **the have-list and
   the receiver repository**.
2. **sender-build** (Linux): download the fixture and the have-list → `sync build --have … --output`
   → upload **the exchange artifact**.
3. **receiver-accept** (non-Linux): download the receiver repository and the artifact → `sync accept`
   → `sync pending` → `sync seal --claim <id>` → **`prikk verify`**.

**`sync summary`/`compare` are negotiation diagnostics, not gates.** **Adjudicate whether to include
them.** My lean is yes — they are part of the workflow a real user runs, and omitting them tests a
path nobody takes — **but if they add a fourth hop for no additional proof, say so and drop them.**

## 4. Four constraints this repository has already paid for

1. **The sender must be sealed locally**, or `sync build` reports *"already in sync"* for a false
   reason — both sides empty rather than equal (found during the `0.26.0` G1 refresh). **The fixture
   is already sealed; do not break that.**
2. **`tar`, never `zip`, for anything containing a repository.** `actions/upload-artifact`'s zip does
   not preserve empty directories, which once silently corrupted the fixture (DC-71 B2). **The
   exchange artifact is a single file and is not exposed; the receiver repository is.**
3. **Every shell command's head token must be classified** by `boundary-check`'s grammar. `rm` is
   not. **Design the steps so no cleanup command is needed** — that constraint already reshaped the
   fixture sequence once.
4. **Leave no stray file in a shared worktree.** prikk shares one worktree across every ref; a file
   left for one ref reads as *new* to the next commit on another. **That is what broke `main` an hour
   ago.**

## 5. `sync seal` needs a claim id from `sync accept`'s own output

`sync accept` prints a claim id; `sync seal <ref> --claim <id>` consumes it. **Parse it from the real
output** — do not hardcode, and do not skip the seal. **A receiver that accepts but never seals has
not finished the exchange**, and `verify` on an unsealed receiver proves less than it appears to.

**If parsing proves fragile, say so** — a machine-readable path may be missing, and that would be a
finding about the CLI worth having.

## 6. What may and may not be claimed — not yours to decide, and not mine alone

**A green run closes criterion 1's limit (b)** — *"no cross-host test exists"* becomes false.

**It closes nothing else.** prikk still does not move the bytes itself (RFC 116, by design), and there
is still no discovery, remote identity, or remote-tracking.

**Do not edit `MILESTONES.md`.** The owner has said that file needs an instruction naming it. **Report
that the evidence exists; I will put the claim to the owner.**

## 7. Out of scope

- **Changing sync behaviour**, or any product code. This is workflow YAML.
- **`MILESTONES.md`** (§6).
- **A second authored repository** — reuse the fixture (§2).
- **Networked transport.** prikk stays off the network by ruling; the artifact moves as a CI artifact,
  which *is* the operator's channel.

## 8. Controls

1. **The receiver ends with the sender's history, verified** — `prikk verify` on the receiver, and
   `prikk log` showing the sender's blocks. **Quote both from the non-Linux job.**
2. **The exchange actually crossed a host boundary** — show the artifact was produced on Linux and
   consumed on the other platform, from the job logs, not asserted.
3. **`sync pending` is empty after sealing** — the exchange is finished, not merely accepted.
4. **The receiver's repository survived its tar round-trip** with required-but-empty directories
   intact (§4.2).
5. **`boundary-check` passes** — every new command classified (§4.3).
6. **Full gate set green locally**, count unmoved — this is YAML.

**The real control is a green CI run**, and I will read per-job results. **A local proxy is not
evidence for a cross-host claim** — say plainly what you could not run.

## 9. What to report

1. **The three jobs and their artifacts**, and which host you chose for the receiver (§2).
2. **Your §3 adjudication** on `summary`/`compare`.
3. **Control 1's output from the receiver job**, quoted.
4. **How you obtained the claim id** (§5), and whether it was fragile.
5. All six controls (§8), quoted.
6. **Every numbered requirement's disposition, including ones that went without incident.**
7. Anything here was wrong.

**Stop and escalate, do not guess**, if: `sync build` reports *"already in sync"* despite a sealed
sender — **that would contradict §4.1 and needs diagnosis before any job is written**; or the receiver
cannot `verify` after sealing — **that is a real sync defect and outranks this increment entirely.**
