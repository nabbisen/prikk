# Trust and Threat Model

Prikk is early implementation software and is not a production Git replacement.

The current trust and threat model is documented authoritatively in
[FDD-04 - Current Trust and Threat Model Reference](https://github.com/nabbisen/prikk/blob/main/rfcs/fdds/FDD-04-TRUST-THREAT-MODEL.md).

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

## Current Boundary

AUTHOR signatures are real role-bound Ed25519 signatures on Patch envelopes. Prikk does not currently
implement repository-wide AUTHOR trust policy.

MAINTAINER signatures are real role-bound Ed25519 signatures on publication objects. Seal verifies the
configured MAINTAINER signer against the repository-local trust store before publication. The current
trust policy supports one trusted maintainer key with `required = 1`.

`verify` checks structural integrity and local publication trust for relevant publication objects. It
does not prove global trustworthiness, historical PKI validity, key revocation, key rotation, remote
trust, or threshold policy beyond the current minimal local policy.

## More Detail

- [FDD-04 - Current Trust and Threat Model Reference](https://github.com/nabbisen/prikk/blob/main/rfcs/fdds/FDD-04-TRUST-THREAT-MODEL.md)
- [RFC index](https://github.com/nabbisen/prikk/blob/main/rfcs/README.md)
- [Implementation status](https://github.com/nabbisen/prikk/blob/main/rfcs/IMPLEMENTATION-STATUS.md)
