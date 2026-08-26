# CI — the richer fixture leaves a file in the shared worktree, and the mutation job commits it

**Base:** current `main` (`eb72e4f`) — **`main` is red.** **Under `003-landing-work-on-main.md`.**
**Owner ruled: investigate before reverting.** This is the investigation's result and the fix.

---

## 1. What failed

`cross-platform history identity (Windows mutation, Linux verification)`. Eleven jobs pass.

```
 block 0e0c24e1…   identical
 block 3c897d0a…   identical
-block fe105b44…   Linux
+block 5926de29…   Windows
```

Two blocks match — the fixture's own, identical by construction since both platforms unpack the same
tar. **The third is the block each platform produced from the same mutation, and it diverges.**

## 2. The cause — verify this before acting on it

**Not a cross-platform identity defect.** `BlockPayload` carries **no timestamp and no platform
field**: `parent_block_ids`, `kind`, `patch_ids`, `state_merkle_root`, `snapshot_blob_ref`,
`mainline_parent_id`, `merge_baseline_block_id`. Every field is content-derived, so a divergence must
come from the patch.

**The chain:**

1. The fixture creates `docs.txt` on `heads/docs` (`ci.yml:167`) and closes the branch
   (`ci.yml:170`) — **but never removes the file.** prikk shares one worktree across every ref.
2. `windows-mutate` and `linux-mutate-reference` each then run
   `echo … >> readme.txt` → **`prikk commit`** → `seal` on `heads/main`.
3. From `heads/main`'s perspective `docs.txt` is a **new file**, and commit appends worktree changes.
4. A new file means a `CreateFile` operation, which carries a **`node_id` minted from the OS CSPRNG**
   (`node_id_gen.rs`).
5. **Each platform mints a different node id** → different patch → different `patch_ids` → different
   block.

**This is exactly the hazard the fixture's own comment names at `ci.yml:147`** — *"returning to
heads/main after opening heads/docs, which would drag docs.txt in."* **The reasoning was applied to
the authoring sequence and not extended to the jobs that consume the fixture afterward.**

**Reproduce it before fixing it.** Two independent local runs of the mutation against the fixture
should produce **different block ids on the same machine** — the randomness is per-mint, not
per-platform. **If they match, my diagnosis is wrong and everything below is void.**

## 3. This is my error, and the shape of it matters

My previous handoff's §4 told you the downstream jobs were safe and that you **need not re-establish
it**. I verified they do not *pin* fixture content — which was true. **I never asked whether a richer
fixture could change what those jobs themselves produce.**

**"Nothing is pinned" is not "nothing can diverge."** You were right to trust the instruction; the
instruction was too narrow.

## 4. The fix

**Do not create a second file.** My original §3 asked for *"more than one commit including an edit to
an already-committed file; a branch; a tag; a closed branch"* — **a second file was never required.**
`docs.txt` was added to give the branch something to commit.

**Have `heads/docs` edit `readme.txt` instead.** The branch still gets a real commit, still gets
closed, and **nothing stray is left in the worktree** — so the mutation job sees only its own append,
mints no node id, and both platforms produce identical blocks.

**Adjudicate if you disagree.** Alternatives exist — removing the file needs `rm`, which
`boundary-check`'s command grammar refuses (you already hit that); materializing a checkout after
close is heavier. **If you find a cleaner shape, take it and say why.**

**What must remain true:** the fixture still carries more than one commit, an edit to an
already-committed file, a branch, a tag, and a closed branch. **Do not solve this by making the
fixture trivial again** — the whole point was a realistic payload, and the old one passed only because
it was too simple to reach this.

## 5. Out of scope

- **Changing `node_id` minting.** Random node ids are by design.
- **Adding `rm` to the command grammar** to accommodate a test.
- **The mutation jobs themselves** — they are correct; the fixture is what changed.
- **Any product code.** This is workflow YAML.

## 6. Controls

1. **The diagnosis reproduces** (§2) — two local mutation runs against the fixture produce different
   block ids. **Quote both.** If they do not, **stop and report that instead.**
2. **The fixed fixture leaves no stray file** — show the worktree contents after the authoring
   sequence, and that `prikk status` on `heads/main` reports nothing unexpected.
3. **All five §4 elements survive** — quote them from the final YAML.
4. **`boundary-check` passes** — the command grammar still accepts every command used.
5. **Full gate set green locally**, count unmoved.

**The real control is a green CI run**, and specifically that `cross-platform history identity`
passes. **I will read per-job results and will not accept an overall conclusion.**

## 7. What to report

1. **Control 1's two block ids**, quoted — this is what confirms or refutes §2.
2. **The revised authoring sequence**, in full.
3. **Your §4 adjudication** if you chose a different shape.
4. All five controls (§6), quoted.
5. **Every numbered requirement's disposition, including ones that went without incident.**
6. Anything here was wrong — **including §2**, which is my reasoning, not a measurement.

**Stop and escalate, do not guess**, if: control 1 shows identical ids across two local runs — **that
would mean the divergence is genuinely platform-dependent, which is a far more serious finding than
this handoff assumes, and it changes what prikk can claim about cross-platform identity.**
