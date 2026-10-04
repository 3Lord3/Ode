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

pub fn description_plain(song: &Song) -> Option<String> {
    use crate::api::models::Description;
    let text = match song.description.as_ref()? {
        Description::Html(h) => strip_tags(h),
        Description::Object { plain, dom } => plain
            .as_deref()
            .filter(|p| !looks_empty(p))
            .map(str::to_owned)
            .or_else(|| dom.as_ref().map(dom_to_text))?,
    };
    let text = decode_entities(&text);
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    (!text.is_empty() && !looks_empty(&text)).then_some(text)
}

/// Genius uses "?" instead of an empty description
fn looks_empty(s: &str) -> bool {
    matches!(s.trim().trim_end_matches(['.', '!', ' ']), "" | "?" | "-")
}

fn strip_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut depth = 0usize;
    for c in html.chars() {
        match c {
            '<' => depth += 1,
            '>' => depth = depth.saturating_sub(1),
            '\n' | '\t' if depth > 0 => out.push(' '),
            '\n' | '\t' => {}
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    out
}

fn decode_entities(s: &str) -> String {
    let named = [
        ("&nbsp;", " "),
        ("&amp;", "&"),
        ("&lt;", "<"),
        ("&gt;", ">"),
        ("&quot;", "\""),
        ("&#39;", "'"),
        ("&apos;", "'"),
        ("&mdash;", "\u{2014}"),
        ("&ndash;", "\u{2013}"),
        ("&hellip;", "\u{2026}"),
        ("&rsquo;", "\u{2019}"),
        ("&lsquo;", "\u{2018}"),
        ("&ldquo;", "\u{201c}"),
        ("&rdquo;", "\u{201d}"),
    ];
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        let tail = &rest[i..];
        let Some(end) = tail.find(';') else {
            out.push('&');
            rest = &tail[1..];
            continue;
        };
        let ent = &tail[..=end];
        let decoded = named
            .iter()
            .find(|(k, _)| *k == ent)
            .map(|(_, v)| (*v).to_owned())
            .or_else(|| {
                let code = ent
                    .strip_prefix("&#x")
                    .or_else(|| ent.strip_prefix("&#X"))
                    .or_else(|| ent.strip_prefix("&#"))
                    .and_then(|n| n.strip_suffix(';'));
                code.and_then(|n| {
                    u32::from_str_radix(
                        n,
                        if ent.starts_with("&#x") || ent.starts_with("&#X") {
                            16
                        } else {
                            10
                        },
                    )
                    .ok()
                    .and_then(char::from_u32)
                })
                .map(String::from)
            });
        match decoded {
            Some(d) => out.push_str(&d),
            None => out.push_str(ent),
        }
        rest = &tail[end + 1..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::{decode_entities, description_plain, dom_to_text, strip_tags};
    use crate::api::models::Song;
    use serde_json::json;

    #[test]
    fn strips_html_in_description() {
        assert_eq!(
            strip_tags("<p>Hello <b>world</b>&amp; more</p>"),
            "Hello world&amp; more"
        );
        assert_eq!(
            decode_entities("a &mdash; b &#8212; c &#x27;d&#39;"),
            "a \u{2014} b \u{2014} c 'd'"
        );
        assert_eq!(
            decode_entities("plain < text &unknown; ok"),
            "plain < text &unknown; ok"
        );
    }

    #[test]
    fn description_is_none_when_absent() {
        let s: Song = serde_json::from_str(r#"{"id":1,"url":"u"}"#).unwrap();
        assert!(description_plain(&s).is_none());
    }

    #[test]
    fn description_object_plain_is_used() {
        let s: Song =
            serde_json::from_str(r#"{"id":1,"url":"u","description":{"plain":"A song"}}"#).unwrap();
        assert_eq!(description_plain(&s).as_deref(), Some("A song"));
    }

    #[test]
    fn question_mark_placeholder_is_skipped() {
        let s: Song =
            serde_json::from_str(r#"{"id":1,"url":"u","description":{"plain":"?"}}"#).unwrap();
        assert!(description_plain(&s).is_none());
    }

    #[test]
    fn description_is_plain_text() {
        let s: Song =
            serde_json::from_str(r#"{"id":1,"url":"u","description":"<p>A <i>song</i></p>"}"#)
                .unwrap();
        assert_eq!(description_plain(&s).as_deref(), Some("A song"));
    }

    #[test]
    fn extracts_text_from_dom() {
        let dom = json!({ "tag":"root", "children":[
            { "tag":"p", "children":["Hello", { "content":", world" }] },
            { "tag":"a", "content":"" }
        ]});
        assert_eq!(dom_to_text(&dom), "Hello, world");
    }
}
