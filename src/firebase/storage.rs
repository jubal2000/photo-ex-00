//! Firebase Storage upload (profile photos, card images).

use super::{http, FbError};
use crate::config;

/// Uploads `bytes` to `path` and returns a public download URL.
pub async fn upload(token: &str, path: &str, content_type: &str, bytes: Vec<u8>) -> Result<String, FbError> {
    let url = format!("{}/o?uploadType=media&name={}", config::storage_base(), urlencoding::encode(path));
    let v = http::post_bytes(&url, Some(token), content_type, bytes).await?;
    let dl_token = v
        .get("downloadTokens")
        .and_then(|t| t.as_str())
        .and_then(|t| t.split(',').next())
        .unwrap_or_default();
    let mut out = format!("{}/o/{}?alt=media", config::storage_base(), urlencoding::encode(path));
    if !dl_token.is_empty() {
        out.push_str(&format!("&token={dl_token}"));
    }
    Ok(out)
}
