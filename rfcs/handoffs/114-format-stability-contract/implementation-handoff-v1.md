# RFC 114 — implementation handoff v1

**RFC:** `rfcs/accepted/114-format-stability-contract.md` (ACCEPTED 2026-08-19, **all five §5 decisions
resolved the same day**). Read §2, §3, §4 and §5 before starting; this handoff does not restate them.

**Answers badge criterion 2.** The increment is small because the owner ruled that prikk has never been
in production, so **formats 1-5 are unsupported and only format 6 onward matters**. The contract was
always a forward obligation.

## 0. An architect correction to §5.6, before you plan against it

§5.6 says the historical migration gate "has **one** case to cover, not five." **That is wrong, and in
the direction that would waste your time.** With formats 1-5 unsupported there is **no historical
migration to test at all** — format 6 is the only supported format, so there is nothing to migrate
*from*.

**What is testable today is the refusal, not a migration.** See §4 below. Plan against this section, not
§5.6's sentence.

## 1. Publish the contract

Into `docs/src/reference/release-compatibility.md` (§5.1), stating §3's promise in the user's terms:

> Any prikk release can read every object any prior **supported** release wrote, and verifies it to the
> same conclusion. Storage may require a migration step, which is documented and tested. Object identity
> and signatures never require one.

**Include the two lists from §2 explicitly** — what is frozen (object-id preimage, per-schema canonical
encoding, signature preimage, algorithm identifiers) and what may change (repository format, containers,
index, WAL, bundle). **A contract that states only the promise and not its surface is unusable**: the
next person changing a container needs to know from the page whether they are inside or outside it.

**State §5.5's rule** — a broken algorithm gets a new identifier; an existing identifier is never
redefined — and **§5.4's** — the obligation covers shipped releases, not every commit.

## 2. Correct `layout.rs`'s five retired-format messages

They currently *offer* a migration path. Per §5.3 they must state that the format is **no longer
supported**, with no instruction the product cannot honour.

**Three of the five are unfulfillable today**: they name "a prikk version that supports format 3/4/5",
and no such release exists — all four bumps happened inside the 0.19.0→0.20.0 window and 0.20.0 shipped
carrying format 6. **Check that claim yourself before relying on it**; if you find a released version
carrying format 3, 4 or 5, that is a finding and this instruction changes.

## 3. Gate A — frozen identity vectors

**First, enumerate and report** the `(object_type, schema_version)` pairs a **format-6** repository can
hold. `file_codec/tests.rs:110-124` carries a mapping — `Block` at 2, `RefState` at 1 and
`REF_STATE_CLOSED_SCHEMA`, `Patch`/`RefUpdate`/`Tag`/`Attestation`/`Blob` at 1, three types rejected
outright. **That is a test's view of the rule, not necessarily the contract's** — confirm it against the
production admission path and report any disagreement rather than inheriting it.

Then, for each pair, a committed literal vector: canonical payload bytes, the expected object id, and the
expected signature preimage where the type is signed. **DC-40's precedent, generalized.** Any change to
§2's frozen list must break a vector.

**Do not generate the expected values at test time from the code under test** — that asserts the code
agrees with itself. The values are literals.

## 4. Gate B — the refusal, and the tripwire that matters

**Today's testable half:** a repository at each retired format is refused, and the message offers no path
the product cannot honour. Cheap, and it pins §2 of this handoff so the messages cannot silently drift
back.

**The half with teeth, and the reason criterion 2 exists:** nothing currently prevents a future format
bump from shipping without a migration path — that is exactly how the `PBNDL002` defect reached `main`.

**Design a tripwire that fails when `CURRENT_FORMAT_VERSION` changes without corresponding migration
coverage.** The shape is yours to propose; the property is that **bumping the format must be unable to
pass CI on its own.** This is the same discipline as RFC 111's cost gates — the gate lands before the
thing it guards, and it must be observed failing.

**Report the shape before building it.** A tripwire that is easy to satisfy by editing the tripwire is
not one, and that is the failure mode to design against.

## 5. Report, do not build: the carry-forward operation

§5.2 requires that when format 7 arrives there is a supported, tested way to carry a format-6 repository
across, existing **before** the change ships. **That operation does not exist**, and the existing
exchange primitive is not it:

- `export_bundle(layout, ref_name)` exports **one ref**.
- `import_bundle` lands history at **`remotes/<origin_ref_name>`** — the received namespace, so a
  "migrated" repository has its history at `remotes/heads/main` and every other branch and tag is lost.

**Do not design or build it in this increment.** Report what a repository-complete carry-forward would
need — whether it extends export/import or is its own operation — so the next increment starts from an
assessment rather than a blank page.

## 6. Constraints

- **No format change.** This increment states and gates the contract; it does not exercise it.
- **No identity-bearing byte change** — that is the thing being frozen.
- `forbid(unsafe_code)`, the workspace-dependency convention, and the full gate set per
  `EXECUTION-ORDER.md` §6 rule 9 all hold as usual.

## 7. Order

§3's enumeration first and reported, then §4's tripwire shape reported, then implementation. **Report
before implementing**, as always — and if §0's correction turns out to be wrong in your reading, say so
before building around it.
