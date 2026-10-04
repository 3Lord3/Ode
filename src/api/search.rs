use super::models::{SearchResponse, SearchResult};
use super::ApiError;
use super::GeniusClient;

pub fn search(client: &GeniusClient, query: &str) -> Result<Vec<SearchResult>, ApiError> {
    let resp: SearchResponse = client.get("/search", &[("q", query)])?;
    Ok(resp.hits.into_iter().map(|h| h.result).collect())
}
