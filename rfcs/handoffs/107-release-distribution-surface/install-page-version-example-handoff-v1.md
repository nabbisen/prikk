# Install page — make the version example stop going stale

**Base:** current `main` (`b4fbe66`, `0.26.0` released). **Under `003-landing-work-on-main.md`.**
**Origin:** you stopped on this during the `0.26.0` cut rather than guessing whether it was in scope.
**That was right, and this is the ruling.**

---

## 1. The ruling: do not bump it

`docs/src/guide/install.md:59-62`:

```
prints the installed version, for example:

```
prikk 0.25.0
```
```

**Changing `0.25.0` to `0.26.0` is the wrong fix.** It re-arms the identical staleness at every future
release, adds one more hand-maintained copy of a version number, and **no gate can catch it** — which
is why it survived the cut that moved every other version site.

**This is not a currency claim like `README.md:45`'s "Latest released implementation."** It illustrates
the output's **shape**. So the fix is to stop pinning a number at all.

## 2. What to replace it with

**Adjudicate the exact form**, but the requirement is: **a reader must be able to tell whether their
install worked, without the page naming a version.**

**My lean** — give them the *criterion* rather than a fixed string: the command prints the version they
installed, and **it should match the release they downloaded**. That is more useful than a sample line,
because matching-what-you-downloaded is the actual check, and a stale sample actively misleads someone
who downloaded something newer.

**If you keep a sample block, it must not contain a real version number.** A placeholder is acceptable
only if it cannot be mistaken for literal output.

**Do not delete the surrounding guidance.** The next line — *"If the shell reports 'command not found'
instead, the binary is not on `PATH` yet"* — is the most useful sentence on the page and must survive.

## 3. Sweep for the same pattern, report only

**This will not be the only pinned version in the docs.** `grep -rn "0\.2[0-9]\.[0-9]" docs/src/` and
**report what you find**, split into:

- **currency claims** — sentences asserting what the current release is, which must track;
- **illustrative examples** — like this one, which should not pin a version;
- **historical statements** — *"Windows became a mutating platform in 0.21.0"* — which are correct
  precisely because they name a past version and **must not be touched**.

**Fix only `install.md`. Report the rest.** The third category is the one most likely to be damaged by
an over-eager sweep, which is why I want the classification before any further edit.

## 4. Out of scope

- **`README.md:45`**, which is a currency claim and correctly tracks the release.
- **Every other page**, pending §3's classification.
- **Any code change.**

## 5. Controls

1. **No version number remains in the example** — show it mechanically.
2. **The "command not found" guidance survives** — quote it from the final text.
3. **`mdbook build` clean**, page still renders.
4. **Full gate set green, test count unmoved** — documentation only.

## 6. What to report

1. The block, before and after.
2. **Your §3 classification**, all three buckets.
3. All four controls (§5), quoted.
4. **Full gate set against the exact commit, after the last edit**, including `mdbook build`.
5. **Every numbered requirement's disposition, including ones that went without incident.**
6. Anything here was wrong.

**Stop and escalate, do not guess**, if: a version in `docs/src/` is genuinely ambiguous between
"currency claim" and "historical statement" — **that distinction is mine to make, not yours to
resolve.**
