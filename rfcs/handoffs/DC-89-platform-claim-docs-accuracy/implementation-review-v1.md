# DC-89 — Implementation Review v1

**Reviewing:** `b0a66ea` on `dc-89-platform-claim-docs-accuracy`, off `main`.

**Verdict: the work is correct and accepted. One amendment to the RFC — mine, not theirs — extends
scope to `README.md` (§3), which my own criterion 1 wrongly excluded.**

## 1. Criterion 3 did its job

It was written because I had twice checked the file in front of me and generalized. It caught a third
instance: **`durability-recovery.md:91`** — "Authoritative directories are traversed through anchored
no-follow handles on the supported Linux mutation path" — a differently-phrased capability claim in a
file that already had three, and not on my list. Found by searching for the claim rather than my quoted
strings, which is exactly what the criterion asked for, and reported explicitly rather than folded into
the diff.

**And they caught an inconsistency in the RFC itself**: §1's prose says "eight more occurrences" while
its own table lists eleven line-references. The prose figure is simply wrong — there were eleven, and
with their find, twelve. They stated their own count and did not chase which of my two numbers was
meant. That is the right handling of an author's contradictory spec.

I re-ran the sweep independently on their branch. `docs/src` is clean: the only remaining hit for any
Linux-only phrasing is `durability-recovery.md:19`'s corrected "requires Linux **or macOS**," which
matches on the word rather than the claim.

## 2. The two judgment calls

**`architecture.md:105-110` — rewritten, not re-counted, as instructed.** The hardcoded gate count is
gone entirely, replaced by the dispatch shape stated qualitatively: each platform's implementor behind
one gated point, "a third platform is one more arm there, not a rewrite of the mutation layer." That
stays true as platforms are added, which is what criterion 2 asked for and what my original sentence
failed at. The added note that Windows resolves to a stub and fails at runtime rather than build time is
accurate and worth having.

**`architecture.md:132`'s costs row** now reads "Windows unimplemented, blocked on DC-88" instead of
"Being addressed, contract first." A concrete current blocker rather than a vague direction, with no
timing claim — criterion 5 respected.

**The evidence-claim family** is corrected to what CI actually runs, and they verified `ci.yml`
themselves rather than taking my characterization of it. Correct.

## 3. `README.md` — their process was right, my criterion was wrong

They found the same false claim in `README.md`, did **not** fix it, and reported it — reading criterion
1's literal wording ("No page in `docs/src`") as deliberate scope. That reading is defensible and the
process was right: report, do not silently expand.

But the outcome leaves the claim standing on the project's front page, which is more user-facing than
any reference page this increment corrected. **That is my criterion's fault.** It is also the third time
in this chain I have scoped "where else does this claim live" too narrowly: `platform-support.md` alone,
then seven reference pages, and I still did not think of `README.md`. The lesson I wrote down after the
second one — "the question is never *is this file accurate*, it is *where else is this claim made*" — I
then failed to apply to the increment created to enforce it.

**Amendment, mine as the RFC's author:** criterion 1 now reads *no user-facing documentation* states or
implies that mutation requires Linux or is exercised only on Linux, and `README.md` is named in scope.

**Not a bundling violation, and the distinction matters.** The argument I have used against bundling —
DC-82 out of DC-81, DC-86 out of DC-78, §3.6 out of DC-87 — is that different work needs different
proofs, so a failure becomes unattributable. Here the proof is *identical*: does any user-facing page
state that mutation is Linux-only? `README.md` was always inside that question; only my wording excluded
it. Extending is correcting the scope, not widening it.

**One correction to their finding, and it is exactly criterion 5's hazard.** They list three README
sites (`:62`, `:128`, `:137`). Only two are false:

- `:62` — "**mutation is Linux-only**" — false, fix.
- `:128` — "**Repository *mutation* is Linux-only**" — false, fix.
- `:137` — "**Prebuilt binaries remain Linux-only** (§ Install above)" — **true, and must not be
  touched.** That is a claim about release artifacts, not mutation; `:105` says the same thing and is
  equally true. Correcting it would be exactly the over-eager fix criterion 5 exists to prevent.

Same care in the surrounding `:128` paragraph: its DC-71 history — "`prikk-store` previously failed to
compile at all off Linux" — is a true statement about the past and stays.

## 4. `ci.yml` — reported correctly, and where it goes

Checked rather than assumed, and the discrimination is good: `:48` and `:92` stale, `:67` already
correct (DC-81 updated its own), `:123` a true statement about how the fixture is authored rather than a
capability claim. All four verified independently; their report is accurate on every one.

**Not a `FINDINGS.md` row** — a stale comment in a workflow carries no risk, and that register is for
risk. Recorded here and in the queue instead, to be fixed by whichever increment next touches `ci.yml`.
That will be **DC-87 Stage 2**, which has to add a Windows mutation job anyway. It keeps my own non-goal
intact rather than reversing it two days later.

## 5. Gates, re-run by me at `b0a66ea`

`mdbook build docs` clean (the `mdbook-mermaid` version warning is pre-existing); `git diff --check`
clean; release-policy `check` 154 oracle cases, `boundary-check` and `reference-check` both
`"valid": true`. Docs-only, seven files, no source or workflow touched — running `reference-check`
rather than assuming prose edits could not affect it was the right instinct.

## 6. What is required to close

One follow-up commit on the same branch: `README.md:62` and `:128` corrected, `:105`/`:137` left alone.
Then the ordinary CI run — this touches no filesystem-backed state, so the three-platform rule does not
bind it.

Nothing else. The seven-file correction is accepted as it stands.
