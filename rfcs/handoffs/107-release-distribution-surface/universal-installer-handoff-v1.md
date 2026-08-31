# Universal installer and uninstaller

**Authority:** `ROADMAP.md` → `## Active Development Themes` → *Universal installer and uninstaller*,
selected by the owner 2026-08-28. **Base:** `d53df51` or later `main`.
**Under `003-landing-work-on-main.md`** — commit locally on `main`, do not push, do not tag.

---

## 1. The goal, and the objection that does not apply

**Owner's:** installing Prikk is harder than it should be for newcomers; an easier path would widen
adoption — a shell installer of the kind `rustup` and `bun` ship.

**DC-35 does not constrain this.** Its scope governs *"official upstream Prikk tags, official
release-page assets, and official package namespaces"* — authority of origin. It says nothing about
installers or how a published asset is fetched; the words do not appear in it. **An earlier architect
objection that DC-35 blocked this was wrong and is retracted in the ROADMAP entry.** The project
already tells users to download these same assets by hand and already publishes to crates.io.

**The one real constraint is what the installer may claim.** `release-signers.toml` still reads
`authorized_primary_fingerprints = []` and is fail-closed, so no release passes the DC-35
signer-authority audit. **The script and its documentation must not imply otherwise.** A checksum
proves integrity of transport, not authority of origin — `README.md`'s Install section already says
this, and the installer must not quietly contradict it.

## 2. The shape the owner named, and both halves are right

- **The script downloads from the release page** — the same assets, the same trust position as the
  manual path it replaces.
- **CI generates the script** — so it is a tracked, reviewed, versioned artifact rather than a
  hand-maintained blob.

## 3. What already exists to build on — measured, not assumed

**Four targets**, from `.github/workflows/release.yml`:

| target | archive | checksum |
|---|---|---|
| `x86_64-unknown-linux-gnu` | `.tar.gz` | `sha256sum` |
| `aarch64-unknown-linux-gnu` | `.tar.gz` | `sha256sum` |
| `aarch64-apple-darwin` | `.tar.gz` | `shasum -a 256` |
| `x86_64-pc-windows-msvc` | `.zip` | PowerShell, **deliberately matched to `sha256sum`'s line shape** |

**Asset names are uniform:** `prikk-<target>.<ext>`, `prikk-<target>.<ext>.sha256`, and
`prikk-<target>.build-info.txt` (target, commit, tag, build command, `rustc -vV`).

**RFC 107 Stage 2 already did the hard part for you.** The Windows checksum is written as lowercase
hex, two spaces, no trailing newline — *"neither a checksum `sha256sum -c` can verify nor free of the
build machine's absolute path"* is what it was fixed away from. **So one verification approach works
across all four targets.** Do not re-solve this.

## 4. What you must adjudicate, and justify

1. **What "CI generates the script" means concretely.** Generated at release time and attached as a
   release asset, or generated into the repository and committed? **These have different trust
   properties** — an asset is versioned with the release it installs; a committed file is reviewable
   in the diff but must stay correct for every release. Say which and why.
2. **How it is fetched and run.** `curl … | sh` is the convention; download-then-inspect-then-run is
   safer and slower. **You may offer both**; say which the documentation leads with.
3. **Version selection.** Latest by default, and can a user pin one? A pinned install is what a CI
   user needs.
4. **Install location and `PATH`.** What it writes, where, and whether it edits shell configuration —
   and if it does, **exactly one place, marked, so uninstall can find it.**
5. **Scope of v1.** POSIX `sh` covering Linux and macOS is the natural first cut; Windows needs
   PowerShell. **Deferring Windows to its own increment is acceptable — saying nothing about it is
   not.**
6. **Uninstall.** Not an afterthought. It must remove what the installer added, including any `PATH`
   edit, and must not remove anything it did not add.

## 5. What must not change

- **No new release targets, and no change to what `release.yml` builds or how it is packaged.** The
  installer consumes the existing surface.
- **Do not weaken the checksum discipline.** Verification is mandatory in the script, not optional.
- **No claim of signer authority** (§1).
- **No production code.** This is packaging and documentation.

## 6. Controls

1. **It installs a working binary, end to end.** Run the script as a user would, then
   `prikk --version` and one real command. **Quote the transcript.**
2. **A corrupted download is refused.** Tamper with the archive or the checksum and show the script
   failing and installing nothing. **This is the control that matters most** — an installer that
   verifies nothing is worse than the manual path, because it looks safer.
3. **Uninstall removes exactly what was installed** — binary and any `PATH` edit — and leaves
   unrelated files alone. Demonstrate both halves.
4. **Re-running the installer is safe.** Idempotent, or it says why not.
5. **The documentation matches the script**, including the claim boundary in §1. `install.md` and
   `README.md`'s Install section both need updating.
6. **Full gate set against the exact final commit**, plus `mdbook build` since docs change.
7. **Per-job CI if you add a workflow or a test.** Say which applies rather than assuming.

## 7. An incidental find, for you to fix or report

`.github/workflows/release.yml:55` says *"Repository mutation is Linux-only (DC-37)"*. **That has been
false since 0.21.0** — macOS and Windows mutate, and CI proves it per release. The comment's
*conclusion* still holds (those two targets need no mutation-limit statement), but its reason is
stale. **You will be reading this file anyway; fix the comment or report it, your call — but do not
change what the workflow does.**

## 8. The report

To `.git-exclude/review-request/`. Include §4's six adjudications with reasoning, all seven controls
quoted, the full gate set, §7's disposition, and **anything in this handoff that was wrong** —
including my table in §3, which I read out of `release.yml` rather than from a published release.
