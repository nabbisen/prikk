# Release 0.23.0 — changelog and version bump: implementation handoff

**Base:** current `main` (`0693cc6`, CI green). **Under `003-landing-work-on-main.md`.**
**Origin:** `.git-exclude/reviewed/release-readiness-assessment-v1.md` §4.3 and §9.

**This is the last increment before the cut.** Everything else on the release list is landed.

**Read this differently from the four increments before it.** Those corrected documents that were wrong.
**This one writes the public record of what prikk became** — the entry a person reads to decide whether
to try it, and the only place the `Tag` compatibility break is ever stated. **It is the most-read thing
you will write in this arc, and the hardest to correct after the tag.**

---

## 1. Version, and the shape of the commit

**`0.23.0`.** Additive features plus one breaking change to a shipped payload; pre-1.0, so the minor bump
carries it.

**One commit**, following the project's own precedent (`df0a951`, `1655732`):

```
release: bump workspace to 0.23.0 and add the changelog entry
```

touching `Cargo.toml`, `Cargo.lock`, `CHANGELOG.md`, and — new this time — `README.md` (§5).

**Do not tag. Do not publish. Do not touch the badge.** The tag is the owner's, always. The
early-implementation badge is governed by the six-criterion board in `MILESTONES.md` and **criterion 4
remains open by the owner's deliberate ruling** — **whether the badge changes is the owner's decision and
is not part of this increment.**

## 2. What the entry covers

**193 commits, 29 merges** since `0.22.1`. Themes, as a **starting point, not the scope** — derive the
list yourself from `git log 0.22.1..main`:

| Theme | What a user gets |
|---|---|
| **RFC 115/116/117** | `prikk sync` — history moves between repositories; tags travel and are adopted |
| **RFC 114** | the format-stability contract: what is frozen forever, what may migrate |
| **RFC 111** | `verify` is linear rather than superlinear |
| **DC-53** | `verify` checks every AUTHOR signature repository-wide |
| **DC-78 / misc** | closure validation on both receiving paths, resolver consolidation, documentation currency |

**Four of the six status-claim criteria were met in this window** (1, 2, 3, 5). That is the entry's real
subject, and it is worth saying plainly.

## 3. House style — match it, it is unusually specific

Read `0.22.0` and `0.22.1`'s entries before writing. The pattern:

- `## 0.23.0 — <date>` — **leave the date to the owner if the cut date is not yet known; say so rather
  than inventing one.**
- **A bold lede** stating what the release *means*, not what changed.
- **A "who is affected" sentence** where one applies.
- `### Added` / `### Changed` / `### Fixed` / `### Known limitation`.
- **`### Why`** — this project's entries explain reasoning, not just contents.
- **`### Verified rather than assumed`** — `0.22.0` used this to separate what is *demonstrated by a
  control* from what is merely claimed. **This release has a great deal to put under it** (twenty-one
  security refusals across RFC 115/116, each with an observed-failing control; the `rfc116_sync_cli.rs`
  test that drives the whole loop through the binary and verifies *both* repositories). **Use it.**

## 4. The `Tag` compatibility break — must be declared

**The owner has ruled** (2026-08-23) that prikk has not been used in production, so previous-version
compatibility is **not a concern** and **`Tag` stays at `schema_version` 1**. **That ruling is settled —
do not reopen it, do not propose a schema 2, and do not describe it as a regret.**

**But the consequence must be stated in the changelog, because this is the only place it is ever stated.**
The facts, verified:

- `0.22.1`'s `TagPayload` has 5 fields; `0.23.0`'s has 7 (`patch_set_digest`, `patch_count`), **both at
  `schema_version` 1**.
- **The break is two-way.** `0.23.0` reading a `0.22.1` tag fails
  `MalformedData("Tag missing patch_set_digest")`; **`0.22.1` reading a `0.23.0` tag fails
  `MalformedData("unknown Tag field tag: 6")`.**
