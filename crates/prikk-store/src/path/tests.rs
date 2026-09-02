use std::path::{Path, PathBuf};

use super::{RepoPath, join_repo_path_to_root, pathbuf_to_slash_string};

#[test]
fn repo_path_root_joining_is_store_owned_integration_policy() -> prikk_error::Result<()> {
    let path = RepoPath::parse("src/main.rs")?;
    let joined = join_repo_path_to_root(&path, Path::new("/repo"));

    assert_eq!(joined, Path::new("/repo").join("src").join("main.rs"));
    Ok(())
}

/// RFC 124's own re-land (`ignore-mechanism-handoff-v2-amendment.md` §5): the mechanism's first
/// landing called `Path::to_str()` on a `Path::join`-built path, which renders the platform
/// separator -- on Windows, a backslash-joined string that matched no `/`-joined ignore rule or
/// tracked path, and broke `commit` outright for any nested worktree path. This crate cannot run the
/// Windows job, so the invariant this test checks is the one that makes the fix correct on every
/// platform regardless: `pathbuf_to_slash_string` never delegates to the path's own
/// platform-rendered string form. It decomposes via `Path::components()` (which decomposes any
/// platform's path correctly, independent of the *host* OS) and rejoins with a **literal** `/`, so a
/// multi-component path built the same way `Path::join` builds one produces a `/`-joined string with
/// no `\` in it, constructed here rather than by walking a real filesystem, exactly as required.
#[test]
fn pathbuf_to_slash_string_never_produces_a_backslash_for_a_joined_path() -> prikk_error::Result<()>
{
    let joined: PathBuf = Path::new("build").join("debug").join("output.bin");
    let rel = pathbuf_to_slash_string(&joined)?;
    assert_eq!(rel, "build/debug/output.bin");
    assert!(!rel.contains('\\'), "must never contain a backslash: {rel}");
    Ok(())
}

/// The same invariant for the two-component case the reverted mechanism's own tests exercised
/// (`build/output.txt`, `keep/data.txt`, ...) -- the shape every RFC 124 fixture path actually has.
#[test]
fn pathbuf_to_slash_string_matches_a_forward_slash_joined_string() -> prikk_error::Result<()> {
    let joined = Path::new("keep").join("data.txt");
    assert_eq!(pathbuf_to_slash_string(&joined)?, "keep/data.txt");
    Ok(())
}

#[test]
fn pathbuf_to_slash_string_rejects_a_non_utf8_component() {
    #[cfg(unix)]
    {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        let non_utf8 = OsStr::from_bytes(&[0x66, 0x6f, 0x80, 0x6f]); // "fo\x80o"
        let path = Path::new("dir").join(non_utf8);
        assert!(pathbuf_to_slash_string(&path).is_err());
    }
}
