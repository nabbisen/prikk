# RFC 138 — `prikk trust maintainer list` and `check`

**RFC:** `rfcs/done/138-trust-policy-read-surface.md` — **§7 rules all of §4 and is settled
input.** Both surfaces, `--format json` on both, and the exit-code ruling.
**Base:** `main` at `373ba5e`.

**§3 is the part to read twice. The exit code is where this goes wrong if it goes wrong.**

---

## 1. What to build

Two subcommands under the existing `trust maintainer`, over a loader that already exists —
`load_maintainer_trust_policy(layout) -> MaintainerTrustPolicy` (`prikk-store/src/trust.rs:214`),
returning `keys: Vec<AdoptedMaintainerKey>` with `key_id: String` and `public_key: [u8; 32]`
(`:54-59`), in adoption order.

**`prikk trust maintainer list [--format json]`** — every currently adopted key: id, public key,
adoption order. An empty policy is a **successful empty result**, not an error.

**`prikk trust maintainer check --key-id <ID> [--format json]`** — whether that id is adopted.

**No new read, no new state, no change to adoption, revocation or what `seal` requires.**

## 2. `--format json` follows `verify`, it does not invent

`verify --format json` emits `"schema_version": "verify-report-v1"`
(`crates/prikk-cli/src/output/verification.rs:98`). **Match that shape** — a `schema_version` string
naming this report, then the payload. Pick a name in the same idiom; do not copy `verify`'s.

**This settles the format for these two commands and nothing else** (§7.2). The general
machine-readable surface is an unopened design question, and one command adopting an existing flag
does not answer it.

## 3. The exit code — where this goes wrong if it goes wrong

RFC 121 ruled the whole vocabulary: **`0` ok · `1` operational failure (findings, integrity failure,
refusal) · `2` usage error.**

> **"Key X is not trusted" is none of those.** The command was asked a question and answered it —
> nothing failed and nothing was refused. **`check` exits `0` whenever it determines the answer**,
> whether the answer is yes or no, and carries the answer in its output.

**Exiting `1` for a negative answer would file a successful query as an operational failure — the
exact conflation the stikk project reported to us in their first letter**, committed inside the
command written to answer their second. It is also the obvious shortcut, which is why it is written
here as a rule rather than left to taste.

`1` and `2` keep their ruled meanings: an unreadable or malformed policy is `1`; a missing or bad
`--key-id` is `2`.

## 4. Three things the output must not do

1. **Must not report a threshold as policy.** `MaintainerTrustPolicy` holds a `Vec` and nothing else —
   trust is any-of-N by construction. `policy: required=1` is a **hard-coded literal** at
   `main.rs:295` and `setup.rs:106`, printed as if read. **Do not add a third site.** (§5 puts fixing
   the existing two out of scope.)
2. **Must not read as ref authority.** `trust.rs:61-64`: a `Block`/`RefState` is trusted if **any**
   adopted key signed it — *"adopting a key never lets it move a ref; `RefStore::publish` still
   requires a signature from this operator's own signer."* **A caller who reads "trusted" as "may
   publish here" has been misled by us, not by themselves.** Word it so that cannot happen.
3. **Must not imply the key material is secret or that listing is privileged.** Every adopted public
   key was typed on the operator's own command line by `trust maintainer add --public-key HEX`.

## 5. Out of scope, and one of these has a fan-out worth knowing

- **The `required=1` literal itself.** It predates this work. **Changing it changes `prikk setup`'s
  printed output, which `docs/src/guide/first-run.md` mirrors and the 0.33.0 changelog quotes** — a
  three-place fan-out into a surface shipped hours ago. It wants its own round and its own changelog
  line. **Name it, do not fix it.**
- Adoption, revocation, thresholds, multi-maintainer policy, remote trust, key rotation — all
  unimplemented and unscheduled (`IMPLEMENTATION-STATUS.md`), and nothing here moves them.
- The general machine-readable error surface (§2).

## 6. Docs are a gate, not a courtesy

Two new registry entries means **rule (B) requires each to be explained in a declared document or
declared undocumented with a reason** (`crates/prikk-cli/src/commands/tests.rs`). Document them where
a reader looking for "can I seal here?" would find them, and declare the page.

**`docs/src/reference/trust-threat-model.md` is the page most likely to be made stale by this** — check
whether it says anything about the trust policy being unreadable, and fix it in the same round if it
does. That is the RFC-135 lesson: a round that makes a declared document false must not land without
fixing it.

## 7. Controls

1. **`check` exits `0` for a negative answer.** Assert the exit code, not just the text. This is §3
   and it is the control most likely to fail.
2. **`check` exits `2` on a missing `--key-id`** and `1` on an unreadable policy.
3. **`list` on a repository with no adopted keys succeeds and reports nothing adopted** — an empty
   policy is an answer.
4. **`list` after two `trust maintainer add` calls reports both, in adoption order**, with the public
   keys that were passed in.
5. **`check` agrees with `seal`.** On a repository where `seal` succeeds, `check` on that key id says
   trusted; after `trust maintainer remove`, it says not — and `seal` then refuses. **The two must not
   be able to disagree**, since the whole request is a caller asking `check` in order to predict
   `seal`.
6. **JSON parses and carries `schema_version`**, for both subcommands, including the empty-policy and
   not-trusted cases.

## 8. Gates

The full set from `rfcs/EXECUTION-ORDER.md` §6 rule 9, run as the last action against the exact final
commit: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features --locked
-- -D warnings`; `cargo test --workspace --locked`; `cargo +1.85.0 test --workspace --locked`;
`cargo +1.85.0 check --workspace --all-targets --locked`; `git diff --check`; `cargo audit
--no-fetch`; `RUSTDOCFLAGS="-D rustdoc::private_intra_doc_links" cargo doc --workspace --no-deps`;
release-policy `check`, `boundary-check`, `reference-check`.

**Quote every command in the gate list.** The last round's `fmt` failure reached review because the
list narrated that gate instead of listing it.

Report the `prikk` test count before and after. Cross-target clippy only if your own diff introduces
`#[cfg(target_os)]` — check the diff.

## 9. This adds a user-facing surface, so it needs a `CHANGELOG.md` entry

Under `## Unreleased` — no version, no date; the release role assigns those, and `changelog_headings`
ignores an undated heading. **Two new commands and a new JSON report is exactly the kind of change
that shipped undocumented in 0.29.0 and 0.33.0**, both times because the handoff did not ask. This one
asks.

## 10. Reporting

Per `.git-exclude/tasks/dev-team/003-landing-work-on-main.md`: commit locally on `main`, do not push,
report to `.git-exclude/review-request/`. Include §7's six control results — **§7.1 and §7.5 are the
ones that matter** — the changelog entry, and every departure.
