//! App-level data operations on top of the Firestore REST client.

use std::collections::{HashMap, HashSet};

use serde_json::{json, Map, Value};

use crate::firebase::firestore::{self as fs, Op, Query};
use crate::firebase::{functions, FbError};
use crate::models::*;
use crate::platform::now_secs;

type R<T> = Result<T, FbError>;

// ---------------- catalogue ----------------

pub async fn load_catalog(tok: &str) -> R<Catalog> {
    let (a, c, s, k) = futures_util::join!(
        fs::list::<Artist>(tok, "artists"),
        fs::list::<Category>(tok, "categories"),
        fs::list::<Series>(tok, "series"),
        fs::list::<Card>(tok, "cards"),
    );
    let mut cat = Catalog { artists: a?, categories: c?, series: s?, cards: k? };
    cat.artists.sort_by(|x, y| x.name_lower.cmp(&y.name_lower));
    cat.categories.sort_by(|x, y| x.name_lower.cmp(&y.name_lower));
    cat.series.sort_by(|x, y| x.name_lower.cmp(&y.name_lower));
    cat.cards.sort_by(|x, y| {
        x.sort_order
            .cmp(&y.sort_order)
            .then_with(|| x.name_lower.cmp(&y.name_lower))
    });
    Ok(cat)
}

fn norm(s: &str) -> String {
    s.trim().to_lowercase()
}

pub enum CreateError {
    Duplicate,
    Fb(FbError),
}

impl From<FbError> for CreateError {
    fn from(e: FbError) -> Self {
        CreateError::Fb(e)
    }
}

pub async fn create_artist(
    tok: &str,
    uid: &str,
    cat: &Catalog,
    name: &str,
    image_url: &str,
    color: &str,
) -> Result<String, CreateError> {
    let lower = norm(name);
    if cat.artists.iter().any(|a| a.name_lower == lower) {
        return Err(CreateError::Duplicate);
    }
    let a = Artist {
        name: name.trim().into(),
        name_lower: lower,
        image_url: image_url.trim().into(),
        color: color.trim().into(),
        created_by: uid.into(),
        created_at: now_secs(),
        ..Default::default()
    };
    Ok(fs::add(tok, "artists", &a).await?)
}

pub async fn create_category(
    tok: &str,
    uid: &str,
    cat: &Catalog,
    artist_id: &str,
    name: &str,
    image_url: &str,
) -> Result<String, CreateError> {
    let lower = norm(name);
    if cat.categories.iter().any(|c| c.artist_id == artist_id && c.name_lower == lower) {
        return Err(CreateError::Duplicate);
    }
    let c = Category {
        artist_id: artist_id.into(),
        name: name.trim().into(),
        name_lower: lower,
        image_url: image_url.trim().into(),
        created_by: uid.into(),
        created_at: now_secs(),
        ..Default::default()
    };
    Ok(fs::add(tok, "categories", &c).await?)
}

pub async fn create_series(
    tok: &str,
    uid: &str,
    cat: &Catalog,
    artist_id: &str,
    category_id: &str,
    name: &str,
    image_url: &str,
    official_type: OfficialType,
) -> Result<String, CreateError> {
    let lower = norm(name);
    let cat_artist = cat.category(category_id).map(|c| c.artist_id.clone()).unwrap_or_default();
    let final_artist_id = if !cat_artist.is_empty() { cat_artist } else { artist_id.to_string() };
    if cat.series.iter().any(|s| s.artist_id == final_artist_id && s.category_id == category_id && s.name_lower == lower) {
        return Err(CreateError::Duplicate);
    }
    let s = Series {
        artist_id: final_artist_id,
        category_id: category_id.into(),
        name: name.trim().into(),
        name_lower: lower,
        image_url: image_url.trim().into(),
        official_type: official_type.code().into(),
        created_by: uid.into(),
        created_at: now_secs(),
        ..Default::default()
    };
    Ok(fs::add(tok, "series", &s).await?)
}

pub async fn create_card(tok: &str, uid: &str, cat: &Catalog, series_id: &str, name: &str, image_url: &str, official_type: OfficialType) -> Result<String, CreateError> {
    let lower = norm(name);
    if cat.cards.iter().any(|c| c.series_id == series_id && c.name_lower == lower) {
        return Err(CreateError::Duplicate);
    }
    let s = cat.series_of(series_id).cloned().unwrap_or_default();
    let next_sort_order = cat
        .cards
        .iter()
        .filter(|c| c.series_id == series_id)
        .map(|c| c.sort_order)
        .max()
        .map(|m| m + 1)
        .unwrap_or(0);
    let c = Card {
        artist_id: s.artist_id,
        category_id: s.category_id,
        series_id: series_id.into(),
        name: name.trim().into(),
        name_lower: lower,
        image_url: image_url.into(),
        sort_order: next_sort_order,
        official_type: official_type.code().into(),
        created_by: uid.into(),
        created_at: now_secs(),
        ..Default::default()
    };
    Ok(fs::add(tok, "cards", &c).await?)
}

