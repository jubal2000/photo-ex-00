use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use dioxus::prelude::*;

use crate::api;
use crate::components::{Avatar, CardThumb};
use crate::models::{Card, TradeItem};
use crate::state::*;

/// 1:1 Photocard exchange composer.
/// - Top row: partner's cards (what I receive).
/// - Bottom row: my cards (what I give).
/// - Cards that either side lacks are dimmed for fair comparison.
/// - Both rows strictly share the exact same card catalogue order.
/// - Direct drag-and-drop or tap-to-pair with visual arrow connection.
/// - 1:1 exchange principle.
/// - Live exchange result preview screen.
#[component]
pub fn Trade(partner_uid: String, category_id: Option<String>, artist_id: Option<String>) -> Element {
    let mut app = use_app();
    let mut selected_cat = use_signal(|| category_id.clone());
    let mut selected_series = use_signal(|| Option::<String>::None);
    let mut pairs = use_signal(Vec::<(String, String)>::new); // (my_card_id, their_card_id)
    let mut selected_my_card = use_signal(|| Option::<String>::None);
    let mut selected_their_card = use_signal(|| Option::<String>::None);
    let mut dragged_my_card = use_signal(|| Option::<String>::None);
    let mut show_preview = use_signal(|| false);
    let mut preview_show_all = use_signal(|| false);
    let mut sending = use_signal(|| false);

    let puid = partner_uid.clone();
    let data = use_resource(move || {
        let puid = puid.clone();
        async move {
            let tok = app.token().await.ok()?;
            let (t, v, p) = futures_util::join!(
                api::partner_tradable(&tok, &puid),
                api::partner_visible(&tok, &puid),
                api::load_profile(&tok, &puid),
            );
            Some((t.ok()?, v.ok()?, p.ok().flatten().unwrap_or_default()))
        }
    });

    let Some(Some((theirs_tradable, their_visible, partner))) = data.read().clone() else {
        return rsx! { div { class: "page", p { class: "muted", {app.tr("loading")} } } };
    };

    let catalog = app.catalog.read().clone();
    let my_all = app.holdings.read().clone();

    // Map holdings by card_id for fast lookup
    let their_tradable_map: Rc<HashMap<String, i64>> = Rc::new(
        theirs_tradable
            .iter()
            .map(|h| (h.card_id.clone(), h.tradable))
            .collect(),
    );

    let their_count_map: Rc<HashMap<String, i64>> = Rc::new(
        their_visible
            .iter()
            .map(|h| (h.card_id.clone(), h.count))
            .collect(),
    );

    let my_tradable_map: Rc<HashMap<String, i64>> = Rc::new(
        my_all
            .iter()
            .map(|h| (h.card_id.clone(), h.tradable))
            .collect(),
    );

    let my_count_map: Rc<HashMap<String, i64>> = Rc::new(
        my_all
            .iter()
            .map(|h| (h.card_id.clone(), h.count))
            .collect(),
    );

    // Determine target artist strictly (strictly isolate trade session to this artist)
    let target_artist_id = artist_id
        .filter(|id| !id.is_empty())
        .or_else(|| {
            theirs_tradable
                .iter()
                .find_map(|h| if !h.artist_id.is_empty() { Some(h.artist_id.clone()) } else { None })
        })
        .or_else(|| {
            category_id.as_ref().and_then(|cid| {
                catalog.category(cid).and_then(|c| {
                    if !c.artist_id.is_empty() {
                        Some(c.artist_id.clone())
                    } else {
                        None
                    }
                })
            })
        })
        .or_else(|| {
            theirs_tradable.first().and_then(|h| catalog.card(&h.card_id).map(|c| c.artist_id.clone())).filter(|id| !id.is_empty())
        })
        .or_else(|| {
            my_all.first().and_then(|h| catalog.card(&h.card_id).map(|c| c.artist_id.clone())).filter(|id| !id.is_empty())
        });

    // Find available categories ONLY within this target artist
    let mut available_categories: Vec<(String, String)> = Vec::new();
    let mut seen_cats = HashSet::new();
    for cat in catalog.categories.iter() {
        if let Some(ref aid) = target_artist_id {
            if &cat.artist_id == aid && seen_cats.insert(cat.id.clone()) {
                available_categories.push((cat.name.clone(), cat.id.clone()));
            }
        }
    }
    available_categories.sort_by(|a, b| a.0.cmp(&b.0));

    // Ensure selected_cat is set to a valid category in this artist
    let cur_cat = selected_cat().or_else(|| {
        available_categories.first().map(|(_, cid)| cid.clone())
    }).or_else(|| {
        theirs_tradable.first().map(|h| h.category_id.clone())
    });

    // Find available series within this category & artist
    let mut available_series: Vec<(String, String)> = Vec::new();
    let mut seen_sers = HashSet::new();
    for c in catalog.cards.iter().filter(|c| {
        let cat_ok = cur_cat.as_ref().map(|cat_id| &c.category_id == cat_id).unwrap_or(true);
        let art_ok = target_artist_id.as_ref().map(|aid| &c.artist_id == aid).unwrap_or(true);
        cat_ok && art_ok
    }) {
        if !c.series_id.is_empty() && seen_sers.insert(c.series_id.clone()) {
            available_series.push((catalog.series_name(&c.series_id), c.series_id.clone()));
        }
    }
    available_series.sort_by(|a, b| a.0.cmp(&b.0));

    let cur_ser = selected_series();
    let display_cards: Vec<Card> = {
        let mut cards: Vec<Card> = catalog.cards.iter().filter(|c| {
            // 1. Strict artist filter: prevent cards of any other artist from leaking into this trade
            if let Some(ref aid) = target_artist_id {
                if !aid.is_empty() && &c.artist_id != aid {
                    return false;
                }
            }
            // 2. Category filter
            if let Some(ref cat_id) = cur_cat {
                if &c.category_id != cat_id {
                    return false;
                }
            }
            // 3. Series filter
            if let Some(ref sid) = cur_ser {
                if &c.series_id != sid {
                    return false;
                }
            }
            true
        }).cloned().collect();
        // Sort strictly by category, then series, then name to ensure identical order
        cards.sort_by(|a, b| {
            let cat_cmp = a.category_id.cmp(&b.category_id);
            if cat_cmp != std::cmp::Ordering::Equal {
                return cat_cmp;
            }
            let ser_cmp = a.series_id.cmp(&b.series_id);
            if ser_cmp != std::cmp::Ordering::Equal {
                return ser_cmp;
            }
            a.name_lower.cmp(&b.name_lower)
        });
        cards
    };

    let pname = if partner.nickname.is_empty() {
        app.tr("unknown_user").to_string()
    } else {
        partner.nickname.clone()
    };

    // Auto-match 1:1 trade pairs by similar rarity and unowned priority
    let auto_match = {
        let display_cards = display_cards.clone();
        let their_tradable_map = Rc::clone(&their_tradable_map);
        let their_count_map = Rc::clone(&their_count_map);
        let my_tradable_map = Rc::clone(&my_tradable_map);
        let my_count_map = Rc::clone(&my_count_map);
        let global_counts = app.global_card_counts.read().clone();
        let protect_single = *app.protect_single_card.read();

        move |_| {
            // My available cards for trade
            let mut my_candidates: Vec<String> = Vec::new();
            for c in &display_cards {
                let tradable = my_tradable_map.get(&c.id).copied().unwrap_or(0);
                let my_cnt = my_count_map.get(&c.id).copied().unwrap_or(0);
                let their_has_cnt = their_count_map.get(&c.id).copied().unwrap_or(0);
                if their_has_cnt > 0 {
                    continue; // Partner already owns this card, do not offer!
                }
                if protect_single && my_cnt <= 1 {
                    continue;
                }
                for _ in 0..tradable {
                    my_candidates.push(c.id.clone());
                }
            }

            // Their available cards for trade
            let mut their_candidates: Vec<String> = Vec::new();
            for c in &display_cards {
                let tradable = their_tradable_map.get(&c.id).copied().unwrap_or(0);
                for _ in 0..tradable {
                    their_candidates.push(c.id.clone());
                }
            }

            if my_candidates.is_empty() || their_candidates.is_empty() {
                app.show_toast(app.tr("auto_match_none"));
                return;
            }

            // Find best pairs (similar rarity + prioritized unowned by me)
            let mut new_pairs: Vec<(String, String)> = Vec::new();
            let mut used_my = vec![false; my_candidates.len()];
            let mut used_their = vec![false; their_candidates.len()];

            struct MatchCandidate {
                my_idx: usize,
                their_idx: usize,
                score: i64,
            }
            let mut all_matches: Vec<MatchCandidate> = Vec::new();

            for (m_idx, my_id) in my_candidates.iter().enumerate() {
                let my_rarity = global_counts.get(my_id).copied().unwrap_or(0);

                for (t_idx, their_id) in their_candidates.iter().enumerate() {
                    if my_id == their_id {
                        continue; // Do not trade for the exact same card
                    }
                    let their_rarity = global_counts.get(their_id).copied().unwrap_or(0);
                    let rarity_diff = (my_rarity - their_rarity).abs();

                    let my_cur_cnt = my_count_map.get(their_id).copied().unwrap_or(0);
                    let mut score = 50 - (rarity_diff * 10);
                    if my_cur_cnt == 0 {
                        score += 100; // prioritize cards I don't own yet!
                    }

                    all_matches.push(MatchCandidate {
                        my_idx: m_idx,
                        their_idx: t_idx,
                        score,
                    });
                }
            }

            all_matches.sort_by(|a, b| b.score.cmp(&a.score));

            for m in all_matches {
                if !used_my[m.my_idx] && !used_their[m.their_idx] {
                    used_my[m.my_idx] = true;
                    used_their[m.their_idx] = true;
                    new_pairs.push((my_candidates[m.my_idx].clone(), their_candidates[m.their_idx].clone()));
                }
            }

            if new_pairs.is_empty() {
                app.show_toast(app.tr("auto_match_none"));
            } else {
                let cnt = new_pairs.len();
                pairs.set(new_pairs);
                let msg = app.tr("auto_match_success").replace("{count}", &cnt.to_string());
                app.show_toast(&msg);
            }
        }
    };

    // Shared submit function
    let submit_trade = {
        let partner_uid = partner_uid.clone();
        move || {
            let pair_list = pairs.read().clone();
            if pair_list.is_empty() {
                app.show_toast(app.tr("trade_no_pairs"));
                return;
            }
            let mut offered_map: HashMap<String, i64> = HashMap::new();
            let mut requested_map: HashMap<String, i64> = HashMap::new();
            for (m, t) in pair_list {
                *offered_map.entry(m).or_insert(0) += 1;
                *requested_map.entry(t).or_insert(0) += 1;
            }
            let offered: Vec<TradeItem> = offered_map
                .into_iter()
                .map(|(card_id, qty)| TradeItem { card_id, qty })
                .collect();
            let requested: Vec<TradeItem> = requested_map
                .into_iter()
                .map(|(card_id, qty)| TradeItem { card_id, qty })
                .collect();

            let p_uid = partner_uid.clone();
            spawn(async move {
                sending.set(true);
                let uid = app.uid();
                match app.token().await {
                    Ok(tok) => match api::create_trade(&tok, &uid, &p_uid, offered, requested).await {
                        Ok(_) => {
                            app.show_toast(app.tr("trade_sent"));
                            app.back();
                        }
                        Err(e) => app.show_error(&e),
                    },
                    Err(e) => app.show_error(&e),
                }
                sending.set(false);
            });
        }
    };

    let submit_btn_click = {
        let mut sub = submit_trade.clone();
        move |_| sub()
    };
    let modal_submit_click = {
        let mut sub = submit_trade;
        move |_| sub()
    };

    // Live update curved SVG lines connecting paired cards
    use_effect(move || {
        let pairs_snapshot = pairs.read().clone();
        let pairs_json = serde_json::to_string(&pairs_snapshot).unwrap_or_else(|_| "[]".to_string());
        let js = format!(
            r#"
            (function() {{
                const pairs = {pairs_json};
                window.__currentTradePairs = pairs;

                function findCard(role, id) {{
                    return document.getElementById('trade-card-' + role + '-' + id) ||
                           document.querySelector('[data-trade-' + role + '="' + id + '"]');
                }}

                function updateCurves() {{
                    const curPairs = window.__currentTradePairs || [];
                    const board = document.getElementById('trade-board');
                    const svg = document.getElementById('trade-curves-svg');
                    const pathGroup = document.getElementById('trade-curves-paths');
                    if (!board || !svg || !pathGroup) return;

                    const bRect = board.getBoundingClientRect();
                    if (bRect.width === 0 || bRect.height === 0) {{
                        setTimeout(updateCurves, 50);
                        return;
                    }}

                    svg.setAttribute('viewBox', '0 0 ' + bRect.width + ' ' + bRect.height);
                    svg.style.position = 'absolute';
                    svg.style.top = '0';
                    svg.style.left = '0';
                    svg.style.width = bRect.width + 'px';
                    svg.style.height = bRect.height + 'px';
                    svg.style.overflow = 'visible';
                    svg.style.pointerEvents = 'none';
                    svg.style.zIndex = '20';

                    while (pathGroup.firstChild) {{
                        pathGroup.removeChild(pathGroup.firstChild);
                    }}

                    // Hide any extra floating buttons if pairs shrank
                    for (let extraIdx = 0; extraIdx < 50; extraIdx++) {{
                        const extraBtn = document.getElementById('trade-cut-btn-' + extraIdx);
                        if (!extraBtn) continue;
                        if (extraIdx >= curPairs.length) {{
                            extraBtn.style.display = 'none';
                        }}
                    }}

                    if (curPairs.length === 0) return;

                    curPairs.forEach(function(pair, idx) {{
                        const myId = pair[0];
                        const theirId = pair[1];
                        const elTheir = findCard('their', theirId);
                        const elMy = findCard('my', myId);
                        if (!elTheir || !elMy) return;

                        const rT = elTheir.getBoundingClientRect();
                        const rM = elMy.getBoundingClientRect();

                        const x1 = (rT.left + rT.right) / 2 - bRect.left;
                        const y1 = rT.bottom - bRect.top;
                        const x2 = (rM.left + rM.right) / 2 - bRect.left;
                        const y2 = rM.top - bRect.top;

                        const cp1Y = y1 + Math.max(25, (y2 - y1) * 0.5);
                        const cp2Y = y2 - Math.max(25, (y2 - y1) * 0.5);

                        // Vivid violet curve line
                        const p = document.createElementNS('http://www.w3.org/2000/svg', 'path');
                        const d = 'M ' + x1 + ' ' + y1 + ' C ' + x1 + ' ' + cp1Y + ', ' + x2 + ' ' + cp2Y + ', ' + x2 + ' ' + y2;
                        p.setAttribute('d', d);
                        p.setAttribute('stroke', '#a855f7');
                        p.setAttribute('stroke-width', '4');
                        p.setAttribute('fill', 'none');
                        p.setAttribute('stroke-linecap', 'round');
                        p.setAttribute('filter', 'drop-shadow(0 0 8px rgba(168, 85, 247, 0.7))');
                        p.setAttribute('class', 'trade-curve-path');
                        pathGroup.appendChild(p);

                        // Start terminal dot (their card bottom - vivid blue dot)
                        const dot1 = document.createElementNS('http://www.w3.org/2000/svg', 'circle');
                        dot1.setAttribute('cx', x1);
                        dot1.setAttribute('cy', y1);
                        dot1.setAttribute('r', '6');
                        dot1.setAttribute('fill', '#38bdf8');
                        dot1.setAttribute('stroke', '#ffffff');
                        dot1.setAttribute('stroke-width', '2');
                        dot1.setAttribute('filter', 'drop-shadow(0 0 6px #38bdf8)');
                        pathGroup.appendChild(dot1);

                        // End terminal dot (my card top - vivid blue dot)
                        const dot2 = document.createElementNS('http://www.w3.org/2000/svg', 'circle');
                        dot2.setAttribute('cx', x2);
                        dot2.setAttribute('cy', y2);
                        dot2.setAttribute('r', '6');
                        dot2.setAttribute('fill', '#38bdf8');
                        dot2.setAttribute('stroke', '#ffffff');
                        dot2.setAttribute('stroke-width', '2');
                        dot2.setAttribute('filter', 'drop-shadow(0 0 6px #38bdf8)');
                        pathGroup.appendChild(dot2);

                        // Midpoint on cubic bezier curve for Cut Icon
                        const midX = (x1 + x2) / 2;
                        const midY = (y1 + 3 * cp1Y + 3 * cp2Y + y2) / 8;

                        // Position native interactive Cut Button
                        const cutBtn = document.getElementById('trade-cut-btn-' + idx);
                        if (cutBtn) {{
                            cutBtn.style.display = 'flex';
                            cutBtn.style.left = midX + 'px';
                            cutBtn.style.top = midY + 'px';
                        }}
                    }});
                }}

                window.__updateTradeCurves = updateCurves;
                updateCurves();
                requestAnimationFrame(updateCurves);
                setTimeout(updateCurves, 50);
                setTimeout(updateCurves, 150);
                setTimeout(updateCurves, 350);

                // Attach real-time independent scroll listeners to both rows with momentum tracking
                let momentumTimer = null;
                function onRowScroll() {{
                    updateCurves();
                    if (momentumTimer) cancelAnimationFrame(momentumTimer);
                    let frames = 40; // Follow momentum scrolling smoothly
                    function follow() {{
                        updateCurves();
                        frames--;
                        if (frames > 0) {{
                            momentumTimer = requestAnimationFrame(follow);
                        }}
                    }}
                    momentumTimer = requestAnimationFrame(follow);
                }}

                const rowT = document.getElementById('trade-row-theirs');
                const rowM = document.getElementById('trade-row-mine');

                if (rowT) {{
                    rowT.onscroll = onRowScroll;
                    rowT.addEventListener('touchmove', onRowScroll, {{ passive: true }});
                }}
                if (rowM) {{
                    rowM.onscroll = onRowScroll;
                    rowM.addEventListener('touchmove', onRowScroll, {{ passive: true }});
                }}
                window.onresize = updateCurves;
            }})();
            "#
        );
        document::eval(&js);
    });

    rsx! {
        div { class: "page trade-page",
            // Partner Header Info
            div { class: "trade-partner-bar",
                Avatar { url: partner.photo_url.clone(), size: 44 }
                div { class: "partner-meta",
                    div { class: "partner-name-row",
                        span { class: "partner-name", "{pname}" }
                    }
                    if !partner.greeting.is_empty() {
                        div { class: "partner-greeting muted small", "{partner.greeting}" }
                    }
                }
            }

            // Current Artist & Category Context Banner
            div { class: "trade-context-banner",
                if let Some(ref aid) = target_artist_id {
                    span { class: "trade-context-item artist",
                        "🎤 " {catalog.artist_name(aid)}
                    }
                }
                if let Some(ref cid) = cur_cat {
                    span { class: "trade-context-sep", "•" }
                    span { class: "trade-context-item category",
                        "📁 " {catalog.category_name(cid)}
                    }
                }
                if let Some(ref sid) = cur_ser {
                    span { class: "trade-context-sep", "•" }
                    span { class: "trade-context-item series",
                        "🏷️ " {catalog.series_name(sid)}
                    }
                }
            }

            // Category Filter Bar (if available)
            if available_categories.len() > 1 {
                div { class: "chips trade-category-chips",
                    button {
                        class: if selected_cat().is_none() { "chip on" } else { "chip" },
                        onclick: move |_| {
                            selected_cat.set(None);
                            selected_series.set(None);
                        },
                        {app.tr("all")}
                    }
                    for (cat_name, cid) in available_categories {
                        {
                            let is_on = selected_cat().as_deref() == Some(&cid);
                            let cid_c = cid.clone();
                            rsx! {
                                button {
                                    key: "{cid}",
                                    class: if is_on { "chip on" } else { "chip" },
                                    onclick: move |_| {
                                        selected_cat.set(Some(cid_c.clone()));
                                        selected_series.set(None);
                                    },
                                    "{cat_name}"
                                }
                            }
                        }
                    }
                }
            }

            // Series Filter Bar (within selected category)
            if !available_series.is_empty() {
                div { class: "chips trade-series-chips",
                    button {
                        class: if selected_series().is_none() { "chip on" } else { "chip" },
                        onclick: move |_| selected_series.set(None),
                        {app.tr("all")}
                    }
                    for (ser_name, sid) in available_series {
                        {
                            let is_on = selected_series().as_deref() == Some(&sid);
                            let sid_c = sid.clone();
                            rsx! {
                                button {
                                    key: "{sid}",
                                    class: if is_on { "chip on" } else { "chip" },
                                    onclick: move |_| selected_series.set(Some(sid_c.clone())),
                                    "{ser_name}"
                                }
                            }
                        }
                    }
                }
            }

            // Pairing Instruction Hint & Auto Match Button
            div { class: "trade-action-bar",
                div { class: "trade-hint-bar",
                    span { class: "hint-icon", "💡" }
                    span { class: "hint-text", {app.tr("trade_drag_hint")} }
                }
                button {
                    class: "btn btn-outline auto-match-btn",
                    onclick: auto_match,
                    {app.tr("auto_match_btn")}
                }
            }

            // Cards Board Container with Independent Scrolling Rows and Live Curved Lines
            div { class: "trade-board", id: "trade-board",
                // SVG Overlay for live bezier curves connecting paired cards
                svg {
                    class: "trade-curves-overlay",
                    id: "trade-curves-svg",
                    xmlns: "http://www.w3.org/2000/svg",
                    g { id: "trade-curves-paths" }
                }

                // Native interactive Floating Cut Buttons Layer (positioned at curve midpoint)
                div { class: "trade-cut-buttons-layer",
                    for idx in 0..pairs.read().len() {
                        {
                            let cut_idx = idx;
                            rsx! {
                                button {
                                    key: "cut-btn-{cut_idx}",
                                    id: "trade-cut-btn-{cut_idx}",
                                    class: "trade-cut-btn-floating",
                                    title: "연결 취소",
                                    onclick: move |e| {
                                        e.stop_propagation();
                                        if cut_idx < pairs.read().len() {
                                            pairs.write().remove(cut_idx);
                                        }
                                    },
                                    "✕"
                                }
                            }
                        }
                    }
                }

                // 1. Partner's Cards Row (What I receive - Independent Horizontal Scroll)
                div { class: "trade-section trade-section-theirs",
                    div { class: "trade-section-head",
                        h3 { class: "sub-title",
                            "📥 " {app.tr("trade_theirs")}
                        }
                        span { class: "muted small",
                            "{display_cards.len()}{app.tr(\"pieces\")}"
                        }
                    }
                    div { class: "h-scroll trade-row trade-row-theirs", id: "trade-row-theirs",
                            for card in display_cards.iter() {
                                {
                                    let cid = card.id.clone();
                                    let their_cnt = their_count_map.get(&cid).copied().unwrap_or(0);
                                    let is_unowned = their_cnt == 0;
                                    let their_tradable = their_tradable_map.get(&cid).copied().unwrap_or(0);
                                    let is_tradable = their_cnt > 0 && their_tradable > 0;
                                    let is_owned_not_tradable = their_cnt > 0 && !is_tradable;
                                    let current_pairs = pairs.read();
                                    let used_cnt = current_pairs.iter().filter(|(_, t)| t == &cid).count() as i64;
                                    let remaining = their_tradable - used_cnt;
                                    let is_selected = selected_their_card().as_deref() == Some(&cid);
                                    let is_connected = used_cnt > 0;

                                    let card_clone = card.clone();
                                    let cid_click = cid.clone();
                                    let cid_drop = cid.clone();
                                    let cid_edit = cid.clone();
                                    let rarity_count = app.global_card_counts.read().get(&cid).copied().unwrap_or(0);

                                    let my_tradable_map_c = Rc::clone(&my_tradable_map);
                                    let their_tradable_map_c = Rc::clone(&their_tradable_map);
                                    let my_tradable_map_d = Rc::clone(&my_tradable_map);
                                    let their_tradable_map_d = Rc::clone(&their_tradable_map);

                                    rsx! {
                                        div {
                                            key: "their-{cid}",
                                            id: "trade-card-their-{cid}",
                                            "data-trade-their": "{cid}",
                                            class: if is_unowned {
                                                "pick unowned"
                                            } else if is_selected {
                                                "pick selected-target"
                                            } else if is_connected {
                                                "pick connected-card"
                                            } else if is_tradable {
                                                "pick available drop-zone"
                                            } else {
                                                "pick owned-untradable"
                                            },
                                            ondragover: move |e| {
                                                if is_tradable && remaining > 0 {
                                                    e.prevent_default();
                                                }
                                            },
                                            ondrop: move |_| {
                                                if is_tradable && remaining > 0 {
                                                    if let Some(my_cid) = dragged_my_card() {
                                                        let my_max = my_tradable_map_d.get(&my_cid).copied().unwrap_or(0);
                                                        let their_max = their_tradable_map_d.get(&cid_drop).copied().unwrap_or(0);
                                                        let used_my = pairs.read().iter().filter(|(m, _)| m == &my_cid).count() as i64;
                                                        let used_their = pairs.read().iter().filter(|(_, t)| t == &cid_drop).count() as i64;
                                                        if used_my < my_max && used_their < their_max {
                                                            pairs.write().push((my_cid, cid_drop.clone()));
                                                            selected_my_card.set(None);
                                                            selected_their_card.set(None);
                                                        }
                                                        dragged_my_card.set(None);
                                                    }
                                                }
                                            },
                                            onclick: move |_| {
                                                if is_unowned || !is_tradable || remaining <= 0 {
                                                    return;
                                                }
                                                if let Some(my_cid) = selected_my_card() {
                                                    let my_max = my_tradable_map_c.get(&my_cid).copied().unwrap_or(0);
                                                    let their_max = their_tradable_map_c.get(&cid_click).copied().unwrap_or(0);
                                                    let used_my = pairs.read().iter().filter(|(m, _)| m == &my_cid).count() as i64;
                                                    let used_their = pairs.read().iter().filter(|(_, t)| t == &cid_click).count() as i64;
                                                    if used_my < my_max && used_their < their_max {
                                                        pairs.write().push((my_cid, cid_click.clone()));
                                                        selected_my_card.set(None);
                                                        selected_their_card.set(None);
                                                    } else {
                                                        app.show_toast(app.tr("not_tradable"));
                                                    }
                                                } else {
                                                    if selected_their_card().as_deref() == Some(&cid_click) {
                                                        selected_their_card.set(None);
                                                    } else {
                                                        selected_their_card.set(Some(cid_click.clone()));
                                                    }
                                                }
                                            },
                                            CardThumb {
                                                card: card_clone,
                                                badge: if is_unowned {
                                                    None
                                                } else if is_connected {
                                                    Some(app.tr("trade_paired").to_string())
                                                } else if is_tradable {
                                                    if remaining > 0 { Some(format!("×{remaining}")) } else { Some(app.tr("trade_paired").to_string()) }
                                                } else if is_owned_not_tradable {
                                                    Some(format!("×{their_cnt}"))
                                                } else {
                                                    None
                                                },
                                                rarity: Some(rarity_count),
                                            }
                                            if !is_unowned && is_owned_not_tradable {
                                                div { class: "trade-card-center-x", "✕" }
                                            }
                                            if is_connected {
                                                div { class: "card-anchor-dot anchor-bottom" }
                                            }
                                            if *app.edit_mode.read() {
                                                button {
                                                    class: "btn-icon-subtle edit-in-trade-btn",
                                                    title: app.tr("edit"),
                                                    onclick: move |e| {
                                                        e.stop_propagation();
                                                        app.push(Screen::EditCard { card_id: cid_edit.clone() });
                                                    },
                                                    "✏️"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                // 2. My Cards Row (What I give - Independent Horizontal Scroll)
                div { class: "trade-section trade-section-mine",
                    div { class: "trade-section-head",
                        h3 { class: "sub-title",
                            "📤 " {app.tr("trade_mine")}
                        }
                        span { class: "muted small",
                            "{display_cards.len()}{app.tr(\"pieces\")}"
                        }
                    }
                    div { class: "h-scroll trade-row trade-row-mine", id: "trade-row-mine",
                            for card in display_cards.iter() {
                                {
                                    let cid = card.id.clone();
                                    let my_cnt = my_count_map.get(&cid).copied().unwrap_or(0);
                                    let is_unowned = my_cnt == 0;
                                    let their_has_cnt = their_count_map.get(&cid).copied().unwrap_or(0);
                                    let partner_already_owns = my_cnt > 0 && their_has_cnt > 0;
                                    let is_protected = my_cnt > 0 && *app.protect_single_card.read() && my_cnt <= 1;
                                    let my_tradable = my_tradable_map.get(&cid).copied().unwrap_or(0);
                                    let is_tradable = my_cnt > 0 && my_tradable > 0 && !is_protected && !partner_already_owns;
                                    let is_owned_not_tradable = my_cnt > 0 && !is_tradable && !partner_already_owns;
                                    let current_pairs = pairs.read();
                                    let used_cnt = current_pairs.iter().filter(|(m, _)| m == &cid).count() as i64;
                                    let remaining = my_tradable - used_cnt;
                                    let is_selected = selected_my_card().as_deref() == Some(&cid);
                                    let is_connected = used_cnt > 0;

                                    let card_clone = card.clone();
                                    let cid_drag = cid.clone();
                                    let cid_click = cid.clone();
                                    let cid_edit = cid.clone();
                                    let rarity_count = app.global_card_counts.read().get(&cid).copied().unwrap_or(0);

                                    let my_tradable_map_c = Rc::clone(&my_tradable_map);
                                    let their_tradable_map_c = Rc::clone(&their_tradable_map);

                                    rsx! {
                                        div {
                                            key: "my-{cid}",
                                            id: "trade-card-my-{cid}",
                                            "data-trade-my": "{cid}",
                                            class: if is_unowned {
                                                "pick unowned"
                                            } else if is_selected {
                                                "pick selected-source"
                                            } else if is_connected {
                                                "pick connected-card"
                                            } else if partner_already_owns {
                                                "pick partner-already-owns"
                                            } else if is_tradable {
                                                "pick available draggable"
                                            } else {
                                                "pick owned-untradable"
                                            },
                                            draggable: if !is_unowned && is_tradable && remaining > 0 { "true" } else { "false" },
                                            ondragstart: move |_| {
                                                if !is_unowned && is_tradable && remaining > 0 {
                                                    dragged_my_card.set(Some(cid_drag.clone()));
                                                }
                                            },
                                            ondragend: move |_| {
                                                dragged_my_card.set(None);
                                            },
                                            onclick: move |_| {
                                                if is_unowned {
                                                    return;
                                                }
                                                if partner_already_owns {
                                                    app.show_toast(app.tr("partner_already_owns"));
                                                    return;
                                                }
                                                if is_protected {
                                                    app.show_toast(app.tr("single_card_protected"));
                                                    return;
                                                }
                                                if !is_tradable || remaining <= 0 {
                                                    return;
                                                }
                                                if let Some(their_cid) = selected_their_card() {
                                                    let my_max = my_tradable_map_c.get(&cid_click).copied().unwrap_or(0);
                                                    let their_max = their_tradable_map_c.get(&their_cid).copied().unwrap_or(0);
                                                    let used_my = pairs.read().iter().filter(|(m, _)| m == &cid_click).count() as i64;
                                                    let used_their = pairs.read().iter().filter(|(_, t)| t == &their_cid).count() as i64;
                                                    if used_my < my_max && used_their < their_max {
                                                        pairs.write().push((cid_click.clone(), their_cid));
                                                        selected_my_card.set(None);
                                                        selected_their_card.set(None);
                                                    } else {
                                                        app.show_toast(app.tr("not_tradable"));
                                                    }
                                                } else {
                                                    if selected_my_card().as_deref() == Some(&cid_click) {
                                                        selected_my_card.set(None);
                                                    } else {
                                                        selected_my_card.set(Some(cid_click.clone()));
                                                    }
                                                }
                                            },
                                            CardThumb {
                                                card: card_clone,
                                                badge: if is_unowned {
                                                    None
                                                } else if partner_already_owns {
                                                    Some(app.tr("already_owned").to_string())
                                                } else if is_protected {
                                                    Some("🔒 ".to_string() + app.tr("protect_single_card"))
                                                } else if is_connected {
                                                    Some(app.tr("trade_paired").to_string())
                                                } else if is_tradable {
                                                    if remaining > 0 { Some(format!("×{remaining}")) } else { Some(app.tr("trade_paired").to_string()) }
                                                } else if is_owned_not_tradable {
                                                    Some(format!("×{my_cnt}"))
                                                } else {
                                                    None
                                                },
                                                rarity: Some(rarity_count),
                                            }
                                            if !is_unowned && (is_owned_not_tradable || partner_already_owns) {
                                                div { class: "trade-card-center-x", "✕" }
                                            }
                                            if is_connected {
                                                div { class: "card-anchor-dot anchor-top" }
                                            }
                                            if *app.edit_mode.read() {
                                                button {
                                                    class: "btn-icon-subtle edit-in-trade-btn",
                                                    title: app.tr("edit"),
                                                    onclick: move |e| {
                                                        e.stop_propagation();
                                                        app.push(Screen::EditCard { card_id: cid_edit.clone() });
                                                    },
                                                    "✏️"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

            // 3. Active Pairs Subpanel (Moved below cards board)
            div { class: "trade-exchange-zone",
                div { class: "exchange-zone-head",
                    div { class: "exchange-zone-title",
                        span { class: "exchange-icon", "⇄" }
                        span { class: "bold", {app.tr("trade_pair_rule")} }
                        span { class: "pairs-count", "({pairs.read().len()})" }
                    }
                    if !pairs.read().is_empty() {
                        button {
                            class: "btn-icon-subtle clear-pairs-btn",
                            title: "Reset",
                            onclick: move |_| pairs.set(Vec::new()),
                            "↺"
                        }
                    }
                }
                if pairs.read().is_empty() {
                    div { class: "exchange-empty-placeholder",
                        div { class: "arrow-placeholder", "⬇  교환 매칭  ⬆" }
                        div { class: "muted small", {app.tr("trade_drag_hint")} }
                    }
                } else {
                    div { class: "exchange-pairs-list",
                        for (idx, (my_id, their_id)) in pairs.read().iter().enumerate() {
                            {
                                let my_card = catalog.card(my_id).cloned().unwrap_or_default();
                                let their_card = catalog.card(their_id).cloned().unwrap_or_default();
                                let pair_num = idx + 1;
                                rsx! {
                                    div { class: "exchange-pair-box", key: "{idx}",
                                        div { class: "pair-number-badge", "#{pair_num}" }
                                        // My card (giving)
                                        div { class: "pair-card-node giving-node",
                                            if !my_card.image_url.is_empty() {
                                                img { class: "pair-thumb-mini", src: "{my_card.image_url}", alt: "{my_card.name}" }
                                            } else {
                                                div { class: "pair-thumb-empty", "🃏" }
                                            }
                                            div { class: "pair-node-info",
                                                div { class: "pair-node-role muted", {app.tr("trade_give_card")} }
                                                div { class: "pair-node-name", "{my_card.name}" }
                                            }
                                        }

                                        // Animated Connecting Arrow
                                        div { class: "pair-arrow-bridge",
                                            span { class: "arrow-head", "➔" }
                                            span { class: "arrow-sub-head", "⇄" }
                                        }

                                        // Their card (receiving)
                                        div { class: "pair-card-node receiving-node",
                                            if !their_card.image_url.is_empty() {
                                                img { class: "pair-thumb-mini", src: "{their_card.image_url}", alt: "{their_card.name}" }
                                            } else {
                                                div { class: "pair-thumb-empty", "🃏" }
                                            }
                                            div { class: "pair-node-info",
                                                div { class: "pair-node-role muted", {app.tr("trade_receive_card")} }
                                                div { class: "pair-node-name", "{their_card.name}" }
                                            }
                                        }

                                        // Delete Pair button
                                        button {
                                            class: "btn-icon-subtle delete-pair-btn",
                                            title: "Delete",
                                            onclick: move |_| {
                                                if idx < pairs.read().len() {
                                                    pairs.write().remove(idx);
                                                }
                                            },
                                            "✕"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Bottom Action Footer Bar
            div { class: "trade-footer-fixed",
                div { class: "trade-footer-meta",
                    div { class: "trade-footer-summary",
                        span { class: "bold",
                            "{pairs.read().len()} " {app.tr("pieces")}
                        }
                    }
                }
                div { class: "trade-footer-actions",
                    button {
                        class: "btn btn-outline preview-trade-btn",
                        disabled: pairs.read().is_empty(),
                        onclick: move |_| show_preview.set(true),
                        "👁 " {app.tr("trade_preview")}
                    }
                    button {
                        class: "btn btn-primary submit-trade-btn",
                        disabled: pairs.read().is_empty() || sending(),
                        onclick: submit_btn_click,
                        "🚀 " {app.tr("trade_submit")}
                    }
                }
            }

            // 4. Exchange Result Preview Modal
            if show_preview() {
                div { class: "scrim", onclick: move |_| show_preview.set(false) }
                div { class: "trade-preview-modal",
                    div { class: "preview-modal-head",
                        div { class: "preview-modal-title bold",
                            "✨ " {app.tr("trade_preview_title")}
                        }
                        button {
                            class: "btn-icon-subtle close-modal-btn",
                            onclick: move |_| show_preview.set(false),
                            "✕"
                        }
                    }

                    div { class: "preview-modal-body",
                        p { class: "muted small preview-intro",
                            "{pname}님과 교환 ({pairs.read().len()}장 ⇄ {pairs.read().len()}장) 완료 시 보유하게 될 목록입니다."
                        }

                        // Filter Chips: Owned only vs Show all (including unowned)
                        div { class: "chips preview-filter-chips",
                            button {
                                class: if !preview_show_all() { "chip on" } else { "chip" },
                                onclick: move |_| preview_show_all.set(false),
                                {app.tr("preview_owned_only")}
                            }
                            button {
                                class: if preview_show_all() { "chip on" } else { "chip" },
                                onclick: move |_| preview_show_all.set(true),
                                "👁 " {app.tr("preview_all")}
                            }
                        }

                        {
                            let current_pairs = pairs.read().clone();
                            let mut given_map: HashMap<String, i64> = HashMap::new();
                            let mut received_map: HashMap<String, i64> = HashMap::new();
                            for (m, t) in &current_pairs {
                                *given_map.entry(m.clone()).or_insert(0) += 1;
                                *received_map.entry(t.clone()).or_insert(0) += 1;
                            }

                            let mut preview_list: Vec<(Card, i64, i64)> = Vec::new();
                            for card in &display_cards {
                                let cur = my_count_map.get(&card.id).copied().unwrap_or(0);
                                let given = given_map.get(&card.id).copied().unwrap_or(0);
                                let received = received_map.get(&card.id).copied().unwrap_or(0);
                                let after = (cur - given + received).max(0);
                                if preview_show_all() || after > 0 {
                                    preview_list.push((card.clone(), cur, after));
                                }
                            }

                            rsx! {
                                div { class: "preview-holdings-grid",
                                    for (card, cur, after) in preview_list {
                                        {
                                            let is_new = cur == 0 && after > 0;
                                            let is_increased = after > cur && cur > 0;
                                            let is_decreased = after < cur;
                                            let is_unowned = cur == 0 && after == 0;
                                            let cid = card.id.clone();
                                            let rarity_count = app.global_card_counts.read().get(&cid).copied().unwrap_or(0);

                                            rsx! {
                                                div {
                                                    class: if is_new {
                                                        "preview-holding-item highlight-new"
                                                    } else if is_unowned {
                                                        "preview-holding-item unowned"
                                                    } else {
                                                        "preview-holding-item"
                                                    },
                                                    key: "{cid}",
                                                    CardThumb {
                                                        card: card.clone(),
                                                        small: true,
                                                        rarity: Some(rarity_count),
                                                    }
                                                    div { class: "preview-holding-info",
                                                        div { class: "preview-cname", "{card.name}" }
                                                        div { class: "preview-cstatus",
                                                            if is_new {
                                                                span { class: "status-pill new", "{app.tr(\"acquired_new\")} (0➔{after}장)" }
                                                            } else if is_increased {
                                                                span { class: "status-pill up", "{app.tr(\"count_increased\")} ({cur}➔{after}장)" }
                                                            } else if is_decreased {
                                                                span { class: "status-pill down", "{app.tr(\"count_decreased\")} ({cur}➔{after}장)" }
                                                            } else if is_unowned {
                                                                span { class: "status-pill unowned", "{app.tr(\"unowned\")} (0장)" }
                                                            } else {
                                                                span { class: "status-pill same", "{after}장 보유" }
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

                    div { class: "preview-modal-footer",
                        button {
                            class: "btn btn-outline",
                            onclick: move |_| show_preview.set(false),
                            {app.tr("back")}
                        }
                        button {
                            class: "btn btn-primary",
                            disabled: sending(),
                            onclick: modal_submit_click,
                            "🚀 " {app.tr("trade_submit")}
                        }
                    }
                }
            }
        }
    }
}
