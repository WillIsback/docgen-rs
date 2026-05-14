use std::path::Path;

pub fn dirty_files(repo_path: &Path) -> Vec<String> {
    let Ok(repo) = git2::Repository::discover(repo_path) else {
        return vec![];
    };
    let mut opts = git2::StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .exclude_submodules(true)
        .include_ignored(false);
    let Ok(statuses) = repo.statuses(Some(&mut opts)) else {
        return vec![];
    };
    statuses
        .iter()
        .filter(|s| s.status() != git2::Status::CURRENT)
        .filter_map(|s| s.path().map(String::from))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_empty_for_nonexistent_path() {
        assert!(dirty_files(Path::new("/nonexistent/path/12345")).is_empty());
    }

    #[test]
    fn does_not_panic_on_git_repo() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let _ = dirty_files(root);
    }
}
