# RFC 129 — Moving to `prikk-vcs/prikk` is not a remote change

**Status.** **Proposed.** Owner direction 2026-09-01: *"we are migrating from
https://github.com/nabbisen/prikk/ to https://github.com/prikk-vcs/prikk/. We will just have to
modify local git config after I edit GitHub repository configuration."*

**EXECUTED 2026-09-01.** Transfer confirmed (`gh repo view prikk-vcs/prikk`), remote repointed,
scoped sweep landed: **31 files, 404 occurrences.** The generated `install.sh` was regenerated and
carries `prikk-vcs/prikk` with no trace of the old slug. **§9 records four corrections this execution
forced on the inventory below — including one where §3's recommendation was simply wrong, and the
project's own gate is what caught it.**

**This RFC exists because that last sentence is not true, and finding out at the next release would
be expensive.** The remote is one line. **267 occurrences across 39 tracked files** carry the old
identity, and three of them break something real rather than merely reading wrong.

**Tracks.** Project identity across the release lane, the shipped installer, the documentation site,
and crate metadata. No product behaviour changes.

---

## 1. What actually breaks

GitHub redirects a renamed or transferred repository's URLs, so most of the 267 keep working — until
someone recreates `nabbisen/prikk`, at which point the redirect stops. **These three do not depend on
that redirect at all, and two of them break the release lane:**

### 1.1 The shipped installer downloads from the old repository — **the worst one**

`tools/release-policy/src/installer.rs:35`:

```rust
const REPO_SLUG: &str = "nabbisen/prikk";
```

`generate` substitutes it into `templates/install.sh.txt` and `uninstall.sh.txt` (`installer.rs:52`),
and the release `publish` job ships the result. **Every installer produced after the move would fetch
release assets from the old repository**, and `docs/src/guide/install.md:11` is the line telling users
to run it:

```sh
curl -fsSL https://github.com/nabbisen/prikk/releases/latest/download/install.sh | sh
```

The `curl` URL redirects. The script it fetches has the old slug compiled in.

### 1.2 The release workflow publishes to the old repository

`.github/workflows/release.yml:193` hard-codes the slug rather than using the ambient repository:

```
gh release create "$TAG" … --repo nabbisen/prikk …
```

**A cut made after the transfer would create the release on the wrong repository, or fail outright.**

### 1.3 The documentation site URL does not redirect

`Cargo.toml:32` — `homepage = "https://nabbisen.github.io/prikk/"`. **GitHub Pages URLs are not
covered by repository redirects.** The site becomes `https://prikk-vcs.github.io/prikk/`, and the old
address stops resolving. Nine tracked references point at it, including the README's documentation
badge, `README.md:299`, `tools/release-policy/src/release_notes.rs:52` (embedded in every generated
release note), and both JSON Schema `$id`s.

## 2. The four classes, and what happens to each

| Class | Where | Disposition |
|---|---|---|
| **Live functional** — breaks if unchanged | `installer.rs:35`, `release.yml:193`, `Cargo.toml:31-32` (`repository`, `homepage`), `docs/book.toml`, `release_notes.rs:52` | **Change, and change before the next cut** |
| **Live informational** — reads wrong, survives on redirect | `README.md`, `docs/src/**` (15 files, mostly Claim-to-Source Anchor tables), `docs/src/guide/install.md` | **Change** |
| **Identity** — an identifier, not a link | `release/oracle/oracle-manifest-v1.schema.json` and `release/schemas/release-evidence-v1.schema.json` `$id` fields | **Change — see §3** |
| **Historical record** — was true when written | `CHANGELOG.md`, `rfcs/done/`, `rfcs/accepted/`, `rfcs/handoffs/`, `release/fixtures/*.json` | **Do not touch — see §4** |

`crates/prikk-cli/Cargo.toml:28`'s binstall `pkg-url` needs nothing: it is
`"{ repo }/releases/download/…"` and follows `repository` automatically.

**Already-published crates.io versions keep the old `repository` URL permanently** — that metadata is
immutable per version. Only future publishes carry the new one, and that is correct: those versions
really were published from there.

## 3. The schema `$id` question, and why it is easy here

