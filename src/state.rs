//! Global app state shared through Dioxus context.

use dioxus::prelude::*;

use crate::firebase::{auth, auth::Session, FbError};
use crate::i18n::{self, Lang};
use crate::models::*;
use crate::platform::{self, storage};

pub const KEY_REFRESH: &str = "refresh_token";
pub const KEY_PROVIDER: &str = "provider";
pub const KEY_AUTO: &str = "auto_login";
pub const KEY_BIO: &str = "biometric";
pub const KEY_LANG: &str = "lang";
pub const KEY_EDIT_MODE: &str = "edit_mode";
pub const KEY_PROTECT_SINGLE_CARD: &str = "protect_single_card";

/// Screens of the single-screen app. There is no GNB; screens are pushed on a stack
/// and the header shows a back button when the stack is deeper than the home screen.
#[derive(Clone, Debug, PartialEq)]
pub enum Screen {
    Home,
    ArtistDetail { artist_id: String, category_id: Option<String> },
    CategoryOffers { category_id: String, artist_id: Option<String> },
    Trade { partner_uid: String, category_id: Option<String>, artist_id: Option<String> },
    Search,
    Profile,
    Trades,
    Holdings,
    Create { initial_kind: Option<String>, initial_artist_id: Option<String> },
    EditCard { card_id: String },
    EditArtist { artist_id: String },
    EditCategory { category_id: String },
    EditSeries { series_id: String },
    Settings,
}

#[derive(Clone, Copy)]
pub struct AppState {
    pub session: Signal<Option<Session>>,
    pub lang: Signal<Lang>,
    pub profile: Signal<Profile>,
    pub catalog: Signal<Catalog>,
    pub holdings: Signal<Vec<Holding>>,
    pub nav: Signal<Vec<Screen>>,
    pub menu_open: Signal<bool>,
    pub toast: Signal<Option<String>>,
    /// Becomes true once the startup auto-login attempt finished.
    pub booted: Signal<bool>,
    /// Global edit mode toggle (shows add/edit buttons when true).
    pub edit_mode: Signal<bool>,
    /// Protect cards with count == 1 from being set as tradable.
    pub protect_single_card: Signal<bool>,
    /// Global total count of cards across all users (for rarity star).
    pub global_card_counts: Signal<std::collections::HashMap<String, i64>>,
}

impl AppState {
    pub fn new() -> Self {
        let lang = storage::get(KEY_LANG).map(|c| Lang::from_code(&c)).unwrap_or(Lang::Ko);
        let edit_mode = storage::get(KEY_EDIT_MODE).map(|v| v == "true" || v == "1").unwrap_or(false);
        let protect_single_card = storage::get(KEY_PROTECT_SINGLE_CARD).map(|v| v == "true" || v == "1").unwrap_or(false);
        Self {
            session: Signal::new(None),
            lang: Signal::new(lang),
            profile: Signal::new(Profile::default()),
            catalog: Signal::new(Catalog::default()),
            holdings: Signal::new(vec![]),
            nav: Signal::new(vec![Screen::Home]),
            menu_open: Signal::new(false),
            toast: Signal::new(None),
            booted: Signal::new(false),
            edit_mode: Signal::new(edit_mode),
            protect_single_card: Signal::new(protect_single_card),
            global_card_counts: Signal::new(std::collections::HashMap::new()),
        }
    }

    pub fn tr(&self, key: &str) -> &'static str {
        i18n::t(*self.lang.read(), key)
    }

    pub fn uid(&self) -> String {
        self.session.read().as_ref().map(|s| s.uid.clone()).unwrap_or_default()
    }

    pub fn current(&self) -> Screen {
        self.nav.read().last().cloned().unwrap_or(Screen::Home)
    }

    pub fn push(&mut self, s: Screen) {
        self.menu_open.set(false);
        self.nav.write().push(s);
    }

    /// Opens a top-level screen from the hamburger menu (replaces the stack).
    pub fn open(&mut self, s: Screen) {
        self.menu_open.set(false);
        let mut nav = self.nav.write();
        nav.clear();
        nav.push(Screen::Home);
        if s != Screen::Home {
            nav.push(s);
        }
    }

    pub fn back(&mut self) {
        let mut nav = self.nav.write();
        if nav.len() > 1 {
            nav.pop();
        }
    }

    pub fn show_toast(&mut self, msg: impl Into<String>) {
        let msg = msg.into();
        self.toast.set(Some(msg.clone()));
        let mut toast = self.toast;
        // Root-scoped so the timer survives the screen that raised the toast being closed.
        dioxus::core::spawn_forever(async move {
            platform::sleep_ms(2500).await;
            if toast.peek().as_deref() == Some(msg.as_str()) {
                toast.set(None);
            }
        });
    }

    pub fn show_error(&mut self, e: &FbError) {
        let base = self.tr("error");
        self.show_toast(format!("{base}: {e}"));
    }

    pub fn set_lang(&mut self, l: Lang) {
        storage::set(KEY_LANG, l.code());
        self.lang.set(l);
    }

    /// Returns a valid id token, transparently refreshing it when close to expiry.
    pub async fn token(&mut self) -> Result<String, FbError> {
        let cur = self.session.peek().clone();
        let Some(s) = cur else { return Err(FbError::Other("not signed in".into())) };
        if s.expires_at > platform::now_secs() + 120 {
            return Ok(s.id_token);
        }
        let fresh = auth::refresh(&s.refresh_token, &s.provider).await?;
        if pref_bool(KEY_AUTO, true) {
            storage::secure_set(KEY_REFRESH, &fresh.refresh_token);
        }
        let tok = fresh.id_token.clone();
        self.session.set(Some(fresh));
        Ok(tok)
    }

    /// Called after any successful sign-in.
    pub fn on_signed_in(&mut self, s: Session, remember: bool) {
        storage::set(KEY_AUTO, if remember { "1" } else { "0" });
        if remember {
            storage::secure_set(KEY_REFRESH, &s.refresh_token);
            storage::set(KEY_PROVIDER, &s.provider);
        } else {
            storage::secure_remove(KEY_REFRESH);
        }
        self.session.set(Some(s));
        self.open(Screen::Home);
    }

    pub fn logout(&mut self) {
        storage::secure_remove(KEY_REFRESH);
        storage::remove(KEY_PROVIDER);
        self.session.set(None);
        self.profile.set(Profile::default());
        self.holdings.set(vec![]);
        self.open(Screen::Home);
    }
}

pub fn pref_bool(key: &str, default: bool) -> bool {
    storage::get(key).map(|v| v == "1").unwrap_or(default)
}

pub fn use_app() -> AppState {
    use_context::<AppState>()
}
