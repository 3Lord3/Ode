<div align="center">

<img alt="Ode" src="packaging/hicolor/512x512/apps/io.ode.lyrics.png" width="128">

# Ode

<img alt="version" src="https://img.shields.io/badge/version-0.1.0-4fb3ff?style=for-the-badge&labelColor=0f131b">
<img alt="license" src="https://img.shields.io/badge/license-MIT-9fe8a0?style=for-the-badge&labelColor=0f131b">
<img alt="GTK4" src="https://img.shields.io/badge/GTK4-4fb3ff?style=for-the-badge&labelColor=0f131b&logo=gtk&logoColor=white">
<img alt="libadwaita" src="https://img.shields.io/badge/libadwaita-4fb3ff?style=for-the-badge&labelColor=0f131b">
<img alt="Rust backend" src="https://img.shields.io/badge/Rust_backend-f0a45a?style=for-the-badge&labelColor=0f131b&logo=rust&logoColor=white">

[English](README.md) · [Русский](README.ru.md)

</div>

### A desktop app for reading lyrics

Ode searches Genius and shows the lyrics of a song in its own window. Built with
Rust, GTK4 and libadwaita, so it follows the system theme, fonts and locale.

# Features

- 🔎 **Search.** Type a query and get a list of songs and artists with artwork and credits. Only songs can be opened; artists and albums are shown for reference.
- 📖 **Lyrics.** Only the song text is shown, without distractions.
- 💡 **Annotations.** Genius annotations are attached to the lines they explain.
- 🌍 **Respects your system.** English and Russian are supported; themes, fonts and everything else are taken from your system settings.
- 💾 **Cache.** Song and search responses are stored on disk for an hour, so repeated lookups are instant. The cache size is capped and the oldest entries are dropped first.

# Requirements

Ode uses the [Genius API](https://docs.genius.com/) and needs an internet
connection. There is no account and nothing to sign in to. Lyrics belong to
their rights holders and are shown courtesy of Genius.

# Building

Ode needs the GTK4, libadwaita and WebKitGTK development packages.

```bash
# Debian / Ubuntu
sudo apt install libgtk-4-dev libadwaita-1-dev libwebkit2gtk-4.1-dev
# Fedora
sudo dnf install gtk4-devel libadwaita-devel webkit2gtk4.1-devel
# ALT Linux
su -
apt-get install libgtk4-devel libadwaita-devel libwebkit2gtk4.1-devel
```

Check the dependencies with `pkg-config --exists gtk4 libadwaita-1`.

Build and check:

```bash
GENIUS_ACCESS_TOKEN=your_token_here cargo build --release
cargo clippy --all-targets
cargo test
```

The token can also be placed in a `.env` file in the project root as
`GENIUS_ACCESS_TOKEN=...`. It is read by `build.rs` and is not committed.

# License

Ode is free software, licensed under the **MIT License**, see [LICENSE](LICENSE).

Not affiliated with Genius. Use of the API is governed by the
[Genius API terms](https://docs.genius.com/).
