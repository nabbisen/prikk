# First Run: Keys and Setup

Read this before [Install](install.md)'s next step, [Tutorial](tutorial.md) — this page is about
getting your *own* signing keys, not the shared example seed the tutorial deliberately reuses so its
own walkthrough is reproducible.

Every `prikk commit` and every `prikk seal` needs a real Ed25519 key. Before `prikk key` and `prikk
setup` existed, obtaining one meant inventing 32 bytes by hand and then having no way at all to derive
the matching public key — reproduced first-hand while preparing this project's own release testing,
by copying a public key out of a CI configuration file rather than deriving it. That gap is what this
page closes.

## The fast path: `prikk setup`

`prikk setup` composes `init`, key generation for both roles, `trust maintainer add`, and the export
lines you need, into one command:

```sh
prikk setup ./my-repo
```

```
initialized Prikk repository at ./my-repo/.prikk
trusted maintainer key: maintainer
policy: required=1

export these before committing:
  export PRIKK_AUTHOR_KEY_ID="author"
  export PRIKK_AUTHOR_SEED="..."
  export PRIKK_MAINTAINER_KEY_ID="maintainer"
  export PRIKK_MAINTAINER_SEED="..."
note: at least one seed above is now in your terminal scrollback -- treat it as a secret

next steps:
  prikk commit -m "<message>"
  prikk seal --allow-no-audit  # no audit trust policy is configured yet; see `prikk seal --help`
```

**The `...`s draw fresh from your OS's CSPRNG every run — copy your own, never anyone else's.**
Run the printed `export` lines, then commit and seal as usual:

```sh
export PRIKK_AUTHOR_KEY_ID="author"
export PRIKK_AUTHOR_SEED="<the value setup printed>"
export PRIKK_MAINTAINER_KEY_ID="maintainer"
export PRIKK_MAINTAINER_SEED="<the value setup printed>"
echo "hello prikk" > ./my-repo/readme.txt
(cd ./my-repo && prikk commit --from-worktree -m "genesis")
(cd ./my-repo && prikk seal --allow-no-audit)
```

**`setup` never invents a location for a seed and never reads one back.** With neither
`--author-seed-out` nor `--maintainer-seed-out`, both seeds print once, here, and nowhere else. Give
either flag a path and that seed is written there instead (mode `0600`, refusing to overwrite) and
**never printed** — see below.

**The trust decision is always shown, never performed silently.** `trusted maintainer key: maintainer`
is the same line `prikk trust maintainer add` itself prints — registering a maintainer key is a trust
act, and composing the steps removes the *typing*, never the *seeing*.

## The commands `setup` composes — and when you'd use them directly

`setup` is not the only way in. Each step is a first-class, documented command in its own right, and
understanding them is what lets you reason about what `setup` actually did.

### `prikk key generate` — a fresh seed

```sh
prikk key generate
```

```
seed: ...
note: this seed is now in your terminal scrollback -- treat it as a secret
public key: ...

next steps:
  prikk trust maintainer add --key-id maintainer --public-key ...
  export PRIKK_MAINTAINER_KEY_ID="maintainer"
  export PRIKK_MAINTAINER_SEED="..."
note: the same seed works as an AUTHOR key instead -- export PRIKK_AUTHOR_KEY_ID/PRIKK_AUTHOR_SEED and skip the trust step
```

**`--out <path>` writes the seed instead of printing it — and then it is never printed at all**, only
the public key and the next steps are:

```sh
prikk key generate --out ./maintainer.seed
```

```
wrote seed to ./maintainer.seed (mode 0600)
public key: ...
...
```

`--out` refuses to overwrite an existing file, and refuses any path with a `.prikk` component — prikk
never invents a secret's location and never manages its lifecycle (writing it once, where you asked,
is the entire commitment). **On Windows, `--out` currently refuses outright**: Unix file permissions
(mode `0600`) have no portable equivalent here without unsafe code or a new dependency, and writing a
secret at whatever permissions the filesystem happens to inherit, silently, is not acceptable. Print
and place the seed yourself instead.

### `prikk key public --seed-env` — derive a public key you already have

If you already hold a seed — from `key generate --out`, from `setup`, or from anywhere else — derive
its public key without regenerating anything:

```sh
export MY_SEED="$(cat ./maintainer.seed)"
prikk key public --seed-env MY_SEED
```

```
public key: ...
```

**The seed is read from the named environment variable, never from an argument.** `--seed-env` takes
the variable's *name* — `MY_SEED`, not the seed itself — because a `--seed <hex>` flag would put key
material into `/proc/<pid>/cmdline` (world-readable on Linux) and into shell history. The name is not
a secret; the value never appears on the command line at all.

### `prikk trust maintainer add` — the trust act itself

```sh
prikk trust maintainer add --key-id maintainer --public-key <the hex key generate printed>
```

Registering a maintainer key is what lets `prikk seal` publish — see
[Security and Signing Setup](security-setup.md) for the full trust model, revocation, and what is and
is not enforced today.

## Which key is which

Two independent roles, two independent seeds:

- **AUTHOR** signs the Patch a `commit` queues. No trust registration needed for it at all.
- **MAINTAINER** signs the Block, RefState, and RefUpdate a `seal` publishes, and must be registered
  with `trust maintainer add` first.

`setup` generates one of each, but a single seed works as either role — `key generate` prints the
maintainer framing because that is the one role requiring a visible trust step, but the same seed
exported as `PRIKK_AUTHOR_KEY_ID`/`PRIKK_AUTHOR_SEED` works too, with no trust step at all.

## Claim-to-Source Anchors

| Claim | Source anchors |
|---|---|
| `prikk key generate`/`prikk key public`/`prikk setup` exist and compose `init`, key generation, and `trust maintainer add`. | [`key.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-cli/src/key.rs), [`setup.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-cli/src/setup.rs), [`commands.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-cli/src/commands.rs) |
| A generated seed draws from the OS CSPRNG and is never accepted on argv; `key public` reads it from a named environment variable. | [`prikk-crypto/src/lib.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-crypto/src/lib.rs) (`generate_seed`), [`key.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-cli/src/key.rs) |
| `--out` writes the seed at mode `0600`, refuses to overwrite, and refuses a path inside `.prikk/`; it refuses outright on Windows. | [`key.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-cli/src/key.rs) (`write_seed_to_path`) |
| `setup` shows the trust decision it makes, and prints nothing that reproduces without your own OS entropy. | [`setup.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-cli/src/setup.rs) |

## Provenance

Written for [RFC 135](https://github.com/prikk-vcs/prikk/blob/main/rfcs/done/135-first-run-entrance-and-configuration.md)
§9, which measured the unfamiliar-step count to a first sealed commit at eleven, with the third step
(deriving a maintainer public key) impossible before this page's own commands existed. See
[Git → prikk](../reference/git-mapping.md) for how prikk's commands relate to Git's, including
`git config`'s row, which is where this page is cross-linked from.
