use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
struct DocgenToml {
    docgen: DocgenSection,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
struct DocgenSection {
    model: Option<String>,
    endpoint: String,
    style: String,
    batch_size: usize,
    timeout_secs: u64,
    connect_timeout_secs: u64,
    targets: TargetsSection,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
struct TargetsSection {
    languages: Vec<String>,
    exclude: Vec<String>,
}

impl Default for DocgenToml {
    fn default() -> Self { Self { docgen: DocgenSection::default() } }
}

impl Default for DocgenSection {
    fn default() -> Self {
        Self {
            model: None,
            endpoint: "http://localhost:30000/v1".to_string(),
            style: "google".to_string(),
            batch_size: 4,
            timeout_secs: 120,
            connect_timeout_secs: 5,
            targets: TargetsSection::default(),
        }
    }
}

impl Default for TargetsSection {
    fn default() -> Self {
        Self {
            languages: vec!["python".to_string(), "typescript".to_string()],
            exclude: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub vllm_base_url: String,
    pub batch_size: usize,
    pub connect_timeout_secs: u64,
    pub vllm_timeout_secs: u64,
    pub style: String,
    pub model_override: Option<String>,
    pub exclude: Vec<String>,
}

impl Config {
    pub fn load(project_root: &Path) -> Self {
        let file = load_toml(project_root);

        let endpoint = std::env::var("VLLM_BASE_URL")
            .unwrap_or_else(|_| file.docgen.endpoint.clone());

        let batch_size = std::env::var("BATCH_SIZE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(file.docgen.batch_size);

        let vllm_timeout_secs = std::env::var("VLLM_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(file.docgen.timeout_secs);

        let model_override = std::env::var("VLLM_MODEL")
            .ok()
            .filter(|v| !v.is_empty())
            .or_else(|| file.docgen.model.clone());

        Config {
            vllm_base_url: endpoint,
            batch_size,
            connect_timeout_secs: file.docgen.connect_timeout_secs,
            vllm_timeout_secs,
            style: file.docgen.style,
            model_override,
            exclude: file.docgen.targets.exclude,
        }
    }

    pub fn models_url(&self) -> String {
        let url = self.vllm_base_url.trim_end_matches('/');
        let url = url.strip_suffix("/v1").unwrap_or(url);
        format!("{url}/v1/models")
    }
}

fn load_toml(root: &Path) -> DocgenToml {
    let path = root.join("docgen.toml");
    if !path.exists() { return DocgenToml::default(); }
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| toml::from_str(&s).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Mutex;
    use tempfile::TempDir;

    // Serialise all env-var-touching tests to prevent races between parallel threads.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn defaults_when_no_file_and_no_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        let tmp = TempDir::new().unwrap();
        unsafe {
            std::env::remove_var("VLLM_BASE_URL");
            std::env::remove_var("BATCH_SIZE");
            std::env::remove_var("VLLM_TIMEOUT_SECS");
            std::env::remove_var("VLLM_MODEL");
        }
        let cfg = Config::load(tmp.path());
        assert_eq!(cfg.vllm_base_url, "http://localhost:30000/v1");
        assert_eq!(cfg.batch_size, 4);
        assert_eq!(cfg.vllm_timeout_secs, 120);
        assert_eq!(cfg.connect_timeout_secs, 5);
        assert_eq!(cfg.style, "google");
        assert!(cfg.model_override.is_none());
        assert!(cfg.exclude.is_empty());
    }

    #[test]
    fn loads_docgen_toml() {
        let _guard = ENV_LOCK.lock().unwrap();
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("docgen.toml"),
            "[docgen]\nendpoint = \"http://dgx:8000/v1\"\nstyle = \"numpy\"\nmodel = \"llama-3\"\n\n[docgen.targets]\nexclude = [\"tests/\", \"vendor/\"]\n"
        ).unwrap();
        unsafe {
            std::env::remove_var("VLLM_BASE_URL");
            std::env::remove_var("VLLM_MODEL");
        }
        let cfg = Config::load(tmp.path());
        assert_eq!(cfg.vllm_base_url, "http://dgx:8000/v1");
        assert_eq!(cfg.style, "numpy");
        assert_eq!(cfg.model_override, Some("llama-3".to_string()));
        assert_eq!(cfg.exclude, vec!["tests/", "vendor/"]);
    }

    #[test]
    fn env_vars_override_toml() {
        let _guard = ENV_LOCK.lock().unwrap();
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("docgen.toml"),
            "[docgen]\nendpoint = \"http://file:8000/v1\"\nmodel = \"from-file\"\n"
        ).unwrap();
        unsafe {
            std::env::set_var("VLLM_BASE_URL", "http://env:9000/v1");
            std::env::set_var("VLLM_MODEL", "from-env");
        }
        let cfg = Config::load(tmp.path());
        assert_eq!(cfg.vllm_base_url, "http://env:9000/v1");
        assert_eq!(cfg.model_override, Some("from-env".to_string()));
        unsafe {
            std::env::remove_var("VLLM_BASE_URL");
            std::env::remove_var("VLLM_MODEL");
        }
    }

    #[test]
    fn models_url_appends_correctly() {
        let _guard = ENV_LOCK.lock().unwrap();
        let tmp = TempDir::new().unwrap();
        unsafe { std::env::remove_var("VLLM_BASE_URL"); }
        let mut cfg = Config::load(tmp.path());
        cfg.vllm_base_url = "http://host:30000/v1".to_string();
        assert_eq!(cfg.models_url(), "http://host:30000/v1/models");
    }
}
