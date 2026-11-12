//! Minimal Firestore REST client (no official Rust SDK exists).
//!
//! Documents are exchanged as `serde_json::Value` objects and converted to/from the
//! Firestore typed-value wire format, so models only need `Serialize`/`Deserialize`.

use serde::{de::DeserializeOwned, Serialize};
use serde_json::{json, Map, Value};

use super::{http, FbError};
use crate::config;

// ---------- value conversion ----------

pub fn to_fs_value(v: &Value) -> Value {
    match v {
        Value::Null => json!({ "nullValue": null }),
        Value::Bool(b) => json!({ "booleanValue": b }),
        Value::Number(n) => {
            if n.is_i64() || n.is_u64() {
                json!({ "integerValue": n.to_string() })
            } else {
                json!({ "doubleValue": n.as_f64() })
            }
        }
        Value::String(s) => json!({ "stringValue": s }),
        Value::Array(a) => json!({ "arrayValue": { "values": a.iter().map(to_fs_value).collect::<Vec<_>>() } }),
        Value::Object(o) => json!({ "mapValue": { "fields": to_fs_fields(o) } }),
    }
}

pub fn to_fs_fields(o: &Map<String, Value>) -> Value {
    let mut m = Map::new();
    for (k, v) in o {
        m.insert(k.clone(), to_fs_value(v));
    }
    Value::Object(m)
}

pub fn from_fs_value(v: &Value) -> Value {
    let Some(o) = v.as_object() else { return Value::Null };
    if let Some(s) = o.get("stringValue") {
        return s.clone();
    }
    if let Some(s) = o.get("integerValue") {
        let i: i64 = s.as_str().and_then(|s| s.parse().ok()).or_else(|| s.as_i64()).unwrap_or(0);
        return json!(i);
    }
    if let Some(d) = o.get("doubleValue") {
        return d.clone();
    }
    if let Some(b) = o.get("booleanValue") {
        return b.clone();
    }
    if let Some(t) = o.get("timestampValue") {
        return t.clone();
    }
    if let Some(r) = o.get("referenceValue") {
        return r.clone();
    }
    if let Some(a) = o.get("arrayValue") {
        let vals = a.get("values").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        return Value::Array(vals.iter().map(from_fs_value).collect());
    }
    if let Some(m) = o.get("mapValue") {
        return from_fs_fields(m.get("fields").unwrap_or(&Value::Null));
    }
    Value::Null
}

pub fn from_fs_fields(fields: &Value) -> Value {
    let mut m = Map::new();
    if let Some(o) = fields.as_object() {
        for (k, v) in o {
            m.insert(k.clone(), from_fs_value(v));
        }
    }
    Value::Object(m)
}

/// Parses a REST document into `T`, injecting the document id as field `id`.
fn parse_doc<T: DeserializeOwned>(doc: &Value) -> Option<T> {
    let name = doc.get("name")?.as_str()?;
    let id = name.rsplit('/').next()?.to_string();
    let mut v = from_fs_fields(doc.get("fields").unwrap_or(&Value::Null));
    v.as_object_mut()?.insert("id".into(), Value::String(id));
    serde_json::from_value(v).ok()
}

fn to_fields<T: Serialize>(data: &T) -> Result<Value, FbError> {
    let mut v = serde_json::to_value(data).map_err(|e| FbError::Other(e.to_string()))?;
    if let Some(o) = v.as_object_mut() {
        o.remove("id");
        Ok(to_fs_fields(o))
    } else {
        Err(FbError::Other("document must be an object".into()))
    }
}

pub fn doc_name(path: &str) -> String {
    format!(
        "projects/{}/databases/(default)/documents/{}",
        config::FIREBASE_PROJECT_ID,
        path
    )
}

// ---------- CRUD ----------

