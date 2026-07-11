# DC-24 FDD Documentation Update - Data Model and Trust/Threat Model References

Status: Companion for done DC-24
Related RFC: `../../done/DC-24-DATA-MODEL-TRUST-THREAT-DOCS.md`
Target FDDs: FDD-00 Data Model, FDD-04 Trust/Threat Model
Related recap: `baseline-recap.md`

## Purpose

DC-24 creates reviewed, current-state reference documentation for Prikk's data model and trust/threat
model. It exists because the published mdBook is operator-focused while the architecture and security
model are currently fragmented across historical RFCs and per-DC handoff updates.

This is a documentation increment only. It must not change repository behavior, object schemas,
signing logic, trust policy, verification logic, CLI behavior, or release semantics.

## Required FDD / Reference Additions

Create:

```text
rfcs/fdds/FDD-00-DATA-MODEL.md
rfcs/fdds/FDD-04-TRUST-THREAT-MODEL.md
```

Both files must explain their numbering and scope. `FDD-00` is a current-state consolidation reference
for data-model material that was previously split across storage, identity, schema, and lifecycle
records. `FDD-04` preserves continuity with the existing threat-model trace. The files must also state
that FDD-01/FDD-02/FDD-03/FDD-05 are not created by this increment and remain unconsolidated or
deferred unless later RFCs create them.

### FDD-00 Data Model

The data-model reference should define the current released model for:

- object envelopes and object identity;
- signed payloads and role-separated signatures;
- Patch, operation, operation sequence, node id, and Patch purpose;
- Block identity and sealed-history lifecycle;
- RefState and RefUpdate publication evidence;
- active WAL and active-session boundaries;
- conceptual repository layout;
- replay/materialization/verify/doctor lifecycle boundaries;
- relationship to archived DC-09 and later released DCs.

It should not resurrect archived DC-09 as live authority without noting which parts are superseded or
implemented differently. It must also state that durability and recovery claims are supported by the
current test evidence, not by a completed crash-matrix or fuzzing campaign, and that Linux is the only
platform currently exercised by project gates.

### FDD-04 Trust/Threat Model

The trust/threat-model reference should define the current released model for:

- AUTHOR signatures;
- MAINTAINER signatures;
- real role-bound Ed25519 MAINTAINER seal signing verified against the local trust store;
- repository-local maintainer trust store;
- explicit local trust setup;
- AUTHOR key input through `PRIKK_AUTHOR_KEY_ID` and `PRIKK_AUTHOR_SEED`, without local secret storage;
- publication-time trust binding;
- repository-wide verification boundaries;
- rollback-draft verification boundaries;
- privacy/redaction expectations for public diagnostics;
- deferred key lifecycle features.

It must state the current limitations plainly:

- no repository-wide AUTHOR trust-store enforcement;
- no key rotation, revocation, expiration, or hardware-signing support;
- no multi-maintainer threshold beyond the current minimal policy;
- no remote trust distribution;
- no sync/hosted-forge trust model;
- no stable repository-format migration guarantee;
- no crash-matrix/fuzz proof for durability and recovery claims yet;
- no fully verified cross-platform filesystem behavior yet.

Both FDD files must include a claim-to-source anchor table that ties each security-sensitive or
data-model claim to current code paths or released RFC/FDD records.

## mdBook Additions

Add:

```text
docs/src/reference/data-model.md
docs/src/reference/trust-threat-model.md
```

The mdBook pages should orient readers and link to the FDD references. They should not copy large FDD
sections. They must include the core safety caveats inline: early implementation software, not a
production Git replacement, repository-local maintainer trust with current minimal `required = 1`
policy, no key rotation/revocation, and `verify` not being a global trust proof.

`docs/src/SUMMARY.md` should add:

```md
# Reference

- [Data Model](reference/data-model.md)
- [Trust and Threat Model](reference/trust-threat-model.md)
```

## Source Audit Checklist

Implementation review should require a checklist confirming the writer audited:

- `baseline-recap.md`, which recaps the untracked requirements, NFR, external-design, and v0.2.0
  handoff inputs in tracked form;
- archived node/data-model material;
- DC-10 and DC-11 signing/trust records;
- DC-14 and DC-15 integrity/trust-boundary records;
- DC-16 through DC-23 patch-algebra, replay, and merge-evidence records;
- FDD handoffs for trust and rollback security notes;
- `crates/prikk-crypto`;
- AUTHOR key-input code and CLI/environment handling for `PRIKK_AUTHOR_KEY_ID` and
  `PRIKK_AUTHOR_SEED`;
- current code for signing, trust-store parsing, seal, verify, doctor, and rollback-draft verification.

Because `.git-exclude/specs/` is scratch space, implementation must not rely on local-only
requirements, NFR, or external-design claims that are absent from `baseline-recap.md` or another
tracked source. The implemented FDD/reference docs must carry the durable current claim or link to a
tracked durable source.

## Review Expectations

Reviewers should reject the implementation if:

- mdBook and FDD references disagree;
- the docs imply production-grade trust;
- `verify` is described as stronger than the implementation supports;
- AUTHOR signatures are described as repository-wide trusted identities;
- deferred key lifecycle or remote trust features are described as implemented;
- RFC/FDD content is copied into multiple diverging pages instead of linked and summarized;
- the mdBook caveats drift from the FDD caveats;
- a security-sensitive or data-model claim lacks a claim-to-source anchor.

## Verification

Required local documentation gates:

```text
mdbook build docs
git diff --check
```

The review package should include:

- a source-audit checklist;
- claim-to-source anchor tables in each new FDD reference;
- evidence that mdBook inline caveats match the FDD caveats without drift;
- changed-file summary;
- mdBook build output;
- orphan/reachability evidence for new mdBook pages;
- references to the reviewed design record.
