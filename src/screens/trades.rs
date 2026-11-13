//! Trades Screen: Trade Requests & Accepted Trade History with tabs.

use std::collections::HashSet;
use std::rc::Rc;
use dioxus::prelude::*;

use crate::api;
use crate::app::reload_holdings;
use crate::components::CardThumb;
use crate::models::{Trade, TradeItem};
use crate::platform::storage;
use crate::state::*;

const KEY_BLOCKED_USERS: &str = "pe_blocked_users";

fn load_cached_blocked() -> HashSet<String> {
    storage::get(KEY_BLOCKED_USERS)
        .map(|s| s.split(',').filter(|x| !x.is_empty()).map(|x| x.to_string()).collect())
        .unwrap_or_default()
}

fn save_cached_blocked(set: &HashSet<String>) {
    let joined = set.iter().cloned().collect::<Vec<_>>().join(",");
    storage::set(KEY_BLOCKED_USERS, &joined);
}

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
    let mut is_processing = use_signal(|| false);
    let mut success_trade = use_signal(|| None::<(Trade, String)>);
    let mut active_chat = use_signal(|| None::<(Trade, String, String)>); // (trade, partner_name, partner_uid)
    let mut confirm_complete = use_signal(|| None::<(Trade, String)>); // (trade, partner_name)
    let mut confirm_block = use_signal(|| None::<(String, String)>); // (partner_uid, partner_name)
    let mut report_target = use_signal(|| None::<(String, String, Option<String>)>); // (partner_uid, partner_name, Option<trade_id>)

    let mut blocked_set = use_signal(load_cached_blocked);

    // Sync blocked users from Firestore
    let _remote_blocks = use_resource(move || async move {
        let uid = app.uid();
        let Ok(tok) = app.token().await else { return };
        if let Ok(remote) = api::load_blocked_users(&tok, &uid).await {
            let mut cur = blocked_set();
            let mut changed = false;
            for b in remote {
                if cur.insert(b) {
                    changed = true;
                }
            }
            if changed {
                save_cached_blocked(&cur);
                blocked_set.set(cur);
            }
        }
    });

    let mut trades = use_resource(move || async move {
        let uid = app.uid();
        let tok = app.token().await.ok()?;
        api::my_trades(&tok, &uid).await.ok()
    });

    let (inbox, outbox) = match trades.read().clone() {
        Some(Some((i, o))) => (i, o),
        _ => (vec![], vec![]),
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

    let names = Rc::new(profiles.read().clone().flatten().unwrap_or_default());
    let unknown_label = app.tr("unknown_user").to_string();
    let name_of = {
        let names = Rc::clone(&names);
        let unknown = unknown_label.clone();
        move |uid: &str| -> String {
            names.get(uid).map(|p| p.nickname.clone()).unwrap_or_else(|| unknown.clone())
        }
    };

    let inbox_for_act = inbox.clone();
    let name_of_for_act = name_of.clone();
    let act = Rc::new(move |id: String, action: &'static str| {
        let id_c = id.clone();
        let is_accept = action == "accept";
        let target_trade = if is_accept {
            inbox_for_act.iter().find(|t| t.id == id).cloned()
        } else {
            None
        };
        let partner_name = if let Some(ref t) = target_trade {
            name_of_for_act(&t.from_uid)
        } else {
            String::new()
        };

        spawn(async move {
            if is_accept {
                is_processing.set(true);
            }
            let Ok(tok) = app.token().await else {
                if is_accept {
                    is_processing.set(false);
                }
                return;
            };
            let res = match action {
                "accept" => api::accept_trade(&tok, &id_c).await,
                "reject" => api::set_trade_status(&tok, &id_c, "rejected").await,
                "complete" => api::set_trade_status(&tok, &id_c, "completed").await,
                _ => api::set_trade_status(&tok, &id_c, "cancelled").await,
            };
            if is_accept {
                is_processing.set(false);
            }
            match res {
                Ok(_) => {
                    trades.restart();
                    reload_holdings(app).await;
                    if is_accept {
                        if let Some(t) = target_trade {
                            success_trade.set(Some((t, partner_name)));
                        }
                    } else if action == "complete" {
                        app.show_toast(app.tr("status_completed"));
                    }
                }
                Err(e) => app.show_error(&e),
            }
        });
    });

    // Blocked set snapshot
    let blocked = blocked_set.read().clone();

    // Pending requests (filtered by blocked users)
    let pending_inbox: Vec<Trade> = inbox
        .iter()
        .filter(|t| t.status == "pending" && !blocked.contains(&t.from_uid))
        .cloned()
        .collect();
    let pending_outbox: Vec<Trade> = outbox
        .iter()
        .filter(|t| t.status == "pending" && !blocked.contains(&t.to_uid))
        .cloned()
        .collect();
    let pending_count = pending_inbox.len() + pending_outbox.len();

    // Accepted & Completed trades (both inbox & outbox, filtered by blocked users)
    let mut accepted_trades: Vec<(Trade, bool)> = Vec::new();
    for t in inbox.iter().filter(|t| (t.status == "accepted" || t.status == "completed") && !blocked.contains(&t.from_uid)) {
        accepted_trades.push((t.clone(), true));
    }
    for t in outbox.iter().filter(|t| (t.status == "accepted" || t.status == "completed") && !blocked.contains(&t.to_uid)) {
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
                                {
                                    let act_c = Rc::clone(&act);
                                    let who = name_of(&t.from_uid);
                                    let puid = t.from_uid.clone();
                                    let tid = t.id.clone();
                                    rsx! {
                                        TradeCard {
                                            key: "{t.id}",
                                            trade: t.clone(),
                                            who: who.clone(),
                                            partner_uid: puid.clone(),
                                            incoming: true,
                                            on_action: move |(id, a)| act_c(id, a),
                                            on_chat: move |_| {},
                                            on_complete: move |_| {},
                                            on_report: {
                                                let tid = tid.clone();
                                                move |(puid, pname)| {
                                                    report_target.set(Some((puid, pname, Some(tid.clone()))));
                                                }
                                            },
                                            on_block: move |(puid, pname)| {
                                                confirm_block.set(Some((puid, pname)));
                                            },
                                        }
                                    }
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
                                {
                                    let act_c = Rc::clone(&act);
                                    let who = name_of(&t.to_uid);
                                    let puid = t.to_uid.clone();
                                    let tid = t.id.clone();
                                    rsx! {
                                        TradeCard {
                                            key: "{t.id}",
                                            trade: t.clone(),
                                            who: who.clone(),
                                            partner_uid: puid.clone(),
                                            incoming: false,
                                            on_action: move |(id, a)| act_c(id, a),
                                            on_chat: move |_| {},
                                            on_complete: move |_| {},
                                            on_report: {
                                                let tid = tid.clone();
                                                move |(puid, pname)| {
                                                    report_target.set(Some((puid, pname, Some(tid.clone()))));
                                                }
                                            },
                                            on_block: move |(puid, pname)| {
                                                confirm_block.set(Some((puid, pname)));
                                            },
                                        }
                                    }
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
                                    let other_uid = if *inc { t.from_uid.clone() } else { t.to_uid.clone() };
                                    let partner_name = name_of(&other_uid);
                                    let act_c = Rc::clone(&act);
                                    let t_chat = t.clone();
                                    let t_complete = t.clone();
                                    let pname_chat = partner_name.clone();
                                    let pname_complete = partner_name.clone();
                                    let puid_chat = other_uid.clone();
                                    let tid = t.id.clone();
                                    rsx! {
                                        TradeCard {
                                            key: "{t.id}",
                                            trade: t.clone(),
                                            who: partner_name.clone(),
                                            partner_uid: other_uid.clone(),
                                            incoming: *inc,
                                            on_action: move |(id, a)| act_c(id, a),
                                            on_chat: move |_| {
                                                active_chat.set(Some((t_chat.clone(), pname_chat.clone(), puid_chat.clone())));
                                            },
                                            on_complete: move |_| {
                                                confirm_complete.set(Some((t_complete.clone(), pname_complete.clone())));
                                            },
                                            on_report: {
                                                let tid = tid.clone();
                                                move |(puid, pname)| {
                                                    report_target.set(Some((puid, pname, Some(tid.clone()))));
                                                }
                                            },
                                            on_block: move |(puid, pname)| {
                                                confirm_block.set(Some((puid, pname)));
                                            },
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
            }

            // Processing Dim Overlay
            if is_processing() {
                div { class: "trade-dim-overlay",
                    div { class: "trade-dim-card",
                        div { class: "spinner large" }
                        p { class: "trade-dim-text", {app.tr("trade_processing")} }
                    }
                }
            }

            // Trade Accepted Success Modal with Celebration Confetti
            if let Some((t, partner)) = success_trade() {
                TradeSuccessModal {
                    trade: t,
                    partner,
                    on_close: move |_| {
                        success_trade.set(None);
                        current_tab.set(TradeTab::Accepted);
                    },
                }
            }

            // Confirm Trade Complete Modal
            if let Some((t, partner_name)) = confirm_complete() {
                {
                    let tid = t.id.clone();
                    let act_c = Rc::clone(&act);
                    rsx! {
                        ConfirmCompleteModal {
                            partner_name,
                            on_cancel: move |_| confirm_complete.set(None),
                            on_confirm: move |_| {
                                confirm_complete.set(None);
                                act_c(tid.clone(), "complete");
                            },
                        }
                    }
                }
            }

            // Confirm Block User Modal
            if let Some((puid, pname)) = confirm_block() {
                BlockConfirmModal {
                    partner_name: pname.clone(),
                    on_cancel: move |_| confirm_block.set(None),
                    on_confirm: {
                        let puid = puid.clone();
                        move |_| {
                            confirm_block.set(None);
                            let mut cur = blocked_set();
                            cur.insert(puid.clone());
                            save_cached_blocked(&cur);
                            blocked_set.set(cur);
                            app.show_toast(app.tr("block_success"));
                            let puid_c = puid.clone();
                            spawn(async move {
                                if let Ok(tok) = app.token().await {
                                    let my_uid = app.uid();
                                    let _ = api::block_user(&tok, &my_uid, &puid_c).await;
                                }
                            });
                        }
                    },
                }
            }

            // Report User Modal
            if let Some((puid, pname, tid)) = report_target() {
                ReportModal {
                    partner_name: pname,
                    partner_uid: puid,
                    trade_id: tid,
                    on_cancel: move |_| report_target.set(None),
                    on_submitted: move |_| report_target.set(None),
                }
            }

            // Trade Chat Modal (Active Channel)
            if let Some((t, partner_name, partner_uid)) = active_chat() {
                TradeChatModal {
                    trade: t,
                    partner_name,
                    partner_uid,
                    on_close: move |_| active_chat.set(None),
                }
            }
        }
    }
}

#[component]
fn TradeCard(
    trade: Trade,
    who: String,
    partner_uid: String,
    incoming: bool,
    on_action: EventHandler<(String, &'static str)>,
    on_chat: EventHandler<()>,
    on_complete: EventHandler<()>,
    on_report: EventHandler<(String, String)>,
    on_block: EventHandler<(String, String)>,
) -> Element {
    let app = use_app();
    let catalog = app.catalog.read().clone();
    let status_key = format!("status_{}", trade.status);
    let mut show_menu = use_signal(|| false);

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
                div { class: "trade-header-right",
                    span { class: "pill status-{trade.status}", {app.tr(&status_key)} }
                    div { class: "trade-more-menu-wrap",
                        button {
                            class: "trade-more-btn",
                            title: "{app.tr(\"more\")}",
                            onclick: move |e| {
                                e.stop_propagation();
                                show_menu.set(!show_menu());
                            },
                            "···"
                        }
                        if show_menu() {
                            div {
                                class: "trade-menu-backdrop",
                                onclick: move |e| {
                                    e.stop_propagation();
                                    show_menu.set(false);
                                },
                            }
                            div { class: "trade-dropdown-menu",
                                button {
                                    class: "trade-dropdown-item",
                                    onclick: {
                                        let puid = partner_uid.clone();
                                        let w = who.clone();
                                        move |e| {
                                            e.stop_propagation();
                                            show_menu.set(false);
                                            on_report.call((puid.clone(), w.clone()));
                                        }
                                    },
                                    {app.tr("report_user")}
                                }
                                button {
                                    class: "trade-dropdown-item danger-text",
                                    onclick: {
                                        let puid = partner_uid.clone();
                                        let w = who.clone();
                                        move |e| {
                                            e.stop_propagation();
                                            show_menu.set(false);
                                            on_block.call((puid.clone(), w.clone()));
                                        }
                                    },
                                    {app.tr("block_user")}
                                }
                            }
                        }
                    }
                }
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

            // Action Buttons for Accepted / Completed trades
            if trade.status == "accepted" || trade.status == "completed" {
                div { class: "trade-card-actions",
                    button {
                        class: "btn btn-sm btn-outline",
                        onclick: move |_| on_chat.call(()),
                        {app.tr("chat_with_user")}
                    }
                    if trade.status == "accepted" {
                        button {
                            class: "btn btn-sm btn-primary",
                            onclick: move |_| on_complete.call(()),
                            {app.tr("trade_complete_btn")}
                        }
                    }
                }
            }
        }
    }
}

