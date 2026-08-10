# DC-87 — Narrow Round Ruling v1

**Reviewing:** `.git-exclude/review-request/prikk-dc-87-narrow-round-v1.md`.

**Both items accepted.** Item 2's shape is accepted with two additions. **Item 1 is accepted as
unresolved, and I am withdrawing the question — setting it as the blocker was my error, and the
consequence is a new design increment (DC-88) rather than more searching.**

## 1. `MOVEFILE_WRITE_THROUGH` — accepted unresolved, and the question retired

Their reading is sound and I am not asking for more work on it. The flag's guarantee sentence is scoped
to a *mechanism* — "a move performed as a copy and delete operation" — that a same-volume rename never
goes through, and the page never separately states the same-volume case. Three independent corroboration
attempts (the MSDN archive thread, the OSR driver community, PostgreSQL's own `durable_rename`
engineering) is more diligence than I asked for. Reporting "I could not determine this" was correct.

**But I set the wrong question, and I should own that rather than let it stand.**

DC-76's thesis, in the contract's own words: *"Guarantee, not syscall — the whole point."* A method named
after a primitive "would already be platform-specific before a second platform exists."
**`durable_directory_entry` is the one method in `DurabilityContract` that is named after its primitive
rather than its requirement**, and the module documentation half-concedes it — "satisfied on Linux by
`fsync` on the directory fd." The guarantee it states, *every mutation under this directory since the
last durability point survives a crash*, is a **directory-scoped batching concept that only exists
because POSIX has directory fsync.**

What DC-38 step 5 actually requires is not "this directory's entry list is durable." It is **"this ref's
pointer transition is atomic and durable."** Directory-entry durability is one implementation of that
requirement, native to POSIX. It is not the requirement.

**An existence proof, offered to show the impasse is not real — not as a design.** A fixed-name pointer
record with two slots, each carrying a sequence number and a checksum, always overwriting the stale slot
and flushing **file content**, needs no directory entry to change and no directory to be synced. Windows
provides file-content durability unambiguously (`FlushFileBuffers`). The classic double-buffered
superblock, and prikk already content-addresses and checksums everything else it writes.

I am not ruling that prikk should do this, and it is incomplete as stated: it addresses *transitions*,
not the first creation of the pointer and log files, where a directory entry genuinely must appear.
DC-88 has to handle both. The point is narrower and it is enough: **"Windows cannot fsync a directory"
does not entail "Windows cannot publish a ref durably."** I blocked Stage 2 on a Windows API fact when
the load-bearing question was about prikk's own contract.

**Ruling.** This fires the RFC's own stop-and-report trigger — §4 and §6 both say a port that appears to
require a change to `DurabilityContract`'s method set stops and reports. It does. That decision is not
DC-87's to take inside an increment, so it goes to its own: **DC-88**, proposed alongside this ruling,
answering one question — does `durable_directory_entry` state a requirement or a primitive, and if the
latter, what replaces it?

**Stage 2 now blocks on DC-88 rather than on an unanswerable Windows API question. Stage 1 is
unaffected and continues.**

**The scope trade is the owner's, and it is real.** The alternative is to ship Windows now with a
documented weaker crash invariant, leaning on DC-38's existing bounded ahead-log recovery, and fix the
contract later. That is faster. I recommend against it: it would put a permanently weaker guarantee into
the platform that most needs the product to be trustworthy about it, and the owner's standing position
is that security is prioritized over function and that we should not be in a hurry especially on it.
DC-88 is a contract question, not an implementation slog — I do not expect it to be large. But whether
0.20.0 waits for it is the owner's call, not mine.

## 2. The mode-carrying shape — accepted, with two additions

I verified the consumer analysis rather than taking it. `matches_stat`
(`commit_index.rs:47-51`) does compare only `size`, `mtime_secs`, `mtime_nanos` — mode is genuinely not
part of the cache-trust condition, so `Option<u32>` is safe there. The `worktree.rs:137-144` reasoning
is right: `None == Some(_)` is `false`, the skip-optimization stops firing on Windows, and `entry.mode`
— not the stat — remains what gets written, so no on-disk behaviour changes. The four call sites are the
four call sites.

**Addition 1 — a documentation defect sitting exactly where this change lands.**
`commit_index.rs:4` says the cache lets commit skip reading content *"when its size, mtime, **and mode**
match what was last recorded for it."* The code does not check mode. They read the function's own doc
comment (lines 44-46), which is silent on mode, and reached the correct conclusion about behaviour — but
the module doc directly contradicts it.

**Fix the doc, not the code.** Excluding mode from a *content*-hash trust condition is correct: a
permission change does not change content. The hazard is the reverse direction — someone later reads the
module doc, concludes `matches_stat` has a bug, adds mode to it, and silently couples the cache's trust
condition to a value that is `None` on Windows. Correct the sentence in the same increment.

**Addition 2 — the `None` branch must be tested on Linux and macOS CI, not merely testable.** They noted
it can be exercised by constructing a `RootFileStat` with `mode: None` directly, the way
`node_authoring.rs:713` already builds synthetic ones. Make that a requirement. Otherwise the only code
path that protects sealed history from the executable-bit drop ships untested until a Windows mutation
job exists, which is the wrong order.

**Accepted as reported:**

- `RootFileStat.mode` and `WorktreeFileMeta.mode` becoming `Option<u32>`, with `normalize_file_mode`
  carrying the `Option` through. The sentinel `0` disappears, which was the ruling's requirement.
- The explicit branch at `node_authoring.rs:405`, and its defence against criterion 2. Declining to
  detect a signal the platform structurally cannot produce is not the same as claiming to have set
  something and not doing it. That distinction is correct and worth keeping in the code comment.
- New files defaulting to non-executable, documented in `platform-support.md` as a platform limitation.
  Correctly separated from the existing-file case: one is a missing capability, the other would have
  been silent corruption of something previously right.
- **No new command surface — and the reasoning improves on my phrasing.** I wrote "change only on an
  explicit operator action." They observed that on a platform with no automatic detection path, that is
  satisfied by the *absence* of a path rather than by adding one, and that adding a `chmod`-equivalent
  would collide with the RFC's own non-goal. Right on both counts. If such a command is wanted later it
  is its own increment, proposed on its own terms.

## 3. Standing

- **Stage 1: continues, unblocked.** Nothing in this round touches it.
- **Item 2's shape: cleared to implement as part of Stage 1**, including Addition 1's doc correction and
  Addition 2's test.
- **Stage 2: blocked on DC-88**, and on the owner's ruling on prikk's first `unsafe` surface, which is
  still open from the previous round.
- Green CI on all three platforms before either stage merges. Unchanged.