pub async fn update_card_sort_orders(tok: &str, updates: &[(String, i64)]) -> R<()> {
    let futures = updates.iter().map(|(card_id, order)| {
        let mut fields = Map::new();
        fields.insert("sortOrder".into(), json!(order));
        let path = format!("cards/{card_id}");
        async move {
            fs::update_fields(tok, &path, fields).await
        }
    });
    let results = futures_util::future::join_all(futures).await;
    for r in results {
        r?;
    }
    Ok(())
}

pub async fn update_card(
    tok: &str,
    card_id: &str,
    name: &str,
    image_url: &str,
    description: &str,
    title_font: &str,
    title_color: &str,
    official_type: OfficialType,
) -> R<()> {
    let lower = norm(name);
    let mut fields = Map::new();
    fields.insert("name".into(), json!(name.trim()));
    fields.insert("nameLower".into(), json!(lower));
    fields.insert("imageUrl".into(), json!(image_url.trim()));
    fields.insert("description".into(), json!(description.trim()));
    fields.insert("titleFont".into(), json!(title_font.trim()));
    fields.insert("titleColor".into(), json!(title_color.trim()));
    fields.insert("officialType".into(), json!(official_type.code()));
    fs::update_fields(tok, &format!("cards/{card_id}"), fields).await
}

pub async fn delete_card(tok: &str, card_id: &str) -> R<()> {
    fs::delete(tok, &format!("cards/{card_id}")).await
}

pub async fn update_artist(
    tok: &str,
    artist_id: &str,
    name: &str,
    image_url: &str,
    color: &str,
    description: &str,
) -> R<()> {
    let lower = norm(name);
    let mut fields = Map::new();
    fields.insert("name".into(), json!(name.trim()));
    fields.insert("nameLower".into(), json!(lower));
    fields.insert("imageUrl".into(), json!(image_url.trim()));
    fields.insert("color".into(), json!(color.trim()));
    fields.insert("description".into(), json!(description.trim()));
    fs::update_fields(tok, &format!("artists/{artist_id}"), fields).await
}

pub async fn update_category(
    tok: &str,
    category_id: &str,
    name: &str,
    image_url: &str,
    description: &str,
    square_cards: bool,
) -> R<()> {
    let lower = norm(name);
    let mut fields = Map::new();
    fields.insert("name".into(), json!(name.trim()));
    fields.insert("nameLower".into(), json!(lower));
    fields.insert("imageUrl".into(), json!(image_url.trim()));
    fields.insert("description".into(), json!(description.trim()));
    fields.insert("squareCards".into(), json!(square_cards));
    fs::update_fields(tok, &format!("categories/{category_id}"), fields).await
}

pub async fn update_series(
    tok: &str,
    series_id: &str,
    name: &str,
    image_url: &str,
    description: &str,
    official_type: OfficialType,
) -> R<()> {
    let lower = norm(name);
    let mut fields = Map::new();
    fields.insert("name".into(), json!(name.trim()));
    fields.insert("nameLower".into(), json!(lower));
    fields.insert("imageUrl".into(), json!(image_url.trim()));
    fields.insert("description".into(), json!(description.trim()));
    fields.insert("officialType".into(), json!(official_type.code()));
    fs::update_fields(tok, &format!("series/{series_id}"), fields).await
}

// ---------------- profile ----------------

pub async fn load_profile(tok: &str, uid: &str) -> R<Option<Profile>> {
    fs::get::<Profile>(tok, &format!("users/{uid}")).await
}

pub async fn save_profile(tok: &str, uid: &str, p: &Profile) -> R<()> {
    fs::set(tok, &format!("users/{uid}"), p).await
}

pub async fn load_profiles(tok: &str, uids: &[String]) -> HashMap<String, Profile> {
    let futs = uids.iter().map(|u| async move { (u.clone(), load_profile(tok, u).await.ok().flatten()) });
    futures_util::future::join_all(futs)
        .await
        .into_iter()
        .filter_map(|(u, p)| p.map(|p| (u, p)))
        .collect()
}

// ---------------- holdings ----------------

