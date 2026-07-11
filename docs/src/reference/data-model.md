# Data Model

Prikk is early implementation software and is not a production Git replacement.

`.prikk/` is Prikk's native repository format and is not Git-compatible storage. The current data model
is documented authoritatively in
[FDD-00 - Current Data Model Reference](https://github.com/nabbisen/prikk/blob/main/rfcs/fdds/FDD-00-DATA-MODEL.md).

## Core Caveats

- Prikk is early implementation software and is not a production Git replacement.
- `.prikk/` is Prikk's native repository format and is not Git-compatible storage.
- Ref files are pointers, not roots of trust.
- Maintainer trust is repository-local with the current minimal `required = 1` policy.
- `verify` is not a global trust proof.
- There is no key rotation, revocation, hardware signing, remote trust, sync trust, or stable migration
  policy yet.
- Durability and recovery claims are supported by current unit and integration tests, not by a
  completed crash-matrix or fuzzing campaign.
- Linux is the only platform exercised by the current project gates; cross-platform fsync and path
  semantics remain design targets.

## Current Shape

- Patch: an identity-bearing logical change with ordered operations and an AUTHOR signature.
- Block: an immutable sealed history unit containing Patch ids and parent Block ids.
- RefState: signed content-addressed state for a branch or tag ref.
- RefUpdate: signed append-only publication evidence for a ref transition.
- WAL: active signed Patch envelopes before sealing.
- Verify and doctor: read-only validation plus narrow repair diagnostics for supported cases.

The current implementation keeps replay and lifecycle semantics internally scoped. Public docs should
not treat `prikk-replay` or other workspace crates as stable external APIs.

## More Detail

- [FDD-00 - Current Data Model Reference](https://github.com/nabbisen/prikk/blob/main/rfcs/fdds/FDD-00-DATA-MODEL.md)
- [RFC index](https://github.com/nabbisen/prikk/blob/main/rfcs/README.md)
- [Implementation status](https://github.com/nabbisen/prikk/blob/main/rfcs/IMPLEMENTATION-STATUS.md)
