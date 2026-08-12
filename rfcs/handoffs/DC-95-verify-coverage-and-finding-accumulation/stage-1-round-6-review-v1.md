# DC-95 Stage 1, Round 6 — Review v1

**Reviewing:** `c1e56e6` on `dc-95-verify-coverage-and-finding-accumulation`.

**Accepted, no conditions.** 21 of 36. §3 rules the supplementary finding they raised rather than
decided.

## 1. The structural argument holds, and I checked its premises rather than its conclusion

The claim is that `ensure_ref_path_shape` is downstream-redundant **by construction**, not merely
observed to be. Its premises:

- **Canonical ref paths are deterministic.** `ref_pointer_path` is
  `refs/by-id/{ref_name_storage_key}.ref` and `ref_log_path` is `refs/logs/{…}.log`, where the key is
  `to_hex(sha256(ref_name))` — always 64 hex characters plus extension. Verified at `layout.rs:260-272`
  and `:386-388`.
- **Shape runs before the canonical check** — `scan.rs:53` then `:58` for pointers, `:111` then `:202`
  for logs. Verified.

Given both, the conclusion is deductive: no path that fails the shape check can equal either function's
output for any ref name, so whenever decode succeeds the canonical-path check necessarily rejects
anything shape would have. **A conclusion that follows from verified premises does not need its
conclusion probed** — but the empirical half is the interesting part and I ran it anyway:

```
expected a ref-path-shape error, got: malformed persisted data: invalid ref pointer magic
```

Matching their first probe exactly. Gates clean at 625 tests.

## 2. What makes this round's method better than round 5's

**They did not stop at the first probe.** Garbage content with the check disabled produced a decode
failure — which proves only that garbage cannot pass, not that a *decodable* file at a wrong-shaped path
could not. So they built a second fixture specifically to close that gap: a real pointer from a genuine
`publish`, its bytes moved verbatim to a short filename, and got `"non-canonical ref pointer"`.

**That is the corrected methodology applied without being told to.** Round 5's lesson was that one
observation can look conclusive and not be; here the second probe was constructed because the first
one's scope was recognised as narrower than the question.

And the generalisation was then argued from the path functions rather than from the two data points —
which is the right order: the data motivated the argument, the argument carries it.

## 3. Ruling on the supplementary finding: keep both, no test, record why

They investigated rather than attempted, and asked for a ruling on the duplicate pointer-identity and
duplicate ref-log-identity checks — provably unreachable, since two entries can only collide under one
ref name via a SHA-256 collision or the literal same file in one `list_directory` pass.

**Keep both. Do not remove or simplify. No test.**

- **Unreachable today is not unreachable by design.** Their unreachability follows from canonical paths
  being a deterministic hash of the ref name. That is a property of the current path scheme, not a
  stated invariant of the format. Change the scheme — add a non-hashed component, shorten the key,
  introduce a namespace prefix — and duplicates become reachable again, with nothing left to catch them.
- **A test is impossible by definition**, and demanding one would be incoherent. Record them as
  *provably unreachable* with the argument, exactly as round 1 recorded the topological-cycle check —
  their own precedent, and the right one.
- **The asymmetry matters:** keeping costs a few lines; removing converts "deterministic canonical
  paths" from an implementation detail into an unguarded load-bearing invariant, silently.

This is the same position I took on redundancy in the round 2 review — *a fact about today's code, not a
property to rely on* — applied to its limit case.

## 4. Standing

- **Round 6: accepted.** 21 of 36, plus two rows classified as provably unreachable and correctly
  untested.
- **Round 7** next: the remaining checks in this cluster needing failpoints, format-1 flips, or raw
  log-byte construction — the harder technique groups round 5 enumerated.
- Green three-platform CI before any merge.
