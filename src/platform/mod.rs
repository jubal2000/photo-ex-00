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

/// Opens `url` in the system browser (a new tab on web).
pub fn open_url(url: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(w) = web_sys::window() {
            let _ = w.open_with_url_and_target(url, "_blank");
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
