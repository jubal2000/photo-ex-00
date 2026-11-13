use std::collections::{BTreeMap, HashMap, HashSet};

use dioxus::prelude::*;

use crate::api;
use crate::app::reload_holdings;
use crate::components::CardThumb;
use crate::models::Card;
use crate::state::*;

/// Artist detail screen: shows categories and series as groups,
/// and cards in a responsive grid list.
#[component]
pub fn ArtistDetail(artist_id: String, category_id: Option<String>) -> Element {
    let mut app = use_app();
    let mut selected_cat = use_signal(|| category_id.clone());

    let catalog = app.catalog.read().clone();
    let maybe_artist = catalog.artist(&artist_id);
    let artist_desc = maybe_artist.map(|a| a.description.clone()).unwrap_or_default();

    let my_holdings: HashMap<String, u32> = app
        .holdings
        .read()
        .iter()
        .map(|h| (h.card_id.clone(), h.count as u32))
        .collect();

    // All cards belonging to this artist
    let all_artist_cards: Vec<Card> = catalog
        .cards
        .iter()
        .filter(|c| c.artist_id == artist_id)
        .cloned()
        .collect();

    // Distinct categories available for this artist
    let mut available_categories: Vec<(String, String)> = Vec::new();
    let mut seen_cats = HashSet::new();
    for c in &all_artist_cards {
        if seen_cats.insert(c.category_id.clone()) {
            available_categories.push((
                catalog.category_name(&c.category_id),
                c.category_id.clone(),
            ));
        }
    }
    for cat in catalog.categories.iter() {
        if (cat.artist_id == artist_id || cat.artist_id.is_empty()) && seen_cats.insert(cat.id.clone()) {
            available_categories.push((
                cat.name.clone(),
                cat.id.clone(),
            ));
        }
    }
    available_categories.sort_by(|a, b| a.0.cmp(&b.0));

    // Filter cards based on selected category (if any)
    let filtered_cards: Vec<Card> = if let Some(ref sel) = selected_cat() {
        all_artist_cards
            .into_iter()
            .filter(|c| &c.category_id == sel)
            .collect()
    } else {
        all_artist_cards
    };

    // Group cards: category_id -> series_id -> Vec<Card>
    // Sorted by category name and series name
    let mut tree: BTreeMap<(String, String), BTreeMap<(String, String), Vec<Card>>> = BTreeMap::new();
    for card in filtered_cards {
        let c_key = (
            catalog.category_name(&card.category_id).to_lowercase(),
            card.category_id.clone(),
        );
        let s_key = (
            catalog.series_name(&card.series_id).to_lowercase(),
            card.series_id.clone(),
        );
        tree.entry(c_key).or_default().entry(s_key).or_default().push(card);
    }

    let add_card = move |card: Card| {
        spawn(async move {
            let uid = app.uid();
            let Ok(tok) = app.token().await else { return };
            let groups = api::load_group_visibility(&tok, &uid).await.unwrap_or_default();
            match api::add_holding(&tok, &uid, &card, &groups).await {
                Ok(_) => {
                    app.show_toast(app.tr("added"));
                    reload_holdings(app).await;
                }
                Err(e) => app.show_error(&e),
            }
        });
    };

    let is_empty = tree.is_empty();

    rsx! {
        div { class: "page artist-detail",
            div { class: "artist-header",
                if !artist_desc.is_empty() {
                    div { class: "artist-header-desc muted small", "{artist_desc}" }
                }
                if !available_categories.is_empty() {
                    div { class: "chips artist-category-chips",
                        button {
                            class: if selected_cat().is_none() { "chip on" } else { "chip" },
                            onclick: move |_| selected_cat.set(None),
                            {app.tr("all")}
                        }
                        for (_, cid) in available_categories {
                            {
                                let is_on = selected_cat().as_deref() == Some(&cid);
                                let cid_clone = cid.clone();
                                let cat_name = catalog.category_name(&cid);
                                rsx! {
                                    button {
                                        key: "{cid}",
                                        class: if is_on { "chip on" } else { "chip" },
                                        onclick: move |_| selected_cat.set(Some(cid_clone.clone())),
                                        "{cat_name}"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if is_empty {
                div { class: "empty",
                    p { {app.tr("no_cards")} }
                    {
                        let aid_create = artist_id.clone();
                        rsx! {
                            button {
                                class: "btn btn-primary",
                                onclick: move |_| app.open(Screen::Create {
                                    initial_kind: Some("card".into()),
                                    initial_artist_id: Some(aid_create.clone()),
                                }),
                                {app.tr("create_card")}
                            }
                        }
                    }
                }
            } else {
                div { class: "artist-tree",
                    for ((_, cid), sers) in tree {
                        {
                            let cat_opt = catalog.category(&cid);
                            let cat_title = catalog.category_name(&cid);
                            let cat_img = cat_opt.map(|c| c.image_url.clone()).unwrap_or_default();
                            let cat_desc = cat_opt.map(|c| c.description.clone()).unwrap_or_default();
                            let cid_for_offer = cid.clone();
                            let cid_for_edit = cid.clone();
                            let aid_for_offer = artist_id.clone();
                            rsx! {
                                div { class: "group group-category", key: "{cid}",
                                    div { class: "group-head level-2",
                                        div { class: "group-head-left",
                                            if !cat_img.is_empty() {
                                                img { class: "group-detail-thumb", src: "{cat_img}", alt: "{cat_title}" }
                                            }
                                            div { class: "group-head-text",
                                                div { class: "group-title-row",
                                                    span { class: "group-title", "{cat_title}" }
                                                    if *app.edit_mode.read() {
                                                        button {
                                                            class: "btn-icon-subtle edit-title-btn",
                                                            title: app.tr("edit_category"),
                                                            aria_label: app.tr("edit_category"),
                                                            onclick: move |_| app.push(Screen::EditCategory { category_id: cid_for_edit.clone() }),
                                                            svg { width: "13", height: "13", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round",
                                                                path { d: "M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" }
                                                                path { d: "M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" }
                                                            }
                                                        }
                                                    }
                                                }
                                                if !cat_desc.is_empty() {
                                                    div { class: "group-desc muted small", "{cat_desc}" }
                                                }
                                            }
                                        }
                                        button {
                                            class: "btn btn-sm btn-ghost",
                                            onclick: move |_| app.push(Screen::CategoryOffers {
                                                category_id: cid_for_offer.clone(),
                                                artist_id: Some(aid_for_offer.clone()),
                                            }),
                                            {app.tr("home_offers")} " ›"
                                        }
                                    }
                                    for ((_, sid), cards) in sers {
                                        {
                                            let ser_opt = catalog.series_of(&sid);
                                            let ser_title = catalog.series_name(&sid);
                                            let ser_img = ser_opt.map(|s| s.image_url.clone()).unwrap_or_default();
                                            let ser_desc = ser_opt.map(|s| s.description.clone()).unwrap_or_default();
                                            let sid_for_edit = sid.clone();
                                            let count = cards.len();
                                            rsx! {
                                                div { class: "group group-series", key: "{sid}",
                                                    div { class: "group-head level-3",
                                                        div { class: "group-head-left",
                                                            if !ser_img.is_empty() {
                                                                img { class: "group-detail-thumb series-thumb", src: "{ser_img}", alt: "{ser_title}" }
                                                            }
                                                            div { class: "group-head-text",
                                                                div { class: "group-title-row",
                                                                    span { class: "group-title", "{ser_title}" }
                                                                    if *app.edit_mode.read() {
                                                                        button {
                                                                            class: "btn-icon-subtle edit-title-btn",
                                                                            title: app.tr("edit_series"),
                                                                            aria_label: app.tr("edit_series"),
                                                                            onclick: move |_| app.push(Screen::EditSeries { series_id: sid_for_edit.clone() }),
                                                                            svg { width: "12", height: "12", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round",
                                                                                path { d: "M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" }
                                                                                path { d: "M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                                if !ser_desc.is_empty() {
                                                                    div { class: "group-desc muted small", "{ser_desc}" }
                                                                }
                                                            }
                                                        }
                                                        span { class: "muted small", "{count} {app.tr(\"pieces\")}" }
                                                    }
                                                    div { class: "card-grid",
                                                        for card in cards {
                                                            {
                                                                let count = my_holdings.get(&card.id).copied().unwrap_or(0);
                                                                let is_owned = count > 0;
                                                                let badge = if is_owned {
                                                                    Some(format!("{} ×{}", app.tr("count"), count))
                                                                } else {
                                                                    None
                                                                };
                                                                let c_add = card.clone();
                                                                let c_id_for_edit = card.id.clone();
                                                                let rarity_count = app.global_card_counts.read().get(&card.id).copied().unwrap_or(0);
                                                                rsx! {
                                                                    div { class: "card-box", key: "{card.id}",
                                                                        CardThumb { card: card.clone(), badge, rarity: Some(rarity_count) }
                                                                        if *app.edit_mode.read() {
                                                                            div { class: "card-box-actions",
                                                                                div { class: "action-left",
                                                                                    button {
                                                                                        class: if is_owned { "btn-icon-subtle active" } else { "btn-icon-subtle" },
                                                                                        title: app.tr("add_to_holdings"),
                                                                                        aria_label: app.tr("add_to_holdings"),
                                                                                        onclick: move |_| add_card(c_add.clone()),
                                                                                        svg { width: "16", height: "16", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2.5", stroke_linecap: "round",
                                                                                            line { x1: "12", y1: "5", x2: "12", y2: "19" }
                                                                                            line { x1: "5", y1: "12", x2: "19", y2: "12" }
                                                                                        }
                                                                                    }
                                                                                }
                                                                                div { class: "action-right",
                                                                                    button {
                                                                                        class: "btn-icon-subtle",
                                                                                        title: app.tr("edit"),
                                                                                        aria_label: app.tr("edit"),
                                                                                        onclick: {
                                                                                            let eid = c_id_for_edit.clone();
                                                                                            move |_| app.push(Screen::EditCard { card_id: eid.clone() })
                                                                                        },
                                                                                        svg { width: "15", height: "15", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round",
                                                                                            path { d: "M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" }
                                                                                            path { d: "M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" }
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
        }
    }
}
