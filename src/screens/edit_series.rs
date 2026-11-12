use dioxus::prelude::*;

use crate::api;
use crate::app::reload_catalog;
use crate::components::{CropShape, ImageCropperModal, UploadProgressOverlay, read_image_data_url};
use crate::firebase::storage;
use crate::state::*;

/// Edit an existing series: name, representative image, and description.
#[component]
pub fn EditSeries(series_id: String) -> Element {
    let mut app = use_app();
    let catalog = app.catalog.read().clone();
    let maybe_series = catalog.series_of(&series_id).cloned();

    let Some(orig) = maybe_series else {
        return rsx! {
            div { class: "page",
                p { class: "muted center", {app.tr("error")} }
                button { class: "btn btn-outline", onclick: move |_| app.back(), {app.tr("back")} }
            }
        };
    };

    let mut name = use_signal(|| orig.name.clone());
    let mut image_url = use_signal(|| orig.image_url.clone());
    let mut description = use_signal(|| orig.description.clone());
    let mut busy = use_signal(|| false);
    let mut crop_img = use_signal(|| None::<String>);
    let mut upload_progress = use_signal(|| None::<u8>);

    let on_image = move |e: Event<FormData>| {
        spawn(async move {
            if let Some(data_url) = read_image_data_url(&e, "edit-series-image-file-input").await {
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
            let path = format!("series/{}_{}", crate::platform::now_secs(), uuid::Uuid::new_v4().simple());
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

    let sid = series_id.clone();
    let save = move |_| {
        let n = name().trim().to_string();
        if n.is_empty() {
            app.show_toast(app.tr("required"));
            return;
        }
        let sid = sid.clone();
        spawn(async move {
            busy.set(true);
            let res = match app.token().await {
                Ok(tok) => api::update_series(
                    &tok,
                    &sid,
                    &name(),
                    &image_url(),
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

    let parent_info = format!(
        "{} / {}",
        catalog.artist_name(&orig.artist_id),
        catalog.category_name(&orig.category_id)
    );

    rsx! {
        div { class: "page edit-page",
            h2 { class: "page-title", {app.tr("edit_series")} }

            // Series Preview
            div { class: "edit-preview-card",
                div { class: "preview-badge-tag", {app.tr("preview")} }
                div { class: "series-preview-wrap",
                    if !image_url().trim().is_empty() {
                        img { class: "series-preview-img", src: "{image_url}", alt: "{name}" }
                    }
                    div { class: "series-preview-body",
                        div { class: "series-preview-parent muted small", "{parent_info}" }
                        div { class: "series-preview-name",
                            if name().trim().is_empty() { "{orig.name}" } else { "{name}" }
                        }
                        if !description().trim().is_empty() {
                            div { class: "series-preview-desc", "{description}" }
                        }
                    }
                }
            }

            // Name
            label { class: "field", span { {app.tr("name")} " *" }
                input { class: "input", value: "{name}", maxlength: 60, oninput: move |e| name.set(e.value()) }
            }

            // Representative image
            div { class: "field", span { {app.tr("representative_image")} }
                div { class: "image-pick",
                    if !image_url().is_empty() {
                        img { class: "preview", src: "{image_url}", alt: "" }
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
                        input { id: "edit-series-image-file-input", r#type: "file", accept: "image/*", onchange: on_image }
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
                "💾 " {app.tr("save_changes")}
            }

            if let Some(src) = crop_img() {
                ImageCropperModal {
                    image_src: src,
                    shape: CropShape::Square,
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

