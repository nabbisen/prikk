# RFC (accepted) - DC-36 Existing-Object Publication Integrity

**Status.** Accepted after architect re-review on 2026-07-15; implementation remains blocked on
DC-37 required-sync primitives and failure semantics.
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
2. validate every destination-directory component without following symlinks using DC-37's required
   directory primitive;
3. open the final entry without following symlinks and require a regular file;
4. read the bytes from that same opened handle;
5. decode and structurally validate the existing envelope;
6. verify its computed ObjectId and object type against the path;
7. require exact persisted-byte equality with the incoming candidate;
8. required-sync the containing directory before returning success.

Any read, decode, identity, type, or byte mismatch returns an integrity/corruption error. The method must
not overwrite, quarantine, delete, or repair the existing file. A symlink, directory, special file, or
symlinked destination component is an integrity error even when its target contains exact candidate
bytes. Type validation and byte comparison use one no-follow opened handle so they cannot refer to
different entries within the supported threat model. Concurrent creation must use the same post-race
comparison and containing-directory sync before success.

On the supported Linux mutation path, final-entry open uses no-follow, nonblocking, and close-on-exec
semantics. The implementation validates the opened handle as a regular file before reading from it, so
a FIFO or other special file cannot block or trigger type-specific behavior before rejection.

### No-clobber installation

An absence check followed by replacing `rename` is forbidden. New immutable objects use this sequence:

1. create a unique temporary regular file in the destination directory with `create_new` semantics;
2. write the complete candidate bytes, perform required file sync, and close the file;
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
error.

Temp ownership is invocation-local. An invocation may remove only the unique temp pathname it created
and still tracks. A later process does not infer ownership from a name and never removes a crash-left
temp while publishing an object. Crash-left temps are non-authoritative diagnosed debris for future
explicit maintenance; they cannot satisfy an object read or publication check. On retry after install
or first directory-sync failure, the required directory walker revalidates and re-syncs observed shard
entries, then the exact-existing path revalidates the opened final file and required-syncs its containing
directory before success. Current-invocation temp unlink and cleanup sync remain mandatory before that
invocation reports success.

### Crash-left temp diagnostics

Canonical object reads ignore crash-left temps, but repository verification must not silently skip
them. `verify` reports each recognized object temp as a non-fatal local-debris warning, and `doctor`
surfaces the same classification as a warning with no automatic repair. Neither command infers temp
ownership. Publication, verification, and doctor never delete these paths under DC-36.

## Required tests

- exact existing bytes are accepted idempotently;
- exact existing bytes required-sync the containing directory before success;
- malformed bytes, wrong object type, wrong computed id, and byte-different signature transport fail;
- a same-payload envelope with different signature bytes fails exact comparison;
- an exact-byte symlink target, directory, bounded FIFO/special-file entry, and symlinked shard
  component fail without blocking before same-handle type validation;
- canonical object reads ignore a crash-left temp while `verify` and `doctor` emit the selected
  non-fatal local-debris warning and perform no cleanup;
- publication does not advance a ref after any existing-object mismatch;
- the create race resolves to exact comparison rather than blind overwrite or success;
- same-process threads and separate processes race different transport bytes for one ObjectId;
- unsupported hard-link/no-replace behavior fails explicitly and never replaces the final path;
- file-sync, install, first directory-sync, temp unlink, and cleanup-sync failures preserve the stated
  final/temp artifacts and return failure;
- a fresh process retry after installed-final directory-sync failure re-syncs the shard chain and final
  directory before exact-existing success, without deleting crash-left temps.

## Non-goals

- No object schema, ObjectId, signature, ref protocol, GC, quarantine, or repair change.
- No repository-wide object rescan on every write.

## Dependencies and acceptance

DC-36 can receive design review independently of DC-34. Implementation depends on DC-37's accepted and
implemented required-sync API and failure semantics: the immutable no-clobber writer must consume that
shared durability boundary rather than define a private sync policy. DC-37 therefore lands before
DC-36 implementation, even while DC-36 remains the current design-closure increment.

DC-36 is complete only with focused store tests and an end-to-end publication refusal test. It ships
only in the combined 0.18.0 corrective release after all M1 blocker RFCs pass review.