pub async fn load_holdings(tok: &str, uid: &str) -> R<Vec<Holding>> {
    fs::list::<Holding>(tok, &format!("users/{uid}/holdings")).await
}

pub async fn load_group_visibility(tok: &str, uid: &str) -> R<HashMap<String, bool>> {
    Ok(fs::list::<GroupVisibility>(tok, &format!("users/{uid}/groupVisibility"))
        .await?
        .into_iter()
        .map(|g| (g.id, g.public))
        .collect())
}

pub fn group_keys(h: &Holding) -> [String; 3] {
    [format!("artist_{}", h.artist_id), format!("category_{}", h.category_id), format!("series_{}", h.series_id)]
}

pub fn effective_visible(h: &Holding, groups: &HashMap<String, bool>) -> bool {
    h.public && group_keys(h).iter().all(|k| *groups.get(k).unwrap_or(&true))
}

pub async fn add_holding(tok: &str, uid: &str, card: &Card, groups: &HashMap<String, bool>) -> R<Holding> {
    let mut h = Holding {
        id: card.id.clone(),
        owner_uid: uid.into(),
        card_id: card.id.clone(),
        artist_id: card.artist_id.clone(),
        category_id: card.category_id.clone(),
        series_id: card.series_id.clone(),
        count: 1,
        tradable: 0,
        public: true,
        visible: true,
    };
    h.visible = effective_visible(&h, groups);
    fs::set(tok, &format!("users/{uid}/holdings/{}", card.id), &h).await?;
    Ok(h)
}

pub async fn save_holding(tok: &str, uid: &str, h: &Holding) -> R<()> {
    fs::set(tok, &format!("users/{uid}/holdings/{}", h.card_id), h).await
}

pub async fn delete_holding(tok: &str, uid: &str, card_id: &str) -> R<()> {
    fs::delete(tok, &format!("users/{uid}/holdings/{card_id}")).await
}

/// Sets a group's visibility and recomputes `visible` on every affected holding atomically.
pub async fn set_group_visibility(
    tok: &str,
    uid: &str,
    key: &str,
    public: bool,
    groups: &mut HashMap<String, bool>,
    holdings: &mut [Holding],
) -> R<()> {
    groups.insert(key.to_string(), public);
    let mut writes = vec![fs::write_update(&format!("users/{uid}/groupVisibility/{key}"), obj(json!({ "public": public })))];
    for h in holdings.iter_mut() {
        if group_keys(h).iter().any(|k| k == key) {
            let v = effective_visible(h, groups);
            if v != h.visible {
                h.visible = v;
                writes.push(fs::write_update(&format!("users/{uid}/holdings/{}", h.card_id), obj(json!({ "visible": v }))));
            }
        }
    }
    for chunk in writes.chunks(450) {
        fs::commit(tok, chunk.to_vec()).await?;
    }
    Ok(())
}

fn obj(v: Value) -> Map<String, Value> {
    v.as_object().cloned().unwrap_or_default()
}

// ---------------- discovery (home) ----------------

/// Other users' visible holdings (any count) for the given artists.
///
/// Non-tradable holdings are included on purpose: they tell us which cards a user
/// already owns, so we can skip users with whom no real trade is possible.
/// `tradable >= 0` matches every holding and lets the query reuse the existing
/// (artistId, visible, tradable) collection-group index.
pub async fn visible_for_artists(tok: &str, artist_ids: &[String], my_uid: &str) -> R<Vec<Holding>> {
    if artist_ids.is_empty() {
        return Ok(vec![]);
    }
    let mut out = vec![];
    for chunk in artist_ids.chunks(30) {
        let rows: Vec<Holding> = Query::group("holdings")
            .filter("artistId", Op::In, json!(chunk))
            .filter("visible", Op::Eq, json!(true))
            .filter("tradable", Op::Gte, json!(0))
            .limit(5000)
            .run(tok)
            .await?;
        out.extend(rows.into_iter().filter(|h| h.owner_uid != my_uid));
    }
    Ok(out)
}

/// Other users' visible holdings (any count) in a category, optionally for one artist.
/// Uses the (categoryId, visible, tradable) index; the artist is filtered client-side.
pub async fn visible_in_category(tok: &str, category_id: &str, artist_id: Option<&str>, my_uid: &str) -> R<Vec<Holding>> {
    let rows: Vec<Holding> = Query::group("holdings")
        .filter("categoryId", Op::Eq, json!(category_id))
        .filter("visible", Op::Eq, json!(true))
        .filter("tradable", Op::Gte, json!(0))
        .limit(5000)
        .run(tok)
        .await?;
    let aid = artist_id.filter(|a| !a.is_empty());
    Ok(rows
        .into_iter()
        .filter(|h| h.owner_uid != my_uid && aid.map_or(true, |a| h.artist_id == a))
        .collect())
}

