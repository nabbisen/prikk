use super::{RepoPath, validate_no_path_collisions};

#[test]
fn repo_path_accepts_lexical_repository_relative_path() -> prikk_error::Result<()> {
    let path = RepoPath::parse("src/main.rs")?;

    assert_eq!(path.as_str(), "src/main.rs");
    Ok(())
}

#[test]
fn repo_path_rejects_filesystem_or_platform_ambiguous_paths() {
    for path in [
        "",
        "/absolute",
        "../escape",
        "src/../escape",
        ".prikk/FORMAT",
        ".PRIKK/FORMAT",
        "src\\main.rs",
        "CON.txt",
        "日本語.txt",
    ] {
        assert!(RepoPath::parse(path).is_err(), "{path}");
    }
}

#[test]
fn repo_path_detects_exact_and_case_folded_collisions() -> prikk_error::Result<()> {
    let exact = [RepoPath::parse("README.md")?, RepoPath::parse("README.md")?];
    assert!(validate_no_path_collisions(&exact).is_err());

    let folded = [RepoPath::parse("README.md")?, RepoPath::parse("readme.md")?];
    assert!(validate_no_path_collisions(&folded).is_err());
    Ok(())
}
