# RFC 118 stage 1 — the command registry

**Base:** current `main`. **Under `003-landing-work-on-main.md`.**
**RFC:** `rfcs/accepted/118-derive-never-transcribe.md`. **Prerequisites 1, 2 and 4 discharged** at
`rfcs/handoffs/118-derive-never-transcribe/prerequisite-ruling-v1.md`.

**No behaviour change. `prikk --help` must be byte-identical before and after — that is the control
(§4).**

---

## 1. The registry must own usage lines, not just command names

**This is the part most likely to be got wrong, and getting it wrong makes the stage worthless.**

`help.rs` prints **48 `prikk …` lines** for **~22 dispatchable commands**. The lines are **usage
variants**, not commands:

- `prikk doctor [path]` **and** `prikk doctor [path] --repair-wal-tail` — one command, two lines.
- `prikk checkout --plan-only …` — one of seven `checkout` variants.
- `prikk trust maintainer add --key-id ID --public-key HEX` — a subcommand path with flags.
- `prikk compact --pointer-index|--received-index|--trust-policy|--all [--plan-only]` — one line,
  alternation inside it.

**A registry holding only `(name, run)` would fix dispatch and leave all 48 lines hand-maintained** —
the duplication RFC 118 exists to remove would survive untouched. **Model the usage lines.**

Suggested shape, not mandated:

```rust
struct Usage { form: &'static str, summary: &'static str }
struct Command { name: &'static str, run: fn(Vec<String>) -> Result<(), String>, usage: &'static [Usage] }
```

**If a better shape emerges from the real data, take it and say why.** What is not acceptable is a
registry that leaves `help.rs` holding literal command text.

## 2. Dispatch

**20 of 22 arms are already `run_x(args.collect())`, and every `run_*` returns
`std::result::Result<(), String>`.** `init` takes `Option<String>`; `status` takes nothing — **one
adapter closure each.**

`--help`, `-h`, `--version`, `-V` are **meta-arms, not commands.** Keep them outside the table.

**Each `run_*` keeps parsing its own arguments exactly as today.** The registry replaces the `match`
and nothing else.

## 3. `help.rs` becomes a renderer

After this it must hold **no literal command text** — it iterates the registry and formats. Its current
column alignment, ordering, and its **one blank-line separator** must be reproduced exactly (§4).

**If alignment cannot be reproduced by rendering, say so and stop** — do not "improve" the output to
make rendering easier.

## 4. The control — byte-identical `--help`

**Before touching anything:** capture `prikk --help` and `prikk --version` from a build of current
`main`.

**After:** capture again. **`diff` must be empty.** Quote that in your report.

**This is the whole proof of "no behaviour change."** A single reordered or reflowed line is a real
regression: `--help` is a documented surface, and three tests already reference it
(`dc60_branch_management.rs`, `dc63_tag_surface.rs`).

**Also required:** the **1317 existing tests** pass unchanged. **They are the behavioural control** — no
new test may substitute for them.

**Negative control:** remove one command from the registry, confirm both that it stops dispatching *and*
that it disappears from `--help`. **Quote the observed failure**, then restore. That single mutation
proves the two surfaces now share one source.

## 5. Out of scope

- **Documentation checking** — RFC 118 §8's join gate is a later stage. **This stage creates the
  authority; it does not yet consume it.**
- **`README.md` and the guides.** Untouched.
- **Any new dependency.** Prerequisite 4: the registry is plain data, help renders with `println!`.
  **If you find yourself wanting a crate, stop and report.**
- **Changing any command's behaviour, arguments, or output.**
- **Deciding how `release-policy` will read the registry** — prerequisite 2's open sub-question,
  deliberately not settled here.

## 6. What to report

1. **The registry shape you chose**, and why, if it differs from §1.
2. **Confirmation `help.rs` holds no literal command text.**
3. **The `--help` diff — empty** (§4), quoted.
4. **The negative control**, with observed output, and confirmation the tree was clean after restoring.
5. **Test counts — expected unchanged at 1317.** A change means behaviour moved; explain it.
6. **Full gate set against the exact commit, after the last edit.**
7. Anything here that was wrong, **including my 48/22 counts**.

**Stop and escalate, do not guess**, if: a usage line cannot be expressed without embedding
command-specific formatting logic in the renderer; the `--help` diff is non-empty and the difference
looks like an improvement (**it is still a regression — report it, do not keep it**); or the registry
would need a macro or trait objects to express the two adapter cases — **prerequisite 1 says it should
not, and if it does, my ruling was wrong.**
