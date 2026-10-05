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
install -Dm644 packaging/io.ode.lyrics.desktop \
  "$appdir/usr/share/applications/io.ode.lyrics.desktop"
install -Dm644 packaging/hicolor/128x128/apps/io.ode.lyrics.svg \
  "$appdir/usr/share/icons/hicolor/128x128/apps/io.ode.lyrics.svg"

fetch() { # $1=url, $2=dest
  [ -x "$2" ] || { curl -L --fail -o "$2" "$1"; chmod +x "$2"; }
}

fetch "https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage" \
  "$cache/linuxdeploy.AppImage"
# The gtk plugin is shipped as a bash script, not an AppImage: an AppImage URL
# 404s (the release has no assets). It is experimental and needs a few dev
# packages of its own, installed below.
if [ -x "$cache/linuxdeploy-plugin-gtk" ]; then
  :
else
  curl -fsSL -o "$cache/linuxdeploy-plugin-gtk" \
    "https://raw.githubusercontent.com/linuxdeploy/linuxdeploy-plugin-gtk/master/linuxdeploy-plugin-gtk.sh"
  chmod +x "$cache/linuxdeploy-plugin-gtk"
fi
fetch "https://github.com/linuxdeploy/linuxdeploy-plugin-appimage/releases/download/continuous/linuxdeploy-plugin-appimage-x86_64.AppImage" \
  "$cache/linuxdeploy-plugin-appimage.AppImage"

# The gtk plugin needs `file`, librsvg and gobject-introspection dev files to
# bundle GTK schemas and typelibs; webkit/gtk dev packages do not cover them.
if command -v apt-get >/dev/null 2>&1 && [ "$(id -u)" -eq 0 ]; then
  apt-get update -qq
  apt-get install -y --no-install-recommends file librsvg2-dev libgirepository1.0-dev
fi

# linuxdeploy discovers its plugins by an exact executable name on PATH; put
# ours on a bin dir we prepend. APPIMAGE_EXTRACT_AND_RUN is needed because the
# runners have no FUSE.
mkdir -p "$cache/bin"
ln -sf "$cache/linuxdeploy-plugin-gtk" "$cache/bin/linuxdeploy-plugin-gtk"
ln -sf "$cache/linuxdeploy-plugin-appimage.AppImage" "$cache/bin/linuxdeploy-plugin-appimage"
export PATH="$cache/bin:$PATH"
export DEPLOY_GTK_VERSION=4
export APPIMAGE_EXTRACT_AND_RUN=1

"$cache/linuxdeploy.AppImage" --appdir "$appdir" \
  --desktop-file "$appdir/usr/share/applications/io.ode.lyrics.desktop" \
  --icon-file "$appdir/usr/share/icons/hicolor/128x128/apps/io.ode.lyrics.svg" \
  --plugin gtk \
  --output appimage

# The appimage plugin writes the result to the process cwd (the repo root)
# as "<binary>-x86_64.AppImage", not into the AppDir. Rename it here.
img=$(ls "$root"/Ode-*.AppImage 2>/dev/null | head -1)
[ -n "$img" ] || { echo "no AppImage produced" >&2; exit 1; }
mv "$img" "target/appimage/$name"
echo "AppImage at target/appimage/$name"