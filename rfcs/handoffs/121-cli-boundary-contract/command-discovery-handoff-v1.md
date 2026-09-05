# Per-command `--help`, and four flags the help does not mention — implementation handoff

**Authority:** `rfcs/done/121-cli-boundary-contract.md` §2.5.
**Base:** current `main` (`42d0d16`). **Under `003-landing-work-on-main.md`.**

**Scope: discovery only** — what the CLI tells you about itself. §2.1 (EPIPE) shipped at `0c96b06`;
the exit-code contract, argument hygiene, `unlock`'s abort and the JSON-printer `panic!` are
`exit-code-contract-handoff-v1.md`. **This increment changes no exit code and rejects no argument.**

---

## 1. There is no per-command help

`--help` is handled in exactly one place — `main.rs:88`, `None | Some("--help") | Some("-h")` — which
prints the whole-CLI listing. **`prikk bundle export --help` is not a thing.** After the README, a
user has no way to ask what a command takes.

## 2. The registry already holds the answer

`commands.rs`'s `Command` carries `help_lines: &'static [&'static str]` — **each command's help text,
pre-formatted, already in the table**, because RFC 118 stage 1 moved it there so `print_help` could
become a pure renderer holding no literal command text of its own.

**So this is routing, not authoring.** `prikk <command> --help` prints that command's own
`help_lines`. **Derive from `COMMANDS`; do not add a second table** — RFC 118 governs, and the §8
doc-coverage gate already binds `COMMANDS` to the documentation.

**Read that module's doc comment before designing.** It explains why `help_lines` stores
already-aligned text rather than a decomposed `{ form, summary }` split, and warns against
re-deriving alignment algorithmically. **Do not restructure it to make rendering tidier** — that is
the "improve the output to make rendering easier" regression it was written to prevent.

## 3. Four flags exist and the help does not mention them

| Flag | State |
|---|---|
| `verify --format json` | works, undocumented in help |
| `verify --stop-on-first-error` | works, undocumented in help |
| `unlock --force` | an undocumented alias of `--yes` (`unlock.rs:27`) |
| `doctor --repair-main-ref` | parsed and **always refused** (`args.rs:85`, `doctor.rs:771`) |

**`--repair-main-ref` must not be removed, and its shape must not change.** `args.rs:85` documents it
as *"Always refused -- no repair is implemented"*, and
`rfcs/handoffs/DC-78-history-exchange/doctor-repair-main-ref-message-handoff-v1.md` already repaired
its refusal message after a prior review. **Recognizing an input and refusing it with a reason is
better than "unknown argument" for an operation a user reaches for in trouble.** Document it as what
it is — recognized, and refused, with the reason.

**`unlock --force` is a decision, not a transcription.** It is an undocumented alias for `--yes`.
Document both, or retire the alias and document one. **Say which you chose and why**; an alias that
exists only because nobody removed it is worth naming either way.

## 4. The trap this increment is most likely to fall into

**Do not let the help text drift from the behaviour while you are documenting it.** Every flag you
add to a help line must be one you have run. `verify --format json` and `verify --stop-on-first-error`
were found by reading the parser, not the help — **check each against the real binary**, including
that the flag names are exactly right, before writing them into a line a user will type.

**And check for a fifth.** The four above came from an external audit reading `args.rs`. **Enumerate
every flag every parser accepts and diff it against what the help says** — my site lists have been
short three times this month, and this is precisely a list-of-flags problem. **If there is a fifth
undocumented flag, that is the finding.**

## 5. Constraints

- **No new dependency.** `prikk-cli` has zero and `boundary-check` enforces it.
- **No change to any command's behaviour** — this increment adds a help path and documents existing
  flags. If documenting one requires changing it, stop and report rather than doing both.
- **`prikk --help` and `prikk -h` keep working exactly as they do**, and the meta-arms
  (`--version`, `-V`, the argument-less case) stay outside `COMMANDS`, per RFC 118's prerequisite
  ruling §1.
- **The RFC 118 §8 doc-coverage gate must stay green** — it binds `COMMANDS` to `docs/src/` and
  `README.md`, so a help change that names a command wrongly fails it. Run it.

## 6. Controls

1. **`prikk <command> --help` demonstrated for at least three commands**, one of them multi-word
   (`trust maintainer add`), against the real binary.
2. **Each of the four flags run**, and its help line quoted beside its real behaviour — including
   `--repair-main-ref` refusing, so the documentation and the refusal are shown to agree.
3. **Your flag enumeration as a result** (§4): every flag every parser accepts, and which were
   already in the help.
4. **`cargo test -p prikk --bin prikk commands`** — the RFC 118 §8 gate — before and after.
5. **A test that per-command help routes from `COMMANDS`**, so a command added later without help
   text cannot pass unnoticed.

## 7. Gates

Full set from `EXECUTION-ORDER.md` §6 rule 9 against the final commit, **clippy as a single
invocation per target with the exit code captured explicitly**, plus `mdbook build` if any doc page
changes. Cross-target clippy judged from your own diff.

**No CI control** — that is the architect's at push time.

One commit on `main`, local, **no push, no tag**. **RFC 121 closes when this and
`exit-code-contract-handoff-v1.md` have both landed.**

## 8. Out of scope

Exit codes, unknown-argument rejection, duplicate-flag refusal, `unlock`'s abort path, the
JSON-printer `panic!`. JSON output for commands that lack it. A config file (`main.rs:344` defers it
deliberately).
