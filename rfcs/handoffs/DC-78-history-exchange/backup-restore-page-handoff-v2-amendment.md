# Amendment to `backup-restore-page-handoff-v1.md` — five required corrections before DC-44 closes

**v1 stands in full. This adds §6–§10 and corrects v1's own §5, which was range-bound.**
**Architect review of `.git-exclude/review-request/backup-restore-page-report-v1.md`, 2026-08-31,
against `a4d875b`.**

## 6. Verdict

**The page is accepted in substance. `a4d875b` stays as it is — nothing in it is reverted, and it is
pushed along with this amendment.** §7.1 leaves one reference page still contradicting it, but that
page contradicted the shipped `bundle` commands before this increment too — landing the page makes
the documentation set less inconsistent, not more, so there is nothing to gain by holding it back.

**I re-derived every load-bearing claim against the compiled binary rather than reading the report,
running the page's own `sh` blocks verbatim in a scratch tree — not the Rust transcription of
them.** Everything the page quotes reproduced exactly:

- First export: `objects: 4`, the single-ref manifest note, `wrote ../backup.bundle`.
- Second export without `--force`: `error: refusing to overwrite existing file at ../backup.bundle
  (pass --force to overwrite it intentionally)`, exit 1.
- **Control 2 confirmed independently, and it is the strongest result in this increment.** After
  `commit` without `seal`, the re-export printed the identical `tip block:
  a3a92d54...` and `objects: 4`, and `cmp` on the two bundle files reported **byte-identical**.
  v1's §3.1 claim was right, and it was right for the reason v1 gave.
- `status` printed `queued patches: 1 targeting heads/main`; seal + `--force` export printed
  `objects: 8`; offline `bundle verify` from a directory with no repository printed all ten quoted
  lines; import printed `received remotes/heads/main` and the no-trust note; `prikk verify` after
  `trust maintainer add` printed `object items: 8 scanned, 0 failed`, `publication trust issues: 0`,
  `sealed blocks: 2`, both `sealed-block ...: dev-maintainer` lines, and `received refs: 1`.
- **A control neither the report nor its test ran: "writes nothing" checked as a fact.** I snapshotted
  the directory and the bundle's checksum around `bundle verify`. No entry created, bundle bytes
  unchanged. The page's claim holds as stated.
- `boundary-check` and `reference-check` re-run here: `{"valid": true, "errors": []}` both.

**§7's items are what the review found. None of them is in `a4d875b`'s new page's argument — three
are in the surrounding documentation the increment was supposed to bring current, and two are
staleness the page will acquire on its own at the next release.**

## 7. Required corrections — one follow-up commit

### 7.1 A fourth page still says backup/restore is deferred — **v1 §5 was range-bound, and that is my error, not yours**

`docs/src/reference/repository-layout.md:322-323`, `## Deferred and Not Stable`:

> Still deferred: garbage collection, cache rebuild semantics, quarantine enforcement, stable
> repository-format migration, **backup/restore workflows**, remote trust, hosted forge semantics, …

v1 §5 named three pages because three is what my own search found. It is a fourth, in the same
`Still deferred:` shape, now contradicted by the page this increment shipped. **Correct it in the
same shape as the other three.** That paragraph already opens with the project's own idiom for this
— `**prikk sync (RFC 116, RFC 117) and prikk merge (DC-74) have since shipped**` — so the correction
has a local pattern to follow rather than a transplanted one.

**Then sweep rather than fix the site I named.** Grep `docs/src/` for `backup` and for
`Still deferred` / `Deferred Work` / `Deferred and Not Promised` sections, and satisfy yourself that
after this commit no page states that single-ref backup/restore is deferred. **If you find a fifth,
that is the finding, and I want it named in the report — do not fix it silently.**
`integrity-recovery.md:62` ("`verify` … does not prove … backup coverage") is **not** one of these:
it is a different and still-true claim about what `verify` proves. Leave it.

### 7.2 `repository-layout.md:317` says format-2, and `init` writes 6

Same paragraph, three lines up:

> The documented writable path is a newly initialized **format-2** repository followed by deliberate
> worktree re-authoring…

The same page's own table at line 299 says `` `6` is the current format. Formats 1-5 are rejected at
open ``. I ran `prikk init` and read `.prikk/FORMAT`: **6**. So the page's stated recovery path
produces a repository the current binary refuses to open. This is pre-existing, not yours — but it
is in the exact paragraph you are already editing for §7.1, and it is the kind of stale number a
reader in trouble acts on. Correct it to the current format, or to a form that does not name a
number if the sentence does not need one.

### 7.3 `integrity-recovery.md:175` dropped a deferral that is still true

Before: `… stable diagnostic schema, backup/restore tooling, stable repository-format migration, and
production readiness.`
After: `… stable diagnostic schema, multi-ref backup export, a rehearsed
repository-format-migration restore, and production readiness.`

