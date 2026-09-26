# Releasing

## What CI produces

The **Desktop build** workflow runs on pull requests, pushes to `master`, version
tags, and manual dispatches. It produces:

- an unsigned Windows NSIS `.exe` installer;
- a Linux x86-64 `~/.local` tarball (`.tar.zst`) containing the app plus
  `install.sh` and `uninstall.sh`; and
- two macOS `.dmg` installers: `aarch64` (Apple Silicon) and `x86_64` (Intel).

Ordinary builds retain these as workflow artifacts for 14 days (four artifacts
in total: Windows, Linux, and two macOS DMGs). Pushing a
version tag publishes a GitHub prerelease and attaches the packaged
applications. This uses the automatic per-run `GITHUB_TOKEN`; no personal access
token or custom secret is required.

Before packaging, CI validates matching application versions, runs frontend
regression tests, and audits JavaScript dependencies. Every platform job compiles
the native Rust backend and tests validation, unsupported-schema rejection,
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

## macOS DMGs (Apple Silicon + Intel)

The `macos-dmg` job runs on `macos-latest` with a two-leg matrix, one Rust
target per Apple architecture (no universal binary):

| Matrix leg | Rust target | Users | Artifact |
| --- | --- | --- | --- |
| `aarch64` | `aarch64-apple-darwin` | Apple Silicon (M1 and later), built natively | `Colemak-DH-Tutor-macOS-aarch64` |
| `x86_64` | `x86_64-apple-darwin` | Intel Macs, cross-compiled on the same runner | `Colemak-DH-Tutor-macOS-x86_64` |

Each leg runs `npm run tauri build -- --bundles dmg --target <triple>` and
uploads `src-tauri/target/<triple>/release/bundle/dmg/*.dmg`. Tauri embeds the
architecture in the DMG filename (`Colemak-DH Tutor_<version>_aarch64.dmg`,
`Colemak-DH Tutor_<version>_x64.dmg`), so release assets are self-labeling. The
per-arch choice (rather than one universal DMG) keeps asset names unambiguous
and matches Tauri's documented GitHub pipeline; a universal binary can be
revisited later if Intel support is ever dropped.

macOS needs no extra system packages on the runner: Xcode tooling ships with
`macos-latest`. To build a DMG locally on a Mac:

```bash
npm ci
npm --prefix frontend ci
npm run tauri build -- --bundles dmg --target aarch64-apple-darwin
```

(replace the target with `x86_64-apple-darwin` for Intel). The DMG appears
under `src-tauri/target/<target>/release/bundle/dmg/`.

### macOS signing and notarization

`src-tauri/tauri.conf.json` sets `bundle.macOS.signingIdentity` to `"-"`
(ad-hoc signing). Ad-hoc signing is committed, needs no secrets, and stops
Apple Silicon Macs from treating downloaded CI builds as damaged. It does
**not** silence Gatekeeper: first launch still needs right-click → Open (or
Privacy & Security approval), and the app is not notarized.

A Developer ID signature with notarization is required before calling any
macOS build a public release. That work needs a paid Apple Developer account
and a Mac, and cannot be validated from Linux/Windows CI alone:

1. Create a **Developer ID Application** certificate (Apple Developer account
   → Certificates, IDs & Profiles, CSR from a Mac) and export it as `.p12`.
2. Add repository secrets (never commit them): `APPLE_CERTIFICATE` (base64 of
   the `.p12`), `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`,
   plus notarization credentials — either App Store Connect API
   (`APPLE_API_ISSUER`, `APPLE_API_KEY`, `APPLE_API_KEY_PATH`) or Apple ID
   (`APPLE_ID` + app-specific `APPLE_PASSWORD`, `APPLE_TEAM_ID`). A
   `KEYCHAIN_PASSWORD` secret is needed for the ephemeral CI keychain.
3. Extend the `macos-dmg` job with a keychain-import step before the build
   (create keychain, `security import` the decoded `.p12`, partition list),
   export the signing/notarization variables from secrets into the build
   step's environment, and either set `APPLE_SIGNING_IDENTITY` or change
   `signingIdentity` in `tauri.conf.json` to the Developer ID identity.
   Tauri notarizes and staples automatically when those variables are present
   (see [Tauri's macOS signing guide](https://v2.tauri.app/distribute/sign/macos/)).

Until that is wired up, treat every macOS DMG as a test build.

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

After all platform jobs pass, the workflow publishes the prerelease with its
downloadable assets. Install-test these unsigned builds before promoting them
to a stable release.

## Before the first public release

1. Review and install all CI artifacts on clean virtual machines (Windows,
   Linux, and both macOS architectures).
2. Acquire a Windows Authenticode certificate and configure Tauri signing.
   Unsigned installers work, but Windows will show an unverified-publisher
   warning. Do not put a certificate or password in the repository.
3. Acquire an Apple Developer ID Application certificate, wire the macOS
   signing/notarization secrets above, and verify Gatekeeper accepts the DMG
   on both Apple Silicon and Intel. Do not call a macOS build public until
   it is signed and notarized.
4. Enable GitHub private vulnerability reporting before making the repository
   public.

Users install the macOS DMG by opening it and dragging **Colemak-DH Tutor**
to **Applications**, then launching from Applications or Spotlight.

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
