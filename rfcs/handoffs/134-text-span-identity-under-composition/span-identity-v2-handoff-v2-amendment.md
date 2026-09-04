# Amendment v2 — pin v2's identity function, not only its encoding

**Amends:** `span-identity-v2-handoff-v1.md`, whose work is **accepted** (`906d015`, gates green,
1565/1565). **Base:** that commit. **Not pushed — this lands on top first.**

**One frozen vector. No behaviour change.**

---

## 1. What the controls found

Dispatch now routes both schemes through one entry point, so I perturbed each identity function
separately to check neither passes by inheriting the other's coverage:

```
compute_span_id    (v1) -> [0x7a; 32]  ->  14 tests FAIL
compute_span_id_v2 (v2) -> [0x5b; 32]  ->   4 tests FAIL
```

**Both bite — that part is sound.** But the four that fail for v2 are all pre-existing
`dc12_span_selection_*` vectors. **None of this increment's new tests fail**: not §3.4's mixed-history
replay, not the four uniqueness-stress tests, not §3.2's bundle round-trip.

**They are self-consistent.** They author with v2 and resolve with v2, so a deterministic but *wrong*
identity function round-trips perfectly. They prove the pair agrees, not that either is right.

**And `rfc114_vector_15` does not close it.** It carries `left_anchor_len: Some(64)` /
`right_anchor_len: Some(96)`, so it pins the **encoding** — tags 10/11 on the wire, the Patch object
id, the signature preimage. But its `span_id` is the literal `[0x8b; 32]`, **not a value produced by
`compute_span_id_v2`**. It pins bytes, not the function.

**Net: v1's identity is pinned by 14 tests, v2's by 4, and every test this increment added is inert to
the correctness of the thing the increment exists to introduce.**

## 2. What to add

**One frozen vector pinning a `compute_span_id_v2` output**, in the style of the `fdd01_text_span_*`
vectors that pin v1:

- **Named, literal inputs** — a fixed `node_id`, a fixed buffer, a chosen span, chosen
  `left_anchor_len`/`right_anchor_len`. No generation, no derivation from the code under test.
- **The expected 32-byte id written out as a literal**, exactly as the v1 vectors do.
- **Compute it once, by running the function, then paste it in.** That is legitimate for a frozen
  vector — the point is that the value can never change again silently. State in the test's own comment
  that the constant is a freeze of observed output, not an independently derived expectation, so nobody
  later mistakes it for a cross-check against a specification.

**Verify it bites**: perturb `compute_span_id_v2` and confirm your new vector is among the failures.
**Report that perturbation result** — a vector that does not fail when the function is broken is
exactly the shape this increment already tripped over.

**Do not** add a second vector for the encoding; `rfc114_vector_15` covers that and is correct.

## 3. Also report, not rework

`crates/prikk-store/proptest-regressions/file_codec/tests.txt` gained a seed in `906d015`:

```
cc 51d477... # shrinks to object_type = Patch, schema_version = 3
```

**A `file_codec` round-trip property failed on the schema this increment adds**, and its seed is now a
committed regression guard. The test passes, so the outcome is healthy — **but it was not in §8's "what
did not change" list, and the story matters**: adding schema 3 initially broke a codec property.

**Say in your report what that property was and why schema 3 broke it.** No code change is expected.

## 4. Gates

Full set from `rfcs/EXECUTION-ORDER.md` §6 rule 9 against your final commit. Expect **+1** test in
`prikk-store` and no other count movement.

Local commit on `main`; **no push.** Report to `.git-exclude/review-request/`.
