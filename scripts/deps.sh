#!/usr/bin/env bash
# Ode build dependency check / install.
#
# Source of truth is the pkg-config module list: module names are identical on
# every distribution, package names are not. The package table below exists only
# to print a ready-to-paste install command.
set -euo pipefail

# --- pkg-config modules Ode links against ------------------------------------
# openssl is pulled in by reqwest (native-tls). Debian/Ubuntu get it
# transitively via the webkit dev package, Fedora does not, which is why it is
# listed here instead of relying on that.
MODULES=(gtk4 libadwaita-1 webkitgtk-6.0 openssl)

# --- distribution detection --------------------------------------------------
detect_distro() {
  if [ -r /etc/os-release ]; then
    # shellcheck disable=SC1091
    . /etc/os-release
    case "${ID:-}" in
      altlinux)                 echo alt     ; return ;;
      debian|ubuntu|linuxmint|pop|elementary)
                                echo debian  ; return ;;
      fedora|rhel|centos|rocky|almalinux)
                                echo fedora  ; return ;;
      arch|manjaro|endeavouros) echo arch    ; return ;;
    esac
    for like in ${ID_LIKE:-}; do
      case "$like" in
        debian) echo debian ; return ;;
        fedora|rhel) echo fedora ; return ;;
        arch)   echo arch   ; return ;;
      esac
    done
  fi
  echo unknown
}

packages_for() {
  case "$1" in
    alt)
      echo "libgtk+4-devel libadwaita-devel libwebkitgtk6.0-devel \
libopenssl-devel gcc gcc-c++ make pkg-config"
      ;;
    debian)
      echo "libgtk-4-dev libadwaita-1-dev libwebkitgtk-6.0-dev libssl-dev \
build-essential pkg-config"
      ;;
    fedora)
      echo "gtk4-devel libadwaita-devel webkitgtk6.0-devel openssl-devel \
gcc gcc-c++ make pkgconf-pkg-config"
      ;;
    arch)
      echo "gtk4 libadwaita webkitgtk-6.0 openssl base-devel"
      ;;
  esac
}

# Root already, or in a container: `sudo` is usually not installed at all.
sudo_prefix() {
  if [ "$(id -u)" -eq 0 ]; then
    echo ""
  elif command -v sudo >/dev/null 2>&1; then
    echo "sudo "
  else
    echo ""
  fi
}

install_cmd_for() {
  local distro="$1" pkgs s
  pkgs="$(packages_for "$distro")"
  s="$(sudo_prefix)"
  case "$distro" in
    alt)    echo "${s}apt-get update && ${s}apt-get install -y $pkgs" ;;
    debian) echo "${s}apt-get update && ${s}apt-get install -y $pkgs" ;;
    fedora) echo "${s}dnf install -y $pkgs" ;;
    arch)   echo "${s}pacman -Sy --needed --noconfirm $pkgs" ;;
  esac
}

# --- checks ------------------------------------------------------------------
missing=()

check_modules() {
  local ok=0
  if ! command -v pkg-config >/dev/null 2>&1; then
    echo "  pkg-config itself is missing" >&2
    missing+=(pkg-config)
    return 1
  fi
  for m in "${MODULES[@]}"; do
    if pkg-config --exists "$m" 2>/dev/null; then
      printf '  \033[32m ok \033[0m %-24s %s\n' "$m" "$(pkg-config --modversion "$m")"
    else
      printf '  \033[31mmiss\033[0m %-24s\n' "$m"
      missing+=("$m")
      ok=1
    fi
  done
  return $ok
}

usage() {
  cat <<EOF
usage: scripts/deps.sh [--check | --install | --list]

  --check    (default) report which pkg-config modules are missing and print
             the install command for this distribution. Changes nothing.
  --install  run that install command.
  --list     print the required pkg-config modules, one per line.
EOF
}

main() {
  local mode="${1:---check}"
  case "$mode" in
    --list) printf '%s\n' "${MODULES[@]}"; return 0 ;;
    --check|--install) ;;
    -h|--help) usage; return 0 ;;
    *) usage >&2; return 2 ;;
  esac

  local distro
  distro="$(detect_distro)"
  echo "distribution: $distro"
  echo "checking pkg-config modules:"

  local mods_ok=0
  check_modules || mods_ok=1

  if [ "$mods_ok" -eq 0 ]; then
    echo
    echo "all build dependencies present."
    return 0
  fi

  if [ "$distro" = unknown ]; then
    echo
    echo "Unrecognised distribution. Install the development packages providing" >&2
    echo "these pkg-config modules, then re-run this script:" >&2
    printf '  %s\n' "${MODULES[@]}" >&2
    return 1
  fi

  local cmd
  cmd="$(install_cmd_for "$distro")"

  if [ "$mode" = --install ]; then
    echo
    echo "+ $cmd"
    eval "$cmd"
    echo
    echo "re-checking:"
    missing=()
    check_modules && { echo; echo "all build dependencies present."; return 0; }
    return 1
  fi

  echo
  echo "install with:"
  echo "  $cmd"
  return 1
}

main "$@"