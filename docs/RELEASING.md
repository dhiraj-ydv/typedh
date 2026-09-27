# Releasing

## What CI produces

The **Desktop build** workflow runs on pull requests, pushes to `master`, version
tags, and manual dispatches. It produces:

- an unsigned Windows NSIS `.exe` installer (plus its updater `.sig`);
- an Arch Linux pacman package `colemak-dh-tutor-<version>-1-x86_64.pkg.tar.zst`
  for x86-64; and
- two macOS `.dmg` installers: `aarch64` (Apple Silicon) and `x86_64` (Intel),
  plus per-arch `.app.tar.gz` updater archives with `.sig` files.

Ordinary builds retain these as workflow artifacts for 14 days (Windows,
Linux, and two macOS artifacts). Pushing a
version tag publishes a GitHub prerelease and attaches the packaged
applications together with `latest.json` (the signed updater manifest for
Windows/macOS) and a `SHA256SUMS` file. This uses the automatic per-run
`GITHUB_TOKEN` plus the `TAURI_SIGNING_PRIVATE_KEY` repository secret for
update signatures; no personal access token is required.

Before packaging, CI validates matching application versions, runs frontend
regression tests, and audits JavaScript dependencies. Every platform job compiles
the native Rust backend and tests validation, unsupported-schema rejection,
pagination, and persistence across restart. Python and PyInstaller are no longer
part of the build. Published assets
include a `SHA256SUMS` file. An already published release cannot be overwritten
by rerunning the workflow; create a new version instead.

The Linux job builds a temporary Debian staging package with Tauri, then runs
`packaging/arch/build-pkg.sh` inside an Arch Linux container (Docker +
`makepkg`) to produce the pacman package. The `.deb` itself is not published.

## Linux Arch package

The `linux-arch` job runs on `ubuntu-24.04`:

1. Install Tauri system prerequisites
   (`libwebkit2gtk-4.1-dev`, `libappindicator3-dev`, `librsvg2-dev`,
   `patchelf`).
2. Build icons and a Debian staging package:
   `npm run tauri build -- --bundles deb`.
3. Repack the `.deb` payload (`usr/bin`, `usr/share/applications`,
   `usr/share/icons`, `usr/share/metainfo`) into `packaging/arch/work/root/`
   and run `makepkg` in `archlinux:latest` via
   `packaging/arch/build-pkg.sh`, producing
   `dist/colemak-dh-tutor-<version>-1-x86_64.pkg.tar.zst`.

`packaging/arch/PKGBUILD` declares the pacman runtime `depends`:

> cairo desktop-file-utils gdk-pixbuf2 glib2 gtk3 hicolor-icon-theme
> libayatana-appindicator librsvg libsoup3 openssl pango webkit2gtk-4.1

Keep `pkgver` in sync with `package.json` / `src-tauri/Cargo.toml` /
`src-tauri/tauri.conf.json` (enforced by `scripts/check-release.mjs`);
`pkgrel` is `1`. The package installs system-wide under `/usr` via
`pacman -U`; it is developer-hosted on GitHub Releases, not published to the
AUR or a custom pacman repo, so `pacman -Syu` alone will not update it.

## macOS DMGs (Apple Silicon + Intel)

The `macos-dmg` job runs on `macos-latest` with a two-leg matrix, one Rust
target per Apple architecture (no universal binary):

| Matrix leg | Rust target | Users | Artifact |
| --- | --- | --- | --- |
| `aarch64` | `aarch64-apple-darwin` | Apple Silicon (M1 and later), built natively | `Colemak-DH-Tutor-macOS-aarch64` |
| `x86_64` | `x86_64-apple-darwin` | Intel Macs, cross-compiled on the same runner | `Colemak-DH-Tutor-macOS-x86_64` |

Each leg runs `npm run tauri build -- --bundles app,dmg --target <triple>` and
uploads the `.dmg` plus `src-tauri/target/<triple>/release/bundle/macos/*.app.tar.gz`
(the stock updater payload, with `.sig` files when the signing secret is set). Tauri embeds the
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

## Decision on the retired `~/.local` tarball

