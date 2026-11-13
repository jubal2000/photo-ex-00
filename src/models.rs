//! Firestore data model.
//!
//! Shared catalogue (anyone signed in can add, see `firestore.rules`):
//!   artists/{id}      name
//!   categories/{id}   artistId, name
//!   series/{id}       artistId, categoryId, name
//!   cards/{id}        artistId, categoryId, seriesId, name, imageUrl
//!
//! Per user:
//!   users/{uid}                        profile
//!   users/{uid}/holdings/{cardId}      count, tradable, public (+ denormalized ids)
//!   users/{uid}/groupVisibility/{key}  public flag for an artist/category/series group
//!
//! Trades:
//!   trades/{id}  fromUid, toUid, offered[], requested[], status

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Profile {
    pub id: String,
    pub nickname: String,
    pub greeting: String,
    pub photo_url: String,
    pub favorite_artists: Vec<String>,
    pub trade_locations: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Artist {
    pub id: String,
    pub name: String,
    pub name_lower: String,
    pub image_url: String,
    pub color: String,
    pub description: String,
    pub created_by: String,
    pub created_at: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Category {
    pub id: String,
    pub artist_id: String,
    pub name: String,
    pub name_lower: String,
    pub image_url: String,
    pub description: String,
    pub created_by: String,
    pub created_at: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Series {
    pub id: String,
    pub artist_id: String,
    pub category_id: String,
    pub name: String,
    pub name_lower: String,
    pub image_url: String,
    pub description: String,
    pub created_by: String,
    pub created_at: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Card {
    pub id: String,
    pub artist_id: String,
    pub category_id: String,
    pub series_id: String,
    pub name: String,
    pub name_lower: String,
    pub image_url: String,
    pub description: String,
    pub title_font: String,
    pub title_color: String,
    pub sort_order: i64,
    pub created_by: String,
    pub created_at: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Holding {
    /// Document id == card id.
    pub id: String,
    pub owner_uid: String,
    pub card_id: String,
    pub artist_id: String,
    pub category_id: String,
    pub series_id: String,
    pub count: i64,
    pub tradable: i64,
    /// Card-level visibility chosen by the user.
    pub public: bool,
    /// Effective visibility = card public AND every parent group public.
    /// Queried by other users (collection-group query).
    pub visible: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GroupVisibility {
    /// `artist_<id>`, `category_<id>` or `series_<id>`.
    pub id: String,
    pub public: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TradeItem {
    pub card_id: String,
    pub qty: i64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Trade {
    pub id: String,
    pub from_uid: String,
    pub to_uid: String,
    pub offered: Vec<TradeItem>,
    pub requested: Vec<TradeItem>,
    /// pending | accepted | rejected | cancelled | completed
    pub status: String,
    pub created_at: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ChatMessage {
    pub id: String,
    pub trade_id: String,
    pub sender_uid: String,
    pub sender_name: String,
    pub text: String,
    pub created_at: u64,
}

/// The full shared catalogue, loaded once and refreshed after creations.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Catalog {
    pub artists: Vec<Artist>,
    pub categories: Vec<Category>,
    pub series: Vec<Series>,
    pub cards: Vec<Card>,
}

impl Catalog {
    pub fn artist(&self, id: &str) -> Option<&Artist> {
        self.artists.iter().find(|a| a.id == id)
    }
    pub fn category(&self, id: &str) -> Option<&Category> {
        self.categories.iter().find(|a| a.id == id)
    }
    pub fn series_of(&self, id: &str) -> Option<&Series> {
        self.series.iter().find(|a| a.id == id)
    }
    pub fn card(&self, id: &str) -> Option<&Card> {
        self.cards.iter().find(|a| a.id == id)
    }
    pub fn artist_name(&self, id: &str) -> String {
        self.artist(id).map(|a| a.name.clone()).unwrap_or_default()
    }
    pub fn category_name(&self, id: &str) -> String {
        self.category(id).map(|a| a.name.clone()).unwrap_or_default()
    }
    pub fn series_name(&self, id: &str) -> String {
        self.series_of(id).map(|a| a.name.clone()).unwrap_or_default()
    }
}
