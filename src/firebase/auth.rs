//! Firebase Authentication via the Identity Toolkit / Secure Token REST APIs.
//!
//! Social sign-in (Google / Apple / Kakao) is performed by the hosted auth page in the
//! system browser (see `firebase/hosting/auth.html`). The page posts the resulting
//! Firebase refresh token to the `authSession` Cloud Function under a random,
//! single-use session id which the app polls. This works identically on web, iOS and
//! Android without deep-link plumbing and without embedding OAuth in a WebView
//! (which Google blocks).

use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{http, FbError};
use crate::config;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Session {
    pub uid: String,
    pub id_token: String,
    pub refresh_token: String,
    /// Unix seconds when `id_token` expires.
    pub expires_at: u64,
    pub provider: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Provider {
    Google,
    Apple,
    Kakao,
}

impl Provider {
    pub fn key(&self) -> &'static str {
        match self {
            Provider::Google => "google",
            Provider::Apple => "apple",
            Provider::Kakao => "kakao",
        }
    }
}

/// Exchanges a refresh token for a fresh id token (used by auto-login & renewal).
pub async fn refresh(refresh_token: &str, provider: &str) -> Result<Session, FbError> {
    let url = format!("{}/token?key={}", config::securetoken_base(), config::FIREBASE_API_KEY);
    let v = http::post_form(&url, &[("grant_type", "refresh_token"), ("refresh_token", refresh_token)]).await?;
    let s = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or_default().to_string();
    let expires_in: u64 = s("expires_in").parse().unwrap_or(3600);
    Ok(Session {
        uid: s("user_id"),
        id_token: s("id_token"),
        refresh_token: s("refresh_token"),
        expires_at: crate::platform::now_secs() + expires_in,
        provider: provider.to_string(),
    })
}

/// Developer-only anonymous sign-in (enabled only with the emulator).
pub async fn sign_in_guest() -> Result<Session, FbError> {
    let url = format!("{}/accounts:signUp?key={}", config::identity_base(), config::FIREBASE_API_KEY);
    let v = http::post_json(&url, None, &json!({ "returnSecureToken": true })).await?;
    let s = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or_default().to_string();
    let expires_in: u64 = s("expiresIn").parse().unwrap_or(3600);
    Ok(Session {
        uid: s("localId"),
        id_token: s("idToken"),
        refresh_token: s("refreshToken"),
        expires_at: crate::platform::now_secs() + expires_in,
        provider: "guest".into(),
    })
}

/// Builds the auth page URL. Call this synchronously in the click handler,
/// then immediately call `platform::open_url`, before any `spawn` / await.
pub fn auth_url(provider: Provider, lang: &str, session_id: &str) -> String {
    let base = format!(
        "{}?provider={}&session={}&lang={}",
        config::auth_page_url(),
        provider.key(),
        session_id,
        lang
    );
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(w) = web_sys::window() {
            if let Ok(loc) = w.location().href() {
                let clean = loc.split('?').next().unwrap_or(&loc);
                return format!("{}&return_url={}", base, urlencoding::encode(clean));
            }
        }
    }
    base
}

/// Polls authSession until the hosted page hands back a token (up to ~6 min).
/// Call this inside `spawn` after `open_url` has already been called synchronously.
pub async fn poll_session(session_id: &str, provider: Provider) -> Result<Session, FbError> {
    let poll = format!("{}/authSession?session={}", config::functions_base(), session_id);
    for _ in 0..180 {
        crate::platform::sleep_ms(2000).await;
        match http::get_json(&poll, None).await {
            Ok(v) => {
                if let Some(rt) = v.get("refreshToken").and_then(|x| x.as_str()) {
                    return refresh(rt, provider.key()).await;
                }
            }
            Err(FbError::Http(404, _)) => continue,
            Err(_) => {
                // Ignore transient network errors during tab switches and continue polling
                continue;
            }
        }
    }
    Err(FbError::Other("timeout".into()))
}

/// Opens the hosted sign-in page for `provider` and waits until it hands back a session.
/// On native (iOS/Android/desktop) this is fine as-is; on web use `auth_url` + `poll_session`
/// directly from the click handler so the browser does not block `window.open`.
#[allow(dead_code)]
pub async fn sign_in_with_provider(provider: Provider, lang: &str) -> Result<Session, FbError> {
    let session_id = uuid::Uuid::new_v4().simple().to_string();
    crate::platform::open_url(&auth_url(provider, lang, &session_id));
    poll_session(&session_id, provider).await
}
