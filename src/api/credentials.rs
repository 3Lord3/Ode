/// Genius client token embedded at build time (see build.rs)
///
/// This is app data, not user data: everyone who builds Ode gets the same token
/// The value comes from the `GENIUS_ACCESS_TOKEN` build variable (or `.env`), so
/// it is absent from the sources but present in the binary, and extracting it from
/// a released executable is easy. That is expected for a public desktop client
pub const ACCESS_TOKEN: &str = env!("GENIUS_ACCESS_TOKEN");

/// Token for API requests, absent when the build had none
pub fn token() -> Option<String> {
    (!ACCESS_TOKEN.trim().is_empty()).then(|| ACCESS_TOKEN.to_string())
}
