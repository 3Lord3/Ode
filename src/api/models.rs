use serde::Deserialize;

// ---- Search ----

#[derive(Debug, Deserialize)]
pub struct SearchResponse {
    pub hits: Vec<Hit>,
}

#[derive(Debug, Deserialize)]
pub struct Hit {
    pub result: SearchResult,
}

/// `Song` is much larger than the other variants, so it goes into a `Box`
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum SearchResult {
    Song(Box<Song>),
    Artist(Artist),
    Album(Album),
}

// ---- Song ----

#[derive(Debug, Deserialize)]
pub struct Song {
    pub id: u64,
    pub url: String,
    pub title: Option<String>,
    pub full_title: Option<String>,
    pub release_date_for_display: Option<String>,
    pub song_art_image_url: Option<String>,
    pub primary_artist: Option<Artist>,
    /// All credited artists ("PORCHY & OTHER"), sent by the search API
    #[serde(default)]
    pub artist_names: Option<String>,
    /// Only filled by `GET /songs/{id}`
    #[serde(default)]
    pub lyrics: Option<String>,
    /// The current API returns lyrics only here, and often not at all
    #[serde(default, rename = "lyrics_plaintext")]
    pub lyrics_plaintext: Option<String>,
}

// ---- Artist ----

#[derive(Debug, Deserialize)]
pub struct Artist {
    pub name: String,
    pub image_url: Option<String>,
}

// ---- Album ----

#[derive(Debug, Deserialize)]
pub struct Album {
    pub full_title: Option<String>,
    pub cover_art_url: Option<String>,
}

// ---- Annotations (lyric referents) ----

/// Response of `GET /referents?song_id=...`
#[derive(Debug, Deserialize)]
pub struct ReferentsResponse {
    #[serde(default)]
    pub referents: Vec<Referent>,
}

#[derive(Debug, Deserialize)]
pub struct Referent {
    pub fragment: Option<String>,
    #[serde(default)]
    pub annotations: Vec<Annotation>,
}

#[derive(Debug, Deserialize)]
pub struct Annotation {
    pub body: AnnotationBody,
}

/// Annotation body: a tree-like DOM we extract the text from
#[derive(Debug, Deserialize)]
pub struct AnnotationBody {
    pub dom: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::parse_envelope;

    #[test]
    fn parse_search() {
        let text = r#"{"response": { "hits": [
            { "type":"song", "result": {
                "id": 1, "url":"https://genius.com/x", "title":"T",
                "song_art_image_url":"https://img/x.png", "explicit":true,
                "primary_artist": {"id":2,"name":"A" } } }
          ] } }"#;
        let r: SearchResponse = parse_envelope(text, "/search").unwrap();
        match &r.hits[0].result {
            SearchResult::Song(s) => {
                assert_eq!(s.id, 1);
                assert_eq!(s.primary_artist.as_ref().unwrap().name, "A");
            }
            _ => panic!("expected song"),
        }
    }

    #[test]
    fn parse_song_with_lyrics() {
        let text = r#"{"response": { "song": {
            "id": 5, "url":"https://genius.com/y",
            "lyrics_plaintext":"[Chorus]\nHello world" } } }"#;
        let s: Song = parse_envelope(text, "/songs/5").unwrap();
        assert_eq!(s.lyrics_plaintext.as_deref(), Some("[Chorus]\nHello world"));
    }
}