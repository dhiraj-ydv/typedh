# TypeDH

[Downloads](#downloads-and-installation) · [Usage](#using-the-app) · [User data](#user-data-and-privacy) · [Architecture](#architecture) · [GitHub workflow](#github-builds-and-releases) · [Development](#development) · [Support](#support-and-security) · [License](#license)

**TypeDH — Colemak-DH typing tutor**, with progressive
lessons, live keyboard and finger guidance, custom text practice, and local
progress history. No account or cloud database is required.

[Download a release](https://github.com/exolithelabs/typedh/releases) ·
[View GitHub builds](https://github.com/exolithelabs/typedh/actions/workflows/desktop-build.yml) ·
[Report a bug](https://github.com/exolithelabs/typedh/issues)

## Downloads and installation

Open [GitHub Releases](https://github.com/exolithelabs/typedh/releases),
choose a version, and expand **Assets**. The published packages are currently
unsigned prereleases for testing. Windows (x64), Linux (x86-64), and macOS
(Apple Silicon and Intel) builds are available; ARM Linux and other package
formats are not configured.

| Download | Purpose |
| --- | --- |
| `TypeDH_<version>_x64-setup.exe` | Windows installer |
| `typedh-<version>-1-x86_64.pkg.tar.zst` | Linux Arch pacman package (`pacman -U`) |
| `TypeDH_<version>_aarch64.dmg` | macOS installer for Apple Silicon (M1 and later) |
| `TypeDH_<version>_x64.dmg` | macOS installer for Intel Macs |

The automatically generated **Source code** archives contain project sources,
not installers. Installed users do not need development tools. The current
source uses a native Rust backend compiled into the Tauri application, with no
Python runtime, sidecar executable, or localhost server. Previously published
v0.1.1 installers still use the Python backend; the native migration is available
in subsequent Actions builds until a new release is tagged.

### Windows

Download and run the `.exe` installer, then launch **TypeDH** from the
Start menu. Installation is configured for the current Windows user. The unsigned
installer may show an unknown-publisher warning.

### Linux (Arch-based, pacman -U install)

Install or update with `pacman -U` using the `.pkg.tar.zst` from GitHub
Releases. This is developer-hosted on Releases (not AUR / `pacman -Syu`).
The same command upgrades an older installed version when given a newer
package file.

#### 1. Download then install/update

Download the `.pkg.tar.zst` from GitHub Releases (browser), then:

```bash
sudo pacman -U ./typedh-<version>-1-x86_64.pkg.tar.zst
```

#### 2. Direct URL install/update (no browser download)

```bash
sudo pacman -U "https://github.com/exolithelabs/typedh/releases/download/v0.1.2/typedh-0.1.2-1-x86_64.pkg.tar.zst"
```

Replace `v0.1.2` / `0.1.2` with the version you want.

#### 3. curl then pacman -U

```bash
curl -L -O "https://github.com/exolithelabs/typedh/releases/download/v0.1.2/typedh-0.1.2-1-x86_64.pkg.tar.zst"
sudo pacman -U ./typedh-0.1.2-1-x86_64.pkg.tar.zst
```

All three are the same packaging method; only how the file reaches `pacman`
differs.

Launch **TypeDH** from your application menu, or run
`typedh` from a terminal. Pacman installs the runtime libraries the
package declares (`webkit2gtk-4.1`, `gtk3`, `libsoup3`,
`libayatana-appindicator`, `librsvg`, `cairo`, `pango`, `gdk-pixbuf2`,
`glib2`, `openssl`, `hicolor-icon-theme`, `desktop-file-utils`; see
`packaging/arch/PKGBUILD`). The app is not published to the AUR or an
official Arch repository; GitHub Releases is the distribution path. Updates
are manual via `pacman -U` with a newer file/URL — `pacman -Syu` alone will
not pick this up without an AUR/repo entry.

#### Migrating from the old `~/.local` tarball

If you previously installed via the retired `.tar.zst` + `install.sh`
`~/.local` method, remove those user-owned files first, then install via
`pacman -U`. From the old extracted directory:

```bash
./uninstall.sh
```

using the same `--prefix` you installed with (default `~/.local`). If you no
longer have that directory, remove at least `~/.local/bin/colemak-dh-tutor`
(the old binary name), the `colemak-dh-tutor.desktop` entry under
`~/.local/share/applications/`, and its icons under
`~/.local/share/icons/`, then install the `typedh` pacman package above.

### macOS (Apple Silicon and Intel)

Download the `.dmg` matching your Mac — `aarch64` for Apple Silicon (M1 and
later), `x64` for Intel — then open it and drag **TypeDH** to
**Applications**. Launch the app from Applications or Spotlight.

The current DMGs are ad-hoc signed, not notarized. On first launch Gatekeeper
may block the app; if so, right-click (Control-click) it, choose **Open**, and
confirm, or allow it under System Settings → Privacy & Security. A Developer
ID signature with notarization is planned before the first public macOS
release (see `docs/RELEASING.md`).

### Updating

Open the menu and choose **Updates**, then **Check for updates**. On Windows
and macOS, when a newer signed release exists, the app downloads it, verifies
its signature, and installs it; unsigned or tampered payloads are rejected.
Your lessons and progress are kept.

- **Windows**: the installer runs and the app closes while it finishes —
  reopen the app if it does not restart by itself.
- **macOS**: updates through the same signed flow as Windows.
- **Linux (pacman `/usr` installs)**: no in-app self-update; update manually
  with `sudo pacman -U` using the newer `.pkg.tar.zst` file or URL (see the
  Linux section). `pacman -Syu` alone will not update this developer-hosted
  package.

## Using the app

1. Enable Colemak-DH in your operating system's keyboard settings. The tutor
   reads your keyboard input; it does not change the system layout.
2. Open the menu and choose a lesson. Lessons progress from home-row practice
   through words, punctuation, numbers, and coding text.
3. Follow the highlighted keys and finger guidance while typing. Completed
   built-in lessons record words per minute (WPM) and accuracy locally.
4. Open **Progress History** to review previous results.
5. Use **Custom Practice** to paste text or load a `.txt` file. Custom text is
   converted to lowercase and whitespace is normalized for practice. Text is
   limited to 10,000 characters. Custom-practice scores are saved in history;
   the pasted or uploaded text is not stored in the database.

| Shortcut | Action |
| --- | --- |
| `Tab` | Focus the typing input |
| `Escape` | Restart the current typing exercise |

## User data and privacy

On first launch, the desktop application creates its data directory for the
current OS user. The backend creates `colemak.db` there and seeds the built-in
lessons. Later launches reuse that database. Different OS users have separate
progress; there is no cloud synchronization or online login.

The product was renamed from Colemak-DH Tutor to **TypeDH** (issue #18), but
the application ID and database filename were deliberately kept unchanged so
existing progress is preserved with no migration: the UI says TypeDH while the
data path stays `io.github.exolithelabs.ColemakDHTutor` / `colemak.db`.

The source repository contains code and bundled assets. Runtime databases,
generated binaries, dependency directories, and build output are ignored by Git.
The installed application passes an explicit per-user data path to the backend,
so normal app startup does not store progress in the repository or install folder.

Typical database paths for the application ID
`io.github.exolithelabs.ColemakDHTutor` (kept across the TypeDH rename):

| Environment | Database location |
| --- | --- |
| Windows | `%APPDATA%\io.github.exolithelabs.ColemakDHTutor\colemak.db` |
| Linux | `$XDG_DATA_HOME/io.github.exolithelabs.ColemakDHTutor/colemak.db`, defaulting to `~/.local/share/io.github.exolithelabs.ColemakDHTutor/colemak.db` |
| macOS | `~/Library/Application Support/io.github.exolithelabs.ColemakDHTutor/colemak.db` |

Tauri resolves the exact path from the OS environment. The webview also stores
small UI preferences, such as the selected lesson, in its local storage. Those
preferences are separate from the SQLite progress database.

To back up progress, fully exit the app, then copy the database
directory somewhere safe. SQLite may also create `colemak.db-wal` and
`colemak.db-shm` companion files; do not delete them while the app is running.
Keeping the same application ID lets subsequent versions locate the existing
database. Back up your progress before upgrading a prerelease.

Fresh installs create a native SQLite database automatically. Since the app is
still in development, Python-era databases are not migrated and no automatic
legacy backups are created. Unsupported database schemas are rejected without
rewriting their contents. To start fresh with an old development database, close
the app and move `colemak.db` and its `-wal`/`-shm` files out of the app-data
folder, then restart. Current native databases retain progress across restarts.
History loads newest first in pages of 100 results.

## Architecture

| Component | Implementation |
| --- | --- |
| Frontend | Vue 3, TypeScript, CSS, and Vite |
| Desktop shell | Tauri v2 with Rust |
| Local backend | Rust Tauri commands, compiled into the desktop app |
| Data storage | SQLite through rusqlite, with SQLite bundled at compile time |
| Frontend/backend communication | Tauri IPC with typed command payloads |

The frontend invokes `get_lessons`, `get_progress`, and `save_progress` directly
through Tauri. Rust serializes database access on a background worker and validates
the command payloads; the webview cannot submit SQL or choose database paths.
The only network use is the update flow on Windows/macOS: fetching the signed
release manifest and update payloads from GitHub Releases, verified against
the updater public key before installing. Linux pacman installs update
manually via `pacman -U` and do not use the in-app updater. There is no other
HTTP traffic, CORS configuration, API tokens, or shell plugin.
SQLite retains WAL mode, full synchronization, foreign keys, and a write timeout.
Failed database opens can be retried from the UI. Writes are never retried
automatically, and loading/save failures are shown in the interface.

Project layout:

```text
frontend/           Vue interface (own npm project: frontend/package.json)
src-tauri/          Rust commands, SQLite backend, tests, and desktop configuration
scripts/            Release version checks and the updater manifest builder
packaging/arch/     Arch Linux PKGBUILD and packaging script (pacman .pkg.tar.zst)
.github/            Build/release workflow and Dependabot configuration
docs/               Additional maintainer documentation (docs/RELEASING.md)
package.json        Root npm project: Tauri CLI plus the version source of truth
```

Two npm manifests are intentional and cannot be consolidated: Tauri builds the
frontend as a separate project (`beforeBuildCommand: npm --prefix frontend run
build`, `frontendDist: ../frontend/dist` in `src-tauri/tauri.conf.json`), and
CI installs/caches/audits both (`npm ci` plus `npm --prefix frontend ci`).
Keep `package.json` / `package-lock.json` and `frontend/package.json` /
`frontend/package-lock.json` in sync; the release check fails the build when
versions disagree.

## GitHub builds and releases

The normal workflow is to edit code locally, commit, and push. GitHub-hosted
runners install dependencies, run backend tests, build the frontend and desktop
app, package the installers, and upload the resulting files. Release packaging
does not depend on this machine's build output.

| Trigger | Result |
| --- | --- |
| Pull request | Windows, Linux, and macOS tests/builds; Actions artifacts |
| Push to `master` | Windows, Linux, and macOS tests/builds; Actions artifacts |
| Push a `v*` tag | All builds, followed by a published GitHub prerelease with downloads |
| Manual **Run workflow** | Builds for the selected ref; publication only when the ref is a `v*` tag |

Actions artifacts are retained for 14 days. Release assets are attached separately
to the corresponding version in **Releases**. A plain commit does not publish a
release, and creating a tag locally does not trigger Actions until it is pushed.

### Publish a version

Update the version in `package.json`, `src-tauri/Cargo.toml`,
`src-tauri/tauri.conf.json`, and `packaging/arch/PKGBUILD`. Keep the corresponding
lock files consistent, and commit those changes. Then push an unused matching tag;
for example, after updating to `0.1.2`:

```bash
git tag v0.1.2
git push origin master v0.1.2
```

After all platform jobs pass, GitHub Actions attaches the packages and publishes
the prerelease automatically. The release job uses GitHub's automatic token;
update signing additionally uses the `TAURI_SIGNING_PRIVATE_KEY` repository
secret (see [the release guide](docs/RELEASING.md)). The workflow
checks that version files and tags agree, runs frontend regression tests and
JavaScript dependency audits, and tests the native Rust backend's validation,
schema rejection, history pagination, and persistence on every operating system.
Actions are pinned to commit IDs. Published releases cannot be overwritten by
a rerun; use a new version tag. Releases include `SHA256SUMS` for the packages.

Windows code signing and pacman package signing are not configured. Those
require real signing credentials and workflow integration. macOS DMGs currently use ad-hoc signing only;
a Developer ID certificate with notarization is required before calling any
macOS build a public release. See
[the release guide](docs/RELEASING.md) for maintainer details. Dependabot is
configured to check npm, Rust, and GitHub Actions dependencies weekly.

## Development

Production packages are built by GitHub Actions. For contributors who need a
local interactive preview, the commands below launch the desktop development
app with its integrated Rust backend.

Use Node.js 22, the current stable Rust toolchain, and the native
Tauri development prerequisites for your OS. Run commands from the repo root.

### Windows preview

```powershell
npm.cmd ci
npm.cmd --prefix frontend ci
npm.cmd run tauri dev
```

### Linux preview

```bash
npm ci
npm --prefix frontend ci
npm run tauri dev
```

### macOS preview

```bash
npm ci
npm --prefix frontend ci
npm run tauri dev
```

Requires Xcode command-line tools. To produce a local DMG instead of a dev
preview, see the macOS notes in [the release guide](docs/RELEASING.md).

The frontend calls Tauri commands to connect to the local backend; running Vite
alone in a browser is not a complete desktop-app preview. Development launches
store progress in a `development/` subdirectory of the app-data directory,
separate from installed release builds.

## Support and security

For bugs, open an [issue](https://github.com/exolithelabs/typedh/issues)
with your app version, OS, installation method, and steps to reproduce. For a
failed build, include the GitHub Actions run URL.

For suspected vulnerabilities, follow [SECURITY.md](SECURITY.md) and report
privately. Do not attach your personal database or signing credentials to a
public issue.

## License

Licensed under the [Elastic License 2.0](LICENSE). See [NOTICE](NOTICE) for
attributions.
