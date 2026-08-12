# RFC 101 §5.5 — Prerequisite Ruling v1

**Reviewing:** `.git-exclude/review-request/prikk-rfc101-prerequisite-5.5-v1.md`.

**Investigation accepted. It found a real gap in DC-87 Stage 2's scope and it refused to round its
answer into my two buckets — correctly, because my framing was wrong.** My recommendation is that
RFC 101 closes. The risk judgment underneath it is the owner's and I am not taking it.

## 1. The gap they found is genuine

**Transactional NTFS appears nowhere in DC-87 Stage 2's trail** — they grepped it for "transactional",
"TxF", "CreateFileTransacted", "$LogFile" and got zero hits. That investigation was thorough on the
primitive-by-primitive Win32 surface and I accepted it as having taken fact-finding as far as it goes.
**It had not.** A whole API family purpose-built for this exact guarantee went unexamined, and I did not
notice when I ruled.

That is worth stating plainly because I have twice praised that investigation's completeness.
Completeness within a chosen scope is not completeness, and the scope was mine to challenge.

## 2. My binary framing was wrong

I asked whether Windows *genuinely lacks* new-name durability or merely lacks a *documented guarantee*,
and said either answer settles the RFC. **The true answer is neither.**

**A general-purpose primitive existed, was fully supported and documented for over a decade, and is
being withdrawn with no replacement covering this need.** That is a third case, and it is materially
**worse** than the "undocumented but stable" case my own §2.1 example described. `FILE_RENAME_INFO`'s
`Flags` field is present and stable with under-enumerated values; TxF is documented, working, and
carries a vendor warning that it *"may not be available in future versions of Microsoft Windows."*

Under-documented and scheduled-for-removal are opposite risks. I collapsed them into one bucket.

## 3. Calibration worth naming

They marked the `$LogFile` negative as **weaker evidence** than DC-87 Stage 2's per-API check — "not
found in one research pass" rather than exhaustively refuted — and asked for that difference to be on
the record rather than presented as equally certain.

**That is the right calibration and it is why the rest of the report is trustworthy.** A report that
graded its own weakest finding as confidently as its strongest would have to be re-derived entirely.

## 4. Recommendation: TxF does not clear the bar, and RFC 101 closes

The report states two conditional branches and declines to choose. I will choose, as a recommendation.

**The decisive argument is the shape of TxF's failure, not its probability.** If TxF is removed, a
repository previously written with TxF-backed durability and later opened by a TxF-less prikk is
**indistinguishable from one that always had the guarantee.** Nothing in the on-disk format changes;
only the guarantee evaporates. That is silent loss of a durability guarantee — the precise failure
prikk exists to make impossible, reintroduced at the foundation.

**It is not analogous to the owner's `unsafe` ruling.** That permits `unsafe` *under control, with
safety and maintainability preserved*. `unsafe` is under our control; a vendor's removal schedule is
not, and no amount of care on our side preserves a guarantee the platform withdraws.

**It does not meet the owner's own criterion.** *"In production use, the stable performance and the data
integrity is more important."* A durability foundation with a vendor-announced expiry is not stable, and
the integrity it underwrites would lapse without a signal.

**And Microsoft's own alternatives close the escape routes**: `ReplaceFile` is already refuted by DC-87
Stage 2 (documented non-atomic, `REPLACEFILE_WRITE_THROUGH` documented as *"not supported"*), and the
remainder — an installer framework, or embedding a database engine — are disproportionate by orders of
magnitude to "durably create one file name."

**So: no supported primitive reaches parity, and RFC 101's question is answered in the negative.**

## 5. What closing means, and what survives

1. **RFC 101 closes with a negative result, on the owner's word.** This is the outcome §6 of the RFC
   named as successful, and it is one. The RFC established what no prior increment had: parity is not
   reachable, and *why* — not a missing effort but a missing primitive, under a storage model where
   every mutation creates new names.
2. **DC-87 Stage 2 returns to the owner's option 2 / option 3 choice**, with both costs now known rather
   than estimated. That is the live decision and it is yours.
3. **§5.2's fifteen-transition table and 31-site call index are retained regardless.** They are the map
   of prikk's new-name surface, nothing else in the project has one, and they belong in the code's
   documentation — the round 7 ruling applied here.
4. **The three `FINDINGS.md` rows stand independently** — T12's silent signed deletion most of all.
5. **§5.4 and §5.6 stay suspended and should now be cancelled** if the owner closes: both presuppose a
   design that will not exist.

## 6. One thing I am not asserting

I have **not** verified how comparable systems handle this — whether Git on Windows, for instance,
simply accepts weaker durability there. If true it would tell the owner that parity is a higher bar
than the field holds itself to, which is relevant to choosing between options 2 and 3. **I am flagging
it as unverified rather than asserting it**, and it is a cheap question to commission if the owner wants
it before deciding.

## 7. Standing

- **RFC 101: recommended for closure.** Owner's call; nothing moves until then.
- **§5.4, §5.6:** suspended, cancel on closure.
- **DC-87 Stage 2:** unblocked as a *decision* — option 2 or option 3, owner's.
- **DC-95 Stage 1 round 9:** unaffected, review pending separately.
