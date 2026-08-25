# Release `0.24.0` — changelog and version bump: implementation handoff

**Base:** current `main` (`02f5c3a`). **Under `003-landing-work-on-main.md`.**
**Origin:** `.git-exclude/reviewed/release-readiness-0-24-0-assessment-v1.md`.

**One commit**, following `df0a951` / `b6681ba`'s precedent:
`release: bump workspace to 0.24.0 and add the changelog entry` — touching `Cargo.toml`, `Cargo.lock`,
`CHANGELOG.md`, `README.md`.

**Do not tag. Do not publish. Do not touch the badge.** The tag and crates.io are the architect's under
the owner's authorization, after this lands and CI is green. **The badge stands — §5.**

---

## 1. The breaking change — the one thing that must not be missed

**A repository written by `0.24.0` cannot be read by `0.23.0`.**

- **`0.23.0` admits `Patch` schema `[1]`.** Current admits `[1, 2]`, and **new patches are written at
  schema 2** (`node_authoring.rs:574`).
- **The error a `0.23.0` binary emits**, which is what a user will search for:
  *"format-2 Patch does not accept envelope schema 2"* (`format.rs`'s `validate_format2_schema` at the
  `0.23.0` tag). **Quote it in the notes** — verify the exact string against the tag rather than
  trusting my transcription.
- **Direction matters and must be stated: `0.24.0` reads `0.23.0` fine.** Only the reverse breaks. **Do
  not describe this as mutual** — `0.23.0`'s Tag break was mutual, and conflating them would misstate
  both.
- **Remedy:** do not downgrade to `0.23.0` after authoring patches under `0.24.0`. **There is no
  in-repository repair**; say so plainly rather than implying one.

**Give it its own `### Breaking change` heading**, as `0.23.0` did — a decode failure is not a stated
absence, and it must be findable by someone searching the error text.

**Note in your report that G1 did not detect this.** The gate covers the forward direction only, by
design (track C §6). **The declaration here is by hand, exactly as it was for `0.23.0`.**

## 2. What the release contains

**63 commits, 12 touching `crates/`.** Derive the contents from `git log 0.23.0..HEAD` rather than from
this list.

**Three user-visible behaviour changes:**
- **`tag create`, `branch create`, `branch close` now gate on the maintainer trust policy** (`053e442`) —
  an untrusted signer is now refused. **Closes a gap open since DC-63**, where DC-11 required the rule
  and DC-63 adopted it in words but not in code.
- **`verify` gained a `LocalTagTrust` stage** (`da29c0f`) — **locally-published** tags' MAINTAINER
  signatures are checked; **received, unadopted tags deliberately are not** (§4).
- **`Patch` schema 2, retiring `parent_patch_ids`** (`8c31a78`) — §1.

**One text change:** `doctor --repair-main-ref`'s refusal no longer names a stale version or an
unreachable format-1 scenario (`6a3d591`).

**Everything else is gates, tests and documentation.** **Say so.** RFC 118's command registry and join
gate, RFC 114 Gate A's pair-granularity and derived sweep bound, RFC 119's G1 and the release-policy
reduction, caller-level trust coverage, a new `status` guide page. **`--help` output is byte-identical.**

**Do not overstate the feature content.** This is predominantly an assurance release, and describing it
otherwise would be the defect this project has spent a month removing.

## 3. House style

Read `0.23.0`'s and `0.22.0`'s entries first. **Bold lede stating what the release *means*; `### Added` /
`### Changed` / `### Fixed` / `### Breaking change` / `### Known limitation` / `### Why` /
`### Verified rather than assumed`.**

**`### Verified rather than assumed` has a great deal to carry here** — Gate A observed failing in both
directions, G1 observed failing under mutation, the trust gates' caller-level negative controls, the
join gate's bidirectional controls. **Use it.**

**Leave the date to the owner if the cut date is not known** — `## 0.24.0 — <cut date…>`, as `0.23.0`
did before `b6cd309` dated it.

## 4. Stated limits — carry them all

**From `MILESTONES.md`'s rows, in the words they were ruled in.** Criterion 1's four; criterion 5's
trust-on-first-use; criterion 3's `seal` residual; tag adoption's superlinear cost; DC-35's empty signer
audit, **unchanged by this release**.

**And one new limit this release creates:** `verify`'s tag checking covers **locally-published tags
only**. A received, unadopted tag is deliberately exempt, because its signature is the sender's under a
key this repository has not adopted. **State it where a reader meets the new capability, not only in a
limitations list.**

## 5. `README.md`

1. **`Latest released implementation: 0.23.0` → `0.24.0`.**
2. **Re-check the limits paragraph.** `verify` now checks locally-published tag signatures; the existing
   trust-on-first-use wording may not cover that. **Adjudicate — do not assume it needs changing.**
3. **Do not touch the badge.** It stands: the paired sentence *"future releases may require migration"*
   is true again, by §1.

## 6. Out of scope

- **Tagging, publishing, `release-signers.toml`, the badge.**
- **Any code change.** If the notes cannot be written truthfully without one, **stop and report.**
- **`MILESTONES.md`, `ROADMAP.md`.**

## 7. What to report

1. **The `### Breaking change` section**, with the error string **verified against the `0.23.0` tag**.
2. **How you derived the contents** (§2), and anything in `git log 0.23.0..HEAD` my list missed.
3. **The limits checklist** (§4), each with where it appears.
4. **Your `README` limits-paragraph adjudication** (§5.2).
5. **The version bump** — `Cargo.toml`, `Cargo.lock`, and confirmation no member carries a literal
   version outside `[workspace.package]`.
6. **Full gate set against the exact commit, after the last edit**, plus `mdbook build`.
7. Test counts — **expected unchanged at 1329 total**.
8. Anything here that was wrong, **including my 63/12 counts and the quoted error string**.

**Stop and escalate, do not guess**, if: the `0.23.0` error string differs from §1's; you find a
**second** compatibility break my assessment missed; or the cut date is needed and you do not have it.
