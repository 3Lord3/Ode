pub mod credentials;
pub mod models;
pub mod search;
pub mod song;

use std::sync::Arc;
use std::time::Duration;

use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, AUTHORIZATION};
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::cache::Cache;

pub const BASE: &str = "https://api.genius.com";

#[derive(Debug)]
pub enum ApiError {
    Auth(anyhow::Error),
    Http(reqwest::Error),
    Json(serde_json::Error),
    Message(String),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::Auth(e) => write!(f, "auth: {e}"),
            ApiError::Http(e) => write!(f, "network: {e}"),
            ApiError::Json(e) => write!(f, "unrecognized response: {e}"),
            ApiError::Message(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for ApiError {}

/// Genius API client: retries on 429 with exponential backoff, fails on 401
#[derive(Clone)]
pub struct GeniusClient {
    http: Client,
    token: String,
    cache: Arc<Cache>,
}

impl GeniusClient {
    pub fn new(token: String, cache: Arc<Cache>) -> Self {
        let http = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .build()
            .expect("reqwest client");
        Self { http, token, cache }
    }

    fn headers(&self) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(
            AUTHORIZATION,
            format!("Bearer {}", self.token).parse().unwrap(),
        );
        h
    }

    /// GET. Responses are served from the on-disk cache while fresh. On 429 retries
    /// up to 3 times with backoff; on 401 returns Auth
    pub fn get<T: DeserializeOwned>(
        &self,
        path: &str,
        params: &[(&str, &str)],
    ) -> Result<T, ApiError> {
        let key = cache_key(path, params);
        if let Some(text) = self.cache.get(&key) {
            if let Ok(v) = self.decode(&text, path) {
                return Ok(v);
            }
        }
        let mut retries = 3u32;
        let mut backoff = 0.5f32;
        loop {
            let resp = self
                .http
                .get(format!("{BASE}{path}"))
                .headers(self.headers())
                .query(params)
                .send()
                .map_err(ApiError::Http)?;

            match resp.status().as_u16() {
                200 => {
                    let text = resp.text().map_err(ApiError::Http)?;
                    self.cache.put(&key, &text);
                    return self.decode(&text, path);
                }
                429 if retries > 0 => {
                    std::thread::sleep(Duration::from_secs_f32(backoff));
                    backoff *= 2.0;
                    retries -= 1;
                }
                401 => return Err(ApiError::Auth(anyhow::anyhow!("401, invalid access token"))),
                s => return Err(ApiError::Message(format!("HTTP {s}"))),
            }
        }
    }

    fn decode<T: DeserializeOwned>(&self, text: &str, path: &str) -> Result<T, ApiError> {
        if path.starts_with("/songs/")
            || path == "/search"
            || path.starts_with("/artists/")
            || path == "/referents"
        {
            parse_envelope(text, path)
        } else {
            Ok(serde_json::from_str(text).map_err(ApiError::Json)?)
        }
    }
}

/// Cache key: the path plus the query parameters in a fixed order
fn cache_key(path: &str, params: &[(&str, &str)]) -> String {
    let q: Vec<String> = params.iter().map(|(k, v)| format!("{k}={v}")).collect();
    if q.is_empty() {
        path.to_string()
    } else {
        format!("{path}?{}", q.join("&"))
    }
}

/// Picks the right part of the API envelope and deserializes it into `T`
/// - `/search`: the whole `response`
/// - `/songs/{id}`: `response.song`
pub fn parse_envelope<T: DeserializeOwned>(text: &str, path: &str) -> Result<T, ApiError> {
    let mut v: Value = serde_json::from_str(text).map_err(ApiError::Json)?;
    let resp = v
        .get_mut("response")
        .ok_or_else(|| ApiError::Message("no `response` field".into()))?;
    if path.starts_with("/songs/") {
        let song = resp
            .get("song")
            .cloned()
            .ok_or_else(|| ApiError::Message("no `song` field".into()))?;
        return serde_json::from_value(song).map_err(ApiError::Json);
    }
    serde_json::from_value(resp.take()).map_err(ApiError::Json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::models::{SearchResponse, SearchResult, Song};

    #[test]
    fn envelope_search() {
        let text = r#"{"meta":{"status":200},"response":{"hits":[{"type":"song","result":{"id":7,"url":"https://g/x"}}]}}"#;
        let r: SearchResponse = parse_envelope(text, "/search").unwrap();
        match &r.hits[0].result {
            SearchResult::Song(s) => assert_eq!(s.id, 7),
            _ => panic!("expected song"),
        }
    }

    #[test]
    fn envelope_song() {
        let text = r#"{"meta":{"status":200},"response":{"song":{"id":3,"url":"https://g/y","lyrics":"hi"}}}"#;
        let s: Song = parse_envelope(text, "/songs/3").unwrap();
        assert_eq!(s.id, 3);
        assert_eq!(s.lyrics.as_deref(), Some("hi"));
    }
}
