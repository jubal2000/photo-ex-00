//! Native Apple Sign In on iOS via AuthenticationServices, with fallback for other platforms.

#[cfg(target_os = "ios")]
use crate::firebase::{auth::Session, FbError};

#[cfg(target_os = "ios")]
pub async fn sign_in_native() -> Result<Session, FbError> {
    use std::ffi::{CStr, c_char, c_void};
    use futures_channel::oneshot;
    use crate::config;
    use crate::firebase::http;

    type RawCallback = extern "C" fn(
        *const c_char, // idToken
        *const c_char, // rawNonce
        *const c_char, // email
        *const c_char, // fullName
        *const c_char, // errorMsg
        *mut c_void,   // context
    );

    unsafe extern "C" {
        fn pe_ios_apple_sign_in(callback: RawCallback, context: *mut c_void);
    }

    struct AuthResult {
        id_token: Option<String>,
        raw_nonce: Option<String>,
        error: Option<String>,
    }

    extern "C" fn on_apple_auth_complete(
        id_token: *const c_char,
        raw_nonce: *const c_char,
        _email: *const c_char,
        _full_name: *const c_char,
        error_msg: *const c_char,
        context: *mut c_void,
    ) {
        let sender = unsafe { Box::from_raw(context as *mut oneshot::Sender<AuthResult>) };
        let res = if !error_msg.is_null() {
            let err_str = unsafe { CStr::from_ptr(error_msg).to_string_lossy().into_owned() };
            AuthResult { id_token: None, raw_nonce: None, error: Some(err_str) }
        } else if !id_token.is_null() && !raw_nonce.is_null() {
            let tok = unsafe { CStr::from_ptr(id_token).to_string_lossy().into_owned() };
            let nonce = unsafe { CStr::from_ptr(raw_nonce).to_string_lossy().into_owned() };
            AuthResult { id_token: Some(tok), raw_nonce: Some(nonce), error: None }
        } else {
            AuthResult { id_token: None, raw_nonce: None, error: Some("empty token from Apple".into()) }
        };
        let _ = sender.send(res);
    }

    let (tx, rx) = oneshot::channel::<AuthResult>();
    let ctx = Box::into_raw(Box::new(tx)) as *mut c_void;

    unsafe {
        pe_ios_apple_sign_in(on_apple_auth_complete, ctx);
    }

    let auth_res = rx.await.map_err(|_| FbError::Other("Apple sign-in cancelled".into()))?;

    if let Some(err) = auth_res.error {
        return Err(FbError::Other(err));
    }

    let id_token = auth_res.id_token.ok_or_else(|| FbError::Other("missing id_token".into()))?;
    let raw_nonce = auth_res.raw_nonce.ok_or_else(|| FbError::Other("missing raw_nonce".into()))?;

    // Exchange Apple identityToken with Firebase Auth via signInWithIdp REST API
    let url = format!("{}/accounts:signInWithIdp?key={}", config::identity_base(), config::FIREBASE_API_KEY);
    let post_body = format!(
        "id_token={}&providerId=apple.com&nonce={}",
        urlencoding::encode(&id_token),
        urlencoding::encode(&raw_nonce)
    );
    let body = serde_json::json!({
        "postBody": post_body,
        "requestUri": "http://localhost",
        "returnIdpCredential": true,
        "returnSecureToken": true,
    });

    let v = http::post_json(&url, None, &body).await?;
    let s = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or_default().to_string();
    let expires_in: u64 = s("expiresIn").parse().unwrap_or(3600);

    Ok(Session {
        uid: s("localId"),
        id_token: s("idToken"),
        refresh_token: s("refreshToken"),
        expires_at: crate::platform::now_secs() + expires_in,
        provider: "apple".into(),
    })
}
