//! Trades Screen: Trade Requests & Accepted Trade History with tabs.

use dioxus::prelude::*;

use crate::api;
use crate::app::reload_holdings;
use crate::components::CardThumb;
use crate::models::{Trade, TradeItem};
use crate::state::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TradeTab {
    Requests,
    Accepted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RequestFilter {
    All,
    Inbox,
    Outbox,
}

#[component]
pub fn TradesScreen() -> Element {
    let mut app = use_app();
    let mut current_tab = use_signal(|| TradeTab::Requests);
    let mut req_filter = use_signal(|| RequestFilter::All);

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
            Some(Some((i, o))) => i
                .iter()
                .map(|t| t.from_uid.clone())
                .chain(o.iter().map(|t| t.to_uid.clone()))
                .collect(),
            _ => vec![],
        };
        async move {
            let tok = app.token().await.ok()?;
            Some(api::load_profiles(&tok, &uids).await)
        }
    });

    let (inbox, outbox) = match trades.read().clone() {
        Some(Some((i, o))) => (i, o),
        _ => (vec![], vec![]),
    };

    let names = profiles.read().clone().flatten().unwrap_or_default();
    let name_of = |uid: &str| {
        names
            .get(uid)
            .map(|p| p.nickname.clone())
            .unwrap_or_else(|| app.tr("unknown_user").to_string())
    };

    // Pending requests
    let pending_inbox: Vec<Trade> = inbox.iter().filter(|t| t.status == "pending").cloned().collect();
    let pending_outbox: Vec<Trade> = outbox.iter().filter(|t| t.status == "pending").cloned().collect();
    let pending_count = pending_inbox.len() + pending_outbox.len();

    // Accepted trades (both inbox & outbox)
    let mut accepted_trades: Vec<(Trade, bool)> = Vec::new();
    for t in inbox.iter().filter(|t| t.status == "accepted") {
        accepted_trades.push((t.clone(), true));
    }
    for t in outbox.iter().filter(|t| t.status == "accepted") {
        accepted_trades.push((t.clone(), false));
    }
    // Sort descending by created_at
    accepted_trades.sort_by(|a, b| b.0.created_at.cmp(&a.0.created_at));

    rsx! {
        div { class: "page trades-page",
            // Top Navigation Tabs
            div { class: "segmented trades-tabs",
                button {
                    class: if current_tab() == TradeTab::Requests { "tab-btn on active" } else { "tab-btn" },
                    onclick: move |_| current_tab.set(TradeTab::Requests),
                    span { class: "tab-label", {app.tr("tab_trade_requests")} }
                    if pending_count > 0 {
                        span { class: "tab-badge", "{pending_count}" }
                    }
                }
                button {
                    class: if current_tab() == TradeTab::Accepted { "tab-btn on active" } else { "tab-btn" },
                    onclick: move |_| current_tab.set(TradeTab::Accepted),
                    span { class: "tab-label", {app.tr("tab_trade_history")} }
                    if !accepted_trades.is_empty() {
                        span { class: "tab-badge muted-badge", "{accepted_trades.len()}" }
                    }
                }
            }

            match current_tab() {
                TradeTab::Requests => rsx! {
                    // Sub filter chips
                    div { class: "trades-filter-bar",
                        button {
                            class: if req_filter() == RequestFilter::All { "chip on" } else { "chip" },
                            onclick: move |_| req_filter.set(RequestFilter::All),
                            {app.tr("trades_filter_all")} " ({pending_count})"
                        }
                        button {
                            class: if req_filter() == RequestFilter::Inbox { "chip on" } else { "chip" },
                            onclick: move |_| req_filter.set(RequestFilter::Inbox),
                            {app.tr("trades_filter_inbox")} " ({pending_inbox.len()})"
                        }
                        button {
                            class: if req_filter() == RequestFilter::Outbox { "chip on" } else { "chip" },
                            onclick: move |_| req_filter.set(RequestFilter::Outbox),
                            {app.tr("trades_filter_outbox")} " ({pending_outbox.len()})"
                        }
                    }

                    if pending_count == 0 {
                        div { class: "empty-state-card",
                            p { class: "muted", {app.tr("trades_empty_requests")} }
                        }
                    } else {
                        // Received Requests Section
                        if req_filter() != RequestFilter::Outbox && !pending_inbox.is_empty() {
                            div { class: "trades-section-title",
                                {app.tr("trades_filter_inbox")}
                                span { class: "count", "({pending_inbox.len()})" }
                            }
                            for t in pending_inbox.iter() {
                                TradeCard {
                                    key: "{t.id}",
                                    trade: t.clone(),
                                    who: name_of(&t.from_uid),
                                    incoming: true,
                                    on_action: move |(id, a)| act(id, a),
                                }
                            }
                        }

                        // Sent Requests Section
                        if req_filter() != RequestFilter::Inbox && !pending_outbox.is_empty() {
                            div { class: "trades-section-title",
                                {app.tr("trades_filter_outbox")}
                                span { class: "count", "({pending_outbox.len()})" }
                            }
                            for t in pending_outbox.iter() {
                                TradeCard {
                                    key: "{t.id}",
                                    trade: t.clone(),
                                    who: name_of(&t.to_uid),
                                    incoming: false,
                                    on_action: move |(id, a)| act(id, a),
                                }
                            }
                        }
                    }
                },
                TradeTab::Accepted => rsx! {
                    if accepted_trades.is_empty() {
                        div { class: "empty-state-card",
                            p { class: "muted", {app.tr("trades_empty_accepted")} }
                        }
                    } else {
                        div { class: "trades-accepted-list",
                            for (t, inc) in accepted_trades.iter() {
                                {
                                    let other_uid = if *inc { &t.from_uid } else { &t.to_uid };
                                    let partner_name = name_of(other_uid);
                                    rsx! {
                                        TradeCard {
                                            key: "{t.id}",
                                            trade: t.clone(),
                                            who: partner_name,
                                            incoming: *inc,
                                            on_action: move |(id, a)| act(id, a),
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
            }
        }
    }
}

#[component]
fn TradeCard(
    trade: Trade,
    who: String,
    incoming: bool,
    on_action: EventHandler<(String, &'static str)>,
) -> Element {
    let app = use_app();
    let catalog = app.catalog.read().clone();
    let status_key = format!("status_{}", trade.status);

    // Perspective: what I receive (get) vs what I give (give)
    let (give, get): (&Vec<TradeItem>, &Vec<TradeItem>) = if incoming {
        (&trade.requested, &trade.offered)
    } else {
        (&trade.offered, &trade.requested)
    };

    let id_reject = trade.id.clone();
    let id_accept = trade.id.clone();
    let id_cancel = trade.id.clone();

    rsx! {
        div { class: "trade-card-box",
            div { class: "trade-card-header",
                div { class: "trade-user-info",
                    span {
                        class: if incoming { "trade-direction-tag incoming" } else { "trade-direction-tag outgoing" },
                        { if incoming { app.tr("trades_filter_inbox") } else { app.tr("trades_filter_outbox") } }
                    }
                    span { class: "trade-user-name", "{who}" }
                }
                span { class: "pill status-{trade.status}", {app.tr(&status_key)} }
            }

            div { class: "trade-card-contents",
                // Left: What I receive
                div { class: "trade-column trade-col-get",
                    span { class: "trade-col-label", {app.tr("trade_you_get")} }
                    div { class: "trade-cards-scroll",
                        for it in get.iter() {
                            if let Some(c) = catalog.card(&it.card_id).cloned() {
                                div { class: "trade-thumb-wrap",
                                    CardThumb {
                                        key: "get-{it.card_id}",
                                        card: c,
                                        badge: Some(format!("+{}", it.qty)),
                                        small: true,
                                    }
                                }
                            }
                        }
                    }
                }

                // Center divider arrow
                div { class: "trade-exchange-arrow", "⇄" }

                // Right: What I give
                div { class: "trade-column trade-col-give",
                    span { class: "trade-col-label", {app.tr("trade_you_give")} }
                    div { class: "trade-cards-scroll",
                        for it in give.iter() {
                            if let Some(c) = catalog.card(&it.card_id).cloned() {
                                div { class: "trade-thumb-wrap",
                                    CardThumb {
                                        key: "give-{it.card_id}",
                                        card: c,
                                        badge: Some(format!("−{}", it.qty)),
                                        small: true,
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Action Buttons for Pending trades
            if trade.status == "pending" {
                div { class: "trade-card-actions",
                    if incoming {
                        button {
                            class: "btn btn-sm btn-outline",
                            onclick: move |_| on_action.call((id_reject.clone(), "reject")),
                            {app.tr("reject")}
                        }
                        button {
                            class: "btn btn-sm btn-primary",
                            onclick: move |_| on_action.call((id_accept.clone(), "accept")),
                            {app.tr("accept")}
                        }
                    } else {
                        button {
                            class: "btn btn-sm btn-outline danger-text",
                            onclick: move |_| on_action.call((id_cancel.clone(), "cancel")),
                            {app.tr("cancel")}
                        }
                    }
                }
            }
        }
    }
}

