#!/usr/bin/env bash
# Build the native packages: .deb via cargo-deb, .rpm via cargo-generate-rpm.
#
# Both tools derive the dependency list from what the binary actually links,
# so a renamed library is caught at packaging time rather than by a user. The
# binary is built once (cargo build --release) and both tools repackage it.
set -euo pipefail

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"

[ -x target/release/ode ] || cargo build --release

# cargo-deb
if ! command -v cargo-deb >/dev/null 2>&1; then
  cargo install cargo-deb --locked
fi
cargo deb --no-build
deb=$(ls target/debian/*.deb)
echo "deb: $deb"

# cargo-generate-rpm
if ! command -v cargo-generate-rpm >/dev/null 2>&1; then
  cargo install cargo-generate-rpm --locked
fi
cargo generate-rpm
rpm=$(ls target/generate-rpm/*.rpm)
echo "rpm: $rpm"