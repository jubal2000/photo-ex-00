use dioxus::prelude::*;

use crate::api;
use crate::app::reload_catalog;
use crate::components::{CropShape, ImageCropperModal, UploadProgressOverlay, read_image_data_url};
use crate::firebase::storage;
use crate::models::Card;
use crate::state::*;

const PRESET_COLORS: &[&str] = &[
    "#ffffff",
    "#1d1a24",
    "#ff7ab6",
    "#7a6bff",
    "#f5a623",
    "#4a90e2",
    "#50e3c2",
    "#ff495c",
];

/// Edit an existing card's details: title, font, font color, image, and description.
#[component]
pub fn EditCard(card_id: String) -> Element {
    let mut app = use_app();
    let catalog = app.catalog.read().clone();
    let maybe_card = catalog.card(&card_id).cloned();

    let Some(orig) = maybe_card else {
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
    let mut title_font = use_signal(|| orig.title_font.clone());
    let mut title_color = use_signal(|| if orig.title_color.is_empty() { "#ffffff".to_string() } else { orig.title_color.clone() });
    let mut busy = use_signal(|| false);
    let mut crop_img = use_signal(|| None::<String>);
    let mut upload_progress = use_signal(|| None::<u8>);
    let mut confirm_delete = use_signal(|| false);

    let on_image = move |e: Event<FormData>| {
        spawn(async move {
            if let Some(data_url) = read_image_data_url(&e, "edit-card-image-file-input").await {
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
            let path = format!("cards/{}_{}", crate::platform::now_secs(), uuid::Uuid::new_v4().simple());
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

    let cid = card_id.clone();
    let save = move |_| {
        let n = name().trim().to_string();
        if n.is_empty() {
            app.show_toast(app.tr("required"));
            return;
        }
        let cid = cid.clone();
        spawn(async move {
            busy.set(true);
            let res = match app.token().await {
                Ok(tok) => api::update_card(
                    &tok,
                    &cid,
                    &name(),
                    &image_url(),
                    &description(),
                    &title_font(),
                    &title_color(),
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

    let cid_del = card_id.clone();
    let delete_action = move || {
        let cid = cid_del.clone();
        spawn(async move {
            busy.set(true);
            let res = match app.token().await {
                Ok(tok) => api::delete_card(&tok, &cid).await,
                Err(e) => Err(e),
            };
            match res {
                Ok(_) => {
                    reload_catalog(app).await;
                    app.show_toast(app.tr("deleted"));
                    app.back();
                }
                Err(e) => app.show_error(&e),
            }
            busy.set(false);
        });
    };

    // Construct a temporary card instance for live preview
    let preview_card = Card {
        id: card_id.clone(),
        artist_id: orig.artist_id.clone(),
        category_id: orig.category_id.clone(),
        series_id: orig.series_id.clone(),
        name: if name().trim().is_empty() { orig.name.clone() } else { name().trim().to_string() },
        image_url: image_url().trim().to_string(),
        description: description().trim().to_string(),
        title_font: title_font().clone(),
        title_color: title_color().clone(),
        ..Default::default()
    };

    let font_css = match title_font().as_str() {
        "serif" => "Georgia, 'Nanum Myeongjo', serif",
        "cursive" => "'Brush Script MT', 'Nanum Pen Script', cursive, sans-serif",
        "monospace" => "ui-monospace, 'Courier New', monospace",
        "fantasy" => "Impact, sans-serif",
        _ => "inherit",
    };
    let color_css = title_color();
    let preview_title_style = format!("font-family:{font_css};color:{color_css};");

    rsx! {
        div { class: "page edit-card-page",
            h2 { class: "page-title", {app.tr("edit_card")} }

            // Live Preview Box
            div { class: "edit-preview-card",
                div { class: "preview-badge-tag", {app.tr("preview")} }
                div { class: "preview-card-wrap",
                    if preview_card.image_url.is_empty() {
                        div { class: "preview-img preview-img-empty" }
                    } else {
                        img { class: "preview-img", src: "{preview_card.image_url}", alt: "{preview_card.name}" }
                    }
                    div { class: "preview-card-info",
                        div { class: "preview-card-name", style: "{preview_title_style}",
                            "{preview_card.name}"
                        }
                        if !description().trim().is_empty() {
                            div { class: "preview-card-desc", "{description}" }
                        }
                    }
                }
            }

            // Name input
            label { class: "field", span { {app.tr("name")} " *" }
                input { class: "input", value: "{name}", maxlength: 60, oninput: move |e| name.set(e.value()) }
            }

            // Title Font Selection
            label { class: "field", span { {app.tr("title_font")} }
                select { class: "input", value: "{title_font}", onchange: move |e| title_font.set(e.value()),
                    option { value: "", {app.tr("font_default")} }
                    option { value: "serif", {app.tr("font_serif")} }
                    option { value: "cursive", {app.tr("font_cursive")} }
                    option { value: "monospace", {app.tr("font_monospace")} }
                    option { value: "fantasy", {app.tr("font_fantasy")} }
                }
            }

            // Title Font Color Selection
            div { class: "field", span { {app.tr("font_color")} }
                div { class: "color-picker-row",
                    input {
                        r#type: "color",
                        class: "color-input",
                        value: "{title_color}",
                        oninput: move |e| title_color.set(e.value())
                    }
                    div { class: "color-presets",
                        for c in PRESET_COLORS {
                            {
                                let color_hex = (*c).to_string();
                                let is_active = title_color().to_lowercase() == color_hex.to_lowercase();
                                let c_clone = color_hex.clone();
                                rsx! {
                                    button {
                                        key: "{color_hex}",
                                        class: if is_active { "color-dot active" } else { "color-dot" },
                                        style: "background-color: {color_hex};",
                                        onclick: move |_| title_color.set(c_clone.clone()),
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Image file upload & URL
            div { class: "field", span { {app.tr("image")} }
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
                        input { id: "edit-card-image-file-input", r#type: "file", accept: "image/*", onchange: on_image }
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

            // Save & Delete Action Row
            div { class: "edit-card-actions-row",
                button {
                    class: "btn btn-primary flex-1 save-btn",
                    disabled: busy() || upload_progress().is_some(),
                    onclick: save,
                    {app.tr("save_changes")}
                }
                button {
                    class: "btn btn-outline danger-btn delete-card-icon-btn",
                    disabled: busy() || upload_progress().is_some(),
                    title: app.tr("remove"),
                    aria_label: app.tr("remove"),
                    onclick: move |_| confirm_delete.set(true),
                    // Trash can SVG icon (No Emoji)
                    svg { width: "19", height: "19", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round",
                        polyline { points: "3 6 5 6 21 6" }
                        path { d: "M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" }
                        line { x1: "10", y1: "11", x2: "10", y2: "17" }
                        line { x1: "14", y1: "11", x2: "14", y2: "17" }
                    }
                }
            }

            // Confirm Delete Modal
            if confirm_delete() {
                div { class: "confirm-modal-overlay", onclick: move |_| confirm_delete.set(false),
                    div { class: "confirm-modal-box", onclick: move |e| e.stop_propagation(),
                        h3 { class: "confirm-modal-title", {app.tr("remove")} }
                        p { class: "confirm-modal-text", {app.tr("delete_card_confirm")} }
                        div { class: "confirm-modal-actions",
                            button {
                                class: "btn btn-outline",
                                onclick: move |_| confirm_delete.set(false),
                                {app.tr("cancel")}
                            }
                            button {
                                class: "btn btn-primary danger-btn",
                                disabled: busy(),
                                onclick: move |_| {
                                    confirm_delete.set(false);
                                    delete_action();
                                },
                                {app.tr("remove")}
                            }
                        }
                    }
                }
            }

            if let Some(src) = crop_img() {
                ImageCropperModal {
                    image_src: src,
                    shape: CropShape::CardRect,
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