/// Modal shown when a trade is accepted, with celebration confetti burst.
#[component]
fn TradeSuccessModal(
    trade: Trade,
    partner: String,
    on_close: EventHandler<()>,
) -> Element {
    let app = use_app();
    let catalog = app.catalog.read().clone();
    let received = &trade.offered;

    rsx! {
        div { class: "scrim", onclick: move |_| on_close.call(()) }
        div { class: "trade-success-modal-wrap",
            div { class: "trade-success-modal",
                // Celebration Confetti Stage (16 particles animated via CSS)
                div { class: "confetti-stage",
                    for i in 0..16 {
                        div { class: "confetti-p confetti-p-{i}", key: "{i}" }
                    }
                }

                // Ribbon / Highlight Mark
                div { class: "trade-success-sparkle-mark" }

                h3 { class: "trade-success-title", {app.tr("trade_accepted_title")} }
                p { class: "trade-success-desc",
                    {app.tr("trade_accepted_desc").replace("{user}", &partner)}
                }

                // Received Cards Preview
                div { class: "trade-success-cards-section",
                    div { class: "trade-success-cards-label", {app.tr("trade_you_get")} }
                    div { class: "trade-success-cards-row",
                        for it in received.iter() {
                            if let Some(c) = catalog.card(&it.card_id).cloned() {
                                div { class: "trade-success-thumb-wrap", key: "{it.card_id}",
                                    CardThumb {
                                        card: c,
                                        badge: Some(format!("+{}", it.qty)),
                                        small: true,
                                    }
                                }
                            }
                        }
                    }
                }

                button {
                    class: "btn btn-primary block trade-success-confirm-btn",
                    onclick: move |_| on_close.call(()),
                    {app.tr("confirm")}
                }
            }
        }
    }
}

