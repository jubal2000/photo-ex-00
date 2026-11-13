use std::collections::BTreeMap;

use dioxus::prelude::*;

use crate::api;
use crate::components::{ArtistAvatar, Avatar, CardThumb};
use crate::models::Holding;
use crate::state::*;

/// Main screen: categories of each favourite artist (created by any user), with the
/// number of other users who have tradable cards in each category.
#[component]
pub fn Home() -> Element {
    let mut app = use_app();
    let favs = use_memo(move || app.profile.read().favorite_artists.clone());

    let offers = use_resource(move || async move {
        let favs = favs();
        let uid = app.uid();
        let tok = app.token().await.ok()?;
        api::tradable_for_artists(&tok, &favs, &uid).await.ok().map(|rows| api::offers_per_artist_category(&rows))
    });

    let catalog = app.catalog.read();
    let counts = offers.read().clone().flatten().unwrap_or_default();

    // 등록된 카드가 1개 이상 있는 관심 아티스트만 필터링 (등록된 카드가 하나도 없는 아티스트는 홈에 표시 X)
    let active_favs: Vec<String> = favs()
        .into_iter()
        .filter(|aid| {
            catalog.cards.iter().any(|k| {
                &k.artist_id == aid
                    || (k.artist_id.is_empty()
                        && catalog
                            .category(&k.category_id)
                            .map(|c| &c.artist_id == aid)
                            .unwrap_or(false))
            })
        })
        .collect();

    if active_favs.is_empty() {
        return rsx! {
            div { class: "empty",
                p { {app.tr("home_no_fav")} }
                button { class: "btn btn-primary", onclick: move |_| app.open(Screen::Profile), {app.tr("menu_profile")} }
            }
        };
    }

    rsx! {
        div { class: "home",
            for aid in active_favs {
                section { class: "artist-section", key: "{aid}",
                    {
                        let a_opt = catalog.artist(&aid);
                        let a_name = catalog.artist_name(&aid);
                        let a_img = a_opt.map(|a| a.image_url.clone()).unwrap_or_default();
                        let a_color = a_opt.map(|a| a.color.clone()).unwrap_or_default();
                        let aid_for_title = aid.clone();
                        rsx! {
                            h2 {
                                class: "section-title clickable artist-title-row",
                                onclick: move |_| app.push(Screen::ArtistDetail {
                                    artist_id: aid_for_title.clone(),
                                    category_id: None,
                                }),
                                ArtistAvatar {
                                    url: a_img,
                                    color: a_color,
                                    name: a_name.clone(),
                                    size: 30,
                                }
                                span { class: "artist-name-text", "{a_name}" }
                                span { class: "muted small arrow-icon", "›" }
                            }
                        }
                    }
                    {
                        let cats: Vec<_> = catalog.categories.iter().filter(|c| c.artist_id == aid || c.artist_id.is_empty()).cloned().collect();
                        if cats.is_empty() {
                            rsx! { p { class: "muted small", {app.tr("home_no_category")} } }
                        } else {
                            rsx! {
                                div { class: "category-grid",
                                    for c in cats {
                                        {
                                            let n = counts.get(&(aid.clone(), c.id.clone())).copied().unwrap_or(0);
                                            let cid = c.id.clone();
                                            let aid_for_tile = aid.clone();
                                            let cover = catalog.cards.iter().find(|k| k.artist_id == aid && k.category_id == c.id && !k.image_url.is_empty()).map(|k| k.image_url.clone())
                                                .or_else(|| catalog.cards.iter().find(|k| k.category_id == c.id && !k.image_url.is_empty()).map(|k| k.image_url.clone()));
                                            let series_n = catalog.series.iter().filter(|s| s.category_id == c.id && s.artist_id == aid).count();
                                            rsx! {
                                                button { key: "{c.id}", class: if n > 0 { "category-tile tradable" } else { "category-tile" },
                                                    onclick: move |_| app.push(Screen::ArtistDetail {
                                                        artist_id: aid_for_tile.clone(),
                                                        category_id: Some(cid.clone()),
                                                    }),
                                                    div { class: "category-cover",
                                                        if let Some(u) = cover { img { src: "{u}", alt: "" } } else { span { class: "cover-placeholder" } }
                                                    }
                                                    div { class: "category-name", "{c.name}" }
                                                    div { class: "muted small", "{series_n} " {app.tr("series")} }
                                                    if n > 0 {
                                                        span { class: "pill", {app.tr("home_tradable")} " {n}" }
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

/// Users who offer tradable cards in a category, with a "request trade" button each.
#[component]
pub fn CategoryOffers(category_id: String, artist_id: Option<String>) -> Element {
    let mut app = use_app();
    let cid = category_id.clone();
    let aid_res = artist_id.clone();
    let data = use_resource(move || {
        let cid = cid.clone();
        let aid = aid_res.clone();
        async move {
            let uid = app.uid();
            let tok = app.token().await.ok()?;
            let rows = api::offers_in_category(&tok, &cid, aid.as_deref(), &uid).await.ok()?;
            let mut by_user: BTreeMap<String, Vec<Holding>> = BTreeMap::new();
            for h in rows {
                by_user.entry(h.owner_uid.clone()).or_default().push(h);
            }
            let uids: Vec<String> = by_user.keys().cloned().collect();
            let profiles = api::load_profiles(&tok, &uids).await;
            Some((by_user, profiles))
        }
    });

    let catalog = app.catalog.read();
    let cat = catalog.category(&category_id).cloned().unwrap_or_default();
    let title_artist = artist_id.as_deref().unwrap_or(&cat.artist_id);
    let title_artist_name = catalog.artist_name(title_artist);
    let page_title = if title_artist_name.is_empty() {
        cat.name.clone()
    } else {
        format!("{title_artist_name} · {}", cat.name)
    };
    let mine: std::collections::HashSet<String> = app.holdings.read().iter().filter(|h| h.count > 0).map(|h| h.card_id.clone()).collect();

    rsx! {
        div { class: "page category-offers-page",
            div { class: "category-offers-head",
                if !cat.image_url.is_empty() {
                    img { class: "category-banner-img", src: "{cat.image_url}", alt: "{cat.name}" }
                }
                div { class: "category-offers-info",
                    h2 { class: "page-title", "{page_title}" }
                    if !cat.description.is_empty() {
                        div { class: "muted small category-offers-desc", "{cat.description}" }
                    }
                }
            }
            h3 { class: "sub-title", {app.tr("home_offers")} }
            match data.read().clone() {
                None => rsx! { p { class: "muted", {app.tr("loading")} } },
                Some(None) => rsx! { p { class: "muted", {app.tr("error")} } },
                Some(Some((by_user, _))) if by_user.is_empty() => rsx! { p { class: "muted", {app.tr("home_no_offers")} } },
                Some(Some((by_user, profiles))) => rsx! {
                    for (uid, items) in by_user {
                        {
                            let p = profiles.get(&uid).cloned().unwrap_or_default();
                            let name = if p.nickname.is_empty() { app.tr("unknown_user").to_string() } else { p.nickname.clone() };
                            let partner = uid.clone();
                            let cid2 = category_id.clone();
                            rsx! {
                                div { class: "offer-card", key: "{uid}",
                                    div { class: "offer-head",
                                        Avatar { url: p.photo_url.clone(), size: 36 }
                                        div { class: "grow",
                                            div { class: "bold", "{name}" }
                                            div { class: "muted small", "{p.greeting}" }
                                        }
                                        {
                                            let aid_c = artist_id.clone();
                                            rsx! {
                                                button { class: "btn btn-primary btn-sm",
                                                    onclick: move |_| app.push(Screen::Trade {
                                                        partner_uid: partner.clone(),
                                                        category_id: Some(cid2.clone()),
                                                        artist_id: aid_c.clone(),
                                                    }),
                                                    {app.tr("request_trade")}
                                                }
                                            }
                                        }
                                    }
                                    div { class: "h-scroll",
                                        for h in items {
                                            if let Some(card) = catalog.card(&h.card_id).cloned() {
                                                {
                                                    let badge = if mine.contains(&h.card_id) { format!("×{}", h.tradable) } else { format!("{} ×{}", app.tr("trade_need"), h.tradable) };
                                                    rsx! { CardThumb { key: "{h.card_id}", card, badge: Some(badge), small: true } }
                                                }
                                            }
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
