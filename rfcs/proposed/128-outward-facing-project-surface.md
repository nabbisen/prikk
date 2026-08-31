# RFC 128 — What an outsider finds before they read any code

**Status.** **Proposed; §2 ruled.** Raised by the external architecture audit of 2026-08-31
(`audit-2026-08-31-task-2.md` §1, §3, §4). All items independently confirmed at `3a8d730`.

**RULED by the project owner 2026-09-01: §2 option 1 — advisories-only, as project policy.**
The channel is `https://github.com/prikk-vcs/prikk/security/advisories/new`. **This supersedes the
same day's earlier acceptance of option 3**; the owner was asked for the email address, declined to
publish one, and stated the policy explicitly when the trade was named. It is recorded as a knowing
trade, not an omission — see §2a.

Both content constraints remain binding: the file states what it does not promise, and it must not
describe release-artifact signature verification as available while `release-signers.toml` carries an
empty `authorized_primary_fingerprints`.

### 2a. Advisories-only: what the trade costs, and what keeps it sound

**Nothing in this RFC is blocked any more.** The recommendation was for both channels; the ruling is
one; the file can be written. Recording the reasoning matters more than which way it went.

**What the ruling buys.** A single channel is a single channel to monitor. A published address that
nobody watches is worse than no address at all — it converts "we have no stated path" into "we have a
stated path that fails silently", and a security policy that fails silently is the failure mode this
project's house style refuses everywhere else. **An unmonitored fallback would have been a worse
outcome than no fallback**, and only the owner can know which of those they were choosing between.

**What it costs, stated plainly so nobody rediscovers it as a surprise.** Disclosure is now bound to
the forge: a reporter with no GitHub account has no path, and **the channel moves whenever the
repository moves.** That is not hypothetical here — the advisory URL above names `prikk-vcs` because
the project is moving *right now* (RFC 129), and a `SECURITY.md` published a week ago would already
be pointing at the wrong organization.

**So the trade stays sound only if the disclosure path is treated as live infrastructure, not as
prose.** Two obligations follow, and they are the substance of this ruling rather than commentary
on it:

1. **`SECURITY.md` joins RFC 129 §2's "live functional" class permanently.** Any future move of this
   repository must update it in the same increment as `release.yml` and `installer.rs` — a stale
   disclosure URL is a functional break, not a cosmetic one.
2. **The README carries the disclosure pointer too**, so the path is discoverable from the front page
   rather than only from a file whose placement GitHub happens to honour today.

**`SECURITY.md` is sequenced behind RFC 129** — the advisory URL already names `prikk-vcs`, so no
rework is needed, but nothing should be published pointing at repository infrastructure mid-move.

**Tracks.** Root-level project files, crate metadata, and the one documentation page a newcomer needs
most. No code.

---

## 1. The shape of the gap

The audit scored this project's documentation 7/10 with an unusual distribution: **566 of 566 public
items documented, zero broken links across 41 book files, RFC provenance exceeding ADR practice** —
and then almost nothing at the layer an outsider meets first. The deep tier is excellent; the entry
tier is thin.

## 2. `SECURITY.md` — and the ruling it needs

**Confirmed absent** at the repository root and under `.github/`; a search for any disclosure phrasing
across README and `docs/` finds nothing.

**For this project specifically, that absence is disproportionate.** The product's entire pitch is
signatures, trust, and verifiable history. Someone who finds a flaw in the signature-preimage binding
or the publication protocol has, today, no stated way to report it privately — so the available
options are a public issue or silence, and both are bad.

**The ruling: what is the channel?**

1. **GitHub private security advisories** — no new address to publish or monitor, integrated with the
   repository, and the reporter needs a GitHub account.
2. **A dedicated email address** — works for reporters without GitHub accounts, and needs an address
   the owner is willing to publish and monitor.
3. **Both**, advisories preferred, email as fallback.

**Recommendation: 3, and the two are not redundancy — they answer different requirements.**

- **Security argues for advisories.** A GitHub private advisory is a channel a security researcher
  already trusts and already knows how to use, it is private by construction rather than by the
  owner's inbox discipline, and it connects to the ecosystem's own machinery — CVE assignment, and
  downstream alerting for anyone depending on the published crates.
