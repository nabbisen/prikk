# RFC 135 — the entrance: `prikk key` and `prikk setup`

**RFC:** `rfcs/proposed/135-first-run-entrance-and-configuration.md` — **§9 is the design and it is
settled input.** §9.1's deferral, §9.2's no-secret-at-rest line, §9.3's argv prohibition, §9.6's
refusal and §9.8's two rulings are all made; nothing here is a re-derivation exercise.
**Base:** `main` at `8fcc0e6`.

**The measure of success is a number, and §7 asks you to take it: how many unfamiliar steps precede a
first working repository. Today it is eleven, and the third is impossible.**

---

## 1. Why this exists, in one paragraph

A visitor who installs prikk cannot reach a sealed commit. They must invent two 32-byte seeds with no
tool, then **derive a public key from one — which no command does**. The evidence is first-hand: while
building a test repository during RFC 135's own design session, **the project architect obtained the
maintainer public key by copying a hex literal out of `.github/workflows/ci.yml`.** That is the
entrance this closes.

## 2. `prikk key`

### 2.1 `prikk key generate [--out <path>]`

Draw **32 bytes from the OS CSPRNG**, and print the public key plus **exactly the next commands the
user needs** — the `trust maintainer add` line with the key filled in, and the `export` lines.

| `--out` | Seed |
|---|---|
| absent | printed, with a plain statement that it is now in terminal scrollback |
| given | **written to that path and never printed** (§9.8.1) — output is the public key and next steps only |

**Printing it as well when `--out` is given defeats the flag's entire purpose.** That is the one thing
in this command that must not be got wrong.

**The `--out` write:** mode `0600`, **refuse to overwrite an existing file**, **refuse any path inside
`.prikk/`**. prikk never invents the location, never reads it back, never manages its lifecycle
(§9.2).

**Windows has no `0600`, and this needs a decision you must report.** `PermissionsExt` is Unix-only,
ACL work needs Win32 and `#![forbid(unsafe_code)]` plus DC-90 make that its own decision, not an
import. **Default ruling: refuse `--out` on Windows**, with an error naming the reason and pointing at
the print-and-place path. **You may implement it instead if you can show a safe permission story with
no `unsafe` and no new dependency — show the evidence, do not assert it.** Writing a secret at
inherited permissions and saying nothing is the one outcome that is not acceptable.

### 2.2 `prikk key public --seed-env <NAME>`

Derive the public key from a seed the user already holds. **`<NAME>` is the name of an environment
variable, and there is no default** — a default invites deriving the wrong key silently.

> **The seed is never accepted on argv. This is a ruling (§9.3), not a preference.**
> `/proc/<pid>/cmdline` is world-readable on Linux and shells record argv in history; a `--seed <hex>`
> flag leaks key material to every process on the machine. **Passing the variable's *name* on argv is
> fine — the name is not the secret.**

### 2.3 `prikk-crypto` needs one new function

`Ed25519KeyPair::from_seed` and `::public_key_bytes` exist, so §2.2 is a thin wrapper. **But
`::generate()` discards the seed** — the struct holds only `signing: SigningKey` and exposes no
accessor (`prikk-crypto/src/lib.rs:41-72`). §2.1 needs a seed-returning generator.

**It goes in `prikk-crypto`, not the CLI.** That crate already owns `getrandom` and already fails
closed when the OS source is unavailable; adding `getrandom` to `prikk-cli` would be a DC-51 placement
decision to justify, not an import to write.

## 3. `prikk setup` — one command, over a first-class sequence

**§9.8.2: a documented sequence reduces the step count by zero and therefore fails §9.5's own success
measure.** The individual commands stay first-class and documented — that is what a reader follows to
*understand* — and `setup` composes them for someone who wants a working repository now.

**Five properties are binding. The flag shape below is a proposal — report if it is wrong.**

1. **One command reaches a working repository** — generate, `init`, `trust maintainer add`, and the
   export block, without the user running anything else first.
2. **prikk invents no location for a secret.** The user names every output path.
3. **No secret reaches scrollback when the user provides output paths.**
4. **The trust decision is shown.** Registering a maintainer key is a trust act; a one-shot flow that
   performs it invisibly teaches a user that trust registration is a formality. **The composition may
   remove the typing, never the seeing** (§9.8.2).
5. **Seeds never on argv** (§2.2). Paths are fine.

Proposed shape:

```
prikk setup <repo-path> [--author-seed-out <path>] [--maintainer-seed-out <path>]
```

With both paths given, no seed is printed. With neither, seeds are printed and the command **says
plainly that they are now in the terminal**.

## 4. Docs are not a follow-up

**§9.7: the entrance and `docs/src/reference/git-mapping.md` are one surface.** With adoption not yet
the goal, the entrance's job is to get a reader to *understand what is different* — which is the job
that page already has. **A `setup` flow that leaves a visitor correctly configured and still surprised
has solved the smaller half.**

Concretely: the first-run path is documented as a guide page, cross-linked with `git-mapping.md`, and
**both are declared in `DECLARED_DOCUMENTS`**. Rule (B) will require it anyway — every new registry
entry must be explained in a declared document or declared undocumented with a reason — so this is not
optional politeness, it is a gate.

## 5. Out of scope — and one of these is the important one

- **`prikk config` and every policy value.** Deferred with a named trigger, **a first real adopter**
  (§9.1). Do not add durable settings, a config file, or a config format.
- **§4's dependency question in its entirety.** No config file means no format, no parser, no
  serde-or-hand-rolled decision. **It left the critical path by becoming unnecessary; do not answer
  it.**
- **A credential-helper boundary** — refused deliberately (§9.6).
- **Any secret storage beyond §2.1's single user-named write.** No keystore, no default location, no
  reading a seed back, no rotation.

## 6. Controls

1. **Count the steps, before and after, by doing it.** Fresh container or clean `HOME`, no
   pre-existing keys, no prikk knowledge assumed: reach a sealed commit and count the unfamiliar
   steps. Today's count is eleven with step 3 impossible; report the new number the same way. **This
   is the success measure, not a nicety.**
2. **`--out` never prints the seed.** Assert on captured stdout/stderr, not by reading the code.
3. **`--out` refuses an existing file, and refuses a path inside `.prikk/`.** Both, with the messages.
4. **A generated seed round-trips**: `key generate --out F`, then `key public --seed-env` over that
   seed, yields the same public key the generate step printed.
5. **`setup` prints what it trusted.** Assert the trust line appears in output (property 4 above).
6. **Two `key generate` runs produce different seeds** — one line, and it is the whole claim that the
   CSPRNG is wired.

## 7. Gates

The full set from `rfcs/EXECUTION-ORDER.md` §6 rule 9, run as the last action against the exact final
commit: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features --locked
-- -D warnings`; `cargo test --workspace --locked`; `cargo +1.85.0 test --workspace --locked`;
`cargo +1.85.0 check --workspace --all-targets --locked`; `git diff --check`; `cargo audit
--no-fetch`; `RUSTDOCFLAGS="-D rustdoc::private_intra_doc_links" cargo doc --workspace --no-deps`;
release-policy `check`, `boundary-check`, `reference-check`.

**Cross-target clippy is very likely required this round** — §2.1's Windows decision means
`#[cfg(target_os)]` or `#[cfg(unix)]` in your own diff. Check the diff rather than inferring, and run
both targets if it is there.

**The `DECLARED_DOCUMENTS` rule (A)/(B) tests will move** — new registry entries, new declared pages.
Report the `prikk` test count before and after.

## 8. Reporting

Per `.git-exclude/tasks/dev-team/003-landing-work-on-main.md`: commit locally on `main`, do not push,
report to `.git-exclude/review-request/`. Include §6's six control results — **§6.1's step count is the
headline** — the §2.1 Windows finding with its evidence, and every departure.

**If §3's flag shape is wrong, say so rather than implementing around it.** The five properties are
binding; the shape is not.
