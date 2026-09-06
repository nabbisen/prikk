# RFC 132 round — required amendment: put the reachability finding where the reader will be

**Follows:** `guard-the-four-lock-conflicts-handoff-v2.md`, **ACCEPTED and pushed** (`0f264f3`).
**Base:** `main` at `0f264f3`.

**This is one doc comment. Nothing else is in scope, and the round is otherwise closed.**

---

## 1. What to add

A doc comment on `RefStore::ensure_current_matches` (`crates/prikk-store/src/refs.rs:450`), which
today has **none**.

**Record what your own round established** — the finding was yours and it is a good one:

- the only production call site is `publish_locked`'s `Ready` branch
  (`refs/publication.rs:118`);
- `classify_state` (`:220`) reaches `Ready` only on the arm where `current == expected`
  (`:240-242`), having read `current` from the same `read_current_ref_state_id`;
- both reads happen under the container locks `publish_locked` holds continuously across them, so by
  the time this runs the comparison has already been established true;
- therefore this is **defence against a lock-discipline regression, not a live CAS gate** — and that
  is a reason to keep it, not to remove it;
- name `refs::tests::ensure_current_matches_refuses_a_mismatched_expectation` as where the behaviour
  is actually exercised.

**Do not remove or weaken the check.** It is correct and it stays.

## 2. Why this is required rather than noted

**The finding currently lives only in `refs/tests.rs`'s test doc.** A reader who wants to know how ref
CAS behaves arrives at the *function*, and what they find reads exactly like the operative
compare-and-swap guard on every publication. Nothing tells them otherwise.

**Absence invites the wrong inference here.** This project has already lost a formal external review
to a comment that misdescribed reality (`checkout.rs`'s "not implemented yet" led an external
architect to a false premise). An absent comment is cheaper than a false one, but the failure mode is
identical: a careful reader reasons from the code and gets it wrong.

**A round that changes what is known about a function leaves that knowledge at the function.**

## 3. Out of scope

- **Any behaviour change**, to this function or anything else.
- **Any test change.** `0f264f3`'s tests are accepted as they stand.
- **The other three guarded sites.** Only this one carries a non-obvious reachability fact.
- **RFC 132 increment 2.** Still deferred.

## 4. Controls

1. **The doc comment's claims are true at the commit you write them against.** Re-check
   `publication.rs:118` and `:240-242` yourself rather than copying the line numbers from this
   handoff — they have drifted twice in this round already. **Cite what you verified, not what I
   wrote.**
2. **`cargo doc` is clean.** A doc comment naming a private item can trip
   `rustdoc::private_intra_doc_links`, which is denied in the gate set — RFC 138's round hit exactly
   this and fixed it by using a plain code span instead of a link. Prefer code spans.

## 5. Gates

The full set, verbatim from `rfcs/EXECUTION-ORDER.md` §6 rule 9:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- `cargo test --workspace --locked`
- `cargo +1.85.0 test --workspace --locked`
- `cargo +1.85.0 check --workspace --all-targets --locked`
- `git diff --check`
- `cargo audit --no-fetch`
- `RUSTDOCFLAGS="-D rustdoc::private_intra_doc_links" cargo doc --workspace --no-deps`
- release-policy `check`, `boundary-check`, `reference-check`

## 6. No `CHANGELOG.md` entry

A doc comment on a private method. Nothing a user can observe. **Ruled here rather than left unsaid.**

## 7. Reporting

`.git-exclude/review-request/`. Short is correct for this one. State which line numbers you
re-verified and whether any had drifted.
