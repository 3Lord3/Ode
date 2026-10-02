// Embed the Genius client access token into the binary at build time
//
// The value comes from the GENIUS_ACCESS_TOKEN environment variable or from the
// .env file in the project root (which must stay out of git, see .gitignore)
// If unset, an empty placeholder is used and the app keeps working as before:
// the user enters a token in the settings

fn main() {
    println!("cargo:rerun-if-env-changed=GENIUS_ACCESS_TOKEN");
    println!("cargo:rerun-if-changed=.env");

    // Environment variables take precedence over .env
    let from_env = dotenv(".env");
    let value = std::env::var("GENIUS_ACCESS_TOKEN")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .or(from_env);
    match value {
        Some(v) => println!("cargo:rustc-env=GENIUS_ACCESS_TOKEN={v}"),
        None => println!("cargo:rustc-env=GENIUS_ACCESS_TOKEN="),
    }
}

/// Minimal `.env` parser: `KEY=VALUE` lines, skipping comments and blank lines
fn dotenv(path: &str) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| l.split_once('='))
        .find(|(k, _)| k.trim() == "GENIUS_ACCESS_TOKEN")
        .map(|(_, v)| v.trim().trim_matches('"').trim_matches('\'').to_string())
        .filter(|v| !v.is_empty())
}