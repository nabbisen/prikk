# Current State

What prikk can do today, what it cannot do yet, and the limits worth knowing before you rely on it.
**This page describes deferrals — things not built yet.** Permanent refusals, which are a different
thing, are in [Non-Goals](non-goals.md).

## What works today

The local core can initialize a repository, author signed patches, seal them into blocks, inspect
history, verify integrity, diagnose common repository issues, perform safe checkout planning and
materialization for the supported subset, display merge evidence and merge plans for explicit sealed
candidates, and **execute a merge** when the two sides are proven confluent — refusing cleanly, with no
object, WAL, or ref write, when they are not.

**Cross-platform history identity is tested, not assumed.** Prikk authors, commits, and checks out on Linux, macOS, and Windows, and CI requires a repository authored on Linux, mutated on Windows, and verified back on Linux to produce byte-identical object ids — so the claim that anyone can verify anyone's history holds across the three.

Known limits worth stating up front: merge-base discovery is manual; conflicts are detected and refused
but never resolved; sync exists between repositories, but **prikk does not move the bytes itself** —
confidentiality is the user's channel's property, not prikk's — negotiation is branch-scoped (tags
travel and are adopted separately, under the receiver's own key), and there is no discovery or
remote-tracking; `verify` cost is linear in history length; `verify` checks author signatures
repository-wide, but only as trust-on-first-use continuity — it proves the same author signed as last
time, not who that author is on first contact; and `verify` checks a locally-published tag's
maintainer signature against this repository's own trust policy, but a received, not-yet-adopted tag
is deliberately exempt — its signature is the sender's, under a key this repository has not adopted.

Next increment candidates are tracked in `ROADMAP.md`.

## Not a good fit yet

Prikk is not yet the right tool if you need:

- a production replacement for Git;
- stable repository-format compatibility;
- Git object compatibility or transparent Git interoperability;
- hosted forge workflows, or remotes;
- complete branch management, or semantic merge;
- plugin/audit execution, attestations, or automated publication controls;
- mature key lifecycle features such as revocation, rotation, hardware signing, or thresholds;
- flexible exclusion of generated files — `.prikkignore` (since 0.29.0) takes literal repo-relative
  path prefixes, one per line, with no globbing, no negation, and no per-directory files, so
  patterns like `*.log` do not work; and a file swept into history by mistake still cannot be
  removed later.
