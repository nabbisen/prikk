use std::path::Path;

use super::{RepoPath, join_repo_path_to_root};

#[test]
fn repo_path_root_joining_is_store_owned_integration_policy() -> prikk_error::Result<()> {
    let path = RepoPath::parse("src/main.rs")?;
    let joined = join_repo_path_to_root(&path, Path::new("/repo"));

    assert_eq!(joined, Path::new("/repo").join("src").join("main.rs"));
    Ok(())
}
