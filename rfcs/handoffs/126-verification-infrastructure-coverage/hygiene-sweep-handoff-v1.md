# RFC 126 §6a and §6b — the five-item hygiene sweep

**Authority:** `rfcs/done/126-verification-infrastructure-coverage.md` **§6a** (workflow
permissions) and **§6b** (AUD-05 through AUD-08, adopted 2026-09-02).
**Base:** current `main` (`8608db0`). **Under `003-landing-work-on-main.md`.**

**Scope: §6a, AUD-05, AUD-06, AUD-07, AUD-08.** RFC 126 §5 (criterion in its own member) and the
kernel doctests are separate increments. AUD-09/AUD-10 are handed over separately under
`rfcs/handoffs/121-cli-boundary-contract/cli-contract-residue-handoff-v1.md` **and take priority over
this one** — AUD-09 is a live contract violation.

**None of the five needs a design decision. Two carry a trap; both are named below.**

---

## 1. §6a — declare `permissions:` on the three check-only workflows

### First, the thing the RFC told whoever took this to check

**I checked it, so you do not have to.** The repository's own default:

```
$ gh api repos/prikk-vcs/prikk/actions/permissions/workflow
{"default_workflow_permissions":"read","can_approve_pull_request_reviews":false}
```

**It is already read-only. This item is therefore not a hardening.** It converts a property currently
held by a repository setting — invisible in the tree, changeable by any admin or an organisation
policy, and not reviewable in a diff — into one declared in the files themselves. **Say this plainly
in the report.** RFC 126 §6a's framing of `security-audit.yml` as "running third-party build scripts
with whatever the default grants" was written before the default was known and overstates the current
exposure.

### The change

`docs.yml` and `release.yml` already declare `permissions:`. Three do not:

| Workflow | trigger |
|---|---|
| `ci.yml` | push, pull request |
| `docs-pr.yml` | pull request |
| `security-audit.yml` | schedule |

Add `permissions:` with `contents: read` to each.

**Verify per workflow rather than assuming.** `contents: read` is right for a job that only checks
out and builds; a job that uploads an artifact, posts a PR comment, or writes a check run needs more,
and a workflow silently starved of a scope fails in CI rather than at review. **Read each workflow's
jobs before writing its block**, and if any job needs a scope beyond `contents: read`, give it that
scope at the job level and say why in the report.

### The gate interaction to be aware of

`reference-check` and `boundary-check` both scan `.github/workflows/`, and `command_scan` has no
shell-keyword awareness. A `permissions:` block is data, not a command, so it should pass cleanly —
**but run both gates and do not assume it.**

---

## 2. AUD-05 — `prikk-crypto` lacks the source-level `#![forbid(unsafe_code)]`

Verified at `8608db0`:

| Crate | source-level attribute |
|---|---|
| `prikk-error`, `prikk-hash`, `prikk-object`, `prikk-store`, `prikk-replay` | `lib.rs:1` |
| `prikk-cli` | `main.rs:1` |
| `tools/release-policy` | `main.rs:3` |
| `prikk-ffi` | exempt — `UNSAFE_EXEMPT_CRATES` (`unsafe_boundary.rs:82`) |
| **`prikk-crypto`** | **absent** |

Add it at `crates/prikk-crypto/src/lib.rs:1`, above the module doc, matching the five libraries.

**This is belt-and-braces, not a hole.** `Cargo.toml:59`'s `[workspace.lints.rust]` already sets
`unsafe_code = "forbid"` and every member inherits it. The attribute makes the property legible in
the file rather than only in the root manifest.

**One question to answer in the report, not to act on.** `unsafe_boundary.rs`'s `check_member` reads
each crate's **manifest**; nothing checks the source attribute. So the moment this is fixed it can
silently regress again. **Is extending that gate to the crate root worth it, and what would it cost?**
Report your assessment. **Do not implement it in this increment** — that is a gate change and gets
its own decision.

---

## 3. AUD-07 — a comment that contradicts the code

`crates/prikk-store/src/refs.rs:504-511`, on `validate_local_tag_ref`:

> `tags/V1` and `tags/v1` both pass and **coexist as distinct refs**, same as branches

**The first half is true and the second is false.** `validate_no_ref_name_collision`
(`crates/prikk-store/src/refs/publication.rs:164-175`) folds through `prikk_object::ascii_fold` and
refuses publication when an existing ref folds to the same string. The validator is permissive; the
system is not. The comment conflates the two.

**The corrected comment must name what genuinely survives, or it replaces one wrong sentence with
another:**

- collisions that **predate** the validator, or arrive by any path that does not go through
  publication;
- **NFC/NFD and non-ASCII case pairs**, which `ascii_fold` by construction cannot see — a limitation
  DC-72 §3.5 already records and which `publication.rs:160-163` already points at.

**Two things to verify rather than inherit from me:**

