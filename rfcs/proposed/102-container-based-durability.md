# RFC (proposed) - 102 Container-Based Durability

**Status.** **PROPOSED 2026-08-12.** Successor to RFC 101, which closed with a negative result the same
day. **Acceptance would clear §6's prerequisites only** — no design, no implementation, no production
code — and a stop-and-report on any of them ends this RFC as it ended 101.
**Independence.** Author-reviewed — the standing ceiling.
**Arises from.** RFC 101's §5.2 transition trace, which established that the obstacle is prikk's storage
model rather than its ref publication; and the owner's direction of 2026-08-12 that Windows read-only
is not an acceptable permanent state.
**Target.** Owner's call. **1.0-scale**, not 0.20.0 — see §9.

## 1. The problem, stated correctly this time

**RFC 101 stated this problem wrongly and that is why it failed.** It framed the obstacle as DC-38's
step 5 versus step 6 — a symptom — and proposed a fix that reached only ref publication. An
independently derived transition trace then showed that fix would have made things *worse*, by creating
a durable ref pointing at a non-durable object.

The correct statement:

> **prikk is content-addressed, so the filename *is* the content hash. Every object write therefore
> creates a name that did not previously exist, and Windows offers no primitive that makes a new name
> durable.**

Creating a file writes two independent things: the file's **contents**, and the **directory entry**
naming it. Windows has `FlushFileBuffers` for the first and nothing for the second. So a power loss can
leave an object whose bytes are intact on disk and whose name is gone.

This is not a ref-publication property, an `fsync` error-handling question, or a network-sync question.
It is a property of one-file-per-object storage.

## 2. What RFC 101 established, so this does not re-derive it

1. **No Windows primitive provides new-name durability** — documented, undocumented, or
   reverse-engineered. DC-87 Stage 2 checked the Win32 surface; RFC 101 §5.5 added Transactional NTFS
   and the NTFS `$LogFile`.
2. **Transactional NTFS did provide it and is being withdrawn**, with Microsoft warning it *"may not be
   available in future versions."* Ruled unusable: if TxF is removed, a repository written under
   TxF-backed durability is indistinguishable from one that always had the guarantee — silent loss of a
   guarantee, which is the failure prikk exists to prevent.
3. **The complete new-name surface is mapped** — RFC 101 §5.2's fifteen transitions and 31-site call
   index, derived independently. **That table is this RFC's primary input**, and its retention was
   ordered regardless of 101's fate.
4. **No per-ref file shape avoids first-appearance at ref creation** (DC-91 §3).

## 3. The direction

**Move durability-bearing repository state into a bounded set of fixed-name container files.**

Appending to or updating a file that already has a name requires only content durability, which Windows
provides. If no new directory entry sits on the durability path, the gap closes — with **no
vendor-specific primitive, no deprecated API, and no weakened invariant.**

It is uniform across Linux, macOS and Windows, so it **satisfies** the one-mechanism constraint rather
than straining it. Packed object storage is well-trodden; this is not a novel storage idea.

**This is a hypothesis, not a design.** RFC 101's hypothesis was equally plausible and died on contact
with §5.2. §6 exists to find out whether this one survives the same treatment.

## 4. The worktree, which cannot be containerized

Worktree files are the user's real files. Materializing them creates new names, always, and no container
format changes that.

**But the danger is not the lost file — it is the inference drawn from its absence.** Per RFC 101's T12,
the commit-authoring path treats any baseline path missing from the worktree as a user deletion, so a
file whose name failed to become durable is re-authored and **signed** as a deletion the user never made.

**Candidate remedy, to be evaluated in §6.5 rather than assumed:** a fixed-name unclean-shutdown marker
— itself a content update, therefore durable — after which prikk refuses to infer deletion until the
worktree has been re-verified against its baseline. That converts silent signed data loss into a
detected condition requiring explicit user action.

**Note the asymmetry that makes this tractable:** the worktree is rebuildable from sealed history; the
repository is not. Losing a worktree file is recoverable. Signing a false deletion is not.

## 5. Non-negotiable constraints

1. **One storage mechanism across all platforms.** A Windows-only container format is a worse outcome
   than not shipping Windows mutation.
2. **B′ adoption semantics unchanged** — a merge seals the other side's patches verbatim: same bytes,
   same `ObjectId`, same author signature. A container format must not disturb object identity.
3. **Object-trust and ref-authority stay separate** (DC-78 §D2).
4. **No conversion of format-2's *rejection* of the ahead-log state into *recovery*.**
5. **Recoverability does not regress below today's audited ceiling** — DC-41 Stage 1's 24/24 reachable
   states — and the audit is re-earned rather than assumed.
6. **A format migration must exist** for repositories already written in the current format, and it is
   in scope for the design even though it is out of scope for §6.

## 6. Blocking prerequisites

1. **What have comparable systems done, and what did it cost them?** Packed and container storage in
   content-addressed and version-control systems — what is packed, what stays loose, how durability is
   claimed, and specifically **how each behaves on Windows**. If the field universally accepts weaker
   Windows durability, that is a finding the owner needs before authorising a storage redesign.
2. **Re-derive §5.2's fifteen transitions against a container model.** For each: does it leave the
   durability path, and what remains? **Derive independently** — do not assume the table transfers.
   A transition that cannot leave the path is a stop-and-report.
3. **What is the bounded fixed-name set**, and what creates each name? Any file created outside `init`
   reintroduces the problem it was meant to remove.
4. **Read-path and concurrency consequences.** Read-only commands take no lock and run concurrently with
   mutation today. Does a container preserve that, and at what cost to lookup?
5. **The worktree question of §4** — is the unclean-shutdown marker sound, and does it close T12 without
   requiring new-name durability? Answer from the commit-authoring code, not from the sketch above.
6. **Cost.** The proof surface to be re-earned, the migration, and what a container does to DC-41's
   failpoint matrix.

## 7. Acceptance criteria

1. **Parity stated as a property of the design**, not as a platform list that happens to pass.
2. **A negative control** per DC-95's standing method: disable the durability-bearing step, demonstrate
   the specific failure `verify` reports, restore, confirm no residual diff.
3. **Green three-platform CI**, macOS included.
4. **DC-41-grade recoverability audit re-earned** at the new design's own state count.
5. **The migration demonstrated on a repository written in the current format**, not only on fresh ones.

## 8. Non-goals

- **G1 path anchoring on Windows.** No Win32 component-by-component no-follow walk exists; out of scope,
  as it was for 101.
- **Windows read-only support**, which works today and is CI-gated.
- **Performance work.** If a container improves object-write throughput that is welcome, not a
  justification, and not a criterion.
- **Changing what a ref log is** — DC-38's append-only audit trail.

## 9. The cost, and the staging consequence

This is a storage-format change: object layout, `verify` and `doctor` state derivation, the durability
contract's platform layer, DC-41's recoverability audit, and a migration for existing repositories.
**It is 1.0-scale work and should not be forced into 0.20.0.**

**What it changes immediately, before any of it is built:** Windows read-only stops being a verdict and
becomes a staging decision — *not yet*, rather than *never*. That is a different thing to ship and a
different thing to document, and it is the honest position while this RFC runs.
