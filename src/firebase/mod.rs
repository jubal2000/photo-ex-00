pub mod auth;
pub mod firestore;
pub mod functions;
pub mod storage;

use std::fmt;

#[derive(Debug, Clone)]
pub enum FbError {
    Http(u16, String),
    Network(String),
    Other(String),
}

impl fmt::Display for FbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FbError::Http(code, msg) => write!(f, "HTTP {code}: {msg}"),
            FbError::Network(m) => write!(f, "network: {m}"),
            FbError::Other(m) => write!(f, "{m}"),
        }
    }
}

pub(crate) mod http {
    use super::FbError;
    use serde_json::Value;
    use std::sync::OnceLock;

    fn client() -> &'static reqwest::Client {
        static C: OnceLock<reqwest::Client> = OnceLock::new();
        C.get_or_init(reqwest::Client::new)
    }

    fn auth(rb: reqwest::RequestBuilder, token: Option<&str>) -> reqwest::RequestBuilder {
        match token {
            Some(t) if !t.is_empty() => rb.bearer_auth(t),
            _ => rb,
        }
    }

    async fn finish(rb: reqwest::RequestBuilder) -> Result<Value, FbError> {
        let res = rb.send().await.map_err(|e| FbError::Network(e.to_string()))?;
        let status = res.status().as_u16();
        let text = res.text().await.map_err(|e| FbError::Network(e.to_string()))?;
        if !(200..300).contains(&status) {
            let msg = serde_json::from_str::<Value>(&text)
                .ok()
                .and_then(|v| v.pointer("/error/message").and_then(|m| m.as_str()).map(String::from))
                .unwrap_or(text);
            return Err(FbError::Http(status, msg));
        }
        if text.trim().is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_str(&text).map_err(|e| FbError::Other(e.to_string()))
    }

    pub async fn get_json(url: &str, token: Option<&str>) -> Result<Value, FbError> {
        finish(auth(client().get(url), token)).await
    }
    pub async fn post_json(url: &str, token: Option<&str>, body: &Value) -> Result<Value, FbError> {
        finish(auth(client().post(url), token).json(body)).await
    }
    pub async fn patch_json(url: &str, token: Option<&str>, body: &Value) -> Result<Value, FbError> {
        finish(auth(client().patch(url), token).json(body)).await
    }
    pub async fn post_form(url: &str, form: &[(&str, &str)]) -> Result<Value, FbError> {
        finish(client().post(url).form(form)).await
    }
    pub async fn post_bytes(url: &str, token: Option<&str>, content_type: &str, bytes: Vec<u8>) -> Result<Value, FbError> {
        finish(auth(client().post(url), token).header("Content-Type", content_type).body(bytes)).await
    }
    pub async fn delete(url: &str, token: Option<&str>) -> Result<(), FbError> {
        finish(auth(client().delete(url), token)).await.map(|_| ())
    }
}
