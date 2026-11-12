use dioxus::prelude::*;

use crate::api;
use crate::app::reload_holdings;
use crate::components::{Avatar, CardThumb, CropShape, ImageCropperModal, UploadProgressOverlay, read_image_data_url};
use crate::firebase::storage;
use crate::models::{Trade, TradeItem};
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
            button { class: "btn btn-primary block", onclick: save, {app.tr("save")} }

            TradeList {}

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

/// Incoming / outgoing trade requests with accept / reject / cancel.
#[component]
fn TradeList() -> Element {
    let mut app = use_app();
    let mut trades = use_resource(move || async move {
        let uid = app.uid();
        let tok = app.token().await.ok()?;
        api::my_trades(&tok, &uid).await.ok()
    });

    let act = move |id: String, action: &'static str| {
        spawn(async move {
            let Ok(tok) = app.token().await else { return };
            let res = match action {
                "accept" => api::accept_trade(&tok, &id).await,
                "reject" => api::set_trade_status(&tok, &id, "rejected").await,
                _ => api::set_trade_status(&tok, &id, "cancelled").await,
            };
            match res {
                Ok(_) => {
                    trades.restart();
                    reload_holdings(app).await;
                }
                Err(e) => app.show_error(&e),
            }
        });
    };

    let profiles = use_resource(move || {
        let uids: Vec<String> = match &*trades.read() {
            Some(Some((i, o))) => i.iter().map(|t| t.from_uid.clone()).chain(o.iter().map(|t| t.to_uid.clone())).collect(),
            _ => vec![],
        };
        async move {
            let tok = app.token().await.ok()?;
            Some(api::load_profiles(&tok, &uids).await)
        }
    });

    let Some(Some((inbox, outbox))) = trades.read().clone() else {
        return rsx! {};
    };
    let names = profiles.read().clone().flatten().unwrap_or_default();
    let name_of = |uid: &str| names.get(uid).map(|p| p.nickname.clone()).unwrap_or_else(|| app.tr("unknown_user").to_string());

    rsx! {
        h3 { class: "sub-title", {app.tr("trades_inbox")} }
        if inbox.is_empty() { p { class: "muted small", {app.tr("trades_none")} } }
        for t in inbox {
            TradeRow { key: "{t.id}", trade: t.clone(), who: name_of(&t.from_uid), incoming: true,
                on_action: move |(id, a)| act(id, a) }
        }
        h3 { class: "sub-title", {app.tr("trades_outbox")} }
        if outbox.is_empty() { p { class: "muted small", {app.tr("trades_none")} } }
        for t in outbox {
            TradeRow { key: "{t.id}", trade: t.clone(), who: name_of(&t.to_uid), incoming: false,
                on_action: move |(id, a)| act(id, a) }
        }
    }
}

#[component]
fn TradeRow(trade: Trade, who: String, incoming: bool, on_action: EventHandler<(String, &'static str)>) -> Element {
    let app = use_app();
    let catalog = app.catalog.read().clone();
    let status_key = format!("status_{}", trade.status);
    // From the viewer's perspective: what I give / what I get.
    let (give, get): (&Vec<TradeItem>, &Vec<TradeItem>) = if incoming { (&trade.requested, &trade.offered) } else { (&trade.offered, &trade.requested) };
    let id1 = trade.id.clone();
    let id2 = trade.id.clone();
    let id3 = trade.id.clone();
    rsx! {
        div { class: "trade-item",
            div { class: "trade-item-head",
                span { class: "bold", "{who}" }
                span { class: "pill status-{trade.status}", {app.tr(&status_key)} }
            }
            div { class: "trade-item-body",
                div { class: "h-scroll",
                    for it in get.iter() {
                        if let Some(c) = catalog.card(&it.card_id).cloned() {
                            CardThumb { key: "g{it.card_id}", card: c, badge: Some(format!("+{}", it.qty)), small: true }
                        }
                    }
                }
                span { class: "trade-arrow", "⇄" }
                div { class: "h-scroll",
                    for it in give.iter() {
                        if let Some(c) = catalog.card(&it.card_id).cloned() {
                            CardThumb { key: "v{it.card_id}", card: c, badge: Some(format!("−{}", it.qty)), small: true }
                        }
                    }
                }
            }
            if trade.status == "pending" {
                div { class: "row-end",
                    if incoming {
                        button { class: "btn btn-sm btn-outline", onclick: move |_| on_action.call((id1.clone(), "reject")), {app.tr("reject")} }
                        button { class: "btn btn-sm btn-primary", onclick: move |_| on_action.call((id2.clone(), "accept")), {app.tr("accept")} }
                    } else {
                        button { class: "btn btn-sm btn-outline", onclick: move |_| on_action.call((id3.clone(), "cancel")), {app.tr("cancel")} }
                    }
                }
            }
        }
    }
}
