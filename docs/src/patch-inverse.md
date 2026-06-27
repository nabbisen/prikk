# Supported Patch Inverse Planning

PR-026 adds read-only inverse planning for the supported patch-operation subset.

```sh
prikk inverse-plan [path] [--ref REF]
```

The command walks the same single-parent sealed block chain used by supported patch replay. While
validating and replaying the chain, PRIKK derives an unsigned inverse Patch payload in reverse
application order.

Supported inverse shapes in PR-026:

- `CreateFile` → inverse `DeleteFile`
- `DeleteFile` → inverse `CreateFile`
- `ReplaceBinary` → inverse `ReplaceBinary` with old/new Blob IDs swapped
- full-file `EditText` → inverse full-file `EditText` using the prior UTF-8 text as replacement

Safety boundaries:

- The command is read-only.
- The inverse Patch is not written to the object store.
- The reported inverse Patch ID is only an unsigned deterministic planning hint.
- Rollback refs, authorization policy, conflict witnesses, commutation, confluence, and arbitrary
  text-span inverse handling remain later increments.
