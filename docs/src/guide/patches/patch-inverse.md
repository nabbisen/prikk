# Supported Patch Inverse Planning

PR-026 adds read-only inverse planning for the supported patch-operation subset.

```sh
prikk inverse-plan [path] [--ref REF]
```

The command walks the same single-parent sealed block chain used by supported patch replay. While
validating and replaying the chain, Prikk derives an unsigned inverse Patch payload in reverse
application order.

Supported inverse shapes in PR-026:

- `CreateFile` → inverse `DeleteFile`
- `DeleteFile` → inverse `CreateFile`

Safety boundaries:

- The command is read-only.
- The inverse Patch is not written to the object store.
- The reported inverse Patch ID is only an unsigned deterministic planning hint.
- `EditText` direct inverse for arbitrary spans remains deferred until the required round-trip vectors
  land.
- Rollback refs, authorization policy, conflict witnesses, commutation, and confluence remain later
  increments.
