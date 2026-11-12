use dioxus::prelude::*;

use crate::i18n::Lang;
use crate::platform::{self, biometric, storage};
use crate::state::*;

/// Settings: language, auto sign-in, biometric sign-in.
#[component]
pub fn Settings() -> Element {
    let mut app = use_app();
    let mut auto = use_signal(|| pref_bool(KEY_AUTO, true));
    let mut bio = use_signal(|| pref_bool(KEY_BIO, false));
    let bio_available = biometric::available();
    let cur = *app.lang.read();

    rsx! {
        div { class: "page",
            div { class: "field", span { {app.tr("language")} }
                div { class: "lang-list",
                    for l in Lang::ALL {
                        button { key: "{l.code()}", class: if l == cur { "lang on" } else { "lang" },
                            onclick: move |_| app.set_lang(l), "{l.native_name()}" }
                    }
                }
            }
            label { class: "setting-row",
                span { {app.tr("auto_login")} }
                input { r#type: "checkbox", checked: auto(),
                    onchange: move |e| {
                        let v = e.checked();
                        auto.set(v);
                        storage::set(KEY_AUTO, if v { "1" } else { "0" });
                        if v {
                            if let Some(s) = app.session.read().as_ref() {
                                storage::secure_set(KEY_REFRESH, &s.refresh_token);
                                storage::set(KEY_PROVIDER, &s.provider);
                            }
                        } else {
                            storage::secure_remove(KEY_REFRESH);
                        }
                    }
                }
            }
            label { class: "setting-row",
                div {
                    div { {app.tr("biometric_login")} }
                    if !bio_available { div { class: "muted small", {app.tr("biometric_unavailable")} } }
                }
                input { r#type: "checkbox", checked: bio(), disabled: !bio_available,
                    onchange: move |e| {
                        let v = e.checked();
                        spawn(async move {
                            // Confirm with biometrics before enabling.
                            if v && biometric::authenticate(app.tr("bio_reason"), app.tr("cancel")).await != biometric::BioResult::Success {
                                app.show_toast(app.tr("bio_failed"));
                                return;
                            }
                            bio.set(v);
                            storage::set(KEY_BIO, if v { "1" } else { "0" });
                        });
                    }
                }
            }
            label { class: "setting-row",
                div {
                    div { {app.tr("edit_mode")} }
                    div { class: "muted small", {app.tr("edit_mode_desc")} }
                }
                input { r#type: "checkbox", checked: *app.edit_mode.read(),
                    onchange: move |e| {
                        let v = e.checked();
                        app.edit_mode.set(v);
                        storage::set(KEY_EDIT_MODE, if v { "1" } else { "0" });
                    }
                }
            }
            label { class: "setting-row",
                div {
                    div { {app.tr("protect_single_card")} }
                    div { class: "muted small", {app.tr("protect_single_card_desc")} }
                }
                input { r#type: "checkbox", checked: *app.protect_single_card.read(),
                    onchange: move |e| {
                        let v = e.checked();
                        app.protect_single_card.set(v);
                        storage::set(KEY_PROTECT_SINGLE_CARD, if v { "1" } else { "0" });
                    }
                }
            }
            div { class: "setting-row muted",
                span { {app.tr("version")} }
                span { "{env!(\"CARGO_PKG_VERSION\")} · {platform::platform_name()}" }
            }
        }
    }
}