The per-user Linux tarball (`colemak-dh-tutor-<version>-x86_64.tar.zst` with
`packaging/linux/build-tarball.sh`, `install.sh`, `uninstall.sh` installing
under `~/.local`, issue #10) was **removed** once the pacman package became
primary (issue #17). Rationale:

- `pacman -U` is the standard Arch install/upgrade path and handles
  system-wide files plus declared runtime `depends`;
- GitHub Releases stays the distribution path (no AUR maintenance);
- one Linux artifact keeps version sync, CI time, and docs simpler.

There is no AUR, custom pacman repo, Flatpak, or AppImage publication path in
this workflow. Pacman `/usr` installs are intentionally excluded from in-app
updates; Linux updates are manual via `pacman -U` (see "In-app updates"
below). Anyone still on the old `~/.local` install should run `./uninstall.sh`
from the old extracted directory (same `--prefix`), then install the pacman
package.

## In-app updates

The app's **Updates** view checks
`https://github.com/exolithelabs/colemak-dh-tutor/releases/latest/download/latest.json`
for a newer signed release. Windows and macOS use the stock Tauri updater
(installer / `.app.tar.gz` payloads). Linux pacman `/usr` installs have no
in-app self-update path (replacing package-manager-owned files is out of
scope); Linux users update manually with `sudo pacman -U` using the newer
`.pkg.tar.zst` file or URL. Unsigned or tampered payloads are rejected before
any file is replaced.

### Keys and secrets

- The updater keypair was generated with `tauri signer generate` (passwordless).
- The public key is committed as `plugins.updater.pubkey` in
  `src-tauri/tauri.conf.json`.
- The private key lives only in the `TAURI_SIGNING_PRIVATE_KEY` repository
  secret and in one offline backup. If it is lost, installed apps can never
  accept another auto-update — back it up before you need it. Never commit it.
- To rotate keys you must ship the new public key inside an update signed by
  the old key; plan rotation as its own release.

### How CI signs

- `bundle.createUpdaterArtifacts` is `true`, so every Tauri build emits
  updater payloads alongside the installers.
- The Windows and macOS build steps export `TAURI_SIGNING_PRIVATE_KEY` from
  secrets, so the bundler writes `.sig` files next to the payloads. Without
  the secret the build still succeeds but unsigned (clients reject those
  updates).
- On `v*` tags the release job runs `scripts/build-updater-manifest.mjs`,
  which pairs each payload with its `.sig` content into `latest.json`
  (`windows-x86_64`, `darwin-aarch64`, `darwin-x86_64`) and attaches it with
  the installers. The Arch `.pkg.tar.zst` is intentionally excluded from the
  manifest. Missing payloads fail the job; missing signatures warn and ship
  empty (clients reject them). The script's `--self-test` runs in CI on every
  build.
- Builds without the secret (forks, Dependabot) stay green via
  `scripts/ensure-updater-config.mjs`, which strips the updater section for
  an unsigned validation build. Those artifacts cannot self-update; every
  same-repo build and all releases stay fully signed.

### Verifying an update locally

1. After a tag release, fetch
   `https://github.com/exolithelabs/colemak-dh-tutor/releases/latest/download/latest.json`
   and confirm all three platform entries have non-empty `signature` fields.
2. Confirm each `url` downloads and its bytes match `SHA256SUMS`. Confirm the
   Arch `.pkg.tar.zst` is attached separately and its bytes match
   `SHA256SUMS`.
3. End-to-end: install build N, push a new version tag, then use the app's
   **Updates** view to move to N+1 on Windows/macOS and confirm the version
   and progress survive. On Linux, verify a fresh `sudo pacman -U` install
   launches from the application menu / `colemak-dh-tutor`, then upgrade with
   `sudo pacman -U` to the newer package and confirm progress survives.

## Creating a release candidate

Ensure the version matches in `package.json`, `src-tauri/Cargo.toml`,
`src-tauri/tauri.conf.json`, `packaging/arch/PKGBUILD`, and their lockfiles,
then push the commit and a matching unused version tag (for example, after
bumping to 0.1.2):

```bash
git tag v0.1.2
git push origin master v0.1.2
```

After all platform jobs pass, the workflow publishes the prerelease with its
downloadable assets. Install-test these builds (including an N → N+1 in-app
update on Windows/macOS and an N → N+1 `pacman -U` upgrade on Linux where
possible) before promoting them to a stable release.

## Before the first public release

1. Review and install all CI artifacts on clean virtual machines (Windows,
   Linux, and both macOS architectures).
2. Confirm `latest.json` on a tag release carries signatures for all three
   updater platforms, and that the Updates view moves an installed
   Windows/macOS build forward. Confirm the Arch package installs and upgrades
   via `pacman -U`.
3. Back up the `TAURI_SIGNING_PRIVATE_KEY` offline, outside the repository.
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

Users install or update the Arch package with `pacman -U`. All three reach
the same package; only how the file reaches `pacman` differs:

```bash
sudo pacman -U ./colemak-dh-tutor-<version>-1-x86_64.pkg.tar.zst
```

```bash
sudo pacman -U "https://github.com/exolithelabs/colemak-dh-tutor/releases/download/v0.1.2/colemak-dh-tutor-0.1.2-1-x86_64.pkg.tar.zst"
```

```bash
curl -L -O "https://github.com/exolithelabs/colemak-dh-tutor/releases/download/v0.1.2/colemak-dh-tutor-0.1.2-1-x86_64.pkg.tar.zst"
sudo pacman -U ./colemak-dh-tutor-0.1.2-1-x86_64.pkg.tar.zst
```

Pacman installs the binary under `/usr/bin`, the desktop entry, icons, and
the declared WebKit/GTK runtime dependencies. This path does not use the AUR
or `pacman -Syu` auto-updates; updates are manual `pacman -U` runs against
newer release files/URLs.

The generated prerelease is suitable for testing. Do not promote it to a
public production release until signing identities are final. Those values
cannot be safely guessed in source control.
