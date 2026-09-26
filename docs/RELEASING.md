# Releasing

## What CI produces

The **Desktop build** workflow runs on pull requests, pushes to `master`, version
tags, and manual dispatches. It produces:

- an unsigned Windows NSIS `.exe` installer; and
- an Arch Linux / pacman `.pkg.tar.zst` package for x86-64.

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
`packaging/arch/build-pkg.sh` inside an Arch Linux container to produce the
pacman package. The `.deb` itself is not published.

## Creating a release candidate

Ensure the version matches in `package.json`, `src-tauri/Cargo.toml`,
`src-tauri/tauri.conf.json`, their lockfiles, and `packaging/arch/PKGBUILD`, then
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
3. Decide whether Arch packages should be signed with a pacman keyring and, if
   so, configure signing in the workflow with secrets kept out of the repository.
4. Enable GitHub private vulnerability reporting before making the repository
   public.

Users can install the Arch package with:

```bash
sudo pacman -U ./colemak-dh-tutor-<version>-1-x86_64.pkg.tar.zst
```

The generated prerelease is suitable for testing. Do not promote it to a
public production release until signing identities are final. Those values
cannot be safely guessed in source control.
