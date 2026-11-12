use dioxus::prelude::*;

pub mod cropper;
pub use cropper::*;

use crate::config;
use crate::models::Card;
use crate::state::{use_app, Screen};

const LOGO: Asset = asset!("/assets/logo.svg");

/// Top bar: logo (left), search + hamburger (right). Shows a back arrow on sub-screens.
#[component]
pub fn Header() -> Element {
    let mut app = use_app();
    let depth = app.nav.read().len();
    rsx! {
        header { class: "header",
            div { class: "header-left",
                if depth > 1 {
                    button { class: "icon-btn", aria_label: app.tr("back"), onclick: move |_| app.back(), "‹" }
                }
                button { class: "logo-btn", onclick: move |_| app.open(Screen::Home),
                    img { src: LOGO, alt: "logo", class: "logo" }
                    span { class: "logo-text", {app.tr("app_name")} }
                }
            }
            div { class: "header-right",
                button { class: "icon-btn", aria_label: app.tr("search"),
                    onclick: move |_| app.open(Screen::Search),
                    svg { width: "22", height: "22", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2",
                        circle { cx: "11", cy: "11", r: "7" }
                        line { x1: "16.5", y1: "16.5", x2: "21", y2: "21" }
                    }
                }
                button { class: "icon-btn", aria_label: "menu",
                    onclick: move |_| { let v = *app.menu_open.read(); app.menu_open.set(!v); },
                    svg { width: "22", height: "22", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2",
                        line { x1: "3", y1: "6", x2: "21", y2: "6" }
                        line { x1: "3", y1: "12", x2: "21", y2: "12" }
                        line { x1: "3", y1: "18", x2: "21", y2: "18" }
                    }
                }
            }
        }
    }
}

/// Hamburger drawer: profile, my cards, create, settings, logout.
#[component]
pub fn SideMenu() -> Element {
    let mut app = use_app();
    let p = app.profile.read().clone();
    rsx! {
        div { class: "scrim", onclick: move |_| app.menu_open.set(false) }
        nav { class: "drawer",
            div { class: "drawer-head",
                Avatar { url: p.photo_url.clone(), size: 48 }
                div {
                    div { class: "drawer-name", "{p.nickname}" }
                    div { class: "muted small", "{p.greeting}" }
                }
            }
            button { class: "drawer-item", onclick: move |_| app.open(Screen::Profile), {app.tr("menu_profile")} }
            button { class: "drawer-item", onclick: move |_| app.open(Screen::Holdings), {app.tr("menu_holdings")} }
            button { class: "drawer-item", onclick: move |_| app.open(Screen::Create { initial_kind: None, initial_artist_id: None }), {app.tr("menu_create")} }
            button { class: "drawer-item", onclick: move |_| app.open(Screen::Settings), {app.tr("menu_settings")} }
            button { class: "drawer-item danger", onclick: move |_| { app.menu_open.set(false); app.logout(); }, {app.tr("menu_logout")} }
        }
    }
}

/// Bottom ad area. Web: Google AdSense when `ADSENSE_CLIENT` is configured.
/// iOS / Android: reserved banner slot (AdMob native SDK integration is a follow-up).
#[component]
pub fn AdBanner() -> Element {
    let app = use_app();
    let has_adsense = cfg!(target_arch = "wasm32") && !config::ADSENSE_CLIENT.is_empty();
    use_effect(move || {
        if has_adsense {
            let js = format!(
                r#"if(!window.__adsLoaded){{window.__adsLoaded=1;var s=document.createElement('script');s.async=true;s.crossOrigin='anonymous';s.src='https://pagead2.googlesyndication.com/pagead/js/adsbygoogle.js?client={c}';document.head.appendChild(s);}}(window.adsbygoogle=window.adsbygoogle||[]).push({{}});"#,
                c = config::ADSENSE_CLIENT
            );
            document::eval(&js);
        }
    });
    rsx! {
        footer { class: "ad-area",
            if has_adsense {
                ins { class: "adsbygoogle", style: "display:block;width:100%;height:60px",
                    "data-ad-client": config::ADSENSE_CLIENT, "data-ad-slot": config::ADSENSE_SLOT,
                    "data-ad-format": "horizontal", "data-full-width-responsive": "true" }
            } else {
                div { class: "ad-placeholder", span { class: "ad-tag", {app.tr("ad")} } "320 × 50" }
            }
        }
    }
}

