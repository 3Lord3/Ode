use super::models::{ReferentsResponse, Song};
use super::ApiError;
use super::GeniusClient;

/// A single annotation ("decoding") for a lyric fragment
pub struct LyricAnnotation {
    pub fragment: String,
    pub text: String,
}

/// Song details plus lyrics (text_format=plain)
pub fn song(client: &GeniusClient, id: u64) -> Result<Song, ApiError> {
    client.get(&format!("/songs/{id}"), &[("text_format", "plain")])
}



/// Lyrics from the API fields (`lyrics_plaintext` / `lyrics`), if present at all
/// The real source is the embedded WebKitWebView (see `ui::song_page`), because the
/// current api.genius.com no longer returns lyrics
pub fn api_lyrics(song: &Song) -> Option<String> {
    song.lyrics_plaintext
        .as_deref()
        .or(song.lyrics.as_deref())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// Genius annotations for lyric lines. Fetches all referents via
/// `GET /referents?song_id=...` and takes the text of the first annotation of each
pub fn annotations(client: &GeniusClient, song_id: u64) -> Result<Vec<LyricAnnotation>, ApiError> {
    let resp: ReferentsResponse = client.get(
        "/referents",
        &[("song_id", &song_id.to_string()), ("per_page", "50")],
    )?;
    let mut out = Vec::new();
    for r in resp.referents {
        let text = r
            .annotations
            .first()
            .map(|a| dom_to_text(&a.body.dom))
            .unwrap_or_default();
        let text = text.trim();
        if text.is_empty() {
            continue;
        }
        out.push(LyricAnnotation {
            fragment: r.fragment.unwrap_or_default().trim().to_string(),
            text: text.to_string(),
        });
    }
    Ok(out)
}

/// Flattens the tree-like `body.dom` of an annotation into plain text
fn dom_to_text(v: &serde_json::Value) -> String {
    let mut out = String::new();
    fn walk(v: &serde_json::Value, out: &mut String) {
        match v {
            serde_json::Value::String(s) => out.push_str(s),
            serde_json::Value::Object(m) => {
                if let Some(serde_json::Value::String(s)) = m.get("content") {
                    out.push_str(s);
                }
                if let Some(serde_json::Value::Array(a)) = m.get("children") {
                    for c in a {
                        walk(c, out);
                    }
                }
            }
            serde_json::Value::Array(a) => {
                for c in a {
                    walk(c, out);
                }
            }
            _ => {}
        }
    }
    walk(v, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::dom_to_text;
    use serde_json::json;

    #[test]
    fn extracts_text_from_dom() {
        let dom = json!({ "tag":"root", "children":[
            { "tag":"p", "children":["Hello", { "content":", world" }] },
            { "tag":"a", "content":"" }
        ]});
        assert_eq!(dom_to_text(&dom), "Hello, world");
    }
}