# DC-70 Prebuilt Binary Distribution - Handoff

**Cleared to start.** Accepted by the project owner on 2026-08-03, at
`rfcs/accepted/DC-70-PREBUILT-BINARY-DISTRIBUTION.md`.
**Authored by** the architect, who holds release scheduling by owner delegation.
**Size:** medium — and **not** the size the title suggests. See §1.
**Touches:** `.github/workflows/`, `release/fixtures/release-evidence-*.json` (likely), possibly
`tools/release-policy`, and the README's install section.

## 1. Read this before estimating

**This looks like a CI workflow. The blocking part is a schema question.**

`release/fixtures/release-evidence-*.json` models a **singular** `archive` — `name`, `checksum_name`,
`archive_sha256`, `checksum_sha256` — built around one source tarball. **Per-target binaries are N
artifacts.** So either the evidence schema extends to describe them all, or release evidence stops
describing what was actually published.

**The second is not acceptable here.** For a project selling verifiability, evidence that omits published
artifacts is a defect, not a simplification. Settle the schema before writing a workflow.

`.github/workflows/` already exists (`ci.yml`, `docs.yml`), so the build side is genuinely additive. That
is the easy half.

## 2. Blocking prerequisites — answer before designing

| Question | Note |
|---|---|
| Does the evidence model extend to N artifacts? | §1. The decision, not a detail |
| Does `release-policy` validate the archive fields? | If `check`/`boundary-check` encode the shape, this changes **oracle cases**, not just a workflow |
| Which targets, and does the code build on them? | DC-37 restricts **mutation** to Linux. Read-only commands may build elsewhere — **verify by building, do not assume** |
| What does `cargo binstall` actually require? | Asset naming plus `[package.metadata.binstall]`. Wrong naming yields a silently unused feature — which looks like success |

All four are answerable by reading plus one trial build.

## 3. The trap that matters most

**Publishing binaries makes the empty signer set more consequential, not less.**

`release-signers.toml` is empty; no release passes the DC-35 signer gate; 0.18.1 states that in its own
notes. A binary is what a user *executes*, and a checksum published beside it on the same page proves
**integrity of transport, not authority of origin**.

**Do not create the appearance of signed releases.** Either the binaries carry the same explicit
"not signer-authority-audited" statement 0.18.1's notes carry, or signing is deferred to DC-43 and said so
plainly at the download.

**Silence is the failure mode.** Nobody has to lie for a downloader to draw the wrong conclusion.

## 4. Other traps

- **Estimating from `.github/workflows/`.** §1.
- **Publishing a non-Linux binary without stating the mutation limit** at the download. A macOS binary that
  cannot commit is a support burden unless its boundary is visible where it is obtained.
- **A build only CI can reproduce.** Criterion 7 requires a documented command a third party can run from
  the tag. For a project selling verifiability, an irreproducible binary is a weak artifact.
- **Configuring `cargo binstall` without demonstrating it.** Criterion 4 says *works end to end*, not
  *configured*.
- **Reaching into package managers.** Homebrew, AUR, distro packages, containers are a **named future
  theme** and out of scope — but leave artifact naming and checksums in a shape those can consume later.

## 5. Definition of done

§2's four questions answered and reported before a workflow is written; binaries built in CI for the agreed
targets, attached to the release tag, with per-artifact checksums; **release evidence describing every
published artifact**; `cargo binstall prikk` demonstrated working; every download surface stating the
release-authority position per §3; non-Linux artifacts stating the mutation limit; a documented
third-party-reproducible build command; full gate set per `rfcs/EXECUTION-ORDER.md` §6 rule 9 plus
release-policy `check`, `boundary-check`, `reference-check`, **commands reported verbatim**.

## 6. Standing request

The install path this replaces was found broken **at its first command** on 2026-08-03 — `prikk init
<path>` fails when the directory does not exist, and the README had shipped that way. It was found by
running the sequence, not reading it. **Run what you publish.** If something here contradicts what the code
or the registry actually does, stop and report it.