- **Longevity argues for the email address.** **An advisory channel lives exactly as long as this
  project stays on that forge.** A published address at a domain the owner controls survives a move,
  an outage, or an account problem, and it is the only option that works for a reporter with no
  account at all. For a project whose repositories are meant to outlive the tooling around them, a
  disclosure channel that cannot outlive its host is the wrong single choice.

**Two constraints on the file's content, both of which matter more than the channel:**

1. **State what is not promised** — no CVE assignment process, no response-time commitment, pre-1.0
   software. A security policy that overpromises is worse than none, and this project's house style
   refuses to overclaim everywhere else.
2. **Do not describe release-artifact signature verification as available.** `release-signers.toml`
   still reads `authorized_primary_fingerprints = []`, and the signer bootstrap is the outstanding
   badge criterion. A `SECURITY.md` that implies a reader can verify who built their binary would be
   the single most damaging overclaim this project could publish, because it is the exact thing the
   file's readers came to check.

**The channel is the owner's to choose; nothing else in this RFC waits on it.**

## 3. `CONTRIBUTING.md`

**Confirmed absent** at root and `.github/`. `docs/src/contributing/development.md` exists and is
good, and GitHub's contributor UI cannot see it.

The owner has already ruled on the duplication question that this raises — *"Duplicate is allowed,
because reader can access to docs from each"* — so a root `CONTRIBUTING.md` may restate the guidance
rather than being reduced to a link stub. **What it must add beyond the existing page:** how work is
reviewed here, and the one thing an outside contributor cannot guess — that this project runs an
architect-review discipline with a fixed gate set, so a drive-by pull request is not the expected
shape of a contribution.

## 4. Crate metadata

Confirmed per manifest:

| Field | State |
|---|---|
| `categories` / `keywords` / `homepage` | inherited by **`prikk-cli` only**; the other 7 crates inherit none |
| `documentation` | absent from all 9 manifests |
| `categories = ["algorithms"]` | wrong crates.io slug for a VCS — `development-tools`, `command-line-utilities` |
| `keywords = ["vcs"]` | 1 of 5 slots used; `version-control`, `dvcs`, `patch`, `merge` free |
| `tools/release-policy` | omits `readme` although the file exists |

Every one of these is a line in a manifest. The reason it is worth doing deliberately rather than
casually: **eight crates are published to crates.io on every release**, and seven of them currently
present as uncategorized, unkeyworded libraries with no documentation link. The publication is the
project's shop window and nobody has looked at it from outside.

## 5. The Git→prikk mapping page

The audit scores migration documentation **1 of 5** and calls this the highest-leverage single page
the project could add. It is right, and the reason is specific: **prikk's vocabulary collides with
Git's.** `commit` does not publish. There is no `HEAD`, no staging area, no branch switching. `seal`
has no Git counterpart at all. A reader who maps the words onto Git's meanings will be wrong about
five things at once and will not know it.

**What the page must contain**, at minimum: the command correspondence table (the audit's own §4
matrix is the first one that has ever existed and can seed it), and the five conceptual deltas — no
staging, no HEAD or switching, `commit` versus `seal`, messages not yet stored (RFC 123), and
file-based distribution instead of remotes.

**Explicitly not RFC 113 and not brygge.** RFC 113 is the *import contract* for carrying history in;
brygge is the tool that would do it. This is a page that explains the model to a human, and it is
useful immediately, with no importer in existence.

## 6. Constraints

- **The mapping page must state limits at the limit site**, this project's standing documentation
  rule — a row that says a Git verb is missing says so where the verb is named, not in a footnote.
- **Every command named on the page must exist**, and the page should be added to
  `DECLARED_DOCUMENTS` (RFC 118 §8) so rule (A) checks that claim mechanically.
- **`SECURITY.md` must not describe a process the owner has not agreed to run.**

## 7. Non-goals

No `CODE_OF_CONDUCT.md`, issue templates, `SUPPORT.md`, or funding metadata — the audit lists them
and they are governance choices, not gaps this RFC should decide. No crates.io description rewrites
beyond the metadata fields above.
