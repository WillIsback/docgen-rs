#[path = "../src/tailwind.rs"]
mod tailwind;

#[test]
fn test_tailwind_config_detection() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("tailwind.config.js");
    std::fs::write(&config_path, "module.exports = {}").unwrap();
    
    let result = tailwind::is_tailwind_enabled(temp_dir.path());
    assert!(result, "Should detect tailwind.config.js");
}

#[test]
fn test_no_tailwind() {
    let temp_dir = tempfile::tempdir().unwrap();
    let result = tailwind::is_tailwind_enabled(temp_dir.path());
    assert!(!result, "Should return false when no Tailwind config");
}