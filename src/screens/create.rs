use dioxus::prelude::*;

use crate::api::{self, CreateError};
use crate::app::reload_catalog;
use crate::components::{ArtistAvatar, CropShape, ImageCropperModal, UploadProgressOverlay, read_image_data_url};
use crate::firebase::storage;
use crate::models::Catalog;
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

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Artist,
    Category,
    Series,
    Card,
}

fn is_auto_card_name(name: &str, prefix: &str) -> bool {
    let t = name.trim();
    let prefix = if prefix.is_empty() { "카드" } else { prefix };
    if let Some(rest) = t.strip_prefix(prefix) {
        let rest_trimmed = rest.trim();
        !rest_trimmed.is_empty() && rest_trimmed.chars().all(|c| c.is_ascii_digit())
    } else {
        false
    }
}

fn next_default_card_name(catalog: &Catalog, series_id: &str, prefix: &str) -> String {
    let prefix = if prefix.is_empty() { "카드" } else { prefix };
    let existing_lower: std::collections::HashSet<String> = catalog
        .cards
        .iter()
        .filter(|c| c.series_id == series_id)
        .map(|c| c.name_lower.clone())
        .collect();

    let mut n = 1;
    loop {
        let candidate = format!("{prefix} {n}");
        if !existing_lower.contains(&candidate.to_lowercase()) {
            return candidate;
        }
        n += 1;
    }
}

