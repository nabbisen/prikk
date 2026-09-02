# Amendment to `command-discovery-handoff-v1.md` — round 3 changed this increment's premise

**v1 stands. This adds one constraint that did not exist when v1 was written, and it is now the
central one.**
**Architect, 2026-09-02, before handing v1 to the dev team.**

---

## 1. `--help` no longer does nothing — it now fails

v1 was written at `825db86`, before round 3's argument hygiene landed (`832c40a`). At that point a
per-command `--help` was **silently ignored**, because unknown arguments were swallowed. That is no
longer true:

```
$ prikk verify --help          → exit 2   error: unknown verify argument: --help
$ prikk log --help             → exit 2   error: unknown log argument: --help
$ prikk bundle export --help   → exit 2   error: unknown bundle export argument: --help
$ prikk status --help          → exit 2   error: unknown status argument: --help
```

**A user asking for help is now told their request is a usage error.** Round 3 was right to reject
unknown arguments — but `--help` is the one argument a lost user is most likely to type, and it is
currently the least helpful thing the CLI does.

**This makes the increment more urgent than v1 framed it, and it makes one design decision for you.**

## 2. The constraint that follows: route before parsing

**`prikk <command> --help` must be recognized before the command's own argument parser runs.** Every
parser now refuses it, so routing after parsing cannot work, and adding a `--help` arm to each of the
26 parsers would put the same decision in 26 places — the exact shape round 3 just removed.

**The natural seam is `commands.rs`'s dispatch**, where `COMMANDS` is already consulted and each
entry already carries its `help_lines`. **Verify that `Command.help_lines` is still intact after
round 3 before designing against it** — round 3 edited `commands.rs`, and I checked it is (`:59`,
`:92`), but check rather than take my word.

**Say what you decided about sub-commands.** `prikk bundle --help` and `prikk bundle export --help`
are different questions, and `COMMANDS` is a flat table of dispatchable names. Whatever you choose,
choose it deliberately and say so.

## 3. One of v1 §3's four flags changed shape in round 3

**`unlock --force` is now duplicate-checked as one flag with `--yes`** — `unlock.rs:29` reads
`"--yes" | "--force" => mark_seen(&mut skip_confirmation, "--yes/--force")?`, so
`prikk unlock --yes --force` is refused as a duplicate.

That is correct, and it bears on v1 §3's open decision (document both, or retire the alias): the two
spellings are now formally one flag, which is an argument for documenting them as one rather than as
two. **Still your call, and still say which and why.**

The other three are unchanged and still undocumented in help: `verify --format json`,
`verify --stop-on-first-error` (both live in `args.rs:123-136`), and `doctor --repair-main-ref`,
whose recognized-and-refused shape round 3 preserved and which **must stay that way**.

## 4. Everything else in v1 is unchanged

Derive from `COMMANDS`, never a second table (RFC 118). Do not restructure `help_lines` to make
rendering tidier — its own doc comment explains why it stores pre-aligned text. No new dependency.
No change to any command's behaviour beyond adding the help path. **Enumerate every flag every parser
accepts and diff it against the help — and if there is a fifth undocumented flag, that is the
finding.**

**Note that round 3's own report contains a partial answer to that enumeration** — it lists the
value-carrying flags it touched across the crate. **It is a starting point, not the answer**: it
lists flags it *changed*, not flags the help omits.

## 5. Added to v1 §6's controls

6. **`prikk <command> --help` for a command whose parser previously rejected it** — before and
   after, with exit codes, so the regression this closes is demonstrated rather than described.

**No CI control** — that is the architect's at push time.
