#!/usr/bin/env bash
# Verify that the built packages actually require the GTK sonames.
#
# cargo-deb runs dpkg-shlibdeps and cargo-generate-rpm runs find-requires, but
# this script is the guard that the metadata did not override those with a
# hand-written (and stale) list. For rpm the requirement must be the *soname*,
# not a distribution package name: that is what makes one package work on every
# rpm distribution without being re-edited when a name changes.
set -euo pipefail

status=0

check() {
  local kind="$1" file="$2" deps
  case "$kind" in
    rpm) deps="$(rpm -qpR "$file" 2>/dev/null || true)" ;;
    deb) deps="$(dpkg -I "$file" 2>/dev/null | sed -n 's/^ Depends: //p' || true)" ;;
  esac

  printf '\n%s: %s\n' "$kind" "$(basename "$file")"
  if [ -z "$deps" ]; then
    echo "  !! no dependencies at all: the generator did not run" >&2
    status=1
    return
  fi

  grep -q "libadwaita" <<<"$deps" && echo "  ok  libadwaita required" \
    || { echo "  !! no libadwaita dependency" >&2; status=1; }
  grep -q "libgtk-4" <<<"$deps" && echo "  ok  gtk4 required" \
    || { echo "  !! no gtk4 dependency" >&2; status=1; }
  grep -q "libwebkitgtk-6.0" <<<"$deps" && echo "  ok  webkitgtk-6.0 required" \
    || { echo "  !! no webkitgtk-6.0 dependency" >&2; status=1; }
}

shopt -s nullglob
found=0
for f in target/generate-rpm/*.rpm; do check rpm "$f"; found=1; done
for f in target/debian/*.deb; do check deb "$f"; found=1; done

if [ "$found" -eq 0 ]; then
  echo "no packages found, run 'scripts/package.sh' first" >&2
  exit 2
fi

exit $status