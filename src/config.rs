//! Build-time configuration.
//!
//! Values are read from environment variables at compile time (see `.env.example`).
//! When `USE_EMULATOR=1`, the app talks to the local Firebase Emulator Suite and
//! shows a developer "guest login" button so the UI can be tested without OAuth keys.

pub const FIREBASE_API_KEY: &str = match option_env!("FIREBASE_API_KEY") {
    Some(v) => v,
    None => "demo-api-key",
};
pub const FIREBASE_PROJECT_ID: &str = match option_env!("FIREBASE_PROJECT_ID") {
    Some(v) => v,
    None => "demo-photo-exchanger",
};
pub const FIREBASE_STORAGE_BUCKET: &str = match option_env!("FIREBASE_STORAGE_BUCKET") {
    Some(v) => v,
    None => "demo-photo-exchanger.appspot.com",
};
/// Hosted sign-in page (Firebase Hosting) that performs Google / Apple / Kakao OAuth
/// in the system browser and hands the Firebase session back to the app.
pub const AUTH_PAGE_URL: &str = match option_env!("AUTH_PAGE_URL") {
    Some(v) => v,
    None => "http://127.0.0.1:5002/auth.html",
};
/// Cloud Functions base URL (authSession, kakaoToken, acceptTrade).
pub const FUNCTIONS_BASE_URL: &str = match option_env!("FUNCTIONS_BASE_URL") {
    Some(v) => v,
    None => "http://127.0.0.1:5001/demo-photo-exchanger/asia-northeast3",
};
/// Google AdSense client id for the web banner (e.g. "ca-pub-xxxxxxxx").
pub const ADSENSE_CLIENT: &str = match option_env!("ADSENSE_CLIENT") {
    Some(v) => v,
    None => "",
};
pub const ADSENSE_SLOT: &str = match option_env!("ADSENSE_SLOT") {
    Some(v) => v,
    None => "",
};

pub const USE_EMULATOR: bool = match option_env!("USE_EMULATOR") {
    Some(v) => !v.is_empty() && v.as_bytes()[0] == b'1',
    None => true,
};

/// Host for emulators. Android emulator reaches the host machine via 10.0.2.2.
pub fn emulator_host() -> &'static str {
    if cfg!(target_os = "android") { "10.0.2.2" } else { "127.0.0.1" }
}

pub fn identity_base() -> String {
    if USE_EMULATOR {
        format!("http://{}:9099/identitytoolkit.googleapis.com/v1", emulator_host())
    } else {
        "https://identitytoolkit.googleapis.com/v1".into()
    }
}

pub fn securetoken_base() -> String {
    if USE_EMULATOR {
        format!("http://{}:9099/securetoken.googleapis.com/v1", emulator_host())
    } else {
        "https://securetoken.googleapis.com/v1".into()
    }
}

pub fn firestore_base() -> String {
    let host = if USE_EMULATOR {
        format!("http://{}:8080", emulator_host())
    } else {
        "https://firestore.googleapis.com".into()
    };
    format!("{host}/v1/projects/{FIREBASE_PROJECT_ID}/databases/(default)/documents")
}

pub fn storage_base() -> String {
    if USE_EMULATOR {
        format!("http://{}:9199/v0/b/{FIREBASE_STORAGE_BUCKET}", emulator_host())
    } else {
        format!("https://firebasestorage.googleapis.com/v0/b/{FIREBASE_STORAGE_BUCKET}")
    }
}

pub fn functions_base() -> String {
    if USE_EMULATOR {
        FUNCTIONS_BASE_URL.replace("127.0.0.1", emulator_host())
    } else {
        FUNCTIONS_BASE_URL.to_string()
    }
}

pub fn auth_page_url() -> String {
    if USE_EMULATOR {
        AUTH_PAGE_URL.replace("127.0.0.1", emulator_host())
    } else {
        AUTH_PAGE_URL.to_string()
    }
}
