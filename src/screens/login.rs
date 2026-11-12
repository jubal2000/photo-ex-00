use dioxus::prelude::*;
use uuid::Uuid;

use crate::config;
use crate::firebase::auth::{self, Provider};
use crate::platform::{self, biometric, storage};
use crate::state::*;

const LOGO: Asset = asset!("/assets/logo.svg");

/// Sign-in / sign-up. The first social sign-in creates the account (and profile doc).
#[component]
pub fn Login() -> Element {
    let mut app = use_app();
    let mut busy = use_signal(|| false);
    let mut remember = use_signal(|| pref_bool(KEY_AUTO, true));
    let has_saved = storage::secure_get(KEY_REFRESH).is_some();
    let bio_ready = has_saved && pref_bool(KEY_BIO, false) && biometric::available();

    let bio_login = move || {
        spawn(async move {
            busy.set(true);
            match biometric::authenticate(app.tr("bio_reason"), app.tr("cancel")).await {
                biometric::BioResult::Success => {
                    if let Some(rt) = storage::secure_get(KEY_REFRESH) {
                        let provider = storage::get(KEY_PROVIDER).unwrap_or_default();
                        match auth::refresh(&rt, &provider).await {
                            Ok(s) => app.on_signed_in(s, true),
                            Err(e) => app.show_error(&e),
                        }
                    }
                }
                _ => app.show_toast(app.tr("bio_failed")),
            }
            busy.set(false);
        });
    };

    // Prompt for biometrics immediately when it is the configured gate.
    use_hook(move || {
        if bio_ready {
            bio_login();
        } else if let Some(session_id) = storage::get("pe_pending_session") {
            let p_key = storage::get("pe_pending_provider").unwrap_or_else(|| "google".into());
            let provider = match p_key.as_str() {
                "apple" => Provider::Apple,
                "kakao" => Provider::Kakao,
                _ => Provider::Google,
            };
            spawn(async move {
                busy.set(true);
                match auth::poll_session(&session_id, provider).await {
                    Ok(s) => {
                        storage::remove("pe_pending_session");
                        storage::remove("pe_pending_provider");
                        app.on_signed_in(s, true);
                    }
                    Err(_) => {
                        storage::remove("pe_pending_session");
                        storage::remove("pe_pending_provider");
                    }
                }
                busy.set(false);
            });
        }
    });

    let social = move |p: Provider| {
        // Generate session and open URL synchronously inside the click handler
        // so the browser does not treat window.open as a blocked popup.
        let session_id = Uuid::new_v4().simple().to_string();
        let lang = app.lang.read().code();
        storage::set("pe_pending_session", &session_id);
        storage::set("pe_pending_provider", p.key());
        platform::open_url(&auth::auth_url(p, lang, &session_id));

        let s_id = session_id.clone();
        spawn(async move {
            busy.set(true);
            match auth::poll_session(&s_id, p).await {
                Ok(s) => {
                    storage::remove("pe_pending_session");
                    storage::remove("pe_pending_provider");
                    app.on_signed_in(s, remember());
                }
                Err(e) => {
                    storage::remove("pe_pending_session");
                    storage::remove("pe_pending_provider");
                    app.show_error(&e);
                }
            }
            busy.set(false);
        });
    };

    rsx! {
        div { class: "login",
            img { class: "login-logo", src: LOGO, alt: "logo" }
            h1 { {app.tr("app_name")} }
            p { class: "muted", {app.tr("login_title")} }

            if busy() {
                div { class: "login-wait", div { class: "spinner" } p { {app.tr("login_waiting")} } }
            } else {
                div { class: "login-buttons",
                    if bio_ready {
                        button { class: "btn btn-bio", onclick: move |_| bio_login(), "🔐 " {app.tr("login_bio")} }
                    }
                    button { class: "btn btn-google", onclick: move |_| social(Provider::Google),
                        span { class: "provider-icon", "G" } {app.tr("login_google")} }
                    button { class: "btn btn-apple", onclick: move |_| social(Provider::Apple),
                        span { class: "provider-icon", "" } {app.tr("login_apple")} }
                    button { class: "btn btn-kakao", onclick: move |_| social(Provider::Kakao),
                        span { class: "provider-icon", "💬" } {app.tr("login_kakao")} }
                    if config::USE_EMULATOR {
                        button { class: "btn btn-ghost",
                            onclick: move |_| {
                                spawn(async move {
                                    busy.set(true);
                                    match auth::sign_in_guest().await {
                                        Ok(s) => app.on_signed_in(s, remember()),
                                        Err(e) => app.show_error(&e),
                                    }
                                    busy.set(false);
                                });
                            },
                            {app.tr("login_guest")}
                        }
                    }
                }
                label { class: "check",
                    input { r#type: "checkbox", checked: remember(), onchange: move |e| remember.set(e.checked()) }
                    {app.tr("login_auto")}
                }
                p { class: "muted small", {app.tr("login_terms")} }
            }
        }
    }
}
