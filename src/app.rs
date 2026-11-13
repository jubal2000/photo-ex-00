use dioxus::prelude::*;

use crate::components::{AdBanner, Header, SideMenu};
use crate::firebase::auth;
use crate::models::Profile;
use crate::platform::{biometric, storage};
use crate::screens;
use crate::state::*;
use crate::api;

const MAIN_CSS: Asset = asset!("/assets/main.css");
const FAVICON: Asset = asset!("/assets/favicon.png");

#[component]
pub fn App() -> Element {
    let mut app = use_context_provider(AppState::new);

    // Startup: auto-login with the stored refresh token (unless biometric gate is on,
    // in which case the login screen asks for biometrics first).
    use_hook(move || {
        spawn(async move {
            #[cfg(target_arch = "wasm32")]
            {
                if let Some(w) = web_sys::window() {
                    if let Ok(search) = w.location().search() {
                        if !search.is_empty() {
                            let mut url_rt = None;
                            let mut url_prov = "google".to_string();
                            for pair in search.trim_start_matches('?').split('&') {
                                let mut it = pair.splitn(2, '=');
                                let k = it.next().unwrap_or_default();
                                let v = it.next().unwrap_or_default();
                                if k == "refreshToken" && !v.is_empty() {
                                    url_rt = urlencoding::decode(v).ok().map(|s| s.into_owned());
                                } else if k == "provider" {
                                    url_prov = urlencoding::decode(v).ok().map(|s| s.into_owned()).unwrap_or_else(|| "google".into());
                                }
                            }
                            if let Some(rt) = url_rt.filter(|s| !s.is_empty()) {
                                storage::remove("pe_pending_session");
                                storage::remove("pe_pending_provider");
                                if let Ok(pathname) = w.location().pathname() {
                                    let _ = w.history().and_then(|h| h.replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&pathname)));
                                }
                                match auth::refresh(&rt, &url_prov).await {
                                    Ok(s) => {
                                        app.on_signed_in(s, true);
                                        app.booted.set(true);
                                        return;
                                    }
                                    Err(e) => {
                                        app.show_error(&e);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            let auto = pref_bool(KEY_AUTO, true);
            let bio = pref_bool(KEY_BIO, false) && biometric::available();
            if let (true, false, Some(rt)) = (auto, bio, storage::secure_get(KEY_REFRESH)) {
                let provider = storage::get(KEY_PROVIDER).unwrap_or_default();
                match auth::refresh(&rt, &provider).await {
                    Ok(s) => app.on_signed_in(s, true),
                    Err(crate::firebase::FbError::Http(400, _)) => {
                        storage::secure_remove(KEY_REFRESH);
                    }
                    Err(_) => {
                        // Keep saved token across transient network errors
                    }
                }
            }
            app.booted.set(true);
        });
    });

    #[cfg(target_arch = "wasm32")]
    use_hook(move || {
        use wasm_bindgen::closure::Closure;
        use wasm_bindgen::JsCast;
        if let Some(w) = web_sys::window() {
            let cb = Closure::wrap(Box::new(move |e: web_sys::MessageEvent| {
                if let Ok(txt) = js_sys::JSON::stringify(&e.data()) {
                    let s: String = txt.into();
                    if s.contains("PHOTO_EXCHANGER_AUTH") {
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&s) {
                            if let Some(rt) = val.get("refreshToken").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
                                let prov = val.get("provider").and_then(|v| v.as_str()).unwrap_or("google").to_string();
                                let rt_owned = rt.to_string();
                                storage::remove("pe_pending_session");
                                storage::remove("pe_pending_provider");
                                spawn(async move {
                                    if let Ok(sess) = auth::refresh(&rt_owned, &prov).await {
                                        let auto = pref_bool(KEY_AUTO, true);
                                        app.on_signed_in(sess, auto);
                                    }
                                });
                            }
                        }
                    }
                }
            }) as Box<dyn FnMut(_)>);
            let _ = w.add_event_listener_with_callback("message", cb.as_ref().unchecked_ref());
            cb.forget();

            let cb_storage = Closure::wrap(Box::new(move |e: web_sys::StorageEvent| {
                if e.key().as_deref() == Some("pe_auth_sync") {
                    if let Some(new_val) = e.new_value() {
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&new_val) {
                            if let Some(rt) = val.get("refreshToken").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
                                let prov = val.get("provider").and_then(|v| v.as_str()).unwrap_or("google").to_string();
                                let rt_owned = rt.to_string();
                                storage::remove("pe_pending_session");
                                storage::remove("pe_pending_provider");
                                spawn(async move {
                                    if let Ok(sess) = auth::refresh(&rt_owned, &prov).await {
                                        let auto = pref_bool(KEY_AUTO, true);
                                        app.on_signed_in(sess, auto);
                                    }
                                });
                            }
                        }
                    }
                }
            }) as Box<dyn FnMut(_)>);
            let _ = w.add_event_listener_with_callback("storage", cb_storage.as_ref().unchecked_ref());
            cb_storage.forget();
        }
    });

    // Load user data whenever the signed-in user changes.
    let uid = use_memo(move || app.session.read().as_ref().map(|s| s.uid.clone()));
    use_effect(move || {
        if let Some(uid) = uid() {
            spawn(async move { load_user_data(app, uid).await });
        }
    });

    // 1. Prevent pinch-zoom on mobile WebViews while safely preserving user tap/click interactions.
    // 2. Protect Korean / CJK IME composition from being cancelled by virtual DOM value overrides.
    use_effect(move || {
        let _ = document::eval(r#"
            if (!window.__peZoomLock) {
                window.__peZoomLock = true;
                document.addEventListener('gesturestart', function(e) { e.preventDefault(); }, { passive: false });
                document.addEventListener('gesturechange', function(e) { e.preventDefault(); }, { passive: false });
                document.addEventListener('gestureend', function(e) { e.preventDefault(); }, { passive: false });
            }

            if (!window.__peImeGuard) {
                window.__peImeGuard = true;

                var isComposing = false;
                document.addEventListener('compositionstart', function(e) {
                    isComposing = true;
                    if (e.target) e.target.__isComposing = true;
                }, true);

                document.addEventListener('compositionend', function(e) {
                    isComposing = false;
                    if (e.target) e.target.__isComposing = false;
                }, true);

                function getPropDesc(obj, prop) {
                    var cur = obj;
                    while (cur) {
                        var d = Object.getOwnPropertyDescriptor(cur, prop);
                        if (d) return d;
                        cur = Object.getPrototypeOf(cur);
                    }
                    return null;
                }

                function patchProto(proto) {
                    var desc = getPropDesc(proto, 'value');
                    if (!desc || !desc.set || !desc.get) return;
                    var origSet = desc.set;
                    var origGet = desc.get;

                    Object.defineProperty(proto, 'value', {
                        get: function() {
                            return origGet.call(this);
                        },
                        set: function(val) {
                            if (this === document.activeElement) {
                                // Do not interrupt active IME composition unless explicitly clearing to empty string
                                if (val !== "" && (isComposing || this.__isComposing)) {
                                    return;
                                }
                                // Avoid redundant property sets that reset Safari/WebKit selection and IME session
                                if (origGet.call(this) === val) {
                                    return;
                                }
                            }
                            origSet.call(this, val);
                        },
                        configurable: true,
                        enumerable: true
                    });
                }

                patchProto(HTMLInputElement.prototype);
                patchProto(HTMLTextAreaElement.prototype);
            }
        "#);
    });

    let signed_in = app.session.read().is_some();
    let booted = *app.booted.read();

    rsx! {
        document::Stylesheet { href: MAIN_CSS }
        document::Link { rel: "icon", r#type: "image/png", href: FAVICON }
        document::Meta { name: "viewport", content: "width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no, viewport-fit=cover" }
        div { class: if !signed_in && booted { "app app-login" } else { "app" },
            if !booted {
                div { class: "splash", div { class: "spinner" } }
            } else if !signed_in {
                screens::Login {}
            } else {
                Header {}
                main { class: "content",
                    {screen_view(app.current())}
                }
                AdBanner {}
                if *app.menu_open.read() {
                    SideMenu {}
                }
            }
            if let Some(msg) = app.toast.read().clone() {
                div { class: "toast", "{msg}" }
            }
        }
    }
}

fn screen_view(s: Screen) -> Element {
    match s {
        Screen::Home => rsx! { screens::Home {} },
        Screen::ArtistDetail { artist_id, category_id } => rsx! { screens::ArtistDetail { artist_id, category_id } },
        Screen::CategoryOffers { category_id, artist_id } => rsx! { screens::CategoryOffers { category_id, artist_id } },
        Screen::Trade { partner_uid, category_id, artist_id } => rsx! { screens::Trade { partner_uid, category_id, artist_id } },
        Screen::Search => rsx! { screens::Search {} },
        Screen::Profile => rsx! { screens::ProfileScreen {} },
        Screen::Trades => rsx! { screens::TradesScreen {} },
        Screen::Holdings => rsx! { screens::Holdings {} },
        Screen::Create { initial_kind, initial_artist_id } => rsx! { screens::Create { initial_kind, initial_artist_id } },
        Screen::EditCard { card_id } => rsx! { screens::EditCard { card_id } },
        Screen::EditArtist { artist_id } => rsx! { screens::EditArtist { artist_id } },
        Screen::EditCategory { category_id } => rsx! { screens::EditCategory { category_id } },
        Screen::EditSeries { series_id } => rsx! { screens::EditSeries { series_id } },
        Screen::Settings => rsx! { screens::Settings {} },
    }
}

pub async fn load_user_data(mut app: AppState, uid: String) {
    let tok = match app.token().await {
        Ok(t) => t,
        Err(e) => return app.show_error(&e),
    };
    let (p, c, h) = futures_util::join!(
        api::load_profile(&tok, &uid),
        api::load_catalog(&tok),
        api::load_holdings(&tok, &uid),
    );
    match p {
        Ok(Some(p)) => app.profile.set(p),
        Ok(None) => {
            // First sign-in: create the profile document (sign-up).
            let p = Profile { id: uid.clone(), nickname: format!("user{}", &uid[..uid.len().min(5)]), ..Default::default() };
            let _ = api::save_profile(&tok, &uid, &p).await;
            app.profile.set(p);
        }
        Err(e) => app.show_error(&e),
    }
    match c {
        Ok(c) => app.catalog.set(c),
        Err(e) => app.show_error(&e),
    }
    match h {
        Ok(h) => {
            let global_counts = api::load_global_card_counts(&tok, &h).await;
            app.global_card_counts.set(global_counts);
            app.holdings.set(h);
        }
        Err(e) => app.show_error(&e),
    }
}

pub async fn reload_catalog(mut app: AppState) {
    if let Ok(tok) = app.token().await {
        match api::load_catalog(&tok).await {
            Ok(c) => app.catalog.set(c),
            Err(e) => app.show_error(&e),
        }
    }
}

pub async fn reload_holdings(mut app: AppState) {
    let uid = app.uid();
    if let Ok(tok) = app.token().await {
        match api::load_holdings(&tok, &uid).await {
            Ok(h) => app.holdings.set(h),
            Err(e) => app.show_error(&e),
        }
    }
}
