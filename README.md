# Colemak-DH Tutor

[Downloads](#downloads-and-installation) · [Usage](#using-the-app) · [User data](#user-data-and-privacy) · [Architecture](#architecture) · [GitHub workflow](#github-builds-and-releases) · [Development](#development) · [Support](#support-and-security) · [License](#license)

A desktop touch-typing tutor for the Colemak-DH keyboard layout, with progressive
lessons, live keyboard and finger guidance, custom text practice, and local
progress history. No account or cloud database is required.

[Download a release](https://github.com/exolithelabs/colemak-dh-tutor/releases) ·
[View GitHub builds](https://github.com/exolithelabs/colemak-dh-tutor/actions/workflows/desktop-build.yml) ·
[Report a bug](https://github.com/exolithelabs/colemak-dh-tutor/issues)

## Downloads and installation

Open [GitHub Releases](https://github.com/exolithelabs/colemak-dh-tutor/releases),
choose a version, and expand **Assets**. The published packages are currently
unsigned prereleases for testing. Windows and Linux x86-64 builds are
available; macOS, ARM, and other Linux package formats are not configured.

| Download | Purpose |
| --- | --- |
| `Colemak-DH.Tutor_<version>_x64-setup.exe` | Windows installer |
| `colemak-dh-tutor-<version>-x86_64.tar.zst` | Linux `~/.local` tarball with `install.sh` |

The automatically generated **Source code** archives contain project sources,
not installers. Installed users do not need development tools. The current
source uses a native Rust backend compiled into the Tauri application, with no
Python runtime, sidecar executable, or localhost server. Previously published
v0.1.1 installers still use the Python backend; the native migration is available
in subsequent Actions builds until a new release is tagged.

### Windows

Download and run the `.exe` installer, then launch **Colemak-DH Tutor** from the
Start menu. Installation is configured for the current Windows user. The unsigned
installer may show an unknown-publisher warning.

### Linux (Arch-based, ~/.local install)

Download the `.tar.zst` archive, extract it, and run its installer from the
extracted directory. No sudo is required; files are installed user-owned
under `~/.local` so a future in-app updater can replace them:

```bash
tar --zstd -xf colemak-dh-tutor-<version>-x86_64.tar.zst
cd colemak-dh-tutor-<version>-x86_64
./install.sh
```

This installs roughly:

- binary → `~/.local/bin/`
- desktop entry → `~/.local/share/applications/`
- icons → `~/.local/share/icons/`

Launch **Colemak-DH Tutor** from your application menu, or run
`colemak-dh-tutor` from a terminal (ensure `~/.local/bin` is on your `PATH`).
To remove it later, run `./uninstall.sh` from the same extracted directory
with the same `--prefix` (default `~/.local`).

The tarball targets Arch-based distributions first and does not bundle the
system WebKit/GTK runtime. Install the runtime libraries for your distro
before launching; on Arch the pacman names include `webkit2gtk-4.1`, `gtk3`,
`libsoup3`, `libayatana-appindicator`, `librsvg`, `cairo`, `pango`,
`gdk-pixbuf2`, `glib2`, `openssl`, `hicolor-icon-theme`, and
`desktop-file-utils`. The app is not published to the AUR or an official Arch
repository; GitHub Releases is the distribution path. The previous
`.pkg.tar.zst` pacman artifact was removed in favor of this tarball (see
`docs/RELEASING.md`).

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

The source repository contains code and bundled assets. Runtime databases,
generated binaries, dependency directories, and build output are ignored by Git.
The installed application passes an explicit per-user data path to the backend,
so normal app startup does not store progress in the repository or install folder.

Typical database paths for the application ID
`io.github.exolithelabs.ColemakDHTutor`:

| Environment | Database location |
| --- | --- |
| Windows | `%APPDATA%\io.github.exolithelabs.ColemakDHTutor\colemak.db` |
| Linux | `$XDG_DATA_HOME/io.github.exolithelabs.ColemakDHTutor/colemak.db`, defaulting to `~/.local/share/io.github.exolithelabs.ColemakDHTutor/colemak.db` |

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
There are no HTTP requests, CORS configuration, API tokens, or shell plugin.
SQLite retains WAL mode, full synchronization, foreign keys, and a write timeout.
Failed database opens can be retried from the UI. Writes are never retried
automatically, and loading/save failures are shown in the interface.

Project layout:

```text
frontend/           Vue interface
src-tauri/          Rust commands, SQLite backend, tests, and desktop configuration
scripts/            Release version checks
packaging/linux/    Linux ~/.local tarball scripts (build-tarball, install, uninstall)
.github/            Build/release workflow and Dependabot configuration
docs/               Additional maintainer documentation
```

## GitHub builds and releases

The normal workflow is to edit code locally, commit, and push. GitHub-hosted
runners install dependencies, run backend tests, build the frontend and desktop
app, package the installers, and upload the resulting files. Release packaging
does not depend on this machine's build output.

| Trigger | Result |
| --- | --- |
| Pull request | Windows and Linux tests/builds; Actions artifacts |
| Push to `master` | Windows and Linux tests/builds; Actions artifacts |
| Push a `v*` tag | Both builds, followed by a published GitHub prerelease with downloads |
| Manual **Run workflow** | Builds for the selected ref; publication only when the ref is a `v*` tag |

Actions artifacts are retained for 14 days. Release assets are attached separately
to the corresponding version in **Releases**. A plain commit does not publish a
release, and creating a tag locally does not trigger Actions until it is pushed.

### Publish a version

Update the version in `package.json`, `src-tauri/Cargo.toml`,
and `src-tauri/tauri.conf.json`. Keep the corresponding
lock files consistent, and commit those changes. Then push an unused matching tag;
for example, after updating to `0.1.2`:

```bash
git tag v0.1.2
git push origin master v0.1.2
```

After both platform jobs pass, GitHub Actions attaches the packages and publishes
the prerelease automatically. The release job uses GitHub's automatic token;
no custom secrets are needed for the current unsigned builds. The workflow
checks that version files and tags agree, runs frontend regression tests and
JavaScript dependency audits, and tests the native Rust backend's validation,
schema rejection, history pagination, and persistence on both operating systems.
Actions are pinned to commit IDs. Published releases cannot be overwritten by
a rerun; use a new version tag. Releases include `SHA256SUMS` for the packages.

Windows code signing is not configured. That
requires real signing credentials and workflow integration (Linux `~/.local`
installs need no signing). See
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

The frontend calls Tauri commands to connect to the local backend; running Vite
alone in a browser is not a complete desktop-app preview. Development launches
store progress in a `development/` subdirectory of the app-data directory,
separate from installed release builds.

## Support and security

For bugs, open an [issue](https://github.com/exolithelabs/colemak-dh-tutor/issues)
with your app version, OS, installation method, and steps to reproduce. For a
failed build, include the GitHub Actions run URL.

For suspected vulnerabilities, follow [SECURITY.md](SECURITY.md) and report
privately. Do not attach your personal database or signing credentials to a
public issue.

## License

Licensed under the [Apache License 2.0](LICENSE). See [NOTICE](NOTICE) for
attributions.
