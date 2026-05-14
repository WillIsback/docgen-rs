use std::path::Path;

const TAILWIND_CONFIG_FILES: &[&str] = &[
    "tailwind.config.js",
    "tailwind.config.ts",
    "tailwind.config.mjs",
    "tailwind.config.cjs",
];

const POSTCSS_CONFIG: &str = "postcss.config.js";

pub fn is_tailwind_enabled(project_root: &Path) -> bool {
    for config_file in TAILWIND_CONFIG_FILES {
        if project_root.join(config_file).exists() {
            return true;
        }
    }
    
    let postcss_path = project_root.join(POSTCSS_CONFIG);
    if postcss_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&postcss_path) {
            if content.contains("tailwindcss") {
                return true;
            }
        }
    }
    
    if let Ok(entries) = std::fs::read_dir(project_root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "css") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if content.contains("@tailwind") || content.contains("@layer") {
                        return true;
                    }
                }
            }
        }
    }
    
    false
}