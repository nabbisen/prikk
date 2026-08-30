# Beginner's on-ramp — tutorial, troubleshooting, FAQ

**Authority:** `ROADMAP.md` → `## Active Development Themes` → *Beginner's help*, selected by the
owner 2026-08-28. **Base:** `6f1adb1` or later `main`. **Under `003-landing-work-on-main.md`** —
commit locally on `main`, do not push, do not tag.

**Filed here because this is documentation work**, alongside TASK-14's non-goals-page handoff in the
same directory.

---

## 1. What is missing, evidenced rather than asserted

`docs/src/guide/` holds **twenty feature-organised pages** — one per command or capability — and
**no tutorial, no FAQ, and no troubleshooting page**. There is no narrative path through a first
repository.

**Three specific things make the current entry steep:**

- **`docs/src/index.md` sends a newcomer to reference pages.** After *Install* it offers *Data Model*
  and *Trust and Threat Model* — correct documents, wrong first stop.
- **The guide's second entry is *Security and Signing Setup*.** A reader meets key management before
  they have created anything.
- **The only end-to-end sequence lives outside the book**, in `README.md`'s Quick Start.

**A newcomer's first questions are not command questions.** From reading the surface as one: *what do
I run first? why did `commit` refuse? what is sealing, and must I do it? why does this need keys at
all? why is it `heads/topic` and not `topic`? how do I switch branches?* Today each is answered only
by inference across several pages.

## 2. What to build

**Three pages. Their exact titles and split are yours** — argue the shape you choose.

**2.1 A tutorial.** One continuous path: a new repository, a first commit, a first seal, then `log`,
`verify`, `doctor`. **It must confront the key setup rather than hide it** — `commit` needs
`PRIKK_AUTHOR_KEY_ID`/`PRIKK_AUTHOR_SEED` and `seal` needs a trusted maintainer, and a tutorial that
defers that will fail at the reader's second command.

**2.2 Troubleshooting.** The refusals a beginner actually hits, each with what it means and what to do.
**Derive these from the real refusal strings in the source** — do not invent or paraphrase them. If a
message is confusing, say so in the report; **do not fix product wording in this increment.**

**2.3 An FAQ.** The conceptual questions above. **Sealing and why keys exist are the two that decide
whether someone continues or leaves.**

## 3. The main adjudication: how does the tutorial not rot?

**A tutorial is prose that can drift from the CLI, and a broken first command is worse than no
tutorial at all** — it is the first thing a newcomer meets.

**`crates/prikk-cli/tests/dc67_ordinary_use_conformance.rs` already drives the compiled binary
through nine ordinary sequences "exactly as a user would."** That is the shape of an answer: a
tutorial whose sequence is also a test's sequence cannot silently stop working.

**Adjudicate and justify.** Options include anchoring the tutorial to a new or existing conformance
test, generating the transcript from a test run, or accepting drift and saying so plainly in the page.
**The criterion is whether a change to the CLI that breaks the tutorial fails something**, and if your
answer is "nothing," that must be a stated limitation rather than an omission.

**Do not build a large mechanism for this.** A test that runs the same commands is proportionate; a
documentation-generation framework is not.

## 4. Placement

**The tutorial belongs immediately after *Install* and before *Security and Signing Setup*** in
`docs/src/SUMMARY.md` — §1's second point is the reason. **Propose otherwise if you disagree**, but
say why.

`docs/src/index.md` should point at the tutorial as the first step after installing. **Its existing
reference links stay** — they are right for a different reader.

## 5. The README Quick Start

Duplication across surfaces is fine here — the owner has ruled that a reader should be able to reach
things from wherever they are. **But a duplicated *link* and a duplicated *sequence* are different
risks**: two copies of a command sequence drift apart silently.

**Adjudicate:** does the Quick Start stay as it is, shrink to a pointer, or become a trimmed summary
whose authority is the tutorial? **Say which and why.**

## 6. TASK-15 overlap — reconcile, do not start a third effort

`ROADMAP.md`'s `Post-0.16.1 Documentation Reference Backlog` carries **`TASK-15` — roles &
user-classes orientation, still `Open`** — which overlaps the audience half of this work.

**Read it before you start and say in the report whether this increment closes it, narrows it, or
leaves it untouched.** Do not edit the backlog table row itself; that is a ROADMAP change and I will
make it once you have reported.

## 7. What must not change

- **No production code.** This is documentation.
- **No invented behaviour.** Every command shown must be one the current binary accepts, and every
  quoted refusal must be a real string from the source.
- **No product wording changes** — including confusing refusal messages. Report them.
- **No new claims about platform support, release authority, or stability.** The badge's own caveat
  stands; a tutorial must not read as an invitation to trust Prikk with important history yet.

## 8. Controls

1. **Every command in the tutorial actually runs, in order, from a clean state, through the compiled
   binary.** Quote the transcript. **This is the control that matters most** — a tutorial nobody
   executed is a guess.
2. **Every quoted refusal message matches the source.** Show the grep.
3. **§3's answer, demonstrated.** If you anchor to a test, show it failing when the sequence breaks.
   If you accept drift, show where the page says so.
4. **`mdbook build` clean, `SUMMARY.md` updated, and every new internal link resolves in the built
   HTML** — `mdbook` does not check links, so verify against `docs/book/`, as TASK-14's increment did.
5. **Full gate set against the exact final commit.**
6. **Per-job CI is not owed unless you add a test** (§3). Say which applies rather than assuming.

## 9. The report

To `.git-exclude/review-request/`. Include §2's page split, §3's rot adjudication, §4's placement
argument, §5's Quick Start decision, §6's TASK-15 answer, all six controls quoted, the full gate set,
and **anything in this handoff that was wrong** — including my reading of what a newcomer's first
questions are, which came from inspecting the docs rather than from watching anyone use them.
