use crate::{config::Config, error::DocgenError};
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
struct ModelsResponse {
    data: Vec<ModelEntry>,
}

#[derive(Deserialize)]
struct ModelEntry {
    id: String,
}

#[derive(Clone)]
pub struct ChatMessage {
    pub role: &'static str,
    pub content: String,
}

pub async fn chat_complete(
    messages: &[ChatMessage],
    model: &str,
    max_tokens: u32,
    temperature: f32,
    cfg: &Config,
) -> Result<String, DocgenError> {
    let base = cfg.vllm_base_url.trim_end_matches('/');
    let url = if base.ends_with("/v1") {
        format!("{base}/chat/completions")
    } else {
        format!("{base}/v1/chat/completions")
    };
    let msgs: Vec<_> = messages
        .iter()
        .map(|m| json!({"role": m.role, "content": m.content}))
        .collect();
    let body = json!({
        "model": model,
        "messages": msgs,
        "max_tokens": max_tokens,
        "temperature": temperature,
        "chat_template_kwargs": {"enable_thinking": false}
    });
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(cfg.vllm_timeout_secs))
        .build()
        .expect("valid client");
    let resp: serde_json::Value = client
        .post(&url)
        .header("Authorization", "Bearer none")
        .json(&body)
        .send()
        .await
        .map_err(|e| DocgenError::RequestFailed {
            reason: e.to_string(),
        })?
        .json()
        .await
        .map_err(|e| DocgenError::RequestFailed {
            reason: e.to_string(),
        })?;
    let content = resp["choices"][0]["message"]["content"]
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| DocgenError::RequestFailed {
            reason: "empty or missing content in response".to_string(),
        })?;
    Ok(content.to_string())
}

pub async fn check_reachable(cfg: &Config) -> bool {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(cfg.connect_timeout_secs))
        .build()
        .expect("valid client");
    client
        .get(cfg.models_url())
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

pub async fn resolve_model(cfg: &Config) -> Result<String, DocgenError> {
    if let Some(model) = &cfg.model_override {
        return Ok(model.clone());
    }
    detect_model(cfg).await
}

async fn detect_model(cfg: &Config) -> Result<String, DocgenError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(cfg.connect_timeout_secs))
        .build()
        .expect("valid client");
    let resp: ModelsResponse = client
        .get(cfg.models_url())
        .send()
        .await
        .map_err(|_| DocgenError::VllmUnreachable {
            url: cfg.models_url(),
        })?
        .json()
        .await?;
    resp.data
        .into_iter()
        .next()
        .map(|m| m.id)
        .ok_or(DocgenError::NoModelsAvailable)
}
