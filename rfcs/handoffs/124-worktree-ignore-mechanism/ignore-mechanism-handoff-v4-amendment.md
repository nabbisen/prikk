# Amendment to `ignore-mechanism-handoff-v1.md` — one line, and RFC 124 closes

**The re-land (`46ecf01`) is accepted and pushed. Windows CI is green.** This is the last item.

---

## 1. §6's error-class change is a regression, and it is the one I ruled on two days ago

Your §6 named it honestly: routing `insert_regular_file` through the shared converter changed a
non-UTF-8 worktree path from `PrikkError::InvalidName` to `PrikkError::Integrity`. You flagged it as
"same fail-closed behavior, only the exact substring changed."

**The substring is the problem.** A user with a filename prikk cannot represent now sees:

```
error: integrity error: worktree path is not UTF-8: …
```

**`integrity error` is what this product says when a repository is damaged.** Nothing is damaged
here — the worktree contains a name outside the supported subset. This is the same misclassification
I ruled on in RFC 122 §4, in the same words: *"it is an unsupported-state refusal, not an integrity
failure, and the difference matters because `integrity error` is what this product says when a
repository is damaged."* A reader who believes it reaches for `doctor`, backups and the recovery
references.

**The shared converter inherited the worse of the two classifications**, because `worktree_status.rs`
had used `Integrity` first. So fixing it improves `worktree-status` too, rather than only restoring
`commit`.

**Do:** `pathbuf_to_slash_string`'s two error arms return `PrikkError::InvalidName`. Restore
`non_utf8_worktree_path_fails_closed`'s assertion to the wording that is then true, rather than the
substring both wordings happened to share.

**Check the "empty worktree path" arm by the same standard** — decide whether that one is genuinely
an integrity condition (it may be: an empty path from a directory walk suggests something is wrong
with the walk, not with the user's file names) and say which you chose and why. **I am not ruling
that one; it is a different question from the UTF-8 arm.**

## 2. Nothing else

The re-land is accepted in full: the shared converter, the `insert_regular_file` fix you went beyond
the instruction to make and declared, the Linux-runnable separator test, the deep-nesting test, and
the preserved directory pruning. §5's list is satisfied.

**RFC 124 closes when this lands.**

## 3. Gates

Full set. **No CI control — that is mine, and this time I ran it: `46ecf01` is green on all 15 jobs,
Windows mutation included.**
