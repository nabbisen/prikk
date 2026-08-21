# RFC 116 stage 4 — the `sync` command: implementation handoff

**Design:** `rfcs/handoffs/116-sync-negotiation/design-v1.md` (the flow in §1.2, as amended: per ref).
**RFC:** `rfcs/accepted/116-sync-negotiation-and-transport.md` — ruling 2: **prikk stays off the
network.** This command reads and writes **local files only**.
**Base:** current `main`. **This is the increment that makes badge criterion 1 reachable.**

**Why this exists, stated plainly:** RFC 115 and RFC 116 built and verified the whole exchange loop in
`prikk-store`, and **every handoff in that arc said "CLI wiring is out of scope."** That was my
exclusion, repeated five times, and I then reported transport as the only remaining gap — which was
wrong. The binary has 22 commands and none of them is `sync`, so **no user can exchange anything, over
any channel.** The machinery is finished and unreachable. This closes that.

---

## 1. Precedent to follow

`crates/prikk-cli/src/bundle.rs` is the model and should be followed closely: `run_bundle` dispatches on
a subcommand, `run_export` writes bytes with `std::fs::write`, `run_import` reads them. **Same shape,
same error-message style, same argument-parsing idiom.** Do not invent a second CLI pattern.

## 2. One library addition first — the comparison is missing

`build_sync_summary` and `decode_sync_summary` exist, but **nothing exported answers "which refs
differ?"** — stage 2's own asymmetry test does that arithmetic inline. The CLI must not be where that
logic lives.

Add to `prikk-store`:

```
compare_sync_summary(layout, remote: &[SyncSummaryRefEntry]) -> Result<Vec<SyncRefComparison>>
```

with a four-state outcome per ref: **`InSync`**, **`Differs`**, **`RemoteOnly`**, **`LocalOnly`**.

**None of the four is a refusal** — this is the property stage 2's review flagged as pinned by a passing
test and by no control. Asymmetric ref sets are ordinary. Give this one a real test **and** a control:
introduce a refusal on the `RemoteOnly`/`LocalOnly` path and show it failing.

Local digests come from `compute_patch_set_digest_for_ref` (RFC 115 Stage 1). Branches only, matching
stage 2's ruled scope.

## 3. The command surface

```
prikk sync summary  --output <file>
prikk sync compare  --summary <file>
prikk sync have     <ref> --output <file>
prikk sync build    <ref> --have <file> --output <file>
prikk sync accept   <file>
prikk sync pending
prikk sync seal     <ref> --claim <id>
```

Each is a thin wrapper: parse, call the exported function, write bytes or print a report. **No logic
in the CLI beyond argument handling and file I/O.**

- **`summary`** → `build_sync_summary`.
- **`compare`** → `decode_sync_summary` then §2's new function; print one line per ref with its state.
- **`have`** → `build_have_list` for one ref.
- **`build`** → `build_sync_artifact`. On `AlreadyInSync`, **print that and write no file** — do not
  emit an empty artifact and do not error.
- **`accept`** → `accept_exchange_artifact`. **Print the claim ids**, from
  `AcceptReport.claim_signature_outcomes`, alongside each claim's signature outcome. This is the only
  way an operator learns what to pass to `sync seal`, so it is load-bearing output, not decoration.
  Print `Unverifiable` outcomes plainly — the operator must be able to see they are about to seal on an
  unattributed order (D6 §11.6).
- **`pending`** → `accepted_but_unsealed_patch_ids`. Until `seal` runs, this is the only observable
  evidence an accept did anything.
- **`seal`** → `seal_from_accepted_claim`. Report `Sealed` / `AlreadySealed` distinctly.

**Naming is the owner's to override.** `sync seal` sits beside the existing top-level `seal`, which does
something different (WAL → block). I judged the namespacing sufficient and the word correct; say so in
your report if it reads as confusing in use, and do not rename on your own initiative.

## 4. Bounds and options

The library already enforces every bound — `DEFAULT_HAVE_LIST_MAX_*`, `DEFAULT_SYNC_SUMMARY_MAX_*`,
`AcceptOptions`. **Pass the defaults; expose overrides the way `bundle import` exposes its own.** Do not
re-implement or re-check bounds in the CLI.

## 5. Security

- **No network. No socket. No new dependency.** Local file I/O only.
- **Every input file is untrusted.** It arrived by some channel prikk knows nothing about. The library's
  refusals do the work — do not weaken them by pre-parsing in the CLI.
- **Do not print key material.** Claim ids, patch ids, block ids and ref names are fine.
- **A failed subcommand must leave nothing behind** — in particular `build` on `AlreadyInSync` writes no
  output file, and a refused `accept` is the library's business, unchanged.

## 6. The test that actually matters

**An end-to-end two-repository sync driven entirely through the CLI**, no direct library calls:

1. Repo A seals a patch. Repo B is empty.
2. `sync summary` in A → file. `sync compare --summary` in B → reports the ref differs.
3. `sync have` in B → file. `sync build --have` in A → artifact.
4. `sync accept` in B → prints a claim id. `sync seal --claim` in B.
5. **Assert B's ref tip now reaches A's patch** — read it back, do not infer it from exit codes.

**This is criterion 1's evidence.** Everything else in this increment is plumbing; this is the test that
says two machines can exchange sealed history and both verify it afterward. Write it so that it would
fail if any one subcommand were removed.

## 7. Out of scope

- **Transport of any kind.** RFC 116 ruling 2. The files move by whatever means the user already has.
- **Tag sync** — stage 2 ruled tags out of the summary; unchanged.
- **Any change to the exchange formats or the claim schema.** The schema window is closed.
- **Discovery, remote identity, remote-tracking semantics.** DC-78 §D4 left these out.

## 8. What to report

1. Control output for §2's asymmetry refusal, and for each behavioural claim in §3 you tested — actual
   failure text, and the single line mutated.
2. **§6's end-to-end test in full**, including what you read back from B's ref tip.
3. The **full gate set against the exact commit, after the last edit**: `cargo fmt --all -- --check`;
   `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`;
   `cargo test --workspace --locked`; `cargo +1.85.0 test --workspace --locked`; `git diff --check`;
   `cargo audit --no-fetch`; release-policy `check` / `boundary-check` / `reference-check`.
   Cross-target clippy pair only if this diff contains `#[cfg(target_os)]`.
4. Test counts before and after, per crate. **`snapshot.txt` must not change.**
5. **Whether the command surface in §3 reads naturally in use** — you will be the first to actually run
   it, and I have only designed it. Say if `sync seal` beside `seal` is confusing.
6. Anything here that turned out to be wrong. **Say so plainly.**

**Stop and escalate, do not guess**, if: §2's comparison cannot be built from the exported primitives;
the flow in §3 needs a step I have not listed; or §6's end-to-end test cannot be driven through the CLI
alone — **that last one would mean the surface is still incomplete, which is this increment's whole
purpose.**
