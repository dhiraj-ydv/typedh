#!/usr/bin/env bash
# Colemak-DH Tutor per-user uninstaller for ~/.local installs.
#
# Removes the files installed by install.sh from the same tarball directory.
# Usage:
#   ./uninstall.sh [--prefix "$HOME/.local"] [--help]
set -euo pipefail

PREFIX="${PREFIX:-$HOME/.local}"
SRC_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

usage() {
  cat <<EOF
Usage: ./uninstall.sh [--prefix DIR] [--help]

Removes Colemak-DH Tutor files installed by ./install.sh.

  --prefix DIR   Install prefix that was used at install time
                 (default: \$HOME/.local).
  --help         Show this help.
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

BINDIR="$PREFIX/bin"
APPSDIR="$PREFIX/share/applications"
ICONDIR="$PREFIX/share/icons"
METADIR="$PREFIX/share/metainfo"

shopt -s nullglob dotglob
removed=0

for src in "$SRC_DIR"/bin/*; do
  [[ -f "$src" ]] || continue
  dest="$BINDIR/$(basename "$src")"
  if [[ -f "$dest" ]]; then
    rm -f "$dest"
    echo "Removed $dest"
    removed=$((removed + 1))
  fi
done

for src in "$SRC_DIR"/share/applications/*.desktop; do
  [[ -f "$src" ]] || continue
  dest="$APPSDIR/$(basename "$src")"
  if [[ -f "$dest" ]]; then
    rm -f "$dest"
    echo "Removed $dest"
    removed=$((removed + 1))
  fi
done

# Remove only the icon/metainfo files shipped in this tarball, then prune
# any directories left empty.
for subdir in icons metainfo; do
  src_root="$SRC_DIR/share/$subdir"
  dest_root="$PREFIX/share/$subdir"
  [[ -d "$src_root" ]] || continue
  while IFS= read -r -d '' src_file; do
    rel="${src_file#$src_root/}"
    dest_file="$dest_root/$rel"
    if [[ -f "$dest_file" ]]; then
      rm -f "$dest_file"
      echo "Removed $dest_file"
      removed=$((removed + 1))
    fi
  done < <(find "$src_root" -type f -print0)
  # Prune empty directories bottom-up. Keep widely shared roots.
  find "$dest_root" -mindepth 1 -type d -empty -delete 2>/dev/null || true
done

if command -v update-desktop-database >/dev/null 2>&1 && [[ -d "$APPSDIR" ]]; then
  update-desktop-database "$APPSDIR" || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1 && [[ -d "$ICONDIR" ]]; then
  gtk-update-icon-cache -f -t "$ICONDIR" 2>/dev/null || true
fi

if [[ $removed -eq 0 ]]; then
  echo "Nothing to remove under $PREFIX."
else
  echo "Uninstalled $removed file(s) from $PREFIX."
fi
