# CI — make the cross-boundary fixture a real repository

**Base:** current `main` (`491b5c0`). **Under `003-landing-work-on-main.md`.**
**Owner-authorized.** First of two increments; the cross-host sync test follows and will be written
against whatever this produces.

---

## 1. The principle this increment applies

**A CI workflow job earns its place only by proving a claim that cannot be proven in one process on
one machine.** Everything else belongs in `cargo test`.

**The existing pipeline already is this**: `fixture` authors on Linux and `non-linux-verify` verifies
on macOS/Windows; `windows-mutate` feeds `verify-cross-platform-history` on Linux. Every job exists
because of a boundary.

**So a full local lifecycle does not get its own job** — init→edit→commit→close on one runner proves
nothing `cargo test` does not. **What it justifies is a better payload for the boundary that already
exists.**

## 2. What crosses the boundary today

The entire authored fixture:

```sh
prikk init fixture-repo
prikk trust maintainer add --key-id … --public-key …
echo "hello prikk" > readme.txt
prikk commit -m genesis
prikk seal --allow-no-audit
```

**One file, one commit, no branch, no tag, nothing closed.** Four downstream jobs — read-only
conformance on two platforms, Windows mutation, Linux reference mutation, and cross-platform identity
— all exercise that.

## 3. Make it a repository someone could plausibly have

**Author a realistic history**, still with the real binary, still on Linux. At minimum:

- **more than one commit**, including an **edit to an already-committed file** — not just new files.
  A modification exercises replay and anchoring that a create never touches;
- **a branch** (`prikk branch create <name> [--from REF]`);
- **a tag** (`prikk tag create <name> --target <ref|block>`);
- **a closed branch** (`prikk branch close <name>`) — *"not delete — pointer, history preserved"*, per
  its own help text, which is exactly the state worth carrying across a boundary.

**Verified these verbs exist** before asking for them (`commands.rs:80-91`). **Order and content are
yours**; make it read like a plausible small project, not a checklist.

**Seal what should be sealed.** Some of it unsealed is fine and arguably better — but **say which and
why**, because the downstream jobs' behaviour depends on it.

## 4. Why this is safe — verified, so you need not re-establish it

- **The conformance job checks exit codes, not content.** It echoes each command and checks it
  separately (DC-71 B2), so more history means more output, not a broken assertion.
- **`verify-cross-platform-history` re-derives object ids on both platforms and `diff`s them against
  each other.** **Nothing is pinned.** A richer fixture changes both sides identically and still
  matches.

**If you find an assertion I missed that a richer fixture breaks, stop and report it** rather than
trimming the fixture to fit.

## 5. What this will not fix — do not try

**`worktree-status` stays excluded from the conformance set.** Its own comment explains why: it
*"requires snapshot-block state that no CLI command can produce against an ordinary patch/WAL-authored
repository."* **That is a model gap, not a richness gap.** A bigger fixture will not reach it, and
attempting to is out of scope.

## 6. Transport discipline — the hazard already paid for

The `fixture` job carries the **DC-71 B2 ruling**: `actions/upload-artifact`'s zip **does not preserve
empty directories**, so the conformance job once downloaded a **silently corrupted repository and never
tested a valid one**. `tar` fixed it.

**Keep tar. Do not "simplify" the packaging.** A richer fixture has *more* required-but-empty
directories to lose, not fewer. **Confirm the unpacked fixture still contains them** on at least one
consumer.

## 7. Out of scope

- **A new CI job.** This is a payload change (§1).
- **The sync cross-host test**, which is the next increment.
- **`worktree-status`** (§5).
- **Any product code.**

## 8. Controls

1. **The richer fixture still passes every downstream job** — all four, on every platform. **This is
   the control that matters**; quote the job outcomes.
2. **The added history is actually present in what crosses the boundary** — show it from a *consumer*
   job (`prikk log` on macOS or Windows), not from the authoring job.
3. **Required-but-empty directories survive the tar round-trip** (§6) — show it after unpacking.
4. **Full gate set green locally**, and say whether the test count moved (it should not — this is
   workflow YAML).

**Quote every failure.** A CI change cannot be proven locally: **the real evidence is a green run on
the pushed commit**, and I will read the per-job results rather than the overall conclusion.

## 9. What to report

1. **The authored sequence**, in full, and what you sealed.
2. **Control 2's output from a consumer job** — the history as a *different platform* sees it.
3. All four controls (§8), quoted.
4. **Anything a richer fixture broke** (§4), if anything.
5. **Every numbered requirement's disposition, including ones that went without incident.**
6. Anything here was wrong.

**Stop and escalate, do not guess**, if: a downstream job pins fixture content I did not find; or a
verb in §3 cannot produce the state described — **that would be a finding about the CLI, not a reason
to quietly drop the step.**
