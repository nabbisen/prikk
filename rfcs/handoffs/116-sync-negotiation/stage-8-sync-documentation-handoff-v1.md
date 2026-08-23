# RFC 116 stage 8 — documenting `sync`: implementation handoff

**Base:** current `main`. **Documentation only** — no behaviour change, no code change beyond what
`SUMMARY.md` needs.
**Origin:** recorded during RFC 116 stage 7's review — **`sync` shipped with no prose documentation**,
while every other major surface has a `docs/src/` page. Confirmed again while writing this: there is
no page describing it, and `--help` is the only description that exists.

---

## 1. Where it goes, and the conventions it must follow

`docs/src/guide/sync.md`, listed in `SUMMARY.md` under **Guide**, after `Merge` — it is the newest
workflow and builds on merge's concepts.

**Follow the existing guide-page shape exactly** (`guide/merge.md` is the closest model):

1. A short opening naming the increment(s) that added it and linking related pages.
2. A `sh` usage block per subcommand.
3. Prose sections for what it does **and does not** do.
4. **A "Claim-to-Source Anchors" table** — `guide/security-setup.md:143` is the reference. **Every
   factual claim on the page gets a source anchor**: the file that implements it, or the RFC that ruled
   it. This is the accuracy mechanism this project uses, and it is not optional here.
5. **A "Provenance" section** stating the page is documentation-only and changes no behaviour.

## 2. The workflow — document it as a person actually runs it

Two repositories, files moved by whatever means the reader already has:

```
prikk sync summary  --output <file>        # A: publish what it has
prikk sync compare  --summary <file>       # B: which refs differ?
prikk sync have     <ref> --output <file>  # B: what B already holds, for one ref
prikk sync build    <ref> --have <file> --output <file>   # A: the delta
prikk sync accept   <file> --claims-out <file>            # B: verify and store
prikk sync pending                                        # B: what is held but unsealed
prikk sync seal     <ref> --claims <file>                 # B: seal it, under B's own key
prikk sync tags                                           # B: received tags and their state
prikk sync adopt-tag <name>                               # B: adopt one, under B's own key
```

**Show the whole loop once, in order, with the files named** — a reader should be able to follow it
without reconstructing which output feeds which input.

## 3. What the page must say plainly — these are the parts a reader will get wrong

**(a) prikk does not move the bytes, and does not encrypt them.** A built artifact contains repository
content **in the clear**. prikk guarantees integrity and authenticity, never secrecy — that belongs to
whatever channel the reader chooses. **This is the single most important sentence on the page.** It is
already stated at `sync build` time; the page must not be weaker than the CLI output.

State the reassuring half too, because it bounds the exposure: the artifact carries **public** key
material only — no secret keys, no seeds. What leaks is exactly the content the sender chose to send.

**(b) prikk stays off the network by design** (RFC 116's accepted ruling), not by omission. Every
refusal in the accept path already treats the transport as untrusted, so moving the bytes itself would
add attack surface and no verification strength. **Write it as a choice with a reason, not as a
missing feature.**

**(c) The receiver seals and tags under its own key.** Nothing arrives already trusted; no artifact can
cause a maintainer key to be adopted. The sender's blocks and tags and the receiver's are **different
objects** — and for tags, the same *global* identity via the patch set. Say this, because "the ids
differ" otherwise reads as a bug.

**(d) Divergence is not corruption.** If an accepted patch will not apply to the receiver's tip, that is
two histories moving differently and is reported as such.

## 4. The limits — state them, do not soften them

- **No remote-tracking, no named remotes, no discovery.** Every sync names files; nothing remembers who
  you synced with or what they had. Each round starts from scratch.
- **Each exchange is O(history), not O(change)** — the have-list is 32 bytes per patch in the ref's
  closure, sent whenever anything differs. Roughly 3 MB at 100,000 patches, every exchange.
- **Adopting a tag resolves by scanning local blocks**, and that cost is superlinear — measured at
  12.6 ms over 500 blocks and 86 ms over 2000. **Do not describe it as fast.**
- **The summary covers `heads/*` only.** `remotes/*` is excluded structurally; **tags are not in the
  summary but do travel in the artifact** and are adopted separately (RFC 117). **Be precise here** —
  "branches only" was true before RFC 117 and is now misleading.

**A page that omits these is worse than no page**, because the reader will infer the opposite of each
one. This project's documentation honesty — README saying "Not implemented yet" where that is true — is
a stated strength; hold the line.

## 5. Out of scope

- **Any code change** beyond adding the `SUMMARY.md` entry.
- **Reference-section pages.** This is a guide page; if you find a `reference/` page contradicted by
  what you write, **report it rather than editing it** — that is a separate finding.
- **Transport, remote-tracking, tag deletion.** Not built; do not document as forthcoming.

## 6. What to report

1. **The claim-to-source anchor table** — confirm every factual claim on the page has one, and say how
   you checked rather than asserting it.
2. **Anything you could not source.** A claim you cannot anchor is either wrong or undocumented
   behaviour; **either way I want to know, and neither should be written as fact.**
3. Whether any existing `reference/` or `guide/` page is contradicted by what you wrote (§5).
4. The **full gate set against the exact commit, after the last edit** — `fmt`, `clippy -D warnings`,
   `test --workspace --locked`, `+1.85.0 test`, `git diff --check`, `audit --no-fetch`, release-policy
   `check`/`boundary-check`/`reference-check`. **`reference-check` matters more than usual here** — it
   is the gate that validates documentation cross-references.
5. Test counts before and after — **expected unchanged**; this adds no test.
6. Anything here that turned out to be wrong. **Say so plainly.**

**Stop and escalate, do not guess**, if: a claim you need to make cannot be anchored to code or an RFC;
the CLI's actual behaviour differs from §2's flow; or you find the documentation contradicting itself
across pages.