/// Users with whom a real trade exists, grouped by `key` (e.g. category), mapped to
/// how many 1:1 pairs an auto-match would make with them.
///
/// A pair is real when, within the same group, the partner gives a card I don't own
/// and I give a card the partner doesn't own (one copy per card, "protect single card"
/// respected — same rules as the trade screen's auto-match). So the pair count is
/// `min(cards they can give me, cards I can give them)`; users with 0 are left out.
pub fn mutual_partners_by<K, F>(others: &[Holding], mine: &[Holding], protect_single: bool, key: F) -> HashMap<K, HashMap<String, usize>>
where
    K: std::hash::Hash + Eq + Clone,
    F: Fn(&Holding) -> K,
{
    #[derive(Default)]
    struct Side {
        owned: HashSet<String>,
        give: HashSet<String>,
    }
    let my_owned: HashSet<&str> = mine.iter().filter(|h| h.count > 0).map(|h| h.card_id.as_str()).collect();
    let mut my_give: HashMap<K, HashSet<String>> = HashMap::new();
    for h in mine {
        if h.tradable > 0 && !(protect_single && h.count <= 1) {
            my_give.entry(key(h)).or_default().insert(h.card_id.clone());
        }
    }
    let mut theirs: HashMap<(K, String), Side> = HashMap::new();
    for h in others {
        let side = theirs.entry((key(h), h.owner_uid.clone())).or_default();
        if h.count > 0 {
            side.owned.insert(h.card_id.clone());
        }
        if h.tradable > 0 {
            side.give.insert(h.card_id.clone());
        }
    }
    let mut out: HashMap<K, HashMap<String, usize>> = HashMap::new();
    for ((k, uid), side) in theirs {
        let Some(mine_give) = my_give.get(&k) else { continue };
        let they_offer_new = side.give.iter().filter(|c| !my_owned.contains(c.as_str())).count();
        let i_offer_new = mine_give.iter().filter(|c| !side.owned.contains(*c)).count();
        let n = they_offer_new.min(i_offer_new);
        if n > 0 {
            out.entry(k).or_default().insert(uid, n);
        }
    }
    out
}

/// A partner's visible, tradable holdings (for the trade screen).
pub async fn partner_tradable(tok: &str, partner_uid: &str) -> R<Vec<Holding>> {
    Query::under(&format!("users/{partner_uid}"), "holdings")
        .filter("visible", Op::Eq, json!(true))
        .filter("tradable", Op::Gt, json!(0))
        .run(tok)
        .await
}

/// A partner's visible holdings of any count (to mark which of my cards they lack).
pub async fn partner_visible(tok: &str, partner_uid: &str) -> R<Vec<Holding>> {
    Query::under(&format!("users/{partner_uid}"), "holdings")
        .filter("visible", Op::Eq, json!(true))
        .run(tok)
        .await
}

// ---------------- trades ----------------

pub async fn create_trade(tok: &str, from: &str, to: &str, offered: Vec<TradeItem>, requested: Vec<TradeItem>) -> R<String> {
    let t = Trade {
        from_uid: from.into(),
        to_uid: to.into(),
        offered,
        requested,
        status: "pending".into(),
        created_at: now_secs(),
        ..Default::default()
    };
    fs::add(tok, "trades", &t).await
}

pub async fn my_trades(tok: &str, uid: &str) -> R<(Vec<Trade>, Vec<Trade>)> {
    let (inbox, outbox) = futures_util::join!(
        Query::new("trades").filter("toUid", Op::Eq, json!(uid)).limit(100).run::<Trade>(tok),
        Query::new("trades").filter("fromUid", Op::Eq, json!(uid)).limit(100).run::<Trade>(tok),
    );
    let mut inbox = inbox?;
    let mut outbox = outbox?;
    inbox.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    outbox.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok((inbox, outbox))
}

pub async fn set_trade_status(tok: &str, trade_id: &str, status: &str) -> R<()> {
    fs::update_fields(tok, &format!("trades/{trade_id}"), obj(json!({ "status": status }))).await
}

pub async fn accept_trade(tok: &str, trade_id: &str) -> R<()> {
    functions::accept_trade(tok, trade_id).await
}