#[component]
pub fn Avatar(url: String, size: u32) -> Element {
    let style = format!("width:{size}px;height:{size}px");
    rsx! {
        if url.is_empty() {
            div { class: "avatar avatar-empty", style: "{style}", "👤" }
        } else {
            img { class: "avatar", style: "{style}", src: "{url}", alt: "" }
        }
    }
}

/// Artist circular avatar: circle image if available; if not, circle with primary color and initial.
#[component]
pub fn ArtistAvatar(
    #[props(default)] url: String,
    #[props(default)] color: String,
    #[props(default)] name: String,
    #[props(default = 32)] size: u32,
) -> Element {
    let sz = size;
    let initial = name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_else(|| "★".to_string());
    let bg_color = if !color.trim().is_empty() {
        color.trim().to_string()
    } else {
        let hash: usize = name.bytes().map(|b| b as usize).sum();
        let fallback_colors = ["#7a6bff", "#ff7ab6", "#4a90e2", "#50e3c2", "#f5a623", "#ff495c", "#8e44ad"];
        fallback_colors[hash % fallback_colors.len()].to_string()
    };

    if !url.trim().is_empty() {
        rsx! {
            img {
                class: "artist-avatar-img",
                style: "width:{sz}px;height:{sz}px;border-radius:50%;object-fit:cover;flex-shrink:0;box-shadow:0 2px 8px rgba(0,0,0,0.3);",
                src: "{url}",
                alt: "{name}",
            }
        }
    } else {
        let font_size = (sz as f32 * 0.48).round() as u32;
        rsx! {
            div {
                class: "artist-avatar-circle",
                style: "width:{sz}px;height:{sz}px;border-radius:50%;background-color:{bg_color};display:inline-flex;align-items:center;justify-content:center;font-weight:700;font-size:{font_size}px;color:#ffffff;flex-shrink:0;box-shadow:0 2px 8px rgba(0,0,0,0.3);",
                "{initial}"
            }
        }
    }
}

/// Rarity badge: 5-point star with global card count inside.
/// Color scales from gray (common) -> rainbow (rare/epic) -> gold (legendary/ultra rare).
#[component]
pub fn RarityBadge(count: i64) -> Element {
    let tier_class = if count <= 1 {
        "rarity-star-badge rarity-gold"
    } else if count <= 4 {
        "rarity-star-badge rarity-rainbow-1"
    } else if count <= 9 {
        "rarity-star-badge rarity-rainbow-2"
    } else {
        "rarity-star-badge rarity-gray"
    };

    rsx! {
        div {
            class: "{tier_class}",
            title: format!("전체 보유량: {count}장"),
            svg {
                class: "rarity-star-svg",
                view_box: "0 0 24 24",
                path {
                    d: "M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z",
                }
            }
            span { class: "rarity-star-num", "{count}" }
        }
    }
}

/// Photocard thumbnail.
#[component]
pub fn CardThumb(
    card: Card,
    #[props(default)] badge: Option<String>,
    #[props(default)] small: bool,
    #[props(default)] rarity: Option<i64>,
) -> Element {
    let class = if small { "card-thumb small" } else { "card-thumb" };
    let font_family = match card.title_font.as_str() {
        "serif" => "Georgia, 'Nanum Myeongjo', serif",
        "cursive" => "'Brush Script MT', 'Nanum Pen Script', cursive, sans-serif",
        "monospace" => "ui-monospace, 'Courier New', monospace",
        "fantasy" => "Impact, sans-serif",
        _ => "inherit",
    };
    let color = if !card.title_color.is_empty() { card.title_color.as_str() } else { "inherit" };
    let title_style = format!("font-family:{font_family};color:{color};");
    rsx! {
        div { class: "{class}",
            if let Some(r) = rarity {
                RarityBadge { count: r }
            }
            if card.image_url.is_empty() {
                div { class: "card-img card-img-empty", "🃏" }
            } else {
                img { class: "card-img", src: "{card.image_url}", alt: "{card.name}", loading: "lazy" }
            }
            if let Some(b) = badge {
                span { class: "badge", "{b}" }
            }
            div { class: "card-name", style: "{title_style}", "{card.name}" }
        }
    }
}

/// − n + stepper.
#[component]
pub fn Stepper(value: i64, min: i64, max: i64, on_change: EventHandler<i64>) -> Element {
    rsx! {
        div { class: "stepper",
            button { disabled: value <= min, onclick: move |e| { e.stop_propagation(); on_change.call((value - 1).max(min)) }, "−" }
            span { "{value}" }
            button { disabled: value >= max, onclick: move |e| { e.stop_propagation(); on_change.call((value + 1).min(max)) }, "+" }
        }
    }
}
