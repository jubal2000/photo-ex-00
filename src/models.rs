//! Firestore data model.
//!
//! Shared catalogue (anyone signed in can add, see `firestore.rules`):
//!   artists/{id}      name
//!   categories/{id}   artistId, name, cardShape, pinFirst, hideRarity
//!   series/{id}       artistId, categoryId, name, officialType
//!   cards/{id}        artistId, categoryId, seriesId, name, imageUrl, officialType
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
    pub email: Option<String>,
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
    /// How card images in this category are shown: `rect` (photocard, default) | `square` | `circle`.
    pub card_shape: String,
    /// Always listed before the other categories.
    pub pin_first: bool,
    /// Hide the rarity star on this category's cards.
    pub hide_rarity: bool,
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
    /// `official` | `sponsored` | `unofficial` (see [`OfficialType`]).
    pub official_type: String,
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
    /// `official` | `sponsored` | `unofficial` (see [`OfficialType`]).
    pub official_type: String,
    pub created_by: String,
    pub created_at: u64,
}

/// Who released a series / card.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OfficialType {
    /// Officially released by the agency.
    Official,
    /// Official sponsorship / collaboration goods.
    Sponsored,
    /// Made by users (fan-made goods etc.).
    Unofficial,
}

impl OfficialType {
    pub const ALL: [OfficialType; 3] = [OfficialType::Official, OfficialType::Sponsored, OfficialType::Unofficial];

    /// Unknown or empty values are treated as unofficial.
    pub fn from_code(s: &str) -> Self {
        match s {
            "official" => OfficialType::Official,
            "sponsored" => OfficialType::Sponsored,
            _ => OfficialType::Unofficial,
        }
    }
    pub fn code(self) -> &'static str {
        match self {
            OfficialType::Official => "official",
            OfficialType::Sponsored => "sponsored",
            OfficialType::Unofficial => "unofficial",
        }
    }
    /// i18n key of the label.
    pub fn label_key(self) -> &'static str {
        self.code()
    }
    /// Official or sponsored (not user-made).
    pub fn is_licensed(self) -> bool {
        self != OfficialType::Unofficial
    }
}

/// Display shape of card images.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageShape {
    /// Photocard rectangle (55:85).
    Rect,
    Square,
    Circle,
}

impl ImageShape {
    pub const ALL: [ImageShape; 3] = [ImageShape::Rect, ImageShape::Square, ImageShape::Circle];

    /// Unknown or empty values fall back to the photocard rectangle.
    pub fn from_code(s: &str) -> Self {
        match s {
            "square" => ImageShape::Square,
            "circle" => ImageShape::Circle,
            _ => ImageShape::Rect,
        }
    }
    pub fn code(self) -> &'static str {
        match self {
            ImageShape::Rect => "rect",
            ImageShape::Square => "square",
            ImageShape::Circle => "circle",
        }
    }
    /// i18n key of the label.
    pub fn label_key(self) -> &'static str {
        match self {
            ImageShape::Rect => "shape_rect",
            ImageShape::Square => "shape_square",
            ImageShape::Circle => "shape_circle",
        }
    }
    /// Extra CSS class for thumbnails / previews ("" for the default rectangle).
    pub fn css_class(self) -> &'static str {
        match self {
            ImageShape::Rect => "",
            ImageShape::Square => "square",
            ImageShape::Circle => "circle",
        }
    }
}

impl Category {
    pub fn shape(&self) -> ImageShape {
        ImageShape::from_code(&self.card_shape)
    }
}

impl Series {
    pub fn kind(&self) -> OfficialType {
        OfficialType::from_code(&self.official_type)
    }
}

impl Card {
    pub fn kind(&self) -> OfficialType {
        OfficialType::from_code(&self.official_type)
    }
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

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BlockEntry {
    pub id: String,
    pub blocked_uid: String,
    pub blocked_at: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ReportEntry {
    pub id: String,
    pub reporter_uid: String,
    pub target_uid: String,
    pub trade_id: Option<String>,
    pub reason: String,
    pub details: String,
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
    /// Sort key for categories: pinned ones first, then by name.
    pub fn category_sort_key(&self, id: &str) -> (bool, String) {
        self.category(id).map_or((true, String::new()), |c| (!c.pin_first, c.name_lower.clone()))
    }
    /// Whether the rarity star is shown for a card (off when its category hides it).
    pub fn shows_rarity(&self, card: &Card) -> bool {
        self.category(&card.category_id).map_or(true, |c| !c.hide_rarity)
    }
    /// Display shape of a card's image, set on its category.
    pub fn card_shape(&self, card: &Card) -> ImageShape {
        self.category(&card.category_id).map_or(ImageShape::Rect, |c| c.shape())
    }
    /// Copy without user-made (unofficial) series and cards, keeping official and
    /// sponsored ones (for the "official & sponsored only" option).
    pub fn official_only(&self) -> Catalog {
        Catalog {
            artists: self.artists.clone(),
            categories: self.categories.clone(),
            series: self.series.iter().filter(|s| s.kind().is_licensed()).cloned().collect(),
            cards: self
                .cards
                .iter()
                .filter(|c| c.kind().is_licensed() && self.series_of(&c.series_id).map_or(true, |s| s.kind().is_licensed()))
                .cloned()
                .collect(),
        }
    }
}
