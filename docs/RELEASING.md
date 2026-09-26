# Releasing

## What CI produces

The **Desktop build** workflow runs on pull requests, pushes to `master`, version
tags, and manual dispatches. It produces:

- an unsigned Windows NSIS `.exe` installer; and
- a Linux x86-64 `~/.local` tarball (`.tar.zst`) containing the app plus
  `install.sh` and `uninstall.sh`.

Ordinary builds retain these as workflow artifacts for 14 days. Pushing a
version tag publishes a GitHub prerelease and attaches the packaged
applications. This uses the automatic per-run `GITHUB_TOKEN`; no personal access
token or custom secret is required.

Before packaging, CI validates matching application versions, runs frontend
regression tests, and audits JavaScript dependencies. Both platform jobs compile
the native Rust backend and test validation, unsupported-schema rejection,
pagination, and persistence across restart. Python and PyInstaller are no longer
part of the build. Published assets
include a `SHA256SUMS` file. An already published release cannot be overwritten
by rerunning the workflow; create a new version instead.

The Linux job builds a temporary Debian staging package with Tauri, then runs
`packaging/linux/build-tarball.sh` (no Docker/Arch container) to repack its
`usr/bin`, `usr/share/applications`, `usr/share/icons`, and `usr/share/metainfo`
payload into a `colemak-dh-tutor-<version>-x86_64/` directory with `install.sh`,
`uninstall.sh`, `README.md`, and `VERSION`. The intermediate `.deb` itself is
not published.

## Decision on the previous `.pkg.tar.zst`

The Arch pacman artifact (`colemak-dh-tutor-<version>-1-x86_64.pkg.tar.zst`,
built via `packaging/arch/PKGBUILD` + `makepkg`) was **removed** once the
tarball became primary (issue #10). Rationale:

- `~/.local` installs are user-owned, enabling a future in-app updater without
  sudo/pacman;
- GitHub Releases stays the distribution path (no AUR maintenance);
- one Linux artifact keeps version sync, CI time, and docs simpler.

There is no pacman/AUR/Flatpak/AppImage publication path in this workflow.
A separate issue is needed to add `uninstall.sh` coverage beyond the bundled
script or an in-app updater for Windows + `~/.local` Linux.

## Creating a release candidate

Ensure the version matches in `package.json`, `src-tauri/Cargo.toml`,
`src-tauri/tauri.conf.json`, and their lockfiles, then
push the commit and a matching unused version tag (for example, after bumping to
0.1.2):

```bash
git tag v0.1.2
git push origin master v0.1.2
```

After both platform jobs pass, the workflow publishes the prerelease with its
downloadable assets. Install-test these unsigned builds before promoting them
to a stable release.

## Before the first public release

1. Review and install both CI artifacts on clean virtual machines.
2. Acquire a Windows Authenticode certificate and configure Tauri signing.
   Unsigned installers work, but Windows will show an unverified-publisher
   warning. Do not put a certificate or password in the repository.
3. Enable GitHub private vulnerability reporting before making the repository
   public.

Users install the Linux tarball with:

```bash
tar --zstd -xf colemak-dh-tutor-<version>-x86_64.tar.zst
cd colemak-dh-tutor-<version>-x86_64
./install.sh
```

This installs the binary to `~/.local/bin/`, the desktop entry to
`~/.local/share/applications/`, and icons to `~/.local/share/icons/`.
Custom prefixes work via `./install.sh --prefix DIR`; `./uninstall.sh`
removes the same files. The desktop menu entry appears after install (the
script refreshes `update-desktop-database` / icon caches when available).

The tarball does not bundle the system WebKit/GTK runtime. Test on an
Arch-based system with the Tauri prerequisites installed; the Arch pacman
names carried over from the old `PKGBUILD` were `cairo desktop-file-utils
gdk-pixbuf2 glib2 gtk3 hicolor-icon-theme libayatana-appindicator librsvg
libsoup3 openssl pango webkit2gtk-4.1`. The generated tarball `README.md`
repeats these names for installers.

The generated prerelease is suitable for testing. Do not promote it to a
public production release until signing identities are final. Those values
cannot be safely guessed in source control.