Changing a `$id` on a `-v1` schema normally means two URLs claiming to be the same schema version —
the thing a versioned identifier exists to prevent. **It is safe here, and I verified why rather than
assuming:** both schemas are loaded by repository-relative path
(`oracle/verify.rs:22,42`, `oracle/self_test/profile.rs:12`, `boundary/package.rs:234,236`), nothing
resolves them by `$id`, and nothing in the workspace fetches a schema over the network. **No consumer
is pinned to the old identifier**, so leaving two `$id`s pointing at a host that will 404 is strictly
worse than moving them with the project.

## 4. What must not be rewritten, and why it matters

**`CHANGELOG.md` and everything under `rfcs/done/`, `rfcs/accepted/`, and `rfcs/handoffs/` are dated
records.** A handoff that said `rfcs/proposed/DC-44-…` said so truly on the day it was issued; a
changelog entry describes a release that really was published from `nabbisen/prikk`. Rewriting them
would make the history of this project's own decisions retroactively false — the same principle that
kept the old path in the DC-44 handoffs when that RFC moved to `done/` last night.

**`release/fixtures/*.json` are synthetic test data**, not links: the occurrences are fake hold-record
URLs like `…/issues/2` inside release-evidence fixtures. They point at nothing, they are compared as
opaque strings by the 57 oracle cases, and changing them risks a gate failure for zero benefit.

**The sweep must therefore be scoped, not global.** A repository-wide `sed` would corrupt all four of
these and pass every gate while doing it.

## 5. Ordering — this sequence is load-bearing

1. **Owner transfers or renames the repository on GitHub**, and confirms Pages is serving at
   `prikk-vcs.github.io/prikk`.
2. **Update the local remote** (`git remote set-url`). Nothing can be pushed before step 1 completes.
3. **Land the scoped sweep** — §2's first three classes — as one increment, gates green.
4. **Verify the release lane before any cut**: regenerate the installer and confirm the emitted
   `install.sh` carries `prikk-vcs/prikk`; confirm `release.yml` resolves the right repository.

**No release may be cut between steps 1 and 4.** Between the transfer and the sweep, the release
workflow targets the wrong repository and the installer it would ship points at the old one.

## 6. Two decisions for the owner

1. **Does `authors = ["nabbisen"]` stay?** It is a person, not a location, and the transfer does not
   change authorship. Recommendation: **keep it.** Raised only so it is a decision rather than an
   omission.
2. **Should `.github/workflows/release.yml` name the repository at all?** `--repo` can be dropped so
   `gh` uses the ambient repository, which makes this class of breakage impossible next time.
   Recommendation: **drop it**, and record why — a workflow that hard-codes its own repository is a
   migration hazard by construction.

## 7. Interaction with RFC 128

RFC 128's `SECURITY.md` must be written **after** the move, or written once against
`prikk-vcs/prikk`. The advisory URL the owner supplied
(`https://github.com/prikk-vcs/prikk/security/advisories/new`) already names the new organization, so
no rework is needed — but a `SECURITY.md` shipped last week would now be pointing at the wrong
organization, which is the clearest possible demonstration of RFC 128 §2's own longevity argument.

**RFC 128 was ruled advisories-only on 2026-09-01, and that ruling adds a permanent entry to §2's
first class.** With no email fallback, the advisory URL *is* the disclosure path, so once
`SECURITY.md` exists it is **live functional infrastructure**: any future move of this repository
must update it in the same increment as `release.yml` and `installer.rs`. A stale disclosure URL
leaves a security reporter with nowhere to go, which is a functional break and not a broken link.

## 8. Non-goals

No change to crate names, published crate ownership, the tag scheme, or the binary name. No
retroactive edit of published crates.io metadata (impossible) or of released artifacts.

---

## 9. Corrections this execution forced — the inventory in §1–§6 was not complete

**Written after doing the work, not before it.** Four things above are wrong or incomplete, and one
of them would have damaged the release-evidence record if the gate had not refused it.

### 9.1 §3 was wrong: the schema `$id`s must NOT change