pub async fn load_trade_messages(tok: &str, trade_id: &str) -> R<Vec<ChatMessage>> {
    let mut msgs = fs::list::<ChatMessage>(tok, &format!("trades/{trade_id}/messages")).await?;
    msgs.sort_by(|a, b| a.created_at.cmp(&b.created_at));
    Ok(msgs)
}

pub async fn send_trade_message(
    tok: &str,
    trade_id: &str,
    sender_uid: &str,
    sender_name: &str,
    text: &str,
) -> R<String> {
    let msg = ChatMessage {
        id: String::new(),
        trade_id: trade_id.into(),
        sender_uid: sender_uid.into(),
        sender_name: sender_name.into(),
        text: text.trim().into(),
        created_at: now_secs(),
    };
    fs::add(tok, &format!("trades/{trade_id}/messages"), &msg).await
}

// ---------------- moderation: block & report ----------------

pub async fn block_user(tok: &str, my_uid: &str, target_uid: &str) -> R<()> {
    let entry = BlockEntry {
        id: target_uid.into(),
        blocked_uid: target_uid.into(),
        blocked_at: now_secs(),
    };
    fs::set(tok, &format!("users/{my_uid}/blocks/{target_uid}"), &entry).await
}

#[allow(dead_code)]
pub async fn unblock_user(tok: &str, my_uid: &str, target_uid: &str) -> R<()> {
    fs::delete(tok, &format!("users/{my_uid}/blocks/{target_uid}")).await
}

pub async fn load_blocked_users(tok: &str, my_uid: &str) -> R<HashSet<String>> {
    let list = fs::list::<BlockEntry>(tok, &format!("users/{my_uid}/blocks")).await?;
    Ok(list.into_iter().map(|b| b.blocked_uid).collect())
}

pub async fn submit_report(tok: &str, report: &ReportEntry) -> R<String> {
    fs::add(tok, "reports", report).await
}

pub async fn load_global_card_counts(tok: &str, my_holdings: &[Holding]) -> HashMap<String, i64> {
    let mut map: HashMap<String, i64> = HashMap::new();
    for h in my_holdings {
        if h.count > 0 {
            *map.entry(h.card_id.clone()).or_insert(0) += h.count;
        }
    }
    if let Ok(rows) = Query::group("holdings")
        .filter("visible", Op::Eq, json!(true))
        .limit(1000)
        .run::<Holding>(tok)
        .await
    {
        for h in rows {
            if !my_holdings.iter().any(|m| m.owner_uid == h.owner_uid && m.card_id == h.card_id) {
                if h.count > 0 {
                    *map.entry(h.card_id.clone()).or_insert(0) += h.count;
                }
            }
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(owner: &str, card: &str, count: i64, tradable: i64) -> Holding {
        Holding { owner_uid: owner.into(), card_id: card.into(), category_id: "cat".into(), count, tradable, ..Default::default() }
    }

    fn partners(others: &[Holding], mine: &[Holding], protect: bool) -> HashSet<String> {
        mutual_partners_by(others, mine, protect, |_| ()).remove(&()).unwrap_or_default().into_keys().collect()
    }

    #[test]
    fn pair_count_is_min_of_both_sides() {
        // I can give A, B (u1 lacks both); u1 gives C, D, E but I already own E.
        let mine = vec![h("me", "A", 2, 1), h("me", "B", 3, 2), h("me", "E", 1, 0)];
        let others = vec![h("u1", "C", 1, 1), h("u1", "D", 4, 3), h("u1", "E", 2, 1)];
        let counts = mutual_partners_by(&others, &mine, false, |_| ()).remove(&()).unwrap();
        assert_eq!(counts.get("u1"), Some(&2));
    }

    #[test]
    fn mutual_trade_filter() {
        let mine = vec![h("me", "A", 2, 1), h("me", "B", 1, 0)];
        let others = vec![
            // ok: offers C (I lack it), lacks A
            h("u1", "C", 2, 1),
            // offers only B, which I already own
            h("u2", "B", 2, 1),
            // offers C but already owns A (my only tradable card)
            h("u3", "C", 2, 1),
            h("u3", "A", 1, 0),
            // owns cards but offers nothing
            h("u4", "D", 1, 0),
        ];
        let p = partners(&others, &mine, false);
        assert_eq!(p, HashSet::from(["u1".to_string()]));

        // Protect single card: my only tradable card with count 1 cannot be offered.
        let mine_single = vec![h("me", "A", 1, 1)];
        assert!(partners(&others, &mine_single, true).is_empty());
        // Without B in my collection, u2's B is now worth receiving too.
        assert_eq!(partners(&others, &mine_single, false), HashSet::from(["u1".to_string(), "u2".to_string()]));
    }
}
