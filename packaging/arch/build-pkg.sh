#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 || ! -f "$1" ]]; then
  echo "Usage: $0 path/to/typedh.deb" >&2
  exit 2
fi

deb_path="$(realpath "$1")"
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
package_dir="$repo_root/packaging/arch"
work_dir="$package_dir/work"
output_dir="$repo_root/dist"
pkgver="$(sed -n 's/^pkgver=//p' "$package_dir/PKGBUILD" | head -n1)"

rm -rf "$work_dir"
mkdir -p "$work_dir/root" "$output_dir"

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
  cp -a usr "$work_dir/root/"
)

tar -C "$work_dir" -cf "$work_dir/root.tar" root
install -m644 "$package_dir/PKGBUILD" "$work_dir/PKGBUILD"

docker run --rm \
  --user 0:0 \
  -e PKGVER="$pkgver" \
  -v "$work_dir:/work" \
  -w /work \
  archlinux:latest \
  bash -euo pipefail -c '
    pacman -Syu --noconfirm --needed base-devel
    useradd -m builder
    echo "builder ALL=(ALL) NOPASSWD: ALL" >/etc/sudoers.d/builder
    chown -R builder:builder /work
    su builder -c "cd /work && makepkg -f --nodeps"
  '

shopt -s nullglob
packages=("$work_dir"/*.pkg.tar.zst)
if [[ ${#packages[@]} -ne 1 ]]; then
  echo "Expected exactly one .pkg.tar.zst from makepkg, found ${#packages[@]}" >&2
  exit 1
fi

install -m644 "${packages[0]}" \
  "$output_dir/typedh-${pkgver}-1-x86_64.pkg.tar.zst"
echo "Wrote $output_dir/typedh-${pkgver}-1-x86_64.pkg.tar.zst"