**`stable repository-format migration` is gone.** It is not the same claim as "a rehearsed
repository-format-migration restore" — one is the migration mechanism being stable, the other is a
restore having been rehearsed across a format change. It remains deferred everywhere else it is
stated: `concurrency-locking.md:255`, `durability-recovery.md:177`, `repository-layout.md:31` and
`:323`, `data-model.md:21`, `path-safety.md:45`, `security-setup.md:140` — **and at
`integrity-recovery.md:61`, in the same file**, which still says `verify` does not prove it. The
report's §3 says "None simply drop the phrase"; that is true of the phrase you were correcting and
not of the one beside it. **Restore it to the list.**

### 7.4 The page hard-codes `tool version: 0.27.1` three times, and nothing will catch it going stale

`docs/src/guide/backup-restore.md:68`, `:140`, `:172`. These are the only three places in all of
`docs/src/` that quote a concrete tool version in an output block — `install.md` deliberately uses
`X.Y.Z` and `<version>` after this project already learned this lesson once. At 0.28.0 every one of
them is wrong, **and the anchor test asserts nothing about that line, so it goes wrong with a green
test** — which makes the page's own closing promise ("A change to the CLI that alters any of them
fails that test") untrue for exactly this line.

**Use the page's own elision idiom** — it already prints `tip block: ...`, `patch id: ...`,
`RefState: ...` — and **add an anchor-test assertion on the line's presence, not its value**
(`contains("tool version: ")`), so the promise becomes true rather than narrower.

**Keep `repository format: 6`.** That number is stated deliberately elsewhere
(`repository-layout.md:299`, `:332`), changes only by a gated format transition rather than every
release, and is load-bearing for a reader deciding whether an old bundle is readable.

**While you are in those blocks:** the setup block (`:33-49`), the `status` block (`:113-120`), and
the seal-and-force block (`:126-146`) each omit lines the binary actually prints — `text edits: 0`,
`note: multi-operation text diff minimization …`, `note: audit plugins remain later PRs`, `status:
multi-operation text diff minimization and plugins not yet implemented`. Trimming is right; the page
already marks trims with a standalone `...` in the `verify` block and does not here. **Mark them the
same way**, so a reader whose real output has extra lines knows the page trimmed rather than that
they did something wrong.

### 7.5 Add the page to `DECLARED_DOCUMENTS`

`crates/prikk-cli/src/commands/tests.rs:34`. The report's reasoning — `tutorial.md`, `install.md`,
`troubleshooting.md`, `faq.md` are absent too — is a correct observation, but I do not think it
carries, for four reasons I want on the record:

1. The list's own doc comment states its purpose as "**declared, not scanned by wildcard (§3), so a
   new `docs/` file cannot silently escape the gate**". This is a new `docs/` file.
2. `docs/src/guide/status.md` is the direct precedent, and it went the other way: the comment says
   it was "added along with the page itself to close the one gap this gate found."
3. `sync.md` — the page this one sits beside in `SUMMARY.md` and is most analogous to — is declared.
4. **The anchor test does not bind the page's text.** It is a hand transcription; a mistyped command
   name inside the page's own fenced block is caught by nothing today. Rule (A) would catch it.

Every command the page names is real, so this should be a one-line addition — but run the gate, do
not assume. **If the four absent pages are absent by a stated rule rather than by omission, that
changes the answer: stop, and escalate with the evidence rather than adding the line.**

## 8. Controls

1. **The four documentation corrections quoted before and after**, and the §7.1 sweep's result
   stated as a result — "I searched X, found Y" — not as an assertion that nothing remains.
2. **The page re-run end to end after the §7.4 edits**, against the compiled binary, confirming the
   elided blocks still match line for line where they are not elided.
3. **`cargo test -p prikk --bin prikk commands` after §7.5**, and the new assertion from §7.4 shown
   failing against a deliberately wrong expected line before it is shown passing — the assertion has
   to be able to fail.
4. **`mdbook build` clean and the new `repository-layout.md` link resolving in `docs/book/`**, by the
   same generated-HTML check §4 of your report already used.

## 9. Observation — no change required in this increment

**`bundle export` silently takes the last `--ref` when given several.**
`prikk bundle export --ref heads/main --ref heads/other --output x` does not reject the repetition;
it exports `heads/other` (here: failed only because that ref does not exist). `parse_export_args`
(`crates/prikk-cli/src/bundle.rs:217-248`) assigns into a single `Option` for `--ref` and `--output`
alike, last write wins.

This is not a page defect — "`bundle export` takes exactly one `--ref`" is accurate. It matters
because **the page's "A bundle is one ref" section is precisely where a user learns they have
several branches**, and the next thing such a reader types is two `--ref` flags, on a CLI that will
quietly back up only one of them. Repeated-flag rejection is a candidate follow-up across the CLI's
hand-rolled parsers, not something to bolt onto this page. **Do not act on it here.** I am recording
it so it is not rediscovered by a user.

## 10. Gates and landing

Full set from `EXECUTION-ORDER.md` §6 rule 9 against the final commit, **clippy as a single
invocation per target with the exit code captured explicitly**, plus `mdbook build`. Cross-target
clippy is not required by the diff itself — re-check that against your own final diff, not against
this sentence.

One commit on `main`, local, no push, no tag. **DC-44 closes when this lands, not before.**
