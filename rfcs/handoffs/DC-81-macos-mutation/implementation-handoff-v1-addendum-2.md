# DC-81 Handoff v1 — Addendum 2: CI ran, macOS mutation passed, one CI-job defect

**Date:** 2026-08-09. **Authored by** the architect. **Review:**
`.git-exclude/reviewed/DC-81-implementation-review-v1.md`.

## 1. First, an architect error you should know about

**I merged `20701b1` to `main` by accident**, violating criterion 5 — my DC-82 docs commit sat on top of
it on local `main`, and pushing carried DC-81 with it. Main went red. **Reverted at `1a30e48`**, verified
clean: no code difference from `746523a`, gate count back to 110, tests passing. `origin/main` is now
DC-81-free; your branch `dc-81-macos-mutation` at `20701b1` is untouched.

**Nothing about your work caused this**, and the branch-push procedure I wrote existed precisely to
prevent it.

## 2. The evidence you could not produce locally now exists — and it is good

The `macOS mutation test suite` job **ran on real macOS** and **every mutation test passed.** That is
DC-81's central unknown resolved: the port works. `non-linux build (macos-latest)`,
`non-linux read-only conformance (macos-latest)`, both Windows jobs, `stable`, and `msrv-1.85.0` all
green.

## 3. One defect, and it is in the CI job, not the port

The run failed on exactly one test:

```
boundary::tests::workspace_and_product_boundaries_hold
  cargo metadata failed: failed to download `bumpalo v3.20.3`
  Caused by: attempting to make an HTTP request, but --offline was specified
```

**The new job runs `cargo test --workspace --locked` with no `cargo fetch --locked` step.** Both ubuntu
test jobs have one (`ci.yml:20` and `:40`), and DC-71 established the requirement — the `fetch` entry in
`procedure.rs` carries that reason in its own comment: *"CI must populate the cache for every target
before the boundary check runs `cargo metadata --locked --offline`."*

**Two fixes; the second may be better and it is your call.**

1. **Add `cargo fetch --locked`** before the test step, matching lines 20 and 40. Already allowlisted, so
   no `procedure.rs` change.
2. **Narrow the job's scope.** It exists to run the **mutation** suite; `--workspace` drags in
   release-policy's boundary test, which is a Linux-CI concern with nothing to do with macOS durability.
   Scoping to the crates that matter avoids the cache problem entirely and makes the job say what it
   means. **If you take this, note it changes the allowlisted command and so needs a `procedure.rs`
   entry** — exact, per the DC-70 B1 / DC-77 precedent.

## 4. What is already verified and not in question

From my review: `mkfifoat`'s apple exclusion exact; cross-target clippy clean on both platforms; 893
tests on Linux; **no doc claims macOS mutation** (criterion 8 honoured); and **my own narrowness control
on your new allowlist entry rejected all three variants I tried** — only the exact vector passes.

Your `mkfifo(1)` shell-out is **endorsed**. One correction: you framed it as forced, but
`[dev-dependencies]` is **deliberately exempt** from the placement gate
(`boundary/placement.rs:46-48` — *"it is the sink this check protects"*), so a dev-dependency was
available. The shell-out is still the better call, and dropping FIFO negative controls on macOS would
have been the wrong one.

## 5. What closing DC-81 now needs

Fix the job, push the branch again, and re-submit with a **green** run. Nothing else is outstanding —
no repair is owed on the port itself.

**And the standing limit still holds:** a green run proves the suite executes and passes on macOS. It
does not prove durability; a CI runner cannot be power-cycled. Do not report it as such, and I will not
accept it as such.

## 6. Then DC-82

`rfcs/done/DC-82-MUTATION-DISPATCH-COLLAPSE.md` and its handoff are open, sequenced after DC-81
closes. §6's gate-reduction target is its subject — and, as recorded there, **DC-81 moving 110 → 135 was
my notification failure, not your miss.**