/// Create a new artist / category / series / card in the shared catalogue.
/// Lower levels must reference an existing parent selected from the DB.
#[component]
pub fn Create(initial_kind: Option<String>, initial_artist_id: Option<String>) -> Element {
    let mut app = use_app();
    let init_k = match initial_kind.as_deref() {
        Some("artist") => Kind::Artist,
        Some("category") => Kind::Category,
        Some("series") => Kind::Series,
        Some("card") => Kind::Card,
        _ => Kind::Card, // Default to Card so rectangular photo card cropper appears by default!
    };
    let init_aid = initial_artist_id.unwrap_or_default();
    let mut kind = use_signal(move || init_k);
    let mut name = use_signal(String::new);
    let mut artist = use_signal(move || init_aid);
    let mut category = use_signal(String::new);
    let mut series = use_signal(String::new);
    let mut image_url = use_signal(String::new);
    let mut artist_color = use_signal(|| "#7a6bff".to_string());
    let mut busy = use_signal(|| false);
    let mut crop_img = use_signal(|| None::<String>);
    let mut upload_progress = use_signal(|| None::<u8>);

    let catalog = app.catalog.read().clone();

    // Auto-populate default card name when series becomes available
    use_effect(move || {
        if kind() == Kind::Card {
            let sid = series();
            if !sid.is_empty() {
                let cur = name();
                if cur.trim().is_empty() {
                    let cat = app.catalog.read().clone();
                    let def_n = next_default_card_name(&cat, &sid, app.tr("card_default_prefix"));
                    name.set(def_n);
                }
            }
        }
    });

    let on_image = move |e: Event<FormData>| {
        spawn(async move {
            let input_id = if kind() == Kind::Artist {
                "create-artist-image-input"
            } else {
                "create-item-image-input"
            };
            if let Some(data_url) = read_image_data_url(&e, input_id).await {
                crop_img.set(Some(data_url));
            }
        });
    };

    let on_cropped = move |cropped_bytes: Vec<u8>| {
        crop_img.set(None);
        upload_progress.set(Some(30));
        let k = kind();
        let folder = match k {
            Kind::Artist => "artists",
            Kind::Category => "categories",
            Kind::Series => "series",
            Kind::Card => "cards",
        };
        spawn(async move {
            let Ok(tok) = app.token().await else {
                upload_progress.set(None);
                return;
            };
            let path = format!("{folder}/{}_{}", crate::platform::now_secs(), uuid::Uuid::new_v4().simple());
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

    // Realtime duplicate name checking
    let cur_name = name().trim().to_string();
    let norm_name = cur_name.to_lowercase();
    let is_duplicate = if norm_name.is_empty() {
        false
    } else {
        match kind() {
            Kind::Artist => {
                catalog.artists.iter().any(|a| a.name_lower == norm_name)
            }
            Kind::Category => {
                let aid = artist();
                catalog.categories.iter().any(|c| {
                    (c.artist_id == aid || c.artist_id.is_empty()) && c.name_lower == norm_name
                })
            }
            Kind::Series => {
                let cat_id = category();
                catalog.series.iter().any(|s| {
                    s.category_id == cat_id && s.name_lower == norm_name
                })
            }
            Kind::Card => {
                let sid = series();
                if sid.is_empty() {
                    false
                } else {
                    catalog.cards.iter().any(|c| {
                        c.series_id == sid && c.name_lower == norm_name
                    })
                }
            }
        }
    };

    let submit = move |_| {
        let k = kind();
        let n = name().trim().to_string();
        let parent_missing = match k {
            Kind::Artist => false,
            Kind::Category => false,
            Kind::Series => artist().is_empty() || category().is_empty(),
            Kind::Card => series().is_empty(),
        };
        if n.is_empty() || parent_missing || is_duplicate {
            app.show_toast(app.tr("required"));
            return;
        }
        let cur_color = artist_color();
        let cur_series = series();
        spawn(async move {
            busy.set(true);
            let uid = app.uid();
            let cat = app.catalog.read().clone();
            let res = match app.token().await {
                Err(e) => Err(CreateError::Fb(e)),
                Ok(tok) => match k {
                    Kind::Artist => api::create_artist(&tok, &uid, &cat, &n, &image_url(), &cur_color).await,
                    Kind::Category => api::create_category(&tok, &uid, &cat, &artist(), &n, &image_url()).await,
                    Kind::Series => api::create_series(&tok, &uid, &cat, &artist(), &category(), &n, &image_url()).await,
                    Kind::Card => api::create_card(&tok, &uid, &cat, &cur_series, &n, &image_url()).await,
                },
            };
            match res {
                Ok(_) => {
                    reload_catalog(app).await;
                    let refreshed_cat = app.catalog.read().clone();
                    if k == Kind::Card && !cur_series.is_empty() {
                        let next_name = next_default_card_name(&refreshed_cat, &cur_series, app.tr("card_default_prefix"));
                        name.set(next_name);
                    } else {
                        name.set(String::new());
                    }
                    image_url.set(String::new());
                    app.show_toast(app.tr("created"));
                }
                Err(CreateError::Duplicate) => app.show_toast(app.tr("duplicate")),
                Err(CreateError::Fb(e)) => app.show_error(&e),
            }
            busy.set(false);
        });
    };

    let k = kind();
    let tab = |kk: Kind, label: &'static str| {
        rsx! {
            button {
                class: if kind() == kk { "on" } else { "" },
                onclick: move |_| {
                    let prev_k = kind();
                    kind.set(kk);
                    image_url.set(String::new());
                    if kk == Kind::Card {
                        let sid = series();
                        if !sid.is_empty() {
                            let cur = name();
                            if cur.trim().is_empty() {
                                let cat = app.catalog.read();
                                let def_n = next_default_card_name(&cat, &sid, app.tr("card_default_prefix"));
                                name.set(def_n);
                            }
                        }
                    } else if prev_k == Kind::Card && is_auto_card_name(&name(), app.tr("card_default_prefix")) {
                        name.set(String::new());
                    }
                },
                {app.tr(label)}
            }
        }
    };

    rsx! {
        div { class: "page",
            div { class: "segmented four",
                {tab(Kind::Artist, "create_artist")}
                {tab(Kind::Category, "create_category")}
                {tab(Kind::Series, "create_series")}
                {tab(Kind::Card, "create_card")}
            }

            if k != Kind::Artist {
                label { class: "field",
                    span {
                        {app.tr("artist")}
                        if k == Kind::Category {
                            span { class: "muted small", " ({app.tr(\"all_artists_common\")})" }
                        } else {
                            " *"
                        }
                    }
                    select { class: "input", value: "{artist}",
                        onchange: move |e| {
                            artist.set(e.value());
                            category.set(String::new());
                            series.set(String::new());
                            if k == Kind::Card && is_auto_card_name(&name(), app.tr("card_default_prefix")) {
                                name.set(String::new());
                            }
                        },
                        option { value: "", {if k == Kind::Category { app.tr("all_artists_common") } else { app.tr("select")}} }
                        for a in catalog.artists.iter() { option { key: "{a.id}", value: "{a.id}", selected: a.id == artist(), "{a.name}" } }
                    }
                }
            }
            if k == Kind::Series || k == Kind::Card {
                label { class: "field", span { {app.tr("category")} " *" }
                    select { class: "input", value: "{category}", disabled: artist().is_empty(),
                        onchange: move |e| {
                            category.set(e.value());
                            series.set(String::new());
                            if k == Kind::Card && is_auto_card_name(&name(), app.tr("card_default_prefix")) {
                                name.set(String::new());
                            }
                        },
                        option { value: "", {app.tr("select")} }
                        for c in catalog.categories.iter().filter(|c| c.artist_id.is_empty() || c.artist_id == artist()) {
                            option { key: "{c.id}", value: "{c.id}", selected: c.id == category(),
                                "{c.name}",
                                if c.artist_id.is_empty() { " ({app.tr(\"common\")})" }
                            }
                        }
                    }
                }
            }
            if k == Kind::Card {
                label { class: "field", span { {app.tr("series")} " *" }
                    select { class: "input", value: "{series}", disabled: category().is_empty(),
                        onchange: move |e| {
                            let new_sid = e.value();
                            series.set(new_sid.clone());
                            if !new_sid.is_empty() {
                                let cat = app.catalog.read();
                                let default_n = next_default_card_name(&cat, &new_sid, app.tr("card_default_prefix"));
                                let cur = name();
                                if cur.trim().is_empty() || is_auto_card_name(&cur, app.tr("card_default_prefix")) {
                                    name.set(default_n);
                                }
                            } else if is_auto_card_name(&name(), app.tr("card_default_prefix")) {
                                name.set(String::new());
                            }
                        },
                        option { value: "", {app.tr("select")} }
                        for s in catalog.series.iter().filter(|s| s.category_id == category() && (artist().is_empty() || s.artist_id == artist())) {
                            option { key: "{s.id}", value: "{s.id}", selected: s.id == series(), "{s.name}" }
                        }
                    }
                }
            }

            label { class: "field",
                div { class: "field-header",
                    span { {app.tr("name")} " *" }
                    if is_duplicate {
                        span { class: "field-error-msg", {app.tr("duplicate_name_warning")} }
                    }
                }
                input {
                    class: if is_duplicate { "input input-error" } else { "input" },
                    value: "{name}",
                    maxlength: 60,
                    placeholder: if k == Kind::Card { "카드 1" } else { "" },
                    oninput: move |e| name.set(e.value())
                }
            }

            if k == Kind::Artist {
                // Color Picker for Artist (primary color fallback)
                div { class: "field", span { {app.tr("primary_color")} }
                    div { class: "color-picker-row",
                        input {
                            r#type: "color",
                            class: "color-input",
                            value: "{artist_color}",
                            oninput: move |e| artist_color.set(e.value())
                        }
                        div { class: "color-presets",
                            for c in PRESET_COLORS {
                                {
                                    let color_hex = (*c).to_string();
                                    let is_active = artist_color().to_lowercase() == color_hex.to_lowercase();
                                    let c_clone = color_hex.clone();
                                    rsx! {
                                        button {
                                            key: "{color_hex}",
                                            class: if is_active { "color-dot active" } else { "color-dot" },
                                            style: "background-color: {color_hex};",
                                            onclick: move |_| artist_color.set(c_clone.clone()),
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                div { class: "field", span { {app.tr("representative_image")} }
                    div { class: "image-pick-circle",
                        ArtistAvatar {
                            url: image_url(),
                            color: artist_color(),
                            name: name(),
                            size: 56,
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
                            input { id: "create-artist-image-input", r#type: "file", accept: "image/*", onchange: on_image }
                        }
                    }
                    input { class: "input", placeholder: app.tr("image_url"), value: "{image_url}", oninput: move |e| image_url.set(e.value()) }
                }
            } else {
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
                            input { id: "create-item-image-input", r#type: "file", accept: "image/*", onchange: on_image }
                        }
                    }
                    input { class: "input", placeholder: app.tr("image_url"), value: "{image_url}", oninput: move |e| image_url.set(e.value()) }
                }
            }

            button { class: "btn btn-primary block", disabled: busy() || upload_progress().is_some() || is_duplicate, onclick: submit, {app.tr("create_btn")} }

            if let Some(src) = crop_img() {
                {
                    let shape = match kind() {
                        Kind::Card => CropShape::CardRect,
                        Kind::Artist => CropShape::Circle,
                        Kind::Category | Kind::Series => CropShape::Square,
                    };
                    rsx! {
                        ImageCropperModal {
                            image_src: src,
                            shape,
                            on_cancel: move |_| crop_img.set(None),
                            on_confirm: on_cropped,
                        }
                    }
                }
            }
            if let Some(p) = upload_progress() {
                UploadProgressOverlay { progress_percent: Some(p) }
            }
        }
    }
}