1. **Whether `validate_local_branch_ref` carries the same false claim.** The comment asserts it "does
   not have one either"; check the branch validator's own comment for the same defect and fix both if
   so.
2. **The `NFR-SEC-03` citation in the existing comment.** I did not verify it still names a live
   requirement. **Re-verify every citation in the paragraph you rewrite, not only the one this
   handoff flagged** — line numbers and requirement identifiers drift silently, and a rewrite is
   exactly when a stale citation gets laundered into looking fresh.

---

## 4. AUD-08 — unchecked `update_seq` increments

**The ROADMAP row names one site. It is an example, not the population.** Sites I found in production
source at `8608db0`:

```
crates/prikk-store/src/merge_execute.rs:187        let update_seq = into_ref_state.update_seq + 1;
crates/prikk-store/src/seal_from_accepted.rs:239   current.as_ref().map_or(1, |tip| tip.update_seq + 1)
crates/prikk-store/src/rfc111_seal_simulation.rs:106  .map(|(_, payload)| payload.update_seq + 1)
crates/prikk-cli/src/seal.rs:188                   .map(|state| state.update_seq + 1)
```

**Derive the list yourself and correct mine.** Test-support files
(`sync_negotiation/sync_test_support.rs:49`, `worktree_patch/tests.rs:152`) are out of scope; decide
`rfc111_seal_simulation.rs` on what it actually is rather than on its path.

**The shape already exists in this codebase** — `refs/publication.rs:271` and `refs/verify.rs:266`:

```rust
.and_then(|value| value.checked_add(1))
.ok_or_else(|| PrikkError::Integrity("ref-log sequence overflow".to_string()))?
```

**Give each site its own message naming its own subject.** Four copies of "ref-log sequence overflow"
in four unrelated places is how an error message stops being evidence.

**On testing it.** A `u64` overflow here is unreachable through ordinary use — the row says
"for consistency rather than for reachability", and that is the honest framing. **If a decode path
can naturally produce a `RefStatePayload` with `update_seq == u64::MAX`, test it. If not, say so and
add no test.** Do not manufacture a test that reaches the arm only by constructing a state the
system cannot otherwise be in; this project has been burned by controls that passed for the wrong
reason.

**This is the one behaviour change in the increment** — overflow becomes an `Integrity` error rather
than a wrap or a debug panic. RFC 126's Status line was amended to record it.

---

## 5. AUD-06 — raise three clippy lints from `warn` to `deny` — do this last

`Cargo.toml`, `[workspace.lints.clippy]`:

```toml
unwrap_used         = "warn"
expect_used         = "warn"
indexing_slicing    = "warn"
```

The audit's claim is that production occurrences are already **zero**, held by review rather than by
the build. Raising to `deny` makes the build hold it.

### Two traps

**`deny`, never `forbid`.** 128 files carry scoped `#![allow(...)]` for these lints — overwhelmingly
tests, which legitimately unwrap. `deny` is locally overridable and those allows keep working;
`forbid` is not, and would break all 128. Note that `undocumented_unsafe_blocks` **is** `forbid` in
the same table, deliberately and for a documented reason (`unsafe_boundary.rs`) — **do not
generalise from it.**

**If the flip produces findings in production code, stop and report. Do not add `#![allow]` to
silence them.** An allow added to make a gate pass is the gate failing quietly, which is the exact
failure shape RFC 127 exists to correct. A findings list is a perfectly good result for this item;
"raised to deny, and here are the six production sites that must be fixed first" is more useful than
a green build bought with allows.

**Do this item last** so that if it turns out to be larger than one line, the other four are already
committed and reviewable.

---

## 6. Gates and reporting

Full set from `rfcs/EXECUTION-ORDER.md` §6 rule 9, run verbatim from there against your final
commit — **not reproduced here**: `reference-check` treats a policy-command line outside its
registered sites as an `unregistered-reference`, and the companion handoff tripped exactly that on
its first draft.

**The set grew on 2026-09-02** and now includes
`RUSTDOCFLAGS="-D rustdoc::private_intra_doc_links" cargo doc --workspace --no-deps`.

**Separate commits per item**, so a problem in AUD-06 does not hold the other four. Local commits on
`main`; **no push, no tag, no publish.** Report to `.git-exclude/review-request/`, and state:

1. That §6a is a declaration of an existing property, not a hardening — with the `gh api` output.
2. Any workflow job needing more than `contents: read`, and why.
3. Your own derived list of `update_seq` sites, and where mine was wrong.
4. Your assessment of gating the `forbid(unsafe_code)` source attribute — **assessment only**.
5. For AUD-06: either "zero production findings, raised to deny" with the evidence, or the findings
   list and no flip.
6. Every place this handoff's claims proved wrong. **Its counts and citations are mine, and this
   project's handoffs have a consistent record of understating them.**
