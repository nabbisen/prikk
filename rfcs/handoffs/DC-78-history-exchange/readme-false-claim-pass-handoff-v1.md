# `README.md` false-claim pass: implementation handoff

**Base:** current `main` (`b3d9b0f`). **One file. Documentation only.**
**Origin:** the staleness-by-omission sweep found it while out of scope, and reported rather than fixed.

**This is the highest-priority currency work remaining, because of where the errors are.** The
paragraph headed *"Known limits worth stating up front"* is the first orientation any reader gets, and
**three of its five clauses are false.** A project whose front page says its newest capability does not
exist — and understates its own verification twice — is misdescribing itself to precisely the reader
who has no other source.

---

## 1. Method: check claims against authorities, not phrases against a grep

The three currency passes before this one had a keyword to search for. **README makes its claims in
prose** — *"there is no networking or sync"* matches no "deferred"/"not implemented" pattern. So the
scope is the file, and the method is comparison against a named authority:

| Authority | Settles |
|---|---|
| `MILESTONES.md`'s criteria board | what is met, and **with what stated limit** |
| `main.rs`'s dispatch table | which commands exist |
| `git tag --sort=-v:refname` | the latest release |
| root `Cargo.toml` | the workspace version |
| `rfcs/accepted/`, `rfcs/done/` | what shipped and when |

**Every correction names which authority settled it.** A correction sourced to nothing is the same
defect in the other direction.

## 2. Scope: every capability or status claim in the file

277 lines. The claim-dense sections are **Current Status** (43-69), **Good Fit** (70-79), **Not a Good
Fit Yet** (80-91), **Core Ideas** (92-106), and **Useful Commands** (202-233) — but **read the whole
file.** Install, Quick Start and Development Gates make claims too.

**The deliverable is an enumeration**, as in the documentation-currency sweep: every capability or
status claim, with a verdict (`STALE` / `CURRENT`) and the authority. Not a diff.

## 3. Six already confirmed `STALE` — seed, not bound

| Line | Claim | Authority |
|---|---|---|
| 45 | *"Latest released implementation: **0.21.0**"* | `0.22.1` is the latest tag; `Cargo.toml` agrees |
| 61 | *"**there is no networking or sync**, so history cannot be exchanged between machines"* | RFC 116/117; criterion 1 **MET** |
| 62 | *"`verify` cost grows steeply with history length"* | RFC 111; criterion 3 **MET** — linear, ratio 1.97 |
| 63 | *"`verify` does not yet check author signatures repository-wide"* | DC-53; criterion 5 **MET** |
| 87 | *"hosted forge workflows, remotes, or **sync**"* | the `sync` clause only |
| 88 | *"complete branch management, **tags**, semantic merge, or **merge execution**"* | `tags` (RFC 117), `merge execution` (DC-74) |

**Lines 87 and 88 are the shape to expect throughout: a list where some terms are stale and some are
not.** On line 88, *"complete branch management"* (no `branch switch`) and *"semantic merge"* (renames,
conflict resolution) are **still true**. Correct by removing shipped terms, not by rewriting the bullet.

Two clauses in line 61-64's paragraph are also **still true** — *"merge-base discovery is manual"* and
*"conflicts are detected and refused but never resolved"*. **Three of five false; keep the two.**

## 4. The correction must not overshoot — this is where an overclaim does most damage

Criterion 5 is **MET with a stated limit**: authorship is checked everywhere, but this is
**trust-on-first-use continuity, not first-contact authenticity**, and there is no AUTHOR trust policy.
Criterion 1 is **MET with four stated limits**, including that **prikk does not move the bytes** and
confidentiality belongs to the user's channel.

**Carry those limits across.** A README that swings from "there is no sync" to "prikk syncs" without
saying prikk never moves the bytes would be a worse error than the one being fixed — **on the front
page, an overclaim is a trust problem, not a documentation problem.**

`MILESTONES.md`'s rows carry each limit in the words they were ruled in. Use them.

## 5. Second, smaller half: the command list

**Useful Commands** (202-233) lists 18 of 23 commands, missing **`bundle`, `sync`, `merge`, `unlock`,
`compact`**. The omission sweep classified this `REPORT` because README is not an exhaustive surface —
correct then, and it would be **incoherent now** to correct "there is no sync" in prose while leaving
`sync` absent from the command list in the same file.

**Add the five.** Keep the section's existing format and brevity.

**Report this separately from §2's enumeration**, so the false-claim work stays cleanly checkable.

## 6. Out of scope

- **Every file except `README.md`.**
- **`ROADMAP.md`** — its own audience, its own pass if it needs one. **If you find it contradicting your
  corrections, report it.**
- **Restructuring.** Correct claims, remove stale terms, add the five commands. **Do not reorganise
  sections or rewrite prose that is accurate.**
- **The Development Gates section's 3-of-9 gate list** — it says *"the relevant subset of"*, so it
  claims nothing false. **Adjudicate it in your enumeration and leave it**; whether README should carry
  the full gate set is a separate judgment.

## 7. What to report

1. **The enumeration** — every capability/status claim, verdict, authority. The deliverable.
2. **For each `STALE`:** the correction, and the limit you carried across if the authority named one (§4).
3. **The command-list additions**, separately (§5).
4. **Anything in `ROADMAP.md` that now contradicts README** (§6) — report only.
5. The **full gate set against the exact commit, after the last edit** — the standard nine.
6. Test counts — **expected unchanged**.
7. Anything here that turned out to be wrong. **Say so plainly, including my six.** Three of my last
   three handoffs have contained a miscount or a mis-stated scope; assume this one does too and check.

**Stop and escalate, do not guess**, if: a claim's truth depends on a criterion you cannot find a ruling
for; a correction would require asserting something no authority in §1 settles; or `MILESTONES.md`
itself looks stale against the code — **that row would be mine to fix, never yours.**
