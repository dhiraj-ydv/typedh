#!/usr/bin/env bash
# Colemak-DH Tutor per-user installer for ~/.local installs.
#
# Expected layout (as shipped in the .tar.zst):
#   <dist-dir>/
#     bin/colemak-dh-tutor (or the Tauri-built binary name)
#     share/applications/*.desktop
#     share/icons/... (optional)
#     share/metainfo/... (optional)
#     install.sh (this file)
#     uninstall.sh
#
# Usage:
#   ./install.sh [--prefix "$HOME/.local"] [--help]
#
# Installs without sudo so a future in-app updater can replace
# user-owned files.
set -euo pipefail

PREFIX="${PREFIX:-$HOME/.local}"
SRC_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

usage() {
  cat <<EOF
Usage: ./install.sh [--prefix DIR] [--help]

Installs Colemak-DH Tutor for the current user.

  --prefix DIR   Install prefix (default: \$HOME/.local).
                 Binary goes to <prefix>/bin,
                 desktop entry to <prefix>/share/applications,
                 icons to <prefix>/share/icons.
  --help         Show this help.

Environment:
  PREFIX         Same as --prefix. The flag wins when both are given.

Examples:
  ./install.sh
  ./install.sh --prefix "\$HOME/.local"
  PREFIX=/tmp/test-local ./install.sh
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --prefix)
      [[ $# -lt 2 ]] && { echo "--prefix requires a value" >&2; exit 2; }
      PREFIX="$2"
      shift 2
      ;;
    --prefix=*)
      PREFIX="${1#--prefix=}"
      shift
      ;;
    -h|--help|help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1 (see --help)" >&2
      exit 2
      ;;
  esac
done

if [[ -z "$PREFIX" ]]; then
  echo "Empty install prefix is not allowed." >&2
  exit 2
fi

BINDIR="$PREFIX/bin"
APPSDIR="$PREFIX/share/applications"
ICONDIR="$PREFIX/share/icons"
METADIR="$PREFIX/share/metainfo"

if [[ ! -d "$SRC_DIR/bin" ]]; then
  echo "Missing bin/ next to install.sh (run from the extracted tarball dir)." >&2
  exit 1
fi

shopt -s nullglob
binaries=("$SRC_DIR"/bin/*)
desktops=("$SRC_DIR"/share/applications/*.desktop)
if [[ ${#binaries[@]} -eq 0 ]]; then
  echo "No binaries found in $SRC_DIR/bin." >&2
  exit 1
fi
if [[ ${#desktops[@]} -eq 0 ]]; then
  echo "No desktop entries found in $SRC_DIR/share/applications." >&2
  exit 1
fi

mkdir -p "$BINDIR" "$APPSDIR"

installed_bins=()
for src in "${binaries[@]}"; do
  base="$(basename "$src")"
  install -m755 "$src" "$BINDIR/$base"
  installed_bins+=("$BINDIR/$base")
done

# Install desktop entries, rewriting absolute /usr/bin Exec/TryExec paths
# produced for system packages to the user prefix.
installed_desktops=()
for src in "${desktops[@]}"; do
  base="$(basename "$src")"
  dest="$APPSDIR/$base"
  sed -e "s#^Exec=/usr/bin/#Exec=$BINDIR/#" \
      -e "s#^TryExec=/usr/bin/#TryExec=$BINDIR/#" \
      "$src" > "$dest.tmp"
  chmod 644 "$dest.tmp"
  mv "$dest.tmp" "$dest"
  installed_desktops+=("$dest")
done

if [[ -d "$SRC_DIR/share/icons" ]]; then
  mkdir -p "$ICONDIR"
  cp -a "$SRC_DIR/share/icons/." "$ICONDIR/"
fi

if [[ -d "$SRC_DIR/share/metainfo" ]]; then
  mkdir -p "$METADIR"
  cp -a "$SRC_DIR/share/metainfo/." "$METADIR/"
fi

# Refresh caches when the host tools exist. Failures here are non-fatal.
if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "$APPSDIR" || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -f -t "$ICONDIR" 2>/dev/null || true
fi

echo "Installed Colemak-DH Tutor to $PREFIX"
echo "  binary:   ${installed_bins[0]}"
echo "  desktop:  ${installed_desktops[0]}"
echo ""
echo "Make sure $BINDIR is on your PATH, then run:"
echo "  ${installed_bins[0]##*/}"
echo "or launch 'Colemak-DH Tutor' from your application menu."
echo ""
echo "To uninstall, run ./uninstall.sh from this directory with the same --prefix."
