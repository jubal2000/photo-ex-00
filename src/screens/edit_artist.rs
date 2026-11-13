use dioxus::prelude::*;

use crate::api;
use crate::app::reload_catalog;
use crate::components::{ArtistAvatar, CropShape, ImageCropperModal, UploadProgressOverlay, read_image_data_url};
use crate::firebase::storage;
use crate::state::*;

const PRESET_COLORS: &[&str] = &[
    "#7a6bff",
    "#ff7ab6",
    "#4a90e2",
    "#50e3c2",
    "#f5a623",
    "#ff495c",
    "#8e44ad",
    "#2ecc71",
];

/// Edit an existing artist: name, representative image, primary color, and description.
#[component]
pub fn EditArtist(artist_id: String) -> Element {
    let mut app = use_app();
    let catalog = app.catalog.read().clone();
    let maybe_artist = catalog.artist(&artist_id).cloned();

    let Some(orig) = maybe_artist else {
        return rsx! {
            div { class: "page",
                p { class: "muted center", {app.tr("error")} }
                button { class: "btn btn-outline", onclick: move |_| app.back(), {app.tr("back")} }
            }
        };
    };

    let mut name = use_signal(|| orig.name.clone());
    let mut image_url = use_signal(|| orig.image_url.clone());
    let mut color = use_signal(|| if orig.color.is_empty() { "#7a6bff".to_string() } else { orig.color.clone() });
    let mut description = use_signal(|| orig.description.clone());
    let mut busy = use_signal(|| false);
    let mut crop_img = use_signal(|| None::<String>);
    let mut upload_progress = use_signal(|| None::<u8>);

    let on_image = move |e: Event<FormData>| {
        spawn(async move {
            if let Some(data_url) = read_image_data_url(&e, "edit-artist-image-file-input").await {
                crop_img.set(Some(data_url));
            }
        });
    };

    let on_cropped = move |cropped_bytes: Vec<u8>| {
        crop_img.set(None);
        upload_progress.set(Some(30));
        spawn(async move {
            let Ok(tok) = app.token().await else {
                upload_progress.set(None);
                return;
            };
            let path = format!("artists/{}_{}", crate::platform::now_secs(), uuid::Uuid::new_v4().simple());
            upload_progress.set(Some(70));
            match storage::upload(&tok, &path, "image/jpeg", cropped_bytes).await {
                Ok(url) => {
                    upload_progress.set(Some(100));
                    image_url.set(url);
                }
                Err(e) => app.show_error(&e),
            }
            upload_progress.set(None);
        });
    };

    let aid = artist_id.clone();
    let save = move |_| {
        let n = name().trim().to_string();
        if n.is_empty() {
            app.show_toast(app.tr("required"));
            return;
        }
        let aid = aid.clone();
        spawn(async move {
            busy.set(true);
            let res = match app.token().await {
                Ok(tok) => api::update_artist(
                    &tok,
                    &aid,
                    &name(),
                    &image_url(),
                    &color(),
                    &description(),
                ).await,
                Err(e) => Err(e),
            };
            match res {
                Ok(_) => {
                    reload_catalog(app).await;
                    app.show_toast(app.tr("saved"));
                    app.back();
                }
                Err(e) => app.show_error(&e),
            }
            busy.set(false);
        });
    };

    rsx! {
        div { class: "page edit-page",
            h2 { class: "page-title", {app.tr("edit_artist")} }

            // Live Preview: Circle avatar + Name + Description
            div { class: "edit-preview-card",
                div { class: "preview-badge-tag", {app.tr("preview")} }
                div { class: "artist-preview-wrap",
                    ArtistAvatar {
                        url: image_url(),
                        color: color(),
                        name: if name().trim().is_empty() { orig.name.clone() } else { name().trim().to_string() },
                        size: 72,
                    }
                    div { class: "artist-preview-info",
                        div { class: "artist-preview-name",
                            if name().trim().is_empty() { "{orig.name}" } else { "{name}" }
                        }
                        if !description().trim().is_empty() {
                            div { class: "artist-preview-desc", "{description}" }
                        }
                    }
                }
            }

            // Name
            label { class: "field", span { {app.tr("name")} " *" }
                input { class: "input", value: "{name}", maxlength: 60, oninput: move |e| name.set(e.value()) }
            }

            // Primary Color (used when image is absent)
            div { class: "field", span { {app.tr("primary_color")} }
                div { class: "color-picker-row",
                    input {
                        r#type: "color",
                        class: "color-input",
                        value: "{color}",
                        oninput: move |e| color.set(e.value())
                    }
                    div { class: "color-presets",
                        for c in PRESET_COLORS {
                            {
                                let color_hex = (*c).to_string();
                                let is_active = color().to_lowercase() == color_hex.to_lowercase();
                                let c_clone = color_hex.clone();
                                rsx! {
                                    button {
                                        key: "{color_hex}",
                                        class: if is_active { "color-dot active" } else { "color-dot" },
                                        style: "background-color: {color_hex};",
                                        onclick: move |_| color.set(c_clone.clone()),
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Representative image
            div { class: "field", span { {app.tr("representative_image")} }
                div { class: "image-pick-circle",
                    ArtistAvatar {
                        url: image_url(),
                        color: color(),
                        name: name(),
                        size: 64,
                    }
                    if cfg!(target_os = "ios") {
                        button {
                            class: "btn-file-select",
                            r#type: "button",
                            onclick: move |_| {
                                spawn(async move {
                                    if let Some(data_url) = crate::platform::image_picker::pick_image().await {
                                        crop_img.set(Some(data_url));
                                    }
                                });
                            },
                            {app.tr("choose_file")}
                        }
                    } else {
                        input { id: "edit-artist-image-file-input", r#type: "file", accept: "image/*", onchange: on_image }
                    }
                }
                input { class: "input", placeholder: app.tr("image_url"), value: "{image_url}", oninput: move |e| image_url.set(e.value()) }
            }

            // Description
            label { class: "field", span { {app.tr("description")} }
                textarea {
                    class: "input",
                    rows: 3,
                    placeholder: app.tr("description_placeholder"),
                    value: "{description}",
                    oninput: move |e| description.set(e.value())
                }
            }

            // Save
            button {
                class: "btn btn-primary block save-btn",
                disabled: busy() || upload_progress().is_some(),
                onclick: save,
                {app.tr("save_changes")}
            }

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

