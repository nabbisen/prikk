# Security Policy

Prikk is pre-1.0 experimental software. This file states what this project can and cannot promise
about security, and where to report a vulnerability privately.

## Reporting a vulnerability

Report privately through [GitHub Security
Advisories](https://github.com/prikk-vcs/prikk/security/advisories/new). **Do not open a public
issue** for a suspected vulnerability.

The interesting classes here are **identity, signature, publication, durability, and path/format
handling** — anything that could let a repository, ref, patch, or block appear authored, signed, or
maintainer-approved when it is not, or that could corrupt or misdirect a write. An ordinary bug (a
crash, a wrong result, a missing feature) is not a security report — use a public issue for those.

## Already known — please don't report these as findings

- The trust-on-first-use authorship boundary, and the absence of key rotation or revocation for
  AUTHOR keys — see [Trust and Threat Model § Core
  Caveats](./docs/src/reference/trust-threat-model.md#core-caveats).
- The platform durability gaps on Windows — see [Platform
  Support](./docs/src/reference/platform-support.md).

## What this project commits to

An accepted report will be acknowledged, and the fix will be made. **There is no CVE assignment
process and no committed response time** — this is pre-1.0 software with one maintainer, and
stating a timeline nobody has agreed to meet would be worse than stating none.

## Release-artifact verification is not yet available

If you came here to check this: **release-signer verification of a `prikk` binary is not yet
available.** The release-signer allowlist is empty and fail-closed, so no release currently carries
that authority — see [Release, Versioning, and Compatibility § Core
Caveats](./docs/src/reference/release-compatibility.md#core-caveats) for the current state. A
checksum on a downloaded binary proves transport integrity, not authorial origin.
