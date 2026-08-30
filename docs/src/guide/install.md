# Install

See the root [README](https://github.com/nabbisen/prikk#install) for what to run: `cargo binstall
prikk`, a direct download from the [release page](https://github.com/nabbisen/prikk/releases), or
`cargo install prikk` from crates.io. This page covers what comes after — the steps a new user
usually gets stuck on.

## Verify the checksum

Every downloaded release archive ships beside a `.sha256` file recording its SHA-256 digest —
for example, `prikk-x86_64-unknown-linux-gnu.tar.gz` ships beside
`prikk-x86_64-unknown-linux-gnu.tar.gz.sha256`. Download both into the same directory, then verify
from inside it, substituting the archive name for your platform.

**Linux**:

```sh
sha256sum -c prikk-x86_64-unknown-linux-gnu.tar.gz.sha256
```

**macOS** (no `sha256sum` by default; `shasum` reads the identical file format):

```sh
shasum -a 256 -c prikk-aarch64-apple-darwin.tar.gz.sha256
```

Both print `<archive name>: OK` on success and exit non-zero on a mismatch.

**Windows** (PowerShell):

```powershell
$expected = (Get-Content prikk-x86_64-pc-windows-msvc.zip.sha256).Split(' ')[0]
$actual = (Get-FileHash prikk-x86_64-pc-windows-msvc.zip -Algorithm SHA256).Hash.ToLower()
$expected -eq $actual
```

This should print `True`. **Unverified on a Windows machine** — this command was derived from how
the release build produces the checksum file (a lowercase hex digest, matching `sha256sum`'s own
format), not confirmed by running it.

A passing checksum proves the download matches what was published; it does not prove *who*
published it — see [Release, Versioning, and Compatibility](../reference/release-compatibility.md#core-caveats).

## Build from source

Prebuilt archives cover four targets: `x86_64` and `aarch64` Linux, `aarch64` macOS, and `x86_64`
Windows. Anywhere else, build it yourself — there is no separate porting step:

```sh
cargo install prikk            # from crates.io, and puts it on PATH for you
```

or from a clone, which is also what you want when working on Prikk itself:

```sh
git clone https://github.com/nabbisen/prikk
cd prikk
cargo build -p prikk --release --locked   # binary at target/release/prikk
```

### Other Linux architectures

**Fully supported.** Nothing in Prikk is gated on CPU architecture — only on the operating system —
so any architecture Rust targets on Linux builds and runs with no reduction in capability.

### FreeBSD, OpenBSD, and other platforms

**They build, but they are read-only.** Prikk compiles for FreeBSD, and nothing in it is
OpenBSD-specific, so the same applies there — but **repository mutation is refused at runtime on any
platform other than Linux, macOS, and Windows**:

```
repository mutation requires Linux, macOS, or Windows root-scoped filesystem capabilities
```

So `init`, `commit`, and `seal` will not work. Reading an existing repository does — `verify`, `log`,
`status`, `doctor`, and the other read-only commands.

**This is a deliberate boundary, not a missing port.** Mutation depends on root-scoped filesystem
primitives that have a reviewed durability implementation on those three platforms only; there is no
reviewed equivalent elsewhere, and Prikk refuses rather than writing history through a path nobody has
audited. [Platform Support](../reference/platform-support.md) states what each supported platform
guarantees, including two narrower guarantees on Windows.

## Put the binary on `PATH`

`cargo binstall`/`cargo install` place the binary in Cargo's own bin directory (`~/.cargo/bin` on
Linux/macOS), which is normally already on `PATH` once Rust is installed.

For a direct download, move the extracted `prikk` (or `prikk.exe` on Windows) into a directory
already on your `PATH` — `~/.local/bin` is a common choice on Linux/macOS if it's already there —
or add the directory you placed it in to `PATH` yourself.

## Confirm it worked

```sh
prikk --version
```

prints the version you installed, in the form:

```
prikk <version>
```

Check that `<version>` matches the release you downloaded — that match is the actual
verification, not any particular number this page could show.

If the shell reports "command not found" instead, the binary is not on `PATH` yet.

## Next: Security and Signing Setup

Installing the binary configures no signing keys. Every commit and seal needs one — continue with
[Security and Signing Setup](security-setup.md), the real first step.

## Uninstalling

Prikk places nothing outside its own binary and, inside each repository it manages, a `.prikk`
directory. Delete the binary, and remove `.prikk` from any repository whose history you no longer
want tracked — there is nothing else to clean up.
