## Prebuilt binaries

Linux only (`x86_64`/`aarch64`); repository *mutation* is Linux-only project-wide (DC-37), so this
is not an artifact-specific limitation. Each archive contains the `prikk` binary, `LICENSE`, and a
sibling `.sha256` checksum plus `.build-info.txt` recording the exact toolchain and command used to
build it — reproduce with:

```sh
git checkout <tag> && cargo build -p prikk --release --target <triple> --locked
```

`cargo install prikk` remains the toolchain-based install path; these binaries are an additional
option, not a replacement.

## Release authority — read before relying on this release

**This release does not pass the DC-35 signer-authority audit, and does not claim to.** The
committed release-signer set (`release-signers.toml`) is empty and fail-closed, so no release
currently satisfies that gate. A checksum published beside a binary on this page proves integrity of
transport, not authority of origin. Verify what you obtain by content, not by release authority —
see `prikk verify` and this project's
[release-compatibility reference](https://nabbisen.github.io/prikk/reference/release-compatibility.html).