- **It fails `verify`, not just `prikk tag list`** — `refs/verify/scan.rs:430` decodes the tag payload
  inside verify's ref scan, and the error propagates.

**So: a repository written by `0.22.1` that contains any tag will not verify under `0.23.0`, and the
error will say malformed data rather than version mismatch.** Say that plainly under
`### Known limitation` or a `### Breaking change` heading — **your call which, but it must be findable by
someone who hits the error and searches for its text.**

**Do not soften it and do not dramatise it.** The honest framing is the owner's own: prikk has not been
used in production, the schema window was deliberately closed, and this is the cost that was accepted.

## 5. `README.md` — the release collapses a distinction it currently makes

`README.md:61` currently splits its limits into **"in the released 0.22.1"** and **"merged on `main` but
not in any published binary yet."** **Cutting `0.23.0` makes the second half released and the first half
wrong.**

Required:

1. **Collapse the two paragraphs back into one**, describing `0.23.0` — which has sync, linear `verify`,
   and repository-wide author-signature checking. **Carry every stated limit across** (§6).
2. **Remove the `# sync: on `main` only, not in the released 0.22.1` comment** from `Useful Commands`.
3. **`Latest released implementation: 0.22.1` → `0.23.0`.**
4. **Re-check `Not a Good Fit Yet`** against the collapsed paragraph — it was corrected against `main`
   already, so expect it to be right, but **verify rather than assume.**

## 6. Stated limits — carry all of them, this is where overclaiming would do most damage

**`MILESTONES.md`'s rows carry each limit in the words it was ruled in. Use them, do not paraphrase.**

- **Criterion 1, four limits:** prikk **does not move the bytes** — confidentiality is the user's
  channel's property, never prikk's; **"two machines" is exercised as two repositories**, not two hosts,
  with no cross-host test; **negotiation is branch-scoped** (tags travel, but deletion and movement do
  not); **no discovery, remote identity, or remote-tracking.**
- **Criterion 5:** authorship is checked everywhere, **but this is trust-on-first-use** — continuity of
  authorship, **not** authenticity of first contact. No key rotation, revocation, or expiration.
- **Criterion 3:** `verify` is linear. **`seal` is not** — it still performs O(N) reads per call, so
  building N commits remains O(N²) in total reads. **Owned by no increment.**
- **Tag adoption** resolves by scanning local blocks at a cost measured **superlinear** — 12.6 ms over 500
  blocks, 86 ms over 2000.
- **DC-35:** **no prikk release passes the signer audit** — `release-signers.toml` is empty and
  fail-closed. `0.22.1`'s entry says so; **this release does not change it.**

**A release entry that lists five capabilities and omits their limits is the same defect class as the
README false claims**, in the document that gets read most.

## 7. What to report

1. **The entry itself**, and how you derived the contents from `git log 0.22.1..main` rather than from §2.
2. **Where you put the `Tag` break** and the exact wording (§4).
3. **The `README.md` changes** (§5), and your verdict on item 4.
4. **A checklist of §6's limits against the entry** — each one, and where it appears. **If you chose to
   omit one, say which and why.**
5. **The version bump** — `Cargo.toml` and `Cargo.lock` both, and confirmation that no crate carries a
   literal version outside `[workspace.package]`.
6. The **full gate set against the exact commit, after the last edit.** `release-policy reference-check`
   matters more than usual here: **a changelog naming commands can move the command inventory.**
7. Test counts — **expected unchanged**, but a version literal in a test would move.
8. Anything here that was wrong. **My handoffs in this arc have carried a miscount, a mis-stated scope,
   and one error serious enough that the review reversed part of the work. Assume this one is wrong
   somewhere.** In particular **I have asserted 193 commits and 29 merges — count them yourself.**

**Stop and escalate, do not guess**, if: the cut date is needed and you do not have it; a limit in §6
reads as stale against the code; you find work in the 193 commits that fits none of §2's themes and looks
significant; or **anything tempts you to change the badge, the criteria board, or `release-signers.toml`
— all three are the owner's.**