pub async fn get<T: DeserializeOwned>(token: &str, path: &str) -> Result<Option<T>, FbError> {
    let url = format!("{}/{}", config::firestore_base(), path);
    match http::get_json(&url, Some(token)).await {
        Ok(doc) => Ok(parse_doc(&doc)),
        Err(FbError::Http(404, _)) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Creates a document with an auto-generated id in `collection`, returning the id.
pub async fn add<T: Serialize>(token: &str, collection: &str, data: &T) -> Result<String, FbError> {
    let url = format!("{}/{}", config::firestore_base(), collection);
    let body = json!({ "fields": to_fields(data)? });
    let res = http::post_json(&url, Some(token), &body).await?;
    let name = res.get("name").and_then(|v| v.as_str()).unwrap_or_default();
    Ok(name.rsplit('/').next().unwrap_or_default().to_string())
}

/// Creates or fully replaces the document at `path`.
pub async fn set<T: Serialize>(token: &str, path: &str, data: &T) -> Result<(), FbError> {
    let url = format!("{}/{}", config::firestore_base(), path);
    let body = json!({ "fields": to_fields(data)? });
    http::patch_json(&url, Some(token), &body).await.map(|_| ())
}

/// Updates only the listed fields of the document at `path` (creates it if missing).
pub async fn update_fields(token: &str, path: &str, fields: Map<String, Value>) -> Result<(), FbError> {
    let mask: Vec<String> = fields
        .keys()
        .map(|k| format!("updateMask.fieldPaths={}", urlencoding::encode(k)))
        .collect();
    let url = format!("{}/{}?{}", config::firestore_base(), path, mask.join("&"));
    let body = json!({ "fields": to_fs_fields(&fields) });
    http::patch_json(&url, Some(token), &body).await.map(|_| ())
}

pub async fn delete(token: &str, path: &str) -> Result<(), FbError> {
    let url = format!("{}/{}", config::firestore_base(), path);
    http::delete(&url, Some(token)).await
}

/// Lists every document of a (sub)collection.
pub async fn list<T: DeserializeOwned>(token: &str, collection: &str) -> Result<Vec<T>, FbError> {
    let mut out = Vec::new();
    let mut page: Option<String> = None;
    loop {
        let mut url = format!("{}/{}?pageSize=300", config::firestore_base(), collection);
        if let Some(p) = &page {
            url.push_str(&format!("&pageToken={}", urlencoding::encode(p)));
        }
        let res = http::get_json(&url, Some(token)).await?;
        if let Some(docs) = res.get("documents").and_then(|d| d.as_array()) {
            out.extend(docs.iter().filter_map(parse_doc::<T>));
        }
        match res.get("nextPageToken").and_then(|v| v.as_str()) {
            Some(p) if !p.is_empty() => page = Some(p.to_string()),
            _ => break,
        }
    }
    Ok(out)
}

/// Writes several documents atomically (`documents:commit`).
pub async fn commit(token: &str, writes: Vec<Value>) -> Result<(), FbError> {
    let url = format!("{}:commit", config::firestore_base());
    http::post_json(&url, Some(token), &json!({ "writes": writes })).await.map(|_| ())
}

/// A write op for [`commit`] that merges `fields` into the document at `path`.
pub fn write_update(path: &str, fields: Map<String, Value>) -> Value {
    let mask: Vec<&String> = fields.keys().collect();
    json!({
        "update": { "name": doc_name(path), "fields": to_fs_fields(&fields) },
        "updateMask": { "fieldPaths": mask },
    })
}

// ---------- structured queries ----------

#[allow(dead_code)]
pub enum Op {
    Eq,
    Gte,
    Lt,
    Gt,
    In,
    ArrayContains,
}

impl Op {
    fn as_str(&self) -> &'static str {
        match self {
            Op::Eq => "EQUAL",
            Op::Gte => "GREATER_THAN_OR_EQUAL",
            Op::Lt => "LESS_THAN",
            Op::Gt => "GREATER_THAN",
            Op::In => "IN",
            Op::ArrayContains => "ARRAY_CONTAINS",
        }
    }
}

pub struct Query {
    collection: String,
    all_descendants: bool,
    filters: Vec<Value>,
    order: Vec<Value>,
    limit: Option<u32>,
    parent: Option<String>,
}

#[allow(dead_code)]
impl Query {
    pub fn new(collection: &str) -> Self {
        Self { collection: collection.into(), all_descendants: false, filters: vec![], order: vec![], limit: None, parent: None }
    }
    /// Collection-group query across every `collection` subcollection.
    pub fn group(collection: &str) -> Self {
        Self { all_descendants: true, ..Self::new(collection) }
    }
    /// Query a subcollection under `parent` (e.g. `users/{uid}`).
    pub fn under(parent: &str, collection: &str) -> Self {
        Self { parent: Some(parent.into()), ..Self::new(collection) }
    }
    pub fn filter(mut self, field: &str, op: Op, value: Value) -> Self {
        self.filters.push(json!({
            "fieldFilter": { "field": { "fieldPath": field }, "op": op.as_str(), "value": to_fs_value(&value) }
        }));
        self
    }
    pub fn order_by(mut self, field: &str, desc: bool) -> Self {
        self.order.push(json!({ "field": { "fieldPath": field }, "direction": if desc { "DESCENDING" } else { "ASCENDING" } }));
        self
    }
    pub fn limit(mut self, n: u32) -> Self {
        self.limit = Some(n);
        self
    }

    pub async fn run<T: DeserializeOwned>(self, token: &str) -> Result<Vec<T>, FbError> {
        let mut sq = json!({
            "from": [{ "collectionId": self.collection, "allDescendants": self.all_descendants }],
        });
        match self.filters.len() {
            0 => {}
            1 => sq["where"] = self.filters[0].clone(),
            _ => sq["where"] = json!({ "compositeFilter": { "op": "AND", "filters": self.filters } }),
        }
        if !self.order.is_empty() {
            sq["orderBy"] = Value::Array(self.order);
        }
        if let Some(l) = self.limit {
            sq["limit"] = json!(l);
        }
        let base = config::firestore_base();
        let url = match &self.parent {
            Some(p) => format!("{base}/{p}:runQuery"),
            None => format!("{base}:runQuery"),
        };
        let res = http::post_json(&url, Some(token), &json!({ "structuredQuery": sq })).await?;
        Ok(res
            .as_array()
            .map(|rows| rows.iter().filter_map(|r| r.get("document")).filter_map(parse_doc::<T>).collect())
            .unwrap_or_default())
    }
}

/// Prefix search helper: `field >= prefix AND field < prefix + \u{f8ff}`.
#[allow(dead_code)]
pub fn prefix_query(collection: &str, field: &str, prefix: &str) -> Query {
    Query::new(collection)
        .filter(field, Op::Gte, json!(prefix))
        .filter(field, Op::Lt, json!(format!("{prefix}\u{f8ff}")))
}
