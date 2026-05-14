use crate::process::PatchResult;
use git2::Repository;
use std::path::Path;

pub fn apply_with_git(patches: Vec<PatchResult>, repo_path: &Path) -> Result<(), git2::Error> {
    let repo = Repository::discover(repo_path)?;
    let head = repo.head()?;
    let original_branch = head.shorthand().unwrap_or("").to_string();
    if original_branch.is_empty() {
        return Err(git2::Error::from_str("docgen requires a named branch (HEAD is detached)"));
    }
    let branch_name = format!("docgen/{}", chrono::Local::now().format("%Y%m%d-%H%M%S"));
    let head_commit = head.peel_to_commit()?;
    repo.branch(&branch_name, &head_commit, false)?;
    repo.set_head(&format!("refs/heads/{branch_name}"))?;
    repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;

    let mut index = repo.index()?;
    for patch in &patches {
        std::fs::write(&patch.path, &patch.content)
            .map_err(|e| git2::Error::from_str(&e.to_string()))?;
        let workdir = repo.workdir().unwrap_or(Path::new("."));
        let abs = patch.path.canonicalize().map_err(|e| git2::Error::from_str(&e.to_string()))?;
        let rel = abs.strip_prefix(workdir).map_err(|_| git2::Error::from_str("patch path outside repo workdir"))?;
        index.add_path(rel)?;
    }
    index.write()?;
    let tree = repo.find_tree(index.write_tree()?)?;
    let sig = git2::Signature::now("docgen", "docgen@localhost")?;
    repo.commit(Some("HEAD"), &sig, &sig, "docs: add docstrings via docgen", &tree, &[&head_commit])?;

    repo.set_head(&format!("refs/heads/{original_branch}"))?;
    repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;

    let feature_commit = repo.find_branch(&branch_name, git2::BranchType::Local)?.get().peel_to_commit()?;
    let original_commit = repo.find_branch(&original_branch, git2::BranchType::Local)?.get().peel_to_commit()?;
    let ancestor = repo.find_commit(repo.merge_base(original_commit.id(), feature_commit.id())?)?;
    let mut merge_index = repo.merge_trees(&ancestor.tree()?, &original_commit.tree()?, &feature_commit.tree()?, None)?;
    if merge_index.has_conflicts() {
        return Err(git2::Error::from_str("merge conflict — manual resolution required"));
    }
    let merge_tree = repo.find_tree(merge_index.write_tree_to(&repo)?)?;
    repo.commit(Some("HEAD"), &sig, &sig, &format!("docs: merge {branch_name}"), &merge_tree, &[&original_commit, &feature_commit])?;
    repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
    repo.find_branch(&branch_name, git2::BranchType::Local)?.delete()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::PatchResult;
    use std::path::PathBuf;

    fn init_git_repo(dir: &Path, filename: &str, content: &str) {
        std::process::Command::new("git").args(["init", "-b", "main"]).current_dir(dir).output().unwrap();
        std::process::Command::new("git").args(["config", "user.email", "test@test.com"]).current_dir(dir).output().unwrap();
        std::process::Command::new("git").args(["config", "user.name", "Test"]).current_dir(dir).output().unwrap();
        std::fs::write(dir.join(filename), content).unwrap();
        std::process::Command::new("git").args(["add", filename]).current_dir(dir).output().unwrap();
        std::process::Command::new("git").args(["commit", "-m", "initial"]).current_dir(dir).output().unwrap();
    }

    #[test]
    fn patches_file_and_creates_merge_commit() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        let original = "export function add(a: number, b: number): number { return a + b; }\n";
        let patched = "/** Adds two numbers. */\nexport function add(a: number, b: number): number { return a + b; }\n";
        init_git_repo(dir, "index.ts", original);
        apply_with_git(
            vec![PatchResult { path: PathBuf::from(dir.join("index.ts")), content: patched.to_string() }],
            dir,
        ).expect("apply_with_git failed");
        assert_eq!(std::fs::read_to_string(dir.join("index.ts")).unwrap(), patched);
        let log = String::from_utf8_lossy(
            &std::process::Command::new("git").args(["log", "--oneline", "--all"]).current_dir(dir).output().unwrap().stdout
        ).to_string();
        assert!(log.contains("merge"), "expected merge commit in log:\n{log}");
    }
}
