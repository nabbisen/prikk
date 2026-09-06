# Prikk Measurement Corpus

Repository-internal tooling for RFC 139 (the measurement corpus). Not part of the shipped product;
not built by default (`tools/corpus` is a workspace member but not a `default-members` one).

A **profile** is a small, human-readable TOML document describing a real history's *shape* — never
its content, never its paths (RFC 139 §4). Increment 2 will add a deterministic **builder** that
reads a profile and materializes a throwaway prikk repository from it, driving the same `prikk` CLI
a user would. This crate currently holds the profile format and the **extractor** that derives one
from already-captured `git log`/`git ls-tree` text; it never spawns `git` itself.

## Re-deriving `profiles/prikk-self.toml`

The profile's own `provenance.extraction_commands` names the exact commands, verbatim. From the
repository root, at the revision named in `provenance.revision`:

```console
git log --pretty=format:'@@%H' --name-status --no-merges -n 600 > /tmp/prikk-self-log.txt
git ls-tree -r -l <revision> > /tmp/prikk-self-ls-tree.txt
```

Then run the extractor against those two files and the committed context recipe:

```console
cargo run --locked -p prikk-corpus --bin extract-profile -- \
  /tmp/prikk-self-log.txt /tmp/prikk-self-ls-tree.txt \
  tools/corpus/profiles/prikk-self.context.toml \
  --out /tmp/prikk-self.toml
diff /tmp/prikk-self.toml tools/corpus/profiles/prikk-self.toml
```

An identical `diff` confirms the committed profile matches what the recorded commands actually
produce today.
