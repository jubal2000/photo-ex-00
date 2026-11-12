//! Calls to HTTPS Cloud Functions that need admin privileges.

use serde_json::json;

use super::{http, FbError};
use crate::config;

/// Accepts a trade: the function moves card counts between both users atomically.
pub async fn accept_trade(token: &str, trade_id: &str) -> Result<(), FbError> {
    let url = format!("{}/acceptTrade", config::functions_base());
    http::post_json(&url, Some(token), &json!({ "tradeId": trade_id })).await.map(|_| ())
}
