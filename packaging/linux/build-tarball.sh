#!/usr/bin/env bash
# Build a per-user Linux tarball (.tar.zst) from a Tauri .deb staging package.
#
# Usage: ./packaging/linux/build-tarball.sh path/to/colemak-dh-tutor.deb
#
# Output: dist/colemak-dh-tutor-<version>-x86_64.tar.zst containing:
#   colemak-dh-tutor-<version>-x86_64/
#     bin/<binary>
#     share/applications/*.desktop
#     share/icons/... (when the .deb ships icons)
#     share/metainfo/... (when the .deb ships metainfo)
#     install.sh / uninstall.sh
#     README.md / VERSION
#
# The tarball installs into ~/.local via ./install.sh (no sudo), which keeps
# installed files user-owned for a future in-app updater.
set -euo pipefail

if [[ $# -ne 1 || ! -f "$1" ]]; then
  echo "Usage: $0 path/to/colemak-dh-tutor.deb" >&2
  exit 2
fi

deb_path="$(realpath "$1")"
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
package_dir="$repo_root/packaging/linux"
work_dir="$package_dir/work"
stage_root="$work_dir/stage"
output_dir="$repo_root/dist"

version="$(node -p "require('./package.json').version" --prefix "$repo_root" 2>/dev/null || true)"
if [[ -z "$version" ]]; then
  version="$(sed -n 's/.*"version": *"\([^"]*\)".*/\1/p' "$repo_root/package.json" | head -n1)"
fi
if [[ -z "$version" ]]; then
  echo "Could not determine version from package.json" >&2
  exit 1
fi

dirname="colemak-dh-tutor-${version}-x86_64"
stage_dir="$stage_root/$dirname"
archive="$output_dir/${dirname}.tar.zst"

rm -rf "$work_dir"
mkdir -p "$stage_dir/bin" "$stage_dir/share" "$output_dir"

extract_dir="$(mktemp -d)"
cleanup() {
  rm -rf "$extract_dir"
}
trap cleanup EXIT

(
  cd "$extract_dir"
  ar x "$deb_path"
  data_archive="$(find . -maxdepth 1 -name 'data.tar.*' -print -quit)"
  if [[ -z "$data_archive" ]]; then
    echo "Debian package is missing data.tar.*" >&2
    exit 1
  fi
  tar -xf "$data_archive"
  if [[ ! -d usr ]]; then
    echo "Debian data archive does not contain usr/" >&2
    exit 1
  fi

  shopt -s nullglob dotglob
  usr_bins=(usr/bin/*)
  usr_desktops=(usr/share/applications/*.desktop)
  if [[ ${#usr_bins[@]} -eq 0 ]]; then
    echo "Debian package has no usr/bin/* binaries" >&2
    exit 1
  fi
  if [[ ${#usr_desktops[@]} -eq 0 ]]; then
    echo "Debian package has no usr/share/applications/*.desktop" >&2
    exit 1
  fi

  cp -a "${usr_bins[@]}" "$stage_dir/bin/"
  chmod 755 "$stage_dir"/bin/*
  mkdir -p "$stage_dir/share/applications"
  cp -a "${usr_desktops[@]}" "$stage_dir/share/applications/"
  chmod 644 "$stage_dir"/share/applications/*.desktop

  if [[ -d usr/share/icons ]]; then
    cp -a usr/share/icons "$stage_dir/share/"
  else
    echo "Warning: usr/share/icons missing in .deb; continuing without icons." >&2
  fi
  if [[ -d usr/share/metainfo ]]; then
    cp -a usr/share/metainfo "$stage_dir/share/"
  elif [[ -d usr/share/appdata ]]; then
    mkdir -p "$stage_dir/share/metainfo"
    cp -a usr/share/appdata/. "$stage_dir/share/metainfo/"
  fi
)

install -m755 "$package_dir/install.sh" "$stage_dir/install.sh"
install -m755 "$package_dir/uninstall.sh" "$stage_dir/uninstall.sh"
printf '%s\n' "$version" > "$stage_dir/VERSION"

cat > "$stage_dir/README.md" <<EOF
# Colemak-DH Tutor ${version} (Linux x86-64, ~/.local tarball)

Per-user install for Arch-based distributions (and other systemd Linux
desktops with the runtime below). No sudo or pacman required; files are
installed user-owned under \`~/.local\` so a future in-app updater can
replace them.

## Install

\`\`\`bash
tar --zstd -xf ${dirname}.tar.zst
cd ${dirname}
./install.sh
\`\`\`

This installs roughly:

- binary → \`~/.local/bin/\`
- desktop entry → \`~/.local/share/applications/\`
- icons → \`~/.local/share/icons/\`

Then launch **Colemak-DH Tutor** from your application menu, or run
\`colemak-dh-tutor\` (ensure \`~/.local/bin\` is on your \`PATH\`).

Custom prefix: \`./install.sh --prefix /custom/prefix\` (or \`PREFIX=...\`).

## Uninstall

From the same extracted directory:

\`\`\`bash
./uninstall.sh
\`\`\`

Use the same \`--prefix\` if install used one.

## Runtime requirements

The tarball does not bundle the system WebKit/GTK runtime. On Arch-based
systems install (pacman names):

> cairo desktop-file-utils gdk-pixbuf2 glib2 gtk3 hicolor-icon-theme
> libayatana-appindicator librsvg libsoup3 openssl pango webkit2gtk-4.1

On Debian/Ubuntu-based build hosts the equivalent development packages
include \`libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev\`.
See https://v2.tauri.app/start/prerequisites/ for the full Tauri system
prerequisites if the app fails to start with missing shared libraries.

## Notes

- x86-64 only in this artifact.
- To verify integrity, check \`SHA256SUMS\` on the GitHub release.
EOF

# Prefer tar's --zstd flag; fall back to -I 'zstd' for older tars.
if tar --help 2>/dev/null | grep -q -- '--zstd'; then
  tar --zstd -cf "$archive" -C "$stage_root" "$dirname"
else
  tar -I 'zstd -19 -T0' -cf "$archive" -C "$stage_root" "$dirname"
fi

echo "Wrote $archive"
tar --zstd -tf "$archive" 2>/dev/null || tar -I zstd -tf "$archive"
