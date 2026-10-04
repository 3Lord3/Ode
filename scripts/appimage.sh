#!/usr/bin/env bash
# Build the AppImage with linuxdeploy + its GTK plugin.
#
# Ode is a GTK4 / libadwaita / WebKitGTK app. linuxdeploy collects every
# library the binary links against into the AppDir (it also follows the
# WebKitWebProcess and the *_gresource binaries), and the GTK plugin repairs
# the GLib wrappers into AppImage-appropriate ones so a Wayland session is not
# dragged through XWayland.
#
# STATUS: written, not yet built in this environment. linuxdeploy and the GTK
# plugin are downloaded on the first run; the GTK plugin only gained GTK4
# support recently, so expect one round of adjustment if a distro still pins an
# old linuxdeploy-plugin-gtk.
set -euo pipefail

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"

[ -x target/release/ode ] || cargo build --release

version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml)
name="Ode_${version}_amd64.AppImage"

appdir="$root/target/appimage/Ode.AppDir"
cache="$HOME/.cache/ode-appimage"
mkdir -p "$appdir" "$cache"

# The desktop file and icon have to be inside the AppDir before linuxdeploy
# runs, or it refuses to look at them.
rm -rf "$appdir/usr" "$appdir/AppRun"
install -Dm755 target/release/ode "$appdir/usr/bin/ode"
install -Dm644 packaging/org.example.Ode.desktop \
  "$appdir/usr/share/applications/org.example.Ode.desktop"
install -Dm644 packaging/hicolor/128x128/apps/org.example.Ode.svg \
  "$appdir/usr/share/icons/hicolor/128x128/apps/org.example.Ode.svg"

fetch() { # $1=url, $2=dest
  [ -x "$2" ] || { curl -L --fail -o "$2" "$1"; chmod +x "$2"; }
}

fetch "https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage" \
  "$cache/linuxdeploy.AppImage"
fetch "https://github.com/linuxdeploy/linuxdeploy-plugin-gtk/releases/download/continuous/linuxdeploy-plugin-gtk-x86_64.AppImage" \
  "$cache/linuxdeploy-plugin-gtk.AppImage"
fetch "https://github.com/linuxdeploy/linuxdeploy-plugin-appimage/releases/download/continuous/linuxdeploy-plugin-appimage-x86_64.AppImage" \
  "$cache/linuxdeploy-plugin-appimage.AppImage"

# linuxdeploy discovers its plugins by an exact executable name on PATH; the
# downloads carry an -x86_64 suffix, so symlink them onto the bin dir we put on
# PATH. APPIMAGE_EXTRACT_AND_RUN is needed because the runners have no FUSE.
mkdir -p "$cache/bin"
ln -sf "$cache/linuxdeploy-plugin-gtk.AppImage" "$cache/bin/linuxdeploy-plugin-gtk"
ln -sf "$cache/linuxdeploy-plugin-appimage.AppImage" "$cache/bin/linuxdeploy-plugin-appimage"
export PATH="$cache/bin:$PATH"
export DEPLOY_GTK_VERSION=4
export APPIMAGE_EXTRACT_AND_RUN=1

"$cache/linuxdeploy.AppImage" --appdir "$appdir" \
  --desktop-file "$appdir/usr/share/applications/org.example.Ode.desktop" \
  --icon-file "$appdir/usr/share/icons/hicolor/128x128/apps/org.example.Ode.svg" \
  --plugin gtk \
  --output appimage

mv "$appdir"/*.AppImage "target/appimage/$name"
echo "AppImage at target/appimage/$name"