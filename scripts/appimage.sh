#!/usr/bin/env bash
# Build the AppImage with linuxdeploy + its GTK plugin. Both are fetched on
# the first run; the GTK plugin needs DEPLOY_GTK_VERSION=4.
set -euo pipefail

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"

[ -x target/release/ode ] || cargo build --release

version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml)
name="Ode_${version}_amd64.AppImage"

appdir="$root/target/appimage/Ode.AppDir"
cache="$HOME/.cache/ode-appimage"
mkdir -p "$appdir" "$cache"

# linuxdeploy needs the desktop file and icon inside the AppDir.
rm -rf "$appdir/usr" "$appdir/AppRun"
install -Dm755 target/release/ode "$appdir/usr/bin/ode"
install -Dm644 packaging/io.ode.lyrics.desktop \
  "$appdir/usr/share/applications/io.ode.lyrics.desktop"
for s in 16 22 24 32 48 64 96 128 256 512; do
  install -Dm644 "packaging/hicolor/${s}x${s}/apps/io.ode.lyrics.png" \
    "$appdir/usr/share/icons/hicolor/${s}x${s}/apps/io.ode.lyrics.png"
done

fetch() { # $1=url, $2=dest
  [ -x "$2" ] || { curl -L --fail -o "$2" "$1"; chmod +x "$2"; }
}

fetch "https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage" \
  "$cache/linuxdeploy.AppImage"
# The gtk plugin ships as a shell script, not an AppImage.
if [ -x "$cache/linuxdeploy-plugin-gtk" ]; then
  :
else
  curl -fsSL -o "$cache/linuxdeploy-plugin-gtk" \
    "https://raw.githubusercontent.com/linuxdeploy/linuxdeploy-plugin-gtk/master/linuxdeploy-plugin-gtk.sh"
  chmod +x "$cache/linuxdeploy-plugin-gtk"
fi
fetch "https://github.com/linuxdeploy/linuxdeploy-plugin-appimage/releases/download/continuous/linuxdeploy-plugin-appimage-x86_64.AppImage" \
  "$cache/linuxdeploy-plugin-appimage.AppImage"

# gtk plugin deps for bundling GTK schemas and typelibs.
if command -v apt-get >/dev/null 2>&1 && [ "$(id -u)" -eq 0 ]; then
  apt-get update -qq
  apt-get install -y --no-install-recommends file librsvg2-dev libgirepository1.0-dev
fi

# linuxdeploy finds plugins by name on PATH. APPIMAGE_EXTRACT_AND_RUN works
# around the missing FUSE in runners.
mkdir -p "$cache/bin"
ln -sf "$cache/linuxdeploy-plugin-gtk" "$cache/bin/linuxdeploy-plugin-gtk"
ln -sf "$cache/linuxdeploy-plugin-appimage.AppImage" "$cache/bin/linuxdeploy-plugin-appimage"
export PATH="$cache/bin:$PATH"
export DEPLOY_GTK_VERSION=4
export APPIMAGE_EXTRACT_AND_RUN=1

# Build the AppDir only: the AppRun hook is patched below before packaging.
"$cache/linuxdeploy.AppImage" --appdir "$appdir" \
  --desktop-file "$appdir/usr/share/applications/io.ode.lyrics.desktop" \
  --icon-file "$appdir/usr/share/icons/hicolor/512x512/apps/io.ode.lyrics.png" \
  --plugin gtk

# The hook forces GTK_THEME and GDK_BACKEND=x11; both break libadwaita on
# Wayland, so drop them.
hook="$appdir/apprun-hooks/linuxdeploy-plugin-gtk.sh"
if [ -f "$hook" ]; then
  sed -i -e 's|^export GTK_THEME=.*|unset GTK_THEME|' \
         -e 's|^export GDK_BACKEND=.*|unset GDK_BACKEND|' "$hook"
fi

# The shell matches a window to a desktop entry and icon by app id; without one
# it shows the raw id and a placeholder icon. Install both into the user's data
# dirs on launch.
cat > "$appdir/apprun-hooks/ode-integrate.sh" <<'EOF'
data="${XDG_DATA_HOME:-$HOME/.local/share}"
[ -n "$APPDIR" ] || return 0
(
  set +e
  exec_path="${APPIMAGE:-ode}"
  install -d "$data/applications"
  sed "s|^Exec=.*|Exec=\"$exec_path\"|" \
    "$APPDIR/usr/share/applications/io.ode.lyrics.desktop" \
    > "$data/applications/io.ode.lyrics.desktop"
  chmod 644 "$data/applications/io.ode.lyrics.desktop"
  for s in 16 22 24 32 48 64 96 128 256 512; do
    install -Dm644 "$APPDIR/usr/share/icons/hicolor/${s}x${s}/apps/io.ode.lyrics.png" \
      "$data/icons/hicolor/${s}x${s}/apps/io.ode.lyrics.png"
  done
  command -v update-desktop-database >/dev/null && update-desktop-database "$data/applications"
  command -v gtk-update-icon-cache >/dev/null && gtk-update-icon-cache -qtf "$data/icons/hicolor"
) || true
EOF
printf '\n. "$this_dir"/apprun-hooks/ode-integrate.sh\n' >> "$hook"

# The appimage plugin drops "<binary>-x86_64.AppImage" in cwd, not the AppDir.
"$cache/linuxdeploy-plugin-appimage.AppImage" --appdir="$appdir"
img=$(ls "$root"/Ode-*.AppImage 2>/dev/null | head -1)
[ -n "$img" ] || { echo "no AppImage produced" >&2; exit 1; }
mv "$img" "target/appimage/$name"
echo "AppImage at target/appimage/$name"