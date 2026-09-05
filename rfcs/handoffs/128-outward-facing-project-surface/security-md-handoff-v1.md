# `SECURITY.md` — implementation handoff

**Authority:** `rfcs/done/128-outward-facing-project-surface.md` §2, §2a.
**Base:** current `main` (`94b6cb7`). **Under `003-landing-work-on-main.md`.**
**Sequenced after RFC 129, which is complete — the repository is now `prikk-vcs/prikk`.**

**Scope: `SECURITY.md` and the README pointer only.** RFC 128 also covers `CONTRIBUTING.md`, the
Cargo metadata pass, and the Git→prikk mapping page. **Those are separate increments.**

---

## 1. Why this file, for this project

There is no disclosure path today — confirmed: no `SECURITY.md` at the root or under `.github/`, and
no disclosure phrasing anywhere in the README or `docs/`. **For a product whose entire pitch is
signatures, trust, and verifiable history, that absence is disproportionate**: someone who finds a
flaw in the signature preimage binding or the publication protocol has only a public issue or
silence, and both are bad.

## 2. The ruled channel

**Advisories-only, as project policy** (owner ruling, 2026-09-01, RFC 128 §2):

```
https://github.com/prikk-vcs/prikk/security/advisories/new
```

**There is no email fallback, and that is deliberate — do not add one.** RFC 128 §2a records the
reasoning on both sides, including what the single channel costs. If you think an address belongs
here, that is an escalation, not an edit.

## 3. Two content constraints, both binding

### 3.1 State what is not promised

No CVE assignment process. No response-time commitment. Pre-1.0 software. **A security policy that
overpromises is worse than none**, and this project's house style refuses to overclaim everywhere
else — write this section in the same register as the README's "Not a Good Fit Yet".

### 3.2 Do not describe release-artifact signature verification as available

**`release-signers.toml` still reads `authorized_primary_fingerprints = []`.** Nobody can verify who
built a `prikk` binary today, and the signer bootstrap is the outstanding badge criterion.

**A reader of `SECURITY.md` came to check exactly this**, so implying it works would be the single
most damaging overclaim this project could publish. If the file mentions release artifacts at all, it
says plainly that release-signer verification is not yet available and points at
`docs/src/reference/release-compatibility.md#core-caveats`, which already owns that statement.

**Do not restate the caveat's content** — link it, per RFC 118. Two copies of a security caveat is
two things to drift.

## 4. What the file should cover, beyond the channel

Keep it short. A newcomer should finish it in under a minute.

- **What to report privately** versus what is an ordinary bug. Say plainly that the interesting
  classes here are identity, signature, publication, durability, and path/format handling.
- **What is already known and documented, so it is not reported as a finding**: the TOFU authorship
  boundary and the absence of key rotation/revocation (`docs/src/reference/trust-threat-model.md`),
  and the platform durability gaps (`docs/src/reference/platform-support.md`). **Link, do not
  restate.**
- **What the project commits to**: acknowledging a report and fixing what it accepts. Nothing about
  timelines that nobody has agreed to meet.

## 5. Placement, and one thing to check before you write

**Root `SECURITY.md`.** GitHub honours root and `.github/`; root is the more visible of the two and
matches where this project already keeps `README.md`, `LICENSE`, `NOTICE`, `CHANGELOG.md`, and
`ROADMAP.md`.

**Check before committing:** `tools/release-policy/src/boundary/package.rs::check_source_tree` holds
a required-path list. I read it and it names only `release/` and `tools/release-policy/` paths, so a
new root file should be outside its scope — **verify that by running `boundary-check`, not by
trusting this paragraph.**

**`DECLARED_DOCUMENTS` (`crates/prikk-cli/src/commands/tests.rs:34`):** add `SECURITY.md` **only if
the file names a real `prikk <command>` in code context.** RFC 118 §8 rule (A) then checks those
names. The simplest outcome is a file that names no commands and needs no entry — prefer that, and
say which way it went.

## 6. The README pointer

RFC 128 §2a's second obligation: **the README carries the disclosure pointer too**, so the path is
findable from the front page and not only from a file whose placement GitHub happens to honour today.
One line, linking `SECURITY.md`. Place it where a reader looking for it would look, not at the bottom.

## 7. A standing obligation to write into the file's own context

RFC 128 §2a, first obligation: **with no email fallback, this advisory URL _is_ the disclosure path**,
so `SECURITY.md` is live functional infrastructure. **Any future move of this repository must update
it in the same increment as `release.yml` and `installer.rs`** (RFC 129 §2's first class). A stale
disclosure URL leaves a reporter with nowhere to go — a functional break, not a broken link.

Record that where the next person doing a migration will see it: **RFC 129 §7 already says it**, so
confirm that stays true and do not duplicate the rule into `SECURITY.md` itself.

## 8. Controls

1. **Both §3 constraints, quoted from the file you wrote** — the not-promised section, and whatever
   the file says (or deliberately does not say) about release-artifact verification.
2. **Every link resolves in `docs/book/`**, checked against generated HTML, the same way DC-44
   increment 4's report checked its own.
3. **`boundary-check` and the RFC 118 §8 doc gate**, with §5's two questions answered as results.
4. **The claim that no disclosure path exists is re-verified as false after your change** — search
   the README and `docs/` again and show the pointer is now findable.

## 9. Gates

Full set from `EXECUTION-ORDER.md` §6 rule 9 against your final commit, **clippy as a single
invocation per target with the exit code captured explicitly**, plus `mdbook build`. Cross-target
clippy is judged from your own diff.

One commit on `main`, local, **no push, no tag**.

## 10. Out of scope

`CONTRIBUTING.md`, the Cargo metadata pass, the Git→prikk mapping page, `CODE_OF_CONDUCT.md`, issue
templates, `SUPPORT.md`, funding metadata. **And no email address.**
