# RFC 102, Stage 2 — Implementation Handoff v1

**Authorized by the project owner 2026-08-14.** Design: `design-v1.md` §3 and §7.
**Stages 3–6 are not authorized. No container is built in this stage, and no storage format changes.**

## 1. What Stage 2 is, and why it runs before any container exists

**Earn the isolate-and-continue read behaviour against the WAL and ref log — formats already in
production — before anything new depends on it.**

Today both hard-`Err` on a mid-stream checksum mismatch (`wal.rs:323-324`, *"WAL checksum mismatch at
byte offset {offset}"*, and `refs/log.rs:206`, *"ref-log checksum mismatch at byte offset {offset}"*). **That is correct for a single-purpose queue and a
blast-radius regression for a container of unrelated objects** — the amended constraint 5 that the RFC's §6.2
found and its §6.3a recorded.

**Stage 3's containers cannot be built without this behaviour, and building it here means it is proven
against real formats with real fixtures rather than co-designed with the thing that needs it.**

## 2. The design's resync scheme — treat it as a hypothesis

`design-v1.md` §3: validate the frame at the cursor; on corruption, **emit a finding naming the record's
offset**, then scan forward for the next magic, validating each candidate's full frame including
checksum. A false positive — the magic appearing inside record bytes — fails the checksum and the scan
continues.

**This is mine and no one else has checked it.** §11 of the design names it as one of the two places I
would look first for an error. Specifically worth doubting:

1. **Is the magic actually at a frame boundary you can scan for**, or does today's framing make resync
   ambiguous? Derive it from `frame_record`/`frame_log_record`, not from my paragraph.
2. **What does a corrupt *length* field do?** My scheme assumes you can fall back to scanning; confirm
   there is no case where a corrupt length makes the reader consume or skip a *sound* record.
3. **Is byte-wise scanning acceptable at container scale**, or does it need bounding?

**A stop-and-report is a complete outcome.** If resync cannot be made sound here, that is a finding
about Stage 3's feasibility, and it is far cheaper learned now.

## 3. The behaviour that must not change

- **A trailing partial frame at EOF stays tolerated**, exactly as today (`wal.rs:318`'s
  `trailing_partial_bytes`). Stage 2 changes what happens *mid-stream*, not at the tail.
- **`verify` must still fail** on a corrupted WAL or ref log. Isolate-and-continue means *report every
  damaged record instead of the first*, **not** *tolerate damage*. If a repository that failed
  verification before now passes, that is a regression, not the feature.
- **DC-95's classification survives.** Several of its 41 rows sit on these decode paths; check them
  rather than assume.
- **No storage format change.** Same magic, same framing, same checksums.

## 4. Where the findings go

Level 1 and Level 2 of DC-95 Stage 2 built the machinery for exactly this: per-item outcomes with a
blocking flag, `doctor` deriving severity from it, and `repair_repository` refusing per item. **Reuse it
— do not invent a second reporting shape.** A damaged record is an item.

## 5. Acceptance criteria

1. **Two independently damaged records in one WAL are both reported**, with their offsets.
2. **Every sound record after a damaged one is still read.** This is the whole point; assert the sound
   records, not just the findings.
3. **A repository that failed verification before still fails it.**
4. **A trailing partial frame is still tolerated**, unchanged.
5. **`repair_repository` still refuses** on a damaged WAL.
6. Full gate set per `EXECUTION-ORDER.md` §6 rule 9, plus **green three-platform CI**.

## 6. Standing

Stage 2 merges before Stage 3 is scoped. Stage 1 is merged (`6d10185`); this builds on it but does not
depend on the marker.
