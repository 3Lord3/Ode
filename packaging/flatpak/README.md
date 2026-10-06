# Flatpak

```sh
flatpak install -y flathub org.gnome.Platform//48 org.gnome.Sdk//48 \
  org.freedesktop.Sdk.Extension.rust-stable//24.08
flatpak-builder --force-clean --user --install --install-deps-from=flathub \
  build packaging/flatpak/io.ode.lyrics.yml
flatpak run io.ode.lyrics
```

cargo fetches the dependency tree at build time, so the `ode` module carries
`--share=network` in its own build-args. A Flathub submission cannot use that
flag: generate `cargo-sources.json` with
[flatpak-builder-tools](https://github.com/flatpak/flatpak-builder-tools) and
drop it.

`target/` is in the module's `skip` list so a stale local build cannot shadow
the sandbox build.

There is no `.env` in the sandbox, so `build.rs` embeds an empty token: the
packaged app starts with the Genius token unset until the user fills it in
Settings.