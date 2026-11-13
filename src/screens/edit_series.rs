use dioxus::prelude::*;

use crate::api;
use crate::app::reload_catalog;
use crate::components::{CropShape, ImageCropperModal, UploadProgressOverlay, read_image_data_url};
use crate::firebase::storage;
use crate::models::Card;
use crate::state::*;

/// Edit an existing series: name, representative image, description, and card order.
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

    let initial_cards: Vec<Card> = catalog
        .cards
        .iter()
        .filter(|c| c.series_id == series_id)
        .cloned()
        .collect();

    let mut name = use_signal(|| orig.name.clone());
    let mut image_url = use_signal(|| orig.image_url.clone());
    let mut description = use_signal(|| orig.description.clone());
    let mut cards_list = use_signal(|| initial_cards);
    let mut dragged_idx = use_signal(|| None::<usize>);
    let mut drag_over_idx = use_signal(|| None::<usize>);
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
        let current_cards = cards_list();
        spawn(async move {
            busy.set(true);
            let res = match app.token().await {
                Ok(tok) => {
                    let r1 = api::update_series(
                        &tok,
                        &sid,
                        &name(),
                        &image_url(),
                        &description(),
                    ).await;
                    if let Err(e) = r1 {
                        Err(e)
                    } else {
                        let updates: Vec<(String, i64)> = current_cards
                            .iter()
                            .enumerate()
                            .map(|(idx, c)| (c.id.clone(), idx as i64))
                            .collect();
                        if !updates.is_empty() {
                            api::update_card_sort_orders(&tok, &updates).await
                        } else {
                            Ok(())
                        }
                    }
                }
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

            // Card Order Horizontal Scroll Section
            div { class: "series-card-order-section",
                div { class: "series-card-order-header",
                    span { class: "series-card-order-title", {app.tr("series_card_order")} }
                    span { class: "series-card-order-hint", {app.tr("series_card_order_hint")} }
                }
                if cards_list().is_empty() {
                    div { class: "series-card-order-empty", {app.tr("no_cards_in_series")} }
                } else {
                    div { class: "series-cards-hscroll-wrap",
                        div { class: "series-cards-hscroll",
                            for (idx, card) in cards_list().into_iter().enumerate() {
                                {
                                    let is_dragging = dragged_idx() == Some(idx);
                                    let is_over = drag_over_idx() == Some(idx);
                                    let total_len = cards_list().len();
                                    let cid = card.id.clone();
                                    rsx! {
                                        div {
                                            key: "{cid}",
                                            class: format!("series-card-order-item {}", if is_dragging { "is-dragging" } else if is_over { "is-drag-over" } else { "" }),
                                            draggable: "true",
                                            ondragstart: move |_| {
                                                dragged_idx.set(Some(idx));
                                            },
                                            ondragover: move |e| {
                                                e.prevent_default();
                                                if drag_over_idx() != Some(idx) {
                                                    drag_over_idx.set(Some(idx));
                                                }
                                            },
                                            ondragleave: move |_| {
                                                if drag_over_idx() == Some(idx) {
                                                    drag_over_idx.set(None);
                                                }
                                            },
                                            ondrop: move |e| {
                                                e.prevent_default();
                                                if let Some(src) = dragged_idx() {
                                                    if src != idx {
                                                        let mut list = cards_list();
                                                        let item = list.remove(src);
                                                        list.insert(idx, item);
                                                        cards_list.set(list);
                                                    }
                                                }
                                                dragged_idx.set(None);
                                                drag_over_idx.set(None);
                                            },
                                            span { class: "series-card-order-badge", "#{idx + 1}" }
                                            div { class: "series-card-order-thumb-wrap",
                                                if !card.image_url.trim().is_empty() {
                                                    img { class: "series-card-order-thumb", src: "{card.image_url}", alt: "{card.name}" }
                                                }
                                            }
                                            div { class: "series-card-order-name", title: "{card.name}", "{card.name}" }
                                            div { class: "series-card-order-controls",
                                                button {
                                                    class: "btn-order-move",
                                                    r#type: "button",
                                                    disabled: idx == 0,
                                                    onclick: move |_| {
                                                        if idx > 0 {
                                                            let mut list = cards_list();
                                                            list.swap(idx, idx - 1);
                                                            cards_list.set(list);
                                                        }
                                                    },
                                                    "‹"
                                                }
                                                button {
                                                    class: "btn-order-move",
                                                    r#type: "button",
                                                    disabled: idx + 1 >= total_len,
                                                    onclick: move |_| {
                                                        let mut list = cards_list();
                                                        if idx + 1 < list.len() {
                                                            list.swap(idx, idx + 1);
                                                            cards_list.set(list);
                                                        }
                                                    },
                                                    "›"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
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

