# RFC 116 stage 6 — assert `verify` after sync: implementation handoff

**Base:** current `main` (`b7b1ca5`).
**Why:** badge criterion 1 reads *"two machines can exchange sealed history, **and both verify it
afterward**."* RFC 116's accepted ruling identified that second clause as the load-bearing one. **It is
asserted nowhere.** `crates/prikk-cli/tests/rfc116_sync_cli.rs` proves the exchange lands and the
receiver's ref tip reaches the patches, and never runs `prikk verify` on either side.

**This is a small increment on purpose.** It adds evidence, not mechanism. It is written up because the
criterion turns on it, not because it is large.

---

## 1. What to add

In `rfc116_sync_cli.rs`, at the end of **both** end-to-end tests — the original single-block one and
`row7_multi_block_sync_completes_through_the_cli_alone` — run `prikk verify` **on both repositories**
and assert each exits successfully.

`verify` returns `Err` on any stage failure, item failure, active-WAL metadata issue, blocking ref
publication state, publication-trust issue, or commit-index divergence (`main.rs:584-600`), so **exit
status is the whole assertion.** On failure, include stdout/stderr in the panic message — a bare
"verify failed" would waste the first hour of diagnosing it.

Assert on **both** sides. The sender is not a formality: it produced the artifacts and its own state
must still be sound afterwards.

## 2. The control — and this is the part that matters

**"`verify` passed" is worthless if `verify` had nothing to check.** The risk here is a vacuous
assertion, not a wrong one.

**Required: plant a defect in the receiver and show the new assertion fires.** Something in the
material the sync itself created — corrupt or remove an object the receiver only has because of the
sync, then confirm `verify` on the receiver exits non-zero and the new assertion catches it.

Without that, the increment proves only that `verify` returns zero, which it would also do on an empty
repository.

Do **not** control this by mutating production code — there is nothing in `verify` to disable that
would be a meaningful mutation of *this* assertion. The defect goes in the fixture.

## 3. If `verify` does not pass — stop

**If either side fails `verify` after a successful sync, that is a real defect and it is not this
increment's job to fix it.** Stop, report exactly what failed and on which side, and do not adjust the
test to accommodate it.

This is the outcome I most want to hear about. Every part of the sync path was reviewed on the
expectation that a synced repository verifies; if it does not, the expectation was wrong somewhere and
I need the evidence, not a passing test written around it.

## 4. Out of scope

- **Any change to `verify`, `sync`, or the exchange formats.** This adds test assertions only.
- **Transport.** Still optional per RFC 116's ruling.
- **New fixtures beyond what §2's control needs.**

## 5. What to report

1. **The control from §2** — what defect you planted, and the actual failure text showing the
   assertion fires.
2. Confirmation that both sides are asserted, in **both** end-to-end tests.
3. The **full gate set against the exact commit, after the last edit**: `cargo fmt --all -- --check`;
   `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`;
   `cargo test --workspace --locked`; `cargo +1.85.0 test --workspace --locked`; `git diff --check`;
   `cargo audit --no-fetch`; release-policy `check` / `boundary-check` / `reference-check`.
   Cross-target clippy pair only if this diff contains `#[cfg(target_os)]`.
4. Test counts before and after. **`snapshot.txt` must not change.**
5. Anything here that turned out to be wrong. **Say so plainly.**

**Stop and escalate, do not guess**, if: `verify` fails on either side (§3 — the important one); or the
planted defect in §2 does not make `verify` fail, which would mean `verify` is not covering the synced
material and is a larger finding than this increment.
