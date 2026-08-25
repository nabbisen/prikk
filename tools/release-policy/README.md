# Prikk Release Policy Tool

This unpublished workspace tool is the authoritative implementation of Prikk's repository
release-policy checks. RFC 119 track B removed the Python implementation it once migrated from,
along with `differential-check`, the tool that compared the two.

The stale-reference gate recognizes exactly one immutable primary authority descriptor:
`tools/release-policy/Cargo.toml` with `cargo run --locked -p prikk-release-policy -- check`.

Each required live documentation path must register that command exactly once; a different
command, missing or non-regular anchors, and unregistered invocations fail closed.

The workflow scanner intentionally preserves the frozen dependency set during stage 2. It normalizes
quoted and whitespace-separated block and flow `run` keys through one structural path and fails closed
when a recognized `run` value cannot be parsed. Replacing this bounded extractor with a YAML dependency
requires a separate architect-reviewed dependency and `Cargo.lock` re-freeze.

Run the policy evaluator from the repository root:

```console
cargo run --locked -p prikk-release-policy -- check
```

Implementation and review gates may also run:

- `oracle-check --format json --self-test`
- `boundary-check --format json`
- `reference-check --format json`

These commands evaluate committed fixtures and repository metadata. They do not publish packages,
create releases, modify signer authority, or authorize a release-policy cutover.

Publication authority is limited to literal `cargo package` and `cargo publish` argument vectors in
`release/publication-command-inventory-v1.json`. Governed shell and workflow files fail closed on
any dynamic command head, dynamic Cargo subcommands, malformed command text, and Cargo-less commands
that otherwise match the Rust policy invocation shape. Command-head recognition accepts scalar
`run:`, sequence-item `- run:`, and equivalent flow-mapping YAML positions; shell assignments; the
documented `env` option grammar with explicit option arity; and `command --`/`command -p`.
Unsupported or incomplete wrapper prefixes fail closed. The scanner does not interpret shell
indirection as release authority. After prefix parsing, an unrecognized literal head with any dynamic
argument also fails closed; only an explicit inert set (`echo`, `printf`, `test`, `true`, `false`, and
the workflow `url` metadata key) may carry dynamic values without becoming executable authority.
Governed procedures also reject executable backtick substitution and opaque shell `-c` option
clusters or `eval` command strings instead of recursively interpreting them.

Workflow YAML is structurally reduced to `run:` scalar or block scripts before command analysis;
metadata expressions are not treated as shell. Governed `run:` scripts and `.sh` files use a
default-closed head model: only the authoritative Python/Rust policy commands, inventory-classified
Cargo publication commands, exact repository CI commands, `mdbook build`, or the inert set above are
accepted. Every other command head fails closed, including wrappers and inline-code interpreters.

The exact CI argv set is a maintenance contract: a legitimate workflow command change must update the
procedure allowlist and its review evidence in the same increment. The YAML extractor accepts arbitrary
whitespace after a sequence dash and locates `run` at any flow-mapping key position; malformed or
unsupported recognized workflow command forms fail closed.
