# RFC 129 — Moving to `prikk-vcs/prikk` is not a remote change

**Status.** **Proposed.** Owner direction 2026-09-01: *"we are migrating from
https://github.com/nabbisen/prikk/ to https://github.com/prikk-vcs/prikk/. We will just have to
modify local git config after I edit GitHub repository configuration."*

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
