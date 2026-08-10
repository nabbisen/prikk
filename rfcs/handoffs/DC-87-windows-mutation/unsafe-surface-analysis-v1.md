# DC-87 — The `unsafe` Surface: Analysis and the Owner's Ruling

**Recorded 2026-08-10.** Written as decision support for the question the architect escalated in the
DC-87 prerequisite ruling §4.5, and completed with the owner's ruling in §5.

## 1. A correction to how the architect framed the question

The escalation said prikk would "gain its first `unsafe` surface." **That framing was wrong and could
have distorted the decision.**

`prikk-store` already depends on `rustix`, which performs unsafe libc FFI internally on every Linux and
macOS build and exposes safe functions. **prikk already runs `unsafe` code today.**
`#![forbid(unsafe_code)]` is a property of the code prikk *writes*, not of the code it *runs*.

The real question was never whether unsafe enters the picture. It is **whose unsafe, and how much of it
can actually be audited.**

## 2. Why some `unsafe` is structurally unavoidable

Any Windows filesystem syscall is FFI, and FFI is `unsafe` in Rust by definition. No crate, flag, or
design choice removes it. It only decides who writes it.

## 3. What safe `std` does cover — more than DC-87's investigation implied

Checked against the standard library documentation rather than assumed:

- **`std::os::windows::fs::OpenOptionsExt`** — every method is **stable since Rust 1.10 and none is
  `unsafe`**. That yields, in safe stable Rust: `share_mode` (so `FILE_SHARE_DELETE`, which §3.4
  identified as a codebase-wide discipline), `custom_flags` (`FILE_FLAG_BACKUP_SEMANTICS` to open
  directory handles, `FILE_FLAG_OPEN_REPARSE_POINT` to refuse following a reparse point at the final
  component), `access_mode`, and `attributes`.
- `File::sync_all` is the `FlushFileBuffers` path; `create_new` is exclusive creation.

So a substantial part of what §3.1 and §3.4 need is reachable without any FFI at all. The
investigation's "std does not suffice" was right as a conclusion but understated how much std does cover.

## 4. The one gap, and a workaround that does not work

**`std` cannot open relative to a directory handle.** There is no `openat` equivalent, and that is
precisely G1's requirement.

The natural workaround is to open each component by full path and then verify that the object opened is
the one walked to, by comparing handle identity. **That is not available on stable Rust.**
`std::os::windows::fs::MetadataExt::file_index` and `volume_serial_number` are **unstable**, behind the
`windows_by_handle` feature (rust-lang#63010).

The architect went looking to propose exactly that approach and checked before proposing it. It fails.
Recorded because a discarded option is worth as much to a later reader as a chosen one.

## 5. The owner's ruling, 2026-08-10

> "`unsafe` is allowed under control with safety and maintainability preserved."

**Accepted and binding.** The escalation is closed. What "under control" means concretely is an
architecture question, which is the architect's — proposed as **DC-90**, so the constraint is written
down and machine-checked rather than left as an intention.

## 6. Why this ruling is defensible on the project's own terms

The instinct that preserving `forbid` is automatically safer does not survive inspection, and the ruling
correctly does not follow it:

| | Third-party crate (`cap-std`) | prikk's own minimal crate |
|---|---|---|
| `forbid` in prikk's own code | Intact everywhere | One documented, gated exception |
| Unsafe actually relied on | A whole crate tree's, unreviewed | A few FFI declarations, readable in one sitting |
| New third-party surface | 13 transitive packages on Windows, including non-optional `ipnet` | None |
| If it is wrong | Someone else's memory-safety bug | prikk's own |

The lint-preserving route keeps the *badge* while making the unsafe surface **larger and less
auditable** — just not prikk's. For a project whose stated position is security over function, "smaller
and fully auditable" is the stronger reading, and it is the one the ruling permits.

**This is a permission, not a preference.** Nothing here says the bespoke crate is the right answer —
only that it is now allowed to be. §8 still stands: price both before choosing.

## 7. On Verus

The owner raised formal verification (Verus) as possibly available. Worth taking seriously, and worth
being precise about where it would and would not help.

**Where it would not help — the risk this ruling actually creates.** The hazard in a bespoke FFI crate
is whether an `extern "system"` declaration matches the real Win32 ABI, and whether pointer, lifetime,
and buffer invariants hold across the boundary. **No Rust-level verifier can prove properties of code
that is not Rust.** A verifier confronted with a foreign function has one option: take its specification
as an assumption. The FFI boundary therefore lands in the trusted computing base by construction —
exactly where the risk is. This is a structural argument about verification tools, not a claim about
Verus's feature list; **the architect has not confirmed Verus's specific mechanisms against primary
sources** and would treat that as its own investigation before any adoption.

**Where it could genuinely help — and this is the more interesting possibility.** If DC-88 lands a
two-slot durable record, its correctness is a *crash-consistency state machine*: sequence numbers,
checksums, which slot is authoritative after an arbitrary interruption. That is pure logic over safe
Rust, it is exactly the shape SMT-based verification handles well, and it is the kind of property that
tests sample rather than prove. Crash-consistency bugs are also precisely the ones that survive a test
suite and surface years later.

**Recommendation: do not couple Verus to this ruling.** It does not mitigate the FFI risk, so making it
a condition of allowing unsafe would buy assurance in the wrong place. Keep it as a candidate for
DC-88's state machine, on its own merits, as its own proposal. If the owner wants it explored, that is a
separate increment and the architect would want it investigated before committing — Verus is a
substantial adoption, not a flag.

## 8. What still has to happen before anyone writes FFI

Unchanged by the ruling, and now the operative constraint:

1. **DC-88 reports first.** It may reduce what Windows needs, or remove the requirement.
2. **Price both options against numbers, not principle** — actual FFI line count for a bespoke crate,
   actual dependency delta for `cap-std`. Deciding in the abstract is how a project ends up with the
   larger unaudited surface while feeling safer.
3. **DC-90's policy and gate land before the first `unsafe` line**, not after. A boundary added
   retroactively documents what happened; one added first constrains it.