/// Modal dialog confirming completion of a trade.
#[component]
fn ConfirmCompleteModal(
    partner_name: String,
    on_confirm: EventHandler<()>,
    on_cancel: EventHandler<()>,
) -> Element {
    let app = use_app();
    rsx! {
        div { class: "scrim", onclick: move |_| on_cancel.call(()) }
        div { class: "confirm-modal-wrap",
            div { class: "confirm-modal-card",
                h3 { class: "confirm-modal-title", {app.tr("trade_complete_confirm_title")} }
                p { class: "confirm-modal-desc muted",
                    {app.tr("trade_complete_confirm_desc")}
                }
                div { class: "confirm-modal-actions",
                    button {
                        class: "btn btn-outline",
                        onclick: move |_| on_cancel.call(()),
                        {app.tr("cancel")}
                    }
                    button {
                        class: "btn btn-primary",
                        onclick: move |_| on_confirm.call(()),
                        {app.tr("trade_complete_btn")}
                    }
                }
            }
        }
    }
}

/// Realtime chat channel modal for a specific trade between two users.
#[component]
fn TradeChatModal(
    trade: Trade,
    partner_name: String,
    partner_uid: String,
    on_close: EventHandler<()>,
) -> Element {
    let mut app = use_app();
    let my_uid = app.uid();
    let mut input_text = use_signal(String::new);
    let sending = use_signal(|| false);

    let tid = trade.id.clone();
    let mut messages = use_resource(move || {
        let tid = tid.clone();
        async move {
            let tok = app.token().await.ok()?;
            api::load_trade_messages(&tok, &tid).await.ok()
        }
    });

    let puid = partner_uid.clone();
    let partner_profile = use_resource(move || {
        let puid = puid.clone();
        async move {
            let tok = app.token().await.ok()?;
            api::load_profile(&tok, &puid).await.ok().flatten()
        }
    });

    // Auto-poll messages every 3 seconds while modal is active
    use_future(move || async move {
        loop {
            crate::platform::sleep_ms(3000).await;
            messages.restart();
        }
    });

    let msg_list = messages.read().clone().flatten().unwrap_or_default();
    let partner_locs = partner_profile
        .read()
        .clone()
        .flatten()
        .map(|p| p.trade_locations)
        .unwrap_or_default();

    let tid_key = trade.id.clone();
    let tid_btn = trade.id.clone();

    rsx! {
        div { class: "scrim", onclick: move |_| on_close.call(()) }
        div { class: "chat-modal-wrap",
            div { class: "chat-modal-card",
                // Header
                div { class: "chat-modal-header",
                    div { class: "chat-header-title-box",
                        h3 { class: "chat-partner-name", "{partner_name}" }
                        span { class: "chat-subtitle muted small", {app.tr("chat_title")} }
                    }
                    button {
                        class: "chat-close-btn",
                        title: app.tr("cancel"),
                        onclick: move |_| on_close.call(()),
                        "✕"
                    }
                }

                // Partner's Trade Locations Notice Bar
                div { class: "chat-locations-banner",
                    span { class: "chat-loc-label", {app.tr("trade_locations")} ":" }
                    if partner_locs.is_empty() {
                        span { class: "chat-loc-all", {app.tr("trade_locations_all")} }
                    } else {
                        div { class: "chat-loc-chips",
                            for (idx, loc) in partner_locs.iter().enumerate() {
                                span { key: "{idx}", class: "chat-loc-chip", "{loc}" }
                            }
                        }
                    }
                }

                // Messages Scroll Area
                div { class: "chat-messages-area",
                    if msg_list.is_empty() {
                        div { class: "chat-empty-state muted small",
                            {app.tr("no_results")}
                        }
                    } else {
                        for m in msg_list.iter() {
                            {
                                let is_me = m.sender_uid == my_uid;
                                let time_str = {
                                    let dt = m.created_at;
                                    let hours = (dt % 86400) / 3600;
                                    let mins = (dt % 3600) / 60;
                                    format!("{:02}:{:02}", (hours + 9) % 24, mins) // KST
                                };
                                rsx! {
                                    div {
                                        key: "{m.id}",
                                        class: if is_me { "chat-msg-row me" } else { "chat-msg-row them" },
                                        if !is_me {
                                            div { class: "chat-sender-name", "{m.sender_name}" }
                                        }
                                        div { class: "chat-bubble-wrap",
                                            if is_me {
                                                span { class: "chat-time", "{time_str}" }
                                            }
                                            div { class: if is_me { "chat-bubble me" } else { "chat-bubble them" },
                                                "{m.text}"
                                            }
                                            if !is_me {
                                                span { class: "chat-time", "{time_str}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Input Area
                div { class: "chat-input-bar",
                    input {
                        id: "trade-chat-input",
                        class: "input chat-input",
                        initial_value: "",
                        placeholder: app.tr("chat_placeholder"),
                        maxlength: 300,
                        disabled: sending(),
                        autocorrect: "off",
                        autocapitalize: "off",
                        spellcheck: "false",
                        oninput: move |e| input_text.set(e.value()),
                        onkeydown: move |e: KeyboardEvent| {
                            if e.key() == Key::Enter {
                                send_chat_message(app, sending, input_text, messages, tid_key.clone());
                            }
                        },
                    }
                    button {
                        class: "btn btn-primary chat-send-btn",
                        disabled: sending() || input_text().trim().is_empty(),
                        onclick: move |_| send_chat_message(app, sending, input_text, messages, tid_btn.clone()),
                        if sending() { {app.tr("loading")} } else { {app.tr("chat_send")} }
                    }
                }
            }
        }
    }
}

fn send_chat_message(
    mut app: AppState,
    mut sending: Signal<bool>,
    mut input_text: Signal<String>,
    mut messages: Resource<Option<Vec<crate::models::ChatMessage>>>,
    trade_id: String,
) {
    let text = input_text().trim().to_string();
    if text.is_empty() || sending() {
        return;
    }
    sending.set(true);
    let uid = app.uid();
    let my_name = app.profile.read().nickname.clone();
    spawn(async move {
        let Ok(tok) = app.token().await else {
            sending.set(false);
            return;
        };
        match api::send_trade_message(&tok, &trade_id, &uid, &my_name, &text).await {
            Ok(_) => {
                input_text.set(String::new());
                let _ = document::eval("let el = document.getElementById('trade-chat-input'); if (el) el.value = '';");
                messages.restart();
            }
            Err(e) => app.show_error(&e),
        }
        sending.set(false);
    });
}

#[component]
fn BlockConfirmModal(
    partner_name: String,
    on_cancel: EventHandler<()>,
    on_confirm: EventHandler<()>,
) -> Element {
    let app = use_app();
    let desc = app.tf("block_confirm_desc", &[("user", partner_name.clone())]);
    rsx! {
        div { class: "modal-overlay", onclick: move |_| on_cancel.call(()),
            div { class: "modal-card block-confirm-card", onclick: move |e| e.stop_propagation(),
                h3 { class: "modal-title", {app.tr("block_confirm_title")} }
                p { class: "block-confirm-desc", "{desc}" }
                div { class: "modal-actions",
                    button {
                        class: "btn btn-outline",
                        onclick: move |_| on_cancel.call(()),
                        {app.tr("cancel")}
                    }
                    button {
                        class: "btn btn-danger",
                        onclick: move |_| on_confirm.call(()),
                        {app.tr("block_user")}
                    }
                }
            }
        }
    }
}

#[component]
fn ReportModal(
    partner_name: String,
    partner_uid: String,
    trade_id: Option<String>,
    on_cancel: EventHandler<()>,
    on_submitted: EventHandler<()>,
) -> Element {
    let mut app = use_app();
    let mut selected_reason = use_signal(|| "fraud".to_string());
    let mut details = use_signal(String::new);
    let mut is_submitting = use_signal(|| false);

    let reasons = [
        ("fraud", "report_reason_fraud"),
        ("abuse", "report_reason_abuse"),
        ("noshow", "report_reason_noshow"),
        ("spam", "report_reason_spam"),
        ("other", "report_reason_other"),
    ];

    rsx! {
        div { class: "modal-overlay", onclick: move |_| on_cancel.call(()),
            div { class: "modal-card report-modal-card", onclick: move |e| e.stop_propagation(),
                div { class: "modal-header-row",
                    h3 { class: "modal-title", "{partner_name} " {app.tr("report_modal_title")} }
                    button {
                        class: "modal-close-btn",
                        onclick: move |_| on_cancel.call(()),
                        "×"
                    }
                }

                div { class: "report-section",
                    label { class: "report-section-label", {app.tr("report_reason")} }
                    div { class: "report-reason-list",
                        for (code, key) in reasons {
                            button {
                                key: "{code}",
                                class: if selected_reason() == code { "report-reason-chip active" } else { "report-reason-chip" },
                                onclick: move |_| selected_reason.set(code.to_string()),
                                {app.tr(key)}
                            }
                        }
                    }
                }

                div { class: "report-section",
                    label { class: "report-section-label", {app.tr("description")} }
                    textarea {
                        class: "report-textarea",
                        placeholder: "{app.tr(\"report_details_placeholder\")}",
                        value: "{details()}",
                        oninput: move |e| details.set(e.value()),
                        rows: 4,
                    }
                }

                div { class: "modal-actions",
                    button {
                        class: "btn btn-outline",
                        onclick: move |_| on_cancel.call(()),
                        {app.tr("cancel")}
                    }
                    button {
                        class: "btn btn-primary",
                        disabled: is_submitting(),
                        onclick: {
                            let puid = partner_uid.clone();
                            let tid = trade_id.clone();
                            move |_| {
                                let puid_c = puid.clone();
                                let tid_c = tid.clone();
                                let reason = selected_reason();
                                let desc = details();
                                is_submitting.set(true);
                                spawn(async move {
                                    let Ok(tok) = app.token().await else {
                                        is_submitting.set(false);
                                        return;
                                    };
                                    let my_uid = app.uid();
                                    let report = crate::models::ReportEntry {
                                        id: String::new(),
                                        reporter_uid: my_uid,
                                        target_uid: puid_c,
                                        trade_id: tid_c,
                                        reason,
                                        details: desc,
                                        created_at: crate::platform::now_secs(),
                                    };
                                    match api::submit_report(&tok, &report).await {
                                        Ok(_) => {
                                            is_submitting.set(false);
                                            app.show_toast(app.tr("report_success"));
                                            on_submitted.call(());
                                        }
                                        Err(e) => {
                                            is_submitting.set(false);
                                            app.show_error(&e);
                                        }
                                    }
                                });
                            }
                        },
                        if is_submitting() {
                            span { class: "spinner small" }
                        }
                        {app.tr("report_submit")}
                    }
                }
            }
        }
    }
}



