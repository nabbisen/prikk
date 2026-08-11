# DC-95 Stage 1, Round 1 — Review v1

**Reviewing:** `166b080` on `dc-95-verify-coverage-and-finding-accumulation`.

**Verdict: ACCEPT the structure and the §5.1 answer. One condition on the fixtures (§3), and it matters
far more than this one test — round 1 sets the bar the other 28 checks will be built to.**

## 1. What is right

**§5.1 is answered better than I asked.** I offered table-driven as a lead; they evaluated it, adopted it
*within* the shape cluster, and declined it across all 36 with a specific reason — the remaining checks
split into fixture families (ref pointer/log construction, raw WAL byte corruption, Ed25519 signature
manipulation, cache staleness) whose only common row type would be "a closure that builds a whole broken
repository," at which point the table adds indirection without removing duplication. **Table-driven per
cluster is the right answer** and I am adopting it.

**One fresh repository per row, because `verify_repository` stops at the first hard error** — so a shared
repository would only ever prove whichever block sorts first by `ObjectId`. That is a real insight about
the test design, and it is exactly the kind of thing that would have made a subtly useless test.

All eight arms are covered; parent objects are real so existence checks (which run first) cannot be what
fires; gates are clean at 615 tests.

## 2. Verified

I re-ran their probe. Disabling `validate_block_v2_shape` fails the new test:

```
case "root-with-parent": expected error containing "Root Block must have zero parents",
got: integrity error: format-2 Block 1cd5e0a2… state root does not match authoritative replay
```

**So the test is load-bearing** — remove shape validation and it fails. That much is established.

## 3. Condition: the fixtures must be wrong *only* in shape

**Look at what that probe output actually says.** With shape validation gone, the repository is still
rejected — by the state-root check. The fixture is caught by **two** checks, and the test distinguishes
them only by asserting the message.

That makes this a **regression guard** for shape validation, which is worth having. **It does not
demonstrate what Stage 1 exists to demonstrate.** DC-95's rule — theirs, refined from mine — is *any
check whose silent absence would let a repository verify clean when it should not*. These fixtures do not
verify clean without shape validation. So they do not show shape validation matches the rule; they show
it produces the first message.

**And this is the exact confounding they solved in DC-92 and did not carry forward.** Their own DC-92
report: *"the first draft of the shape-violation test used an arbitrary wrong root... an unrelated
root-mismatch check caught it instead. Fixed by computing the block's replay-correct root via a new
`naive_continue` test helper, so the block is correct-under-replay but wrong only in shape, isolating
exactly what the test claims to cover."*

Here all eight fixtures use arbitrary roots — `MerkleRoot([0xAA; 32])` through `[0xB1; 32]`.
`naive_continue` already exists (`block_state/tests.rs:298`) and its own doc comment states the purpose.

**A wording point, because the conflation is what let this through:** the report calls this "verified
non-confounded per DC-92's bar." It verified the test is *load-bearing*. **Non-confounded** — DC-92's
actual bar — means the fixture isolates the check under test, and two checks firing on one fixture is the
definition of not isolating it.

**Required:** give each row a replay-correct `state_merkle_root`, so removing shape validation makes
`verify_repository` **pass** and the test fail on the missing rejection rather than on a different
message. **If any of the eight cannot be constructed that way, say which and why** — a `Merge` block with
no mainline parent may have no well-defined "correct" root, and that would be a legitimate reported
exception rather than something to force.

**Why this is a condition and not a note:** round 1 sets the pattern for 28 more checks across four
clusters. Fixing the standard here costs one round; discovering it at round 4 costs four.

## 4. Not conditions

- **Round 1 covering 8 of 36.** Staging is right and precedented (DC-41's four stages, DC-92's arc).
- **The remaining-28 breakdown by cluster.** Useful sequencing, correctly not committed to an order.
- **`verify/objects.rs` as the likely next cluster** — same construction family, so the table carries.

## 5. Standing

- **Accept on §3.** Then round 2.
- Green three-platform CI before any merge — `crates/prikk-store`.
- Stage 2 stays behind all of Stage 1, scoped as the two pieces the prerequisite ruling identified.
