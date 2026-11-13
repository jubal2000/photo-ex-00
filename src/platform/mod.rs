//! Platform abstraction: time, sleeping, opening URLs, key-value storage,
//! secure (token) storage and biometric authentication.

pub mod biometric;
pub mod image_picker;
pub mod storage;

pub fn now_secs() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        (js_sys::Date::now() / 1000.0) as u64
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }
}

pub async fn sleep_ms(ms: u32) {
    #[cfg(target_arch = "wasm32")]
    gloo_timers::future::TimeoutFuture::new(ms).await;
    #[cfg(not(target_arch = "wasm32"))]
    tokio::time::sleep(std::time::Duration::from_millis(ms as u64)).await;
}

/// Opens `url` in the system browser.
/// On desktop web, opens a centered popup so completion auto-closes the popup.
/// On mobile web, navigates in the current window to preserve single-tab state.
pub fn open_url(url: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(w) = web_sys::window() {
            let is_mobile = w.navigator().user_agent().map(|ua| {
                let u = ua.to_lowercase();
                u.contains("iphone") || u.contains("ipad") || u.contains("ipod") || u.contains("android") || u.contains("mobile")
            }).unwrap_or(false);

            if is_mobile {
                let _ = w.location().set_href(url);
            } else {
                let features = "width=520,height=680,menubar=no,toolbar=no,location=no,status=no,resizable=yes,scrollbars=yes";
                if w.open_with_url_and_target_and_features(url, "pe_auth_popup", features).is_err() {
                    let _ = w.open_with_url_and_target(url, "_blank");
                }
            }
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = webbrowser::open(url);
    }
}

pub fn platform_name() -> &'static str {
    if cfg!(target_arch = "wasm32") {
        "web"
    } else if cfg!(target_os = "ios") {
        "ios"
    } else if cfg!(target_os = "android") {
        "android"
    } else {
        "desktop"
    }
}
