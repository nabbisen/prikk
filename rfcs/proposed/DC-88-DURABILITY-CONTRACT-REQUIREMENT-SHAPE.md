# RFC (proposed) - DC-88 Durability Contract Requirement Shape

**Status.** **PROPOSED** — needs the project owner's acceptance. **Scope decision attached:** accepting
this blocks DC-87 Stage 2 until it lands. See §5.
**Independence.** Author-reviewed — the standing ceiling.
**Arises from.** DC-87's narrow prerequisite round, 2026-08-10, and the architect's own error in setting
its blocking question. **Fires DC-87 §4/§6's stop-and-report trigger** on `DurabilityContract`'s method
set.
**Target.** Owner's call — 0.20.0 before DC-87 Stage 2, or later with Stage 2 shipping a documented
weaker Windows invariant. §5 states the trade.

## 1. The question

**Does `durable_directory_entry` state a requirement, or a primitive?**

DC-76 built `DurabilityContract` on one thesis, in its own words: *"Guarantee, not syscall — the whole
point."* A method named after a primitive *"would already be platform-specific before a second platform
exists."* The contract's worked example of getting this right is `atomic_replace` — "replace this file's
content atomically, durably," never "write a temp file and call `renameat`."

`durable_directory_entry` is the one method that does not meet its own bar. Its guarantee — *every
mutation made under `relative` since the last durability point survives a crash* — is a
**directory-scoped batching concept that exists because POSIX has directory fsync.** The module
documentation half-concedes this: "satisfied on Linux by `fsync` on the directory fd."

DC-87's investigation established that Windows has no supported user-mode way to provide it:
`FlushFileBuffers` does not apply to directories, `REPLACEFILE_WRITE_THROUGH` is documented "not
supported," `MOVEFILE_WRITE_THROUGH`'s guarantee is scoped to a cross-volume mechanism a same-volume
rename never uses, and the one native candidate is framed throughout its own documentation as
driver-only.

**That is a fact about Windows. It becomes a blocker only because of how the contract is shaped.** What
DC-38 step 5 requires is that *a ref's pointer transition be atomic and durable* — directory-entry
durability is one implementation of that, not the requirement itself.

## 2. Why this is not DC-87's to absorb

DC-87 §6 lists any change to `DurabilityContract`'s method set as a non-goal and instructs the increment
to stop and report if the port appears to require one. It does. Absorbing it would also repeat the
mistake DC-82 was created to avoid: a contract redesign and a new-platform backend have different
proofs, and bundled, a reviewer cannot tell which half a failure came from.

## 3. Candidate shape — to evaluate, not to inherit

Offered so §4 starts from a proposition rather than a blank page. **It is not a ruling.** The architect's
design assertions on platform work have needed correction repeatedly, including the one that produced
this RFC.

**Restate the method as the transition it protects.** If the requirement is "this state transition is
atomic and durable" rather than "this directory is synced," then POSIX satisfies it with
`rename` + directory `fsync` exactly as today — no Linux or macOS behaviour changes — and a platform
without directory fsync is free to satisfy it another way.

**An existence proof that a non-POSIX platform can:** a fixed-name record with two slots, each carrying
a sequence number and a checksum, always overwriting the stale slot and flushing **file content**, needs
no directory entry to change. Windows provides file-content durability unambiguously.

**It is incomplete as stated, deliberately.** It addresses *transitions*, not the first creation of a
pointer or log file, where a directory entry genuinely must appear and become durable. §4.3 is that gap.

## 4. Blocking prerequisites

1. **Enumerate every caller of `durable_directory_entry`** and state, for each, what it actually
   requires. Some may genuinely want directory-scoped batching; if so, say which and why — the answer
   "it is a real requirement in these N places" is a legitimate outcome and ends this RFC.
2. **Does restating the method leave Linux and macOS byte-for-byte unchanged?** If the POSIX
   implementation is not literally the code that exists today, report that before designing.
3. **What happens at first creation**, where a directory entry must appear? Report whether the
   ordering hazard DC-38 exists to prevent can arise there, and under what conditions.
4. **Does DC-38's state machine still hold** under the restated method, on POSIX, unchanged? DC-38 is
   the reason this matters; a change that quietly weakens it on Linux is worse than the problem.

## 5. The scope trade, for the owner

**Accepting this blocks DC-87 Stage 2 until it lands.** The alternative is to ship Windows mutation with
a documented weaker crash invariant — DC-38's "format-2 publication never permits an ahead log" would not
hold there — leaning on the bounded ahead-log recovery DC-38 already defines for the format-1
compatibility case. That is faster and it is not reckless; the recovery path exists.

**The architect recommends against it.** It would put a permanently weaker guarantee into the platform
where the product most needs to be trustworthy about exactly this, and turning format-2's *rejection* of
an ahead log into *recovery* is itself a security change needing its own analysis — so the "faster" path
is not as short as it looks. This is a contract question rather than an implementation slog, and is not
expected to be large.

**Whether 0.20.0 waits for it is the owner's decision, not the architect's.**

## 6. Non-goals

- **Any change to the nine guarantees.** This is about how one of them is *stated*, not what is
  guaranteed.
- **Any Linux or macOS behaviour change.** If the restatement cannot be done without one, stop and
  report.
- **Any Windows implementation.** That is DC-87 Stage 2, after this.
- **Any change to DC-38's state machine.** §4.4 checks it survives; it does not reopen it.
