use std::collections::HashSet;

use dioxus::prelude::*;

use crate::api;
use crate::app::reload_holdings;
use crate::components::{ArtistAvatar, CardThumb};
use crate::models::Card;
use crate::state::*;

/// Keyword search over the shared catalogue with All/Owned scope and
/// artist → category (upper level) filters.
#[component]
pub fn Search() -> Element {
    let mut app = use_app();
    let mut keyword = use_signal(String::new);
    let mut owned_only = use_signal(|| false);
    let mut artist = use_signal(String::new);
    let mut category = use_signal(String::new);

    let catalog = app.catalog.read().clone();
    let owned: HashSet<String> = app.holdings.read().iter().map(|h| h.card_id.clone()).collect();

    let kw = keyword().trim().to_lowercase();
    let results: Vec<Card> = catalog
        .cards
        .iter()
        .filter(|c| !owned_only() || owned.contains(&c.id))
        .filter(|c| artist().is_empty() || c.artist_id == artist())
        .filter(|c| category().is_empty() || c.category_id == category())
        .filter(|c| {
            kw.is_empty()
                || c.name_lower.contains(&kw)
                || catalog.series_name(&c.series_id).to_lowercase().contains(&kw)
                || catalog.category_name(&c.category_id).to_lowercase().contains(&kw)
                || catalog.artist_name(&c.artist_id).to_lowercase().contains(&kw)
        })
        .take(200)
        .cloned()
        .collect();

    let add = move |card: Card| {
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

    rsx! {
        div { class: "page",
            input {
                class: "input search-input",
                r#type: "search",
                autofocus: true,
                placeholder: app.tr("search_placeholder"),
                initial_value: "{keyword}",
                autocorrect: "off",
                autocapitalize: "off",
                spellcheck: "false",
                oninput: move |e| keyword.set(e.value()),
            }
            div { class: "filters",
                div { class: "segmented",
                    button { class: if !owned_only() { "on" } else { "" }, onclick: move |_| owned_only.set(false), {app.tr("scope_all")} }
                    button { class: if owned_only() { "on" } else { "" }, onclick: move |_| owned_only.set(true), {app.tr("scope_owned")} }
                }
                select { class: "input", value: "{artist}",
                    onchange: move |e| { artist.set(e.value()); category.set(String::new()); },
                    option { value: "", {app.tr("all_artists")} }
                    for a in catalog.artists.iter() { option { key: "{a.id}", value: "{a.id}", selected: a.id == artist(), "{a.name}" } }
                }
                select { class: "input", value: "{category}",
                    onchange: move |e| category.set(e.value()),
                    option { value: "", {app.tr("all_categories")} }
                    for c in catalog.categories.iter().filter(|c| artist().is_empty() || c.artist_id.is_empty() || c.artist_id == artist()) {
                        option { key: "{c.id}", value: "{c.id}", selected: c.id == category(),
                            "{c.name}",
                            if c.artist_id.is_empty() { " ({app.tr(\"common\")})" }
                        }
                    }
                }
            }
            if results.is_empty() {
                p { class: "muted center", {app.tr("no_results")} }
            }
            div { class: "card-grid",
                for c in results {
                    {
                        let is_owned = owned.contains(&c.id);
                        let a_opt = catalog.artist(&c.artist_id);
                        let a_name = catalog.artist_name(&c.artist_id);
                        let a_img = a_opt.map(|a| a.image_url.clone()).unwrap_or_default();
                        let a_color = a_opt.map(|a| a.color.clone()).unwrap_or_default();
                        let crumb = format!("{} · {}", catalog.category_name(&c.category_id), catalog.series_name(&c.series_id));
                        let c2 = c.clone();
                        let edit_id = c.id.clone();
                        let rarity_count = app.global_card_counts.read().get(&c.id).copied().unwrap_or(0);
                        rsx! {
                            div { class: "card-box", key: "{c.id}",
                                CardThumb {
                                    card: c.clone(),
                                    badge: if is_owned { Some(app.tr("scope_owned").to_string()) } else { None },
                                    rarity: Some(rarity_count),
                                }
                                div { class: "muted small crumb search-crumb-row",
                                    ArtistAvatar {
                                        url: a_img,
                                        color: a_color,
                                        name: a_name.clone(),
                                        size: 16,
                                    }
                                    span { class: "crumb-artist-name", "{a_name}" }
                                    span { "· {crumb}" }
                                }
                                if *app.edit_mode.read() {
                                    div { class: "card-box-actions",
                                        div { class: "action-left",
                                            button {
                                                class: if is_owned { "btn-icon-subtle active" } else { "btn-icon-subtle" },
                                                title: app.tr("add_to_holdings"),
                                                aria_label: app.tr("add_to_holdings"),
                                                onclick: move |_| add(c2.clone()),
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
                                                    let eid = edit_id.clone();
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
