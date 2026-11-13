use std::collections::{BTreeMap, HashMap};

use dioxus::prelude::*;

use crate::api;
use crate::components::{CardThumb, Stepper};
use crate::models::Holding;
use crate::state::*;

/// My cards grouped by artist → category → series. Every group and every card has a
/// public toggle; each card has owned count and tradable count (≤ owned).
#[component]
pub fn Holdings() -> Element {
    let mut app = use_app();
    let mut groups = use_signal(HashMap::<String, bool>::new);
    let mut show_all = use_signal(|| false);
    let mut selected_artist = use_signal(|| None::<String>);

    use_future(move || async move {
        let uid = app.uid();
        if let Ok(tok) = app.token().await {
            if let Ok(g) = api::load_group_visibility(&tok, &uid).await {
                groups.set(g);
            }
        }
    });

    let catalog = app.catalog.read().clone();
    let holdings = app.holdings.read().clone();

    // Map existing holdings for fast lookup
    let holdings_map: HashMap<String, Holding> = holdings
        .iter()
        .map(|h| (h.card_id.clone(), h.clone()))
        .collect();

    // Decide items to display: owned only vs all cards (including unowned)
    let items_to_show: Vec<Holding> = if show_all() {
        catalog
            .cards
            .iter()
            .map(|c| {
                if let Some(h) = holdings_map.get(&c.id) {
                    h.clone()
                } else {
                    Holding {
                        id: c.id.clone(),
                        owner_uid: app.uid(),
                        card_id: c.id.clone(),
                        artist_id: c.artist_id.clone(),
                        category_id: c.category_id.clone(),
                        series_id: c.series_id.clone(),
                        count: 0,
                        tradable: 0,
                        public: false,
                        visible: false,
                    }
                }
            })
            .collect()
    } else {
        holdings.clone()
    };

    // Extract available artist list from items_to_show
    let mut available_artists: BTreeMap<String, String> = BTreeMap::new();
    for h in &items_to_show {
        if !h.artist_id.is_empty() {
            let name = catalog.artist_name(&h.artist_id);
            available_artists.insert(h.artist_id.clone(), name);
        }
    }

    // Filter by selected artist if set
    let filtered_items: Vec<Holding> = if let Some(ref aid) = selected_artist() {
        items_to_show.into_iter().filter(|h| &h.artist_id == aid).collect()
    } else {
        items_to_show
    };

    // artist -> category -> series -> holdings (sorted by names)
    let mut tree: BTreeMap<(String, String), BTreeMap<(String, String), BTreeMap<(String, String), Vec<Holding>>>> = BTreeMap::new();
    for h in filtered_items {
        let a = (catalog.artist_name(&h.artist_id).to_lowercase(), h.artist_id.clone());
        let c = (catalog.category_name(&h.category_id).to_lowercase(), h.category_id.clone());
        let s = (catalog.series_name(&h.series_id).to_lowercase(), h.series_id.clone());
        tree.entry(a).or_default().entry(c).or_default().entry(s).or_default().push(h);
    }

    let toggle_group = move |key: String, public: bool| {
        spawn(async move {
            let uid = app.uid();
            let Ok(tok) = app.token().await else { return };
            let mut g = groups.read().clone();
            let mut hs = app.holdings.read().clone();
            match api::set_group_visibility(&tok, &uid, &key, public, &mut g, &mut hs).await {
                Ok(_) => {
                    groups.set(g);
                    app.holdings.set(hs);
                }
                Err(e) => app.show_error(&e),
            }
        });
    };

    let g = groups.read().clone();
    let is_pub = move |k: &str| *g.get(k).unwrap_or(&true);

    rsx! {
        div { class: "page holdings",
            // Artist Filter Bar (Scrollable chips)
            if !available_artists.is_empty() {
                div { class: "artist-filter-bar chips",
                    button {
                        class: if selected_artist().is_none() { "chip on" } else { "chip" },
                        onclick: move |_| selected_artist.set(None),
                        {app.tr("all_artists")}
                    }
                    for (aid, aname) in available_artists {
                        {
                            let is_sel = selected_artist().as_ref() == Some(&aid);
                            let aid_btn = aid.clone();
                            rsx! {
                                button {
                                    key: "{aid}",
                                    class: if is_sel { "chip on" } else { "chip" },
                                    onclick: move |_| {
                                        if is_sel {
                                            selected_artist.set(None);
                                        } else {
                                            selected_artist.set(Some(aid_btn.clone()));
                                        }
                                    },
                                    "{aname}"
                                }
                            }
                        }
                    }
                }
            }

            // View Mode Filter Bar (Owned only vs Show all)
            div { class: "holdings-filter-bar chips",
                button {
                    class: if !show_all() { "chip on" } else { "chip" },
                    onclick: move |_| show_all.set(false),
                    {app.tr("holdings_only")}
                }
                button {
                    class: if show_all() { "chip on" } else { "chip" },
                    onclick: move |_| show_all.set(true),
                    {app.tr("show_all_cards")}
                }
            }

            if tree.is_empty() {
                div { class: "empty",
                    p { {app.tr("holdings_empty")} }
                    button { class: "btn btn-primary", onclick: move |_| app.open(Screen::Search), {app.tr("search")} }
                }
            } else {
                div { class: "holdings-tree",
                    for ((_, aid), cats) in tree {
                        div { class: "group group-artist", key: "{aid}",
                            GroupHeader { level: 1, title: catalog.artist_name(&aid), public: is_pub(&format!("artist_{aid}")),
                                on_toggle: { let k = format!("artist_{aid}"); move |v| toggle_group(k.clone(), v) } }
                            for ((_, cid), sers) in cats {
                                div { class: "group group-category", key: "{cid}",
                                    GroupHeader { level: 2, title: catalog.category_name(&cid), public: is_pub(&format!("category_{cid}")),
                                        on_toggle: { let k = format!("category_{cid}"); move |v| toggle_group(k.clone(), v) } }
                                    for ((_, sid), items) in sers {
                                        div { class: "group group-series", key: "{sid}",
                                            GroupHeader { level: 3, title: catalog.series_name(&sid), public: is_pub(&format!("series_{sid}")),
                                                on_toggle: { let k = format!("series_{sid}"); move |v| toggle_group(k.clone(), v) } }
                                            for h in items {
                                                HoldingRow { key: "{h.card_id}", holding: h.clone(), groups: groups.read().clone() }
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

#[component]
fn GroupHeader(level: u8, title: String, public: bool, on_toggle: EventHandler<bool>) -> Element {
    let app = use_app();
    rsx! {
        div { class: "group-head level-{level}",
            span { class: "group-title", "{title}" }
            label { class: "switch",
                span { class: "small muted", {app.tr("public")} }
                input { r#type: "checkbox", checked: public, onchange: move |e| on_toggle.call(e.checked()) }
            }
        }
    }
}

#[component]
fn HoldingRow(holding: Holding, groups: HashMap<String, bool>) -> Element {
    let mut app = use_app();
    let catalog = app.catalog.read();
    let Some(card) = catalog.card(&holding.card_id).cloned() else { return rsx! {} };

    let is_unowned = holding.count == 0;
    let mut confirm_all = use_signal(|| None::<i64>);

    let update = {
        let groups = groups.clone();
        move |mut h: Holding| {
            h.tradable = h.tradable.min(h.count).max(0);
            h.visible = api::effective_visible(&h, &groups);
            // optimistic local update
            {
                let mut all = app.holdings.write();
                if let Some(x) = all.iter_mut().find(|x| x.card_id == h.card_id) {
                    *x = h.clone();
                } else if h.count > 0 {
                    all.push(h.clone());
                }
            }
            spawn(async move {
                let uid = app.uid();
                let Ok(tok) = app.token().await else { return };
                if let Err(e) = api::save_holding(&tok, &uid, &h).await {
                    app.show_error(&e);
                }
            });
        }
    };
    let (mut u1, mut u2, mut u3, mut u_add, u_confirm) = (update.clone(), update.clone(), update.clone(), update.clone(), update);
    let (h1, h2, h3, h_add) = (holding.clone(), holding.clone(), holding.clone(), holding.clone());
    let cid = holding.card_id.clone();

    rsx! {
        div { class: if is_unowned { "holding-row unowned" } else { "holding-row" },
            CardThumb {
                card,
                small: true,
                badge: if is_unowned { Some(app.tr("unowned").to_string()) } else { None },
            }
            if is_unowned {
                div { class: "holding-controls holding-unowned-ctrl",
                    span { class: "pill-unowned small muted", {app.tr("unowned")} }
                    button {
                        class: "btn btn-outline small btn-quick-add",
                        onclick: move |_| {
                            let mut h = h_add.clone();
                            h.count = 1;
                            h.tradable = 0;
                            h.public = true;
                            u_add(h);
                        },
                        "+ " {app.tr("add_to_holdings")}
                    }
                }
            } else {
                div { class: "holding-controls",
                    label { class: "switch",
                        span { class: "small muted", {app.tr("public")} }
                        input { r#type: "checkbox", checked: holding.public,
                            onchange: move |e| { let mut h = h1.clone(); h.public = e.checked(); u1(h); } }
                    }
                    div { class: "ctrl",
                        span { class: "small muted", {app.tr("count")} }
                        Stepper { value: holding.count, min: 1, max: 999, on_change: move |v| { let mut h = h2.clone(); h.count = v; u2(h); } }
                    }
                    div { class: "ctrl",
                        span { class: "small muted",
                            if *app.protect_single_card.read() && holding.count <= 1 {
                                {app.tr("protect_single_card")}
                            } else {
                                {app.tr("tradable")}
                            }
                        }
                        Stepper {
                            value: if *app.protect_single_card.read() && holding.count <= 1 { 0 } else { holding.tradable },
                            min: 0,
                            max: if *app.protect_single_card.read() && holding.count <= 1 { 0 } else { holding.count },
                            on_change: move |v| {
                                if *app.protect_single_card.read() && holding.count <= 1 && v > 0 {
                                    app.show_toast(app.tr("single_card_protected"));
                                    return;
                                }
                                if v == holding.count && holding.count > 0 && holding.tradable < holding.count {
                                    confirm_all.set(Some(v));
                                } else {
                                    let mut h = h3.clone();
                                    h.tradable = v;
                                    u3(h);
                                }
                            },
                        }
                    }
                    button { class: "link small",
                        onclick: {
                            let edit_id = cid.clone();
                            move |_| app.push(Screen::EditCard { card_id: edit_id.clone() })
                        },
                        {app.tr("edit")}
                    }
                    button { class: "link danger small",
                        onclick: move |_| {
                            let cid = cid.clone();
                            app.holdings.write().retain(|x| x.card_id != cid);
                            spawn(async move {
                                let uid = app.uid();
                                let Ok(tok) = app.token().await else { return };
                                if let Err(e) = api::delete_holding(&tok, &uid, &cid).await { app.show_error(&e); }
                            });
                        },
                        {app.tr("remove")}
                    }
                }
            }

            if let Some(target_v) = confirm_all() {
                div { class: "confirm-modal-overlay",
                    onclick: move |_| confirm_all.set(None),
                    div { class: "confirm-modal-box",
                        onclick: move |e| e.stop_propagation(),
                        h3 { class: "confirm-modal-title", {app.tr("warn_all_tradable_title")} }
                        p { class: "confirm-modal-text",
                            {app.tr("warn_all_tradable_desc").replace("{count}", &holding.count.to_string())}
                        }
                        div { class: "confirm-modal-actions",
                            button {
                                class: "btn btn-outline",
                                onclick: move |_| confirm_all.set(None),
                                {app.tr("cancel")}
                            }
                            button {
                                class: "btn btn-danger",
                                onclick: {
                                    let mut u = u_confirm.clone();
                                    let mut h = holding.clone();
                                    move |_| {
                                        confirm_all.set(None);
                                        h.tradable = target_v;
                                        u(h.clone());
                                    }
                                },
                                {app.tr("confirm")}
                            }
                        }
                    }
                }
            }
        }
    }
}
