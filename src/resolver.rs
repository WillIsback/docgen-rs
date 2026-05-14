use std::path::{Path, PathBuf};

const EXTENSIONS: &[&str] = &["py", "ts", "tsx"];

pub fn resolve_files(target: &Path, recursive: bool, exclude: &[String]) -> Vec<PathBuf> {
    if target.is_file() {
        let ext = target.extension().and_then(|e| e.to_str()).unwrap_or("");
        if EXTENSIONS.contains(&ext) && !is_excluded(target, exclude) {
            return vec![target.to_path_buf()];
        }
        return vec![];
    }
    let mut out = collect_dir(target, recursive, exclude);
    out.sort();
    out
}

fn is_excluded(path: &Path, exclude: &[String]) -> bool {
    let s = path.to_string_lossy();
    exclude.iter().any(|p| s.contains(p.as_str()))
}

fn collect_dir(dir: &Path, recursive: bool, exclude: &[String]) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return vec![];
    };
    let mut out = vec![];
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() && recursive {
            out.extend(collect_dir(&path, true, exclude));
        } else if path.is_file() {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if EXTENSIONS.contains(&ext) && !is_excluded(&path, exclude) {
                out.push(path);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn resolves_single_py_file() {
        let tmp = TempDir::new().unwrap();
        let f = tmp.path().join("foo.py");
        fs::write(&f, "").unwrap();
        assert_eq!(resolve_files(&f, false, &[]), vec![f]);
    }

    #[test]
    fn skips_non_source_extensions() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("README.md"), "").unwrap();
        assert!(resolve_files(tmp.path(), false, &[]).is_empty());
    }

    #[test]
    fn flat_does_not_recurse() {
        let tmp = TempDir::new().unwrap();
        let sub = tmp.path().join("sub");
        fs::create_dir(&sub).unwrap();
        fs::write(tmp.path().join("a.py"), "").unwrap();
        fs::write(sub.join("b.py"), "").unwrap();
        let files = resolve_files(tmp.path(), false, &[]);
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("a.py"));
    }

    #[test]
    fn recursive_finds_nested() {
        let tmp = TempDir::new().unwrap();
        let sub = tmp.path().join("sub");
        fs::create_dir(&sub).unwrap();
        fs::write(tmp.path().join("a.py"), "").unwrap();
        fs::write(sub.join("b.ts"), "").unwrap();
        assert_eq!(resolve_files(tmp.path(), true, &[]).len(), 2);
    }

    #[test]
    fn excludes_pattern_match() {
        let tmp = TempDir::new().unwrap();
        let tests_dir = tmp.path().join("tests");
        fs::create_dir(&tests_dir).unwrap();
        fs::write(tmp.path().join("main.py"), "").unwrap();
        fs::write(tests_dir.join("test_foo.py"), "").unwrap();
        let files = resolve_files(tmp.path(), true, &["tests/".to_string()]);
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("main.py"));
    }
}
