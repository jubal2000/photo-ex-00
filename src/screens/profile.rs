use dioxus::prelude::*;

use crate::api;
use crate::components::{Avatar, CropShape, ImageCropperModal, UploadProgressOverlay, read_image_data_url};
use crate::firebase::storage;
use crate::state::*;

/// Profile: photo, nickname, greeting, favourite artists (multiple) + trade requests.
#[component]
pub fn ProfileScreen() -> Element {
    let mut app = use_app();
    let init = app.profile.read().clone();
    let mut nickname = use_signal(|| init.nickname.clone());
    let mut greeting = use_signal(|| init.greeting.clone());
    let mut photo = use_signal(|| init.photo_url.clone());
    let mut favs = use_signal(|| init.favorite_artists.clone());
    let mut locations = use_signal(|| init.trade_locations.clone());
    let mut new_location_input = use_signal(String::new);
    let mut crop_img = use_signal(|| None::<String>);
    let mut upload_progress = use_signal(|| None::<u8>);

    let catalog = app.catalog.read().clone();

    let on_photo = move |e: Event<FormData>| {
        spawn(async move {
            if let Some(data_url) = read_image_data_url(&e, "profile-photo-file-input").await {
                crop_img.set(Some(data_url));
            }
        });
    };

    let on_cropped = move |cropped_bytes: Vec<u8>| {
        crop_img.set(None);
        upload_progress.set(Some(30));
        spawn(async move {
            let uid = app.uid();
            let Ok(tok) = app.token().await else {
                upload_progress.set(None);
                return;
            };
            let path = format!("users/{uid}/avatar_{}", crate::platform::now_secs());
            upload_progress.set(Some(70));
            match storage::upload(&tok, &path, "image/jpeg", cropped_bytes).await {
                Ok(url) => {
                    upload_progress.set(Some(100));
                    photo.set(url);
                }
                Err(e) => app.show_error(&e),
            }
            upload_progress.set(None);
        });
    };

    let save = move |_| {
        spawn(async move {
            let uid = app.uid();
            let mut p = app.profile.read().clone();
            p.nickname = nickname().trim().to_string();
            p.greeting = greeting().trim().to_string();
            p.photo_url = photo();
            p.favorite_artists = favs();
            p.trade_locations = locations();
            let Ok(tok) = app.token().await else { return };
            match api::save_profile(&tok, &uid, &p).await {
                Ok(_) => {
                    app.profile.set(p);
                    app.show_toast(app.tr("saved"));
                }
                Err(e) => app.show_error(&e),
            }
        });
    };

    rsx! {
        div { class: "page",
            div { class: "profile-photo",
                Avatar { url: photo(), size: 96 }
                button {
                    class: "btn btn-sm btn-outline",
                    r#type: "button",
                    onclick: move |_| {
                        spawn(async move {
                            if let Some(data_url) = crate::platform::image_picker::pick_image().await {
                                crop_img.set(Some(data_url));
                            }
                        });
                    },
                    if upload_progress().is_some() { {app.tr("loading")} } else { {app.tr("profile_photo")} }
                }
                input { id: "profile-photo-file-input", r#type: "file", accept: "image/*", hidden: true, onchange: on_photo }
            }
            label { class: "field", span { {app.tr("nickname")} }
                input { class: "input", value: "{nickname}", maxlength: 20, oninput: move |e| nickname.set(e.value()) }
            }
            label { class: "field", span { {app.tr("greeting")} }
                textarea { class: "input", rows: 2, maxlength: 100, value: "{greeting}", oninput: move |e| greeting.set(e.value()) }
            }
            div { class: "field", span { {app.tr("fav_artists")} }
                div { class: "chips",
                    for a in catalog.artists.iter() {
                        {
                            let id = a.id.clone();
                            let on = favs.read().contains(&id);
                            rsx! {
                                button { key: "{a.id}", class: if on { "chip on" } else { "chip" },
                                    onclick: move |_| { let mut f = favs.write(); if let Some(i) = f.iter().position(|x| *x == id) { f.remove(i); } else { f.push(id.clone()); } },
                                    "{a.name}"
                                }
                            }
                        }
                    }
                }
            }
            div { class: "field",
                div { class: "field-header-row",
                    span { {app.tr("trade_locations")} }
                    span { class: "field-hint muted small", {app.tr("trade_locations_hint")} }
                }
                div { class: "location-input-row",
                    input {
                        class: "input location-input",
                        value: "{new_location_input}",
                        placeholder: app.tr("location_placeholder"),
                        maxlength: 30,
                        disabled: locations.read().len() >= 5,
                        oninput: move |e| new_location_input.set(e.value()),
                        onkeydown: {
                            let mut app = app;
                            move |e: KeyboardEvent| {
                                if e.key() == Key::Enter {
                                    let text = new_location_input().trim().to_string();
                                    if text.is_empty() { return; }
                                    if locations.read().len() >= 5 {
                                        app.show_toast(app.tr("max_locations_reached"));
                                        return;
                                    }
                                    if !locations.read().contains(&text) {
                                        locations.write().push(text);
                                    }
                                    new_location_input.set(String::new());
                                }
                            }
                        },
                    }
                    button {
                        class: "btn btn-outline btn-sm",
                        r#type: "button",
                        disabled: locations.read().len() >= 5 || new_location_input().trim().is_empty(),
                        onclick: {
                            let mut app = app;
                            move |_| {
                                let text = new_location_input().trim().to_string();
                                if text.is_empty() { return; }
                                if locations.read().len() >= 5 {
                                    app.show_toast(app.tr("max_locations_reached"));
                                    return;
                                }
                                if !locations.read().contains(&text) {
                                    locations.write().push(text);
                                }
                                new_location_input.set(String::new());
                            }
                        },
                        {app.tr("add_location")}
                    }
                }
                if locations.read().is_empty() {
                    div { class: "location-empty-notice muted small",
                        {app.tr("trade_locations_all")}
                    }
                } else {
                    div { class: "chips location-chips",
                        for (idx, loc) in locations.read().iter().enumerate() {
                            {
                                let loc_c = loc.clone();
                                rsx! {
                                    div { key: "{idx}-{loc}", class: "location-chip",
                                        span { class: "location-name", "{loc_c}" }
                                        button {
                                            class: "location-del-btn",
                                            r#type: "button",
                                            title: app.tr("cancel"),
                                            onclick: move |_| {
                                                locations.write().retain(|x| x != &loc_c);
                                            },
                                            "×"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            button { class: "btn btn-primary block", onclick: save, {app.tr("save")} }

            if let Some(src) = crop_img() {
                ImageCropperModal {
                    image_src: src,
                    shape: CropShape::Circle,
                    on_cancel: move |_| crop_img.set(None),
                    on_confirm: on_cropped,
                }
            }
            if let Some(p) = upload_progress() {
                UploadProgressOverlay { progress_percent: Some(p) }
            }
        }
    }
}
