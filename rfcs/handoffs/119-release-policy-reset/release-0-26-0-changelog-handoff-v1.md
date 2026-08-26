# 0.26.0 — changelog and version bump

**Base:** current `main` (`81fdabd`, CI green on all 12 jobs). **Under `003-landing-work-on-main.md`.**
**Authorized by the owner.** Version decided: **`0.26.0`**.

**You write the changelog and bump the version. You do not tag and you do not publish** — those are
mine, per authorized cut.

---

## 1. Why this release exists — say it in the lead

**crates.io currently serves `"Prikk CLI initial scaffold."` as `prikk`'s description**, and
`"Prikk storage crate scaffold."` for `prikk-store`. Those are fixed in the repository but
**crates.io serves each published version's own metadata**, so the correction reaches nobody until
the next publish. The seven crate READMEs render directly beneath, carrying the same stale text.

**That is the reason for this cut**, and the lead should say so plainly rather than opening with a
feature. **This is a thin release by function and the entry must read like one** — `0.25.0`'s own
*"Everything else is smaller"* is the right register. **Do not oversell it.**

## 2. Heading

```
## 0.26.0 — 2026-08-26
```

**The separator must be an em-dash, bytes `e2 80 94`.** `release-notes` extracts by exact
`## X.Y.Z — DATE` match and **fails the release** otherwise. **Byte-check it** against `0.25.0`'s
heading — that check has caught this before.

**`0.25.0` carries the same date.** That is correct and not a conflict: extraction matches on the
version, and today is the cut date per standing instruction.

## 3. There is no breaking change — do not invent a section

**Zero public API additions or removals since `0.25.0`** — I diffed for `pub fn`/`pub struct`/
`pub enum`/`pub const`/`pub use` and found none. **Zero diff lines** in `format.rs` or
`crates/prikk-object/src/payload/`.

**So: no `### Breaking change` section, and no `DECLARED_BREAKS` entry.** `0.25.0`'s entry has one
directly above; **do not mirror its shape out of habit.**

## 4. What a user actually gets

- **Conflict witnesses now report a path.** Ten sites previously discarded one that was available —
  merge evidence now says *where* a conflict is, not only that one exists. **This is the one
  behaviour change**, and it is why the version moves rather than the patch level.
- **An install page**, which the documentation site never had: checksum verification commands per
  platform, `PATH` setup, confirming the install, and uninstalling.
- **Corrected crate metadata** (§1).

**Everything else is documentation currency and test-gate hardening.** Summarize; do not enumerate
fourteen commits.

## 5. The version bump

- **`Cargo.toml:26`** — workspace `version`.
- **`Cargo.toml:37–43`** — the internal crate pins. **Count them yourself.**
- **`README.md:45`** — *"Latest released implementation: **0.25.0**"*. **Re-read the whole sentence**,
  not just the number; if anything else in it is now false, **say so**.
- **`Cargo.lock` must be regenerated.** Bumping `Cargo.toml` leaves nine member entries at the old
  version and **every `--locked` gate then fails for a reason unrelated to anything wrong.** You
  caught this at `0.25.0` when my handoff omitted it; it is a named step now.

**MSRV stays at `1.85`.** No rise, so the rise policy owes nothing. **If anything suggests otherwise,
stop** — that would mean the MSRV gate disagrees with the manifest.

## 6. Out of scope

- **Tagging, pushing a tag, and crates.io.** Mine.
- **Any code change.** If a gate fails, report it; do not fix it inside a release commit.
- **`DECLARED_BREAKS`** (§3).
- **G1's fixture**, which goes one release stale on this cut. Known, structural, and its refresh is a
  separate increment.

## 7. Controls

1. **`release-notes` extracts the new section.** Run it. **A cheap read-only proof**: invoke it with a
   nonexistent dist dir — failing on the *dist dir* means the changelog section parsed, while a bogus
   version fails earlier with `no CHANGELOG.md entry for X`. **Quote both.**
2. **The heading's em-dash is `e2 80 94`** — quote the hexdump.
3. **Every version site moved** — show no `0.25.0` remains outside `CHANGELOG.md` and `rfcs/`, and that
   `Cargo.lock` carries the new version for every member.
4. **Full gate set green** against the exact commit, after the last edit. **The test count must not
   move** — this is metadata and prose only.

## 8. What to report

1. The changelog entry as written.
2. **Your own count of the version sites**, including anything I missed.
3. All four controls (§7), quoted.
4. **Full gate set against the exact commit, after the last edit.**
5. **Every numbered requirement's disposition, including ones that went without incident.**
6. Anything here was wrong.

**Stop and escalate, do not guess**, if: `release-notes` refuses the section; a public API change turns
up that my §3 diff missed; or a version site exists beyond `Cargo.toml`, its pins, `README.md:45`, and
`Cargo.lock`.
