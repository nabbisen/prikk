# RFC (proposed) - DC-36 Existing-Object Publication Integrity

**Status.** Proposed; architect design review required.
**Target milestone.** M1 - 0.18.0 corrective release.
**Tracks.** Architect review B4.
**Touches.** `FileObjectStore::write_object`, object-file decoding/encoding, corruption errors, and
focused publication tests.

## Problem

When an ObjectId destination already exists as a file, `write_object` currently returns success without
reading it. A malformed, wrong-type, or byte-different envelope can therefore be accepted as the
expected object during publication.

## Design

An existing final object path is an idempotence check, not a success shortcut. Before returning success,
the store must:

1. encode the incoming validated envelope using the normal persisted-file codec;
2. read the existing final-path bytes;
3. decode and structurally validate the existing envelope;
4. verify its computed ObjectId and object type against the path;
5. require exact persisted-byte equality with the incoming candidate.

Any read, decode, identity, type, or byte mismatch returns an integrity/corruption error. The method must
not overwrite, quarantine, delete, or repair the existing file. Concurrent creation must use the same
post-race comparison before success.

### No-clobber installation

An absence check followed by replacing `rename` is forbidden. New immutable objects use this sequence:

1. create a unique temporary regular file in the destination directory with `create_new` semantics;
2. write the complete candidate bytes and perform required file sync;
3. atomically install without replacement using a reviewed same-filesystem hard-link/no-replace
   primitive;
4. if installation reports `AlreadyExists`, read and compare the winner exactly before returning;
5. after successful install, required-sync the parent directory, remove the temporary link, and
   required-sync the parent again before reporting success.

On the equal-winner `AlreadyExists` path, exact comparison is followed by removal of the unique
candidate temp and required parent-directory sync before success. If removal or cleanup sync fails,
return cleanup-incomplete failure; do not report success with race-temp durability unresolved.

If no atomic no-clobber primitive is available, the mutation returns an explicit unsupported
filesystem/platform error. It must not fall back to replacing rename. A failed write or file sync
retains or removes only the unique temp that is not authoritative. A failed install or directory sync
returns failure and retains enough information for retry; an installed final path is never removed on
error. Retry validates the final winner before cleaning same-operation temp debris.

## Required tests

- exact existing bytes are accepted idempotently;
- malformed bytes, wrong object type, wrong computed id, and byte-different signature transport fail;
- a same-payload envelope with different signature bytes fails exact comparison;
- publication does not advance a ref after any existing-object mismatch;
- the create race resolves to exact comparison rather than blind overwrite or success;
- same-process threads and separate processes race different transport bytes for one ObjectId;
- unsupported hard-link/no-replace behavior fails explicitly and never replaces the final path;
- file-sync, install, first directory-sync, temp unlink, and cleanup-sync failures preserve the stated
  final/temp artifacts and return failure.

## Non-goals

- No object schema, ObjectId, signature, ref protocol, GC, quarantine, or repair change.
- No repository-wide object rescan on every write.

## Dependencies and acceptance

DC-36 can be implemented after its own design review and does not depend on DC-34. It is complete only
with focused store tests and an end-to-end publication refusal test. It ships only in the combined
0.18.0 corrective release after all M1 blocker RFCs pass review.