§3 argued that moving both JSON Schema `$id`s was safe because nothing resolves them by `$id`. **That
half is true** — verified again here: the schemas load by repository-relative path, nothing in the
workspace fetches a schema over the network, and `docs/src/schemas/` does not exist, so **neither the
old nor the new URL ever resolved to anything.**

**The conclusion drawn from it was wrong.** `release-policy check` refused the change:

```
input-identity:length:release/schemas/release-evidence-v1.schema.json
```

The normative schema's `byte_length` and `sha256` are pinned **per oracle case** — roughly **90
pinned copies** across `release/oracle/oracle-manifest-v1.json` and
`release/oracle/parked-cases-v1.json`. Each case records the schema identity it was evaluated
against. **Changing one byte of `$id` would have required rewriting ~90 per-case provenance records
so that every past case claimed it had been evaluated against a schema that did not exist when the
case was recorded.**

That is the same principle that keeps `CHANGELOG.md` and `rfcs/done/` out of this sweep (§4), and
§3 failed to apply it because it looked for `$id` *resolution* and never asked whether the *file* was
pinned. **Both `$id`s are therefore left exactly as they are**, naming a host that never served them,
because the alternative is falsifying evidence records to fix a string nothing reads.

**The gate found this, not the review.** A pinned normative input refusing a cosmetic edit is
precisely what that pin is for.

### 9.2 Three live surfaces §1 missed, one of them security-protocol

| Site | What it does | Why §1 missed it |
|---|---|---|
| `tools/release-policy/src/policy/challenge.rs:53` | The release-signer challenge validator **hard-compares the repository URL**: a challenge naming the new repository would have been rejected outright | §1 searched for links and slugs in release plumbing; this is a protocol constant inside a security check. **It is criterion-4 (signer bootstrap) infrastructure** — free to change only because `release-signers.toml` is still empty and no challenge has ever been issued. After a bootstrap it would not have been free |
| `tools/release-policy/src/command_scan/procedure.rs:222` | `gh_release_create` destructures the release command and requires `repo == "nabbisen/prikk"` — the workflow's slug is **gated**, not merely written | §1 read `release.yml` and stopped there |
| `tools/release-policy/src/installer/tests.rs:15-16` | Asserts the generated installer contains the slug | §1 named `installer.rs` and not its tests |

### 9.3 §4's "release fixtures are inert" is only half true

`release/fixtures/release-evidence-*.json` and
`release/oracle/parked-packs/release-evidence-governance-v1.json` are inert: every occurrence is a
fake `…/issues/N` hold-record URL compared as an opaque string. **Correctly excluded.**

`release/fixtures/signer-challenge-cases.json` and
`release/oracle/parked-packs/signer-challenge-v1.json` are **not** inert. Their content is challenge
text fed to §9.2's validator, so they are coupled to that constant and **had to change with it**.
A sweep that trusted §4's sentence would have left the challenge oracle cases failing.

### 9.4 §6 decision 2 reverses: keep `--repo`

§6 recommended dropping `--repo` from `release.yml` so `gh` uses the ambient repository, on the
grounds that a workflow hard-coding its own repository is a migration hazard. **§9.2 shows it is not
a hazard here, because it is pinned by a gate**: `command_scan`'s matcher requires the flag *and* its
exact value, so a move that forgets `release.yml` fails `reference-check` loudly. Dropping the flag
would delete a pinned literal from the gate whose purpose is pinning literals, and would replace a
loud failure with an implicit dependency on ambient state.

**Recommendation withdrawn. Keep `--repo`.** §6 decision 1 (`authors = ["nabbisen"]` stays — a
person, not a location) is unchanged and was applied: `Cargo.toml`, `docs/book.toml`, `LICENSE`, and
`NOTICE` all keep the name.

### 9.5 The headline count was lines, not occurrences

"267 occurrences across 39 tracked files" counted **lines containing a match**. The executed sweep
changed **404 occurrences across 31 files**, leaving 15 files deliberately untouched. The file count
fell because §4's exclusions removed eight; the occurrence count rose because multi-match lines are
common in the Claim-to-Source Anchor tables.

### 9.6 One pre-existing defect fixed in passing

`README.md:299` carried trailing whitespace that `git diff --check` flags only once the line is
otherwise modified. Removed — the line is in this diff, so it is in scope.
