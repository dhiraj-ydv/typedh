//! Self-update support for Linux `~/.local` installs.
//!
//! The stock Tauri updater only handles AppImage bundles on Linux, while this
//! application ships a `~/.local` tarball (issue #10). Because those files are
//! user-owned, the application can safely replace them itself:
//!
//! 1. Download the release `latest.json` manifest and the `.tar.zst` tarball.
//! 2. Verify the tarball against its minisign signature using the same updater
//!    public key the stock updater uses on Windows/macOS. Anything that fails
//!    verification (or arrives unsigned) is rejected before touching disk.
//! 3. Extract the tarball over the install prefix (binary, desktop entry with
//!    the prefix rewritten in, icons, metainfo) and let the frontend restart.
//!
//! System installs (for example `/usr` via pacman) are refused: replacing
//! files owned by the package manager is out of scope.

use std::{
    collections::HashMap,
    io::Read,
    path::{Path, PathBuf},
    time::Duration,
};

use serde::{Deserialize, Serialize};

/// Static update manifest published on every `v*` release.
pub const UPDATE_MANIFEST_URL: &str =
    "https://github.com/exolithelabs/colemak-dh-tutor/releases/latest/download/latest.json";

/// Minisign public key (raw base64 key line). The same key is embedded in the
/// base64-encoded `.pub` file stored as `plugins.updater.pubkey` in
/// `tauri.conf.json`; the `pubkey_matches_config` test enforces that. The
/// private half lives only in the `TAURI_SIGNING_PRIVATE_KEY` GitHub Actions
/// secret.
pub const UPDATE_PUBKEY_BASE64: &str =
    "RWSCdzIGCe3FZVct7rUWtYpkiT3vPgxjmPw/Ls3GUJvmRO5NMTa3m/Xz";

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const TARBALL_ARCH_SUFFIX: &str = "x86_64";

/// Returned to the frontend so it can decide between the stock Tauri updater
/// (Windows/macOS, or any non-`~/.local` setup) and this custom flow.
#[derive(Clone, Debug, Serialize)]
pub struct LinuxInstallInfo {
    pub managed: bool,
    pub prefix: Option<String>,
    pub reason: String,
}

/// Progress events emitted on `linux-update-progress` while installing.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinuxUpdateProgress {
    pub phase: &'static str,
    pub downloaded: u64,
    pub total: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct UpdateManifest {
    version: String,
    platforms: HashMap<String, PlatformEntry>,
}

#[derive(Debug, Deserialize)]
struct PlatformEntry {
    url: String,
    signature: String,
}

/// Compare dotted numeric versions (`v` prefix tolerated). Anything
/// unparseable compares as *not* newer, which fails closed: the install path
/// refuses instead of guessing.
pub fn is_newer(current: &str, candidate: &str) -> bool {
    fn parts(version: &str) -> Option<Vec<u64>> {
        let clean = version.trim().strip_prefix('v').unwrap_or(version.trim());
        let core = clean.split(['-', '+']).next().unwrap_or(clean);
        let numbers: Option<Vec<u64>> =
            core.split('.').map(|part| part.parse::<u64>().ok()).collect();
        let numbers = numbers?;
        if numbers.is_empty() {
            return None;
        }
        Some(numbers)
    }
    match (parts(current), parts(candidate)) {
        (Some(a), Some(b)) => {
            let width = a.len().max(b.len());
            for i in 0..width {
                let x = a.get(i).copied().unwrap_or(0);
                let y = b.get(i).copied().unwrap_or(0);
                if x != y {
                    return y > x;
                }
            }
            false
        }
        _ => false,
    }
}

/// Derive the install prefix from the running executable, expecting the
/// `<prefix>/bin/<binary>` layout produced by `install.sh`.
pub fn install_prefix_from_exe(exe: &Path) -> Option<PathBuf> {
    let bin_dir = exe.parent()?;
    if bin_dir.file_name()?.to_str()? != "bin" {
        return None;
    }
    Some(bin_dir.parent()?.to_path_buf())
}

/// Refuse to self-update when running from a Cargo target directory (dev and
/// local CI builds); updating there would overwrite the developer's binary.
pub fn is_dev_build_path(exe: &Path) -> bool {
    exe.components().any(|component| {
        component.as_os_str().to_str().is_some_and(|part| part == "target")
    })
}

/// Only user-owned prefixes are eligible. System locations such as `/usr`
/// belong to the package manager.
pub fn check_prefix_eligible(prefix: &Path) -> Result<(), String> {
    if !prefix.is_absolute() {
        return Err("Install location is not an absolute path.".to_string());
    }
    if prefix == Path::new("/") || prefix == Path::new("/usr") {
        return Err(
            "This copy is installed system-wide. Update it with your system package manager instead of in-app updates."
                .to_string(),
        );
    }
    Ok(())
}

pub fn describe_install_target() -> LinuxInstallInfo {
    let not_managed = |reason: &str| LinuxInstallInfo {
        managed: false,
        prefix: None,
        reason: reason.to_string(),
    };
    if std::env::consts::OS != "linux" {
        return not_managed("In-app Linux updates only apply on Linux.");
    }
    let exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(_) => return not_managed("Could not locate the running application."),
    };
    if is_dev_build_path(&exe) {
        return not_managed("Updates are only supported for installed builds, not development builds.");
    }
    let Some(prefix) = install_prefix_from_exe(&exe) else {
        return not_managed("Could not determine the install location.");
    };
    if let Err(reason) = check_prefix_eligible(&prefix) {
        return not_managed(&reason);
    }
    LinuxInstallInfo {
        managed: true,
        prefix: Some(prefix.to_string_lossy().into_owned()),
        reason: "Installed under a user-owned prefix; in-app updates are supported.".to_string(),
    }
}

/// Mirror `packaging/linux/install.sh`: point the desktop entry at the prefix
/// this update is installing into, whether the shipped entry uses an absolute
/// system path or a bare binary name.
pub fn rewrite_desktop_exec(contents: &str, bindir: &Path, app_bin: &str) -> String {
    let bindir = bindir.to_string_lossy();
    contents
        .lines()
        .map(|line| {
            if let Some(rest) = line.strip_prefix("Exec=/usr/bin/") {
                return format!("Exec={bindir}/{rest}");
            }
            if let Some(rest) = line.strip_prefix("TryExec=/usr/bin/") {
                return format!("TryExec={bindir}/{rest}");
            }
            if line == format!("Exec={app_bin}") || line.starts_with(&format!("Exec={app_bin} ")) {
                let args = line["Exec=".len() + app_bin.len()..].to_string();
                return format!("Exec={bindir}/{app_bin}{args}");
            }
            if line == format!("TryExec={app_bin}") {
                return format!("TryExec={bindir}/{app_bin}");
            }
            line.to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

/// Verify tarball bytes against a minisign signature text (the `.sig` file
/// content, as embedded in `latest.json`). Anything else is rejected.
///
/// `allow_legacy` is true so both signature modes verify: `tauri signer`
/// artifacts may be standard or pre-hashed Ed25519 minisign signatures, and
/// both are equally trustworthy here.
pub fn verify_signature(data: &[u8], signature_text: &str) -> Result<(), String> {
    let public_key = minisign_verify::PublicKey::from_base64(UPDATE_PUBKEY_BASE64)
        .map_err(|_| "Updater public key is invalid. The app installation may be damaged.".to_string())?;
    let signature = minisign_verify::Signature::decode(signature_text)
        .map_err(|_| "Update signature is malformed. The update was blocked.".to_string())?;
    public_key
        .verify(data, &signature, true)
        .map_err(|_| "Update signature did not verify. The update was blocked.".to_string())
}

fn manifest_platform_key() -> String {
    format!("linux-{}", std::env::consts::ARCH)
}

fn select_platform_entry<'a>(
    manifest: &'a UpdateManifest,
    platform_key: &str,
) -> Result<&'a PlatformEntry, String> {
    manifest.platforms.get(platform_key).ok_or_else(|| {
        format!("The update manifest has no entry for {platform_key}. The release may still be publishing; retry shortly.")
    })
}

fn tarball_top_dir(expected_version: &str) -> String {
    format!("colemak-dh-tutor-{expected_version}-{TARBALL_ARCH_SUFFIX}")
}

/// Validate the staged tarball layout and copy it over the install prefix.
/// All checks run before any file is replaced.
fn install_staged_tree(
    staged_root: &Path,
    prefix: &Path,
    expected_version: &str,
) -> Result<(), String> {
    let source = staged_root.join(tarball_top_dir(expected_version));
    if !source.is_dir() {
        return Err(format!(
            "The update package does not contain {} for version {expected_version}. The update was blocked.",
            tarball_top_dir(expected_version)
        ));
    }

    let bin_source = source.join("bin");
    let entries: Vec<PathBuf> = std::fs::read_dir(&bin_source)
        .map_err(|_| "The update package has no usable binary. The update was blocked.".to_string())?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.is_file())
        .collect();
    if entries.is_empty() {
        return Err("The update package has no usable binary. The update was blocked.".to_string());
    }
    let app_bin = entries[0]
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("The update package has no usable binary. The update was blocked.")?
        .to_string();

    let bindir = prefix.join("bin");
    let appsdir = prefix.join("share").join("applications");
    std::fs::create_dir_all(&bindir)
        .and_then(|_| std::fs::create_dir_all(&appsdir))
        .map_err(|error| format!("Could not write to the install location: {error}"))?;

    for binary in &entries {
        let name = binary
            .file_name()
            .ok_or("The update package has no usable binary. The update was blocked.")?;
        let dest = bindir.join(name);
        std::fs::copy(binary, &dest)
            .map_err(|error| format!("Could not replace the application binary: {error}"))?;
        set_executable(&dest)?;
    }

    let desktops_source = source.join("share").join("applications");
    let mut installed_desktop = false;
    if desktops_source.is_dir() {
        for entry in std::fs::read_dir(&desktops_source)
            .map_err(|error| format!("Could not read the update package: {error}"))?
        {
            let path = entry
                .map_err(|error| format!("Could not read the update package: {error}"))?
                .path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("desktop") {
                continue;
            }
            let contents = std::fs::read_to_string(&path)
                .map_err(|error| format!("Could not read the update package: {error}"))?;
            let rewritten = rewrite_desktop_exec(&contents, &bindir, &app_bin);
            let dest = appsdir.join(
                path.file_name()
                    .ok_or("Could not read the update package.".to_string())?,
            );
            std::fs::write(&dest, rewritten)
                .map_err(|error| format!("Could not install the desktop entry: {error}"))?;
            set_data_file(&dest)?;
            installed_desktop = true;
        }
    }
    if !installed_desktop {
        return Err("The update package has no desktop entry. The update was blocked.".to_string());
    }

    for shared in ["icons", "metainfo"] {
        let from = source.join("share").join(shared);
        if from.is_dir() {
            let to = prefix.join("share").join(shared);
            copy_tree(&from, &to)
                .map_err(|error| format!("Could not install {shared}: {error}"))?;
        }
    }

    refresh_desktop_caches(prefix);
    Ok(())
}

fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let dest = to.join(entry.file_name());
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            copy_tree(&entry.path(), &dest)?;
        } else if file_type.is_file() {
            std::fs::copy(entry.path(), &dest)?;
        }
    }
    Ok(())
}

#[cfg(unix)]
fn set_executable(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
        .map_err(|error| format!("Could not mark the new binary executable: {error}"))
}

#[cfg(not(unix))]
fn set_executable(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(unix)]
fn set_data_file(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o644))
        .map_err(|error| format!("Could not set permissions on installed data files: {error}"))
}

#[cfg(not(unix))]
fn set_data_file(_path: &Path) -> Result<(), String> {
    Ok(())
}

/// Best-effort cache refreshes, mirroring `install.sh`. Failures are ignored:
/// the files are installed regardless.
fn refresh_desktop_caches(prefix: &Path) {
    let appsdir = prefix.join("share").join("applications");
    let icondir = prefix.join("share").join("icons");
    let _ = std::process::Command::new("update-desktop-database")
        .arg(&appsdir)
        .status();
    let _ = std::process::Command::new("gtk-update-icon-cache")
        .args(["-f", "-t"])
        .arg(&icondir)
        .status();
}

fn download_bytes(url: &str, emit: &dyn Fn(LinuxUpdateProgress)) -> Result<Vec<u8>, String> {
    let response = ureq::get(url)
        .timeout(REQUEST_TIMEOUT)
        .call()
        .map_err(|error| format!("Could not download the update: {error}"))?;
    let total = response
        .header("Content-Length")
        .and_then(|value| value.parse::<u64>().ok());
    emit(LinuxUpdateProgress {
        phase: "download",
        downloaded: 0,
        total,
    });
    let mut reader = response.into_reader();
    let mut downloaded: u64 = 0;
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 32 * 1024];
    loop {
        let read = reader
            .read(&mut chunk)
            .map_err(|error| format!("Could not download the update: {error}"))?;
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&chunk[..read]);
        downloaded += read as u64;
        emit(LinuxUpdateProgress {
            phase: "download",
            downloaded,
            total,
        });
    }
    Ok(bytes)
}

fn download_manifest(emit: &dyn Fn(LinuxUpdateProgress)) -> Result<UpdateManifest, String> {
    emit(LinuxUpdateProgress {
        phase: "check",
        downloaded: 0,
        total: None,
    });
    let response = ureq::get(UPDATE_MANIFEST_URL)
        .timeout(REQUEST_TIMEOUT)
        .call()
        .map_err(|error| {
            format!("Could not reach the update server. Check your connection and retry. ({error})")
        })?;
    let text = response
        .into_string()
        .map_err(|error| format!("Could not read the update manifest: {error}"))?;
    serde_json::from_str(&text).map_err(|_| {
        "The update manifest is invalid. The update was blocked.".to_string()
    })
}

/// Download, verify, and install `expected_version` over `prefix`.
/// Blocking; callers should run this off the UI thread.
pub fn perform_install(
    prefix: &Path,
    expected_version: &str,
    emit: &dyn Fn(LinuxUpdateProgress),
) -> Result<(), String> {
    check_prefix_eligible(prefix)?;
    if !is_newer(CURRENT_VERSION, expected_version) {
        return Err(format!(
            "Version {expected_version} is not newer than the installed {CURRENT_VERSION}. The update was blocked."
        ));
    }

    let manifest = download_manifest(emit)?;
    if manifest.version != expected_version {
        return Err(format!(
            "The update server no longer offers {expected_version} (it offers {}). Check for updates again.",
            manifest.version
        ));
    }
    if !is_newer(CURRENT_VERSION, &manifest.version) {
        return Err(format!(
            "Version {} is not newer than the installed {CURRENT_VERSION}.",
            manifest.version
        ));
    }
    let platform_key = manifest_platform_key();
    let entry = select_platform_entry(&manifest, &platform_key)?;

    let tarball = download_bytes(&entry.url, emit)?;
    emit(LinuxUpdateProgress {
        phase: "verify",
        downloaded: tarball.len() as u64,
        total: Some(tarball.len() as u64),
    });
    verify_signature(&tarball, &entry.signature)?;

    emit(LinuxUpdateProgress {
        phase: "install",
        downloaded: tarball.len() as u64,
        total: Some(tarball.len() as u64),
    });
    let staging = tempfile::tempdir()
        .map_err(|error| format!("Could not stage the update: {error}"))?;
    let decoder = zstd::stream::read::Decoder::new(&tarball[..])
        .map_err(|_| "The update package is corrupt. The update was blocked.".to_string())?;
    let mut archive = tar::Archive::new(decoder);
    archive
        .unpack(staging.path())
        .map_err(|_| "The update package is corrupt. The update was blocked.".to_string())?;
    install_staged_tree(staging.path(), prefix, expected_version)?;

    emit(LinuxUpdateProgress {
        phase: "done",
        downloaded: tarball.len() as u64,
        total: Some(tarball.len() as u64),
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pubkey_matches_config() {
        use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};

        let config = include_str!("../tauri.conf.json");
        let parsed: serde_json::Value =
            serde_json::from_str(config).expect("tauri.conf.json must parse");
        // Unsigned validation builds (forks, Dependabot) strip the updater
        // section; see scripts/ensure-updater-config.mjs. Nothing to compare
        // there — secret-having builds always run the full check below.
        let Some(updater) = parsed.pointer("/plugins/updater") else {
            return;
        };
        let configured = updater
            .pointer("/pubkey")
            .and_then(|value| value.as_str())
            .expect("plugins.updater.pubkey must be set");
        // Tauri stores the whole base64-encoded .pub file here; the backend
        // uses the raw key line from inside it. Both must carry the same key.
        let decoded = BASE64
            .decode(configured)
            .expect("updater pubkey must be base64");
        let text = String::from_utf8(decoded).expect("updater pubkey must decode to text");
        assert!(
            text.lines().any(|line| line.trim() == UPDATE_PUBKEY_BASE64),
            "tauri.conf.json pubkey must embed the backend updater key"
        );
        let endpoints = parsed
            .pointer("/plugins/updater/endpoints")
            .and_then(|value| value.as_array())
            .expect("plugins.updater.endpoints must be set");
        assert!(
            endpoints.iter().any(|endpoint| endpoint.as_str() == Some(UPDATE_MANIFEST_URL)),
            "tauri.conf.json endpoints must include the manifest URL"
        );
    }

    #[test]
    fn version_comparison() {
        assert!(is_newer("0.1.2", "0.1.3"));
        assert!(is_newer("0.1.2", "0.2.0"));
        assert!(is_newer("0.1.2", "1.0.0"));
        assert!(is_newer("0.1.2", "v0.1.3"));
        assert!(!is_newer("0.1.3", "0.1.3"));
        assert!(!is_newer("0.1.3", "0.1.2"));
        assert!(is_newer("0.2.0", "0.10.0"));
        assert!(!is_newer("0.1.2", "not-a-version"));
        assert!(!is_newer("0.1.2", ""));
    }

    #[test]
    fn prefix_detection() {
        assert_eq!(
            install_prefix_from_exe(Path::new("/home/u/.local/bin/colemak-dh-tutor")),
            Some(PathBuf::from("/home/u/.local"))
        );
        assert_eq!(
            install_prefix_from_exe(Path::new("/usr/bin/colemak-dh-tutor")),
            Some(PathBuf::from("/usr"))
        );
        assert_eq!(install_prefix_from_exe(Path::new("/opt/app/bin")), None);
        assert_eq!(
            install_prefix_from_exe(Path::new("/home/u/.local/sbin/colemak-dh-tutor")),
            None
        );
    }

    #[test]
    fn dev_builds_are_refused() {
        assert!(is_dev_build_path(Path::new(
            "/home/u/src/app/src-tauri/target/debug/colemak-dh-tutor"
        )));
        assert!(is_dev_build_path(Path::new(
            "/home/u/src/app/src-tauri/target/release/colemak-dh-tutor"
        )));
        assert!(!is_dev_build_path(Path::new(
            "/home/u/.local/bin/colemak-dh-tutor"
        )));
    }

    #[cfg(unix)]
    #[test]
    fn system_prefixes_are_refused() {
        assert!(check_prefix_eligible(Path::new("/usr")).is_err());
        assert!(check_prefix_eligible(Path::new("/")).is_err());
        assert!(check_prefix_eligible(Path::new("/home/u/.local")).is_ok());
        assert!(check_prefix_eligible(Path::new("/tmp/prefix")).is_ok());
    }

    #[cfg(windows)]
    #[test]
    fn system_prefixes_are_refused() {
        // `/usr` has no meaning on Windows; refusal is about absoluteness
        // there, while real eligibility is decided on Linux at runtime.
        assert!(check_prefix_eligible(Path::new("/usr")).is_err());
        assert!(check_prefix_eligible(Path::new("relative/prefix")).is_err());
        assert!(check_prefix_eligible(Path::new("C:\\Users\\u\\.local")).is_ok());
    }

    #[test]
    fn desktop_exec_rewrite_matches_installer() {
        let bindir = Path::new("/home/u/.local/bin");
        let app = "colemak-dh-tutor";
        let rewritten = rewrite_desktop_exec(
            "[Desktop Entry]\nExec=/usr/bin/colemak-dh-tutor %U\nTryExec=/usr/bin/colemak-dh-tutor\n",
            bindir,
            app,
        );
        assert!(rewritten.contains("Exec=/home/u/.local/bin/colemak-dh-tutor %U"));
        assert!(rewritten.contains("TryExec=/home/u/.local/bin/colemak-dh-tutor"));

        let rewritten = rewrite_desktop_exec("[Desktop Entry]\nExec=colemak-dh-tutor\n", bindir, app);
        assert!(rewritten.contains("Exec=/home/u/.local/bin/colemak-dh-tutor\n"));

        let rewritten = rewrite_desktop_exec(
            "[Desktop Entry]\nExec=colemak-dh-tutor %U\nTryExec=colemak-dh-tutor\n",
            bindir,
            app,
        );
        assert!(rewritten.contains("Exec=/home/u/.local/bin/colemak-dh-tutor %U"));
        assert!(rewritten.contains("TryExec=/home/u/.local/bin/colemak-dh-tutor"));

        let untouched = rewrite_desktop_exec("[Desktop Entry]\nExec=/opt/other/app --flag\n", bindir, app);
        assert!(untouched.contains("Exec=/opt/other/app --flag"));
    }

    #[test]
    fn manifest_platform_selection() {
        let manifest: UpdateManifest = serde_json::from_str(
            r#"{"version":"0.2.0","platforms":{"linux-x86_64":{"url":"https://example.invalid/x.tar.zst","signature":"sig"}}}"#,
        )
        .unwrap();
        let entry = select_platform_entry(&manifest, "linux-x86_64").unwrap();
        assert_eq!(entry.url, "https://example.invalid/x.tar.zst");
        assert!(select_platform_entry(&manifest, "linux-aarch64").is_err());
    }

    /// Minisign-format fixtures built in-test from a fixed seed with real
    /// Ed25519: proves the production verification path accepts a genuine
    /// signature and rejects tampering, without any secret in the repo.
    mod signature_fixtures {
        use super::*;
        use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
        use ed25519_dalek::{Signer, SigningKey};

        const SEED: [u8; 32] = [7; 32];
        const KEYNUM: [u8; 8] = *b"testkey1";
        const PAYLOAD: &[u8] = b"colemak-dh-tutor updater signature fixture";

        fn signing_key() -> SigningKey {
            SigningKey::from_bytes(&SEED)
        }

        fn pubkey_base64() -> String {
            let mut raw = vec![b'E', b'd'];
            raw.extend_from_slice(&KEYNUM);
            raw.extend_from_slice(signing_key().verifying_key().as_bytes());
            BASE64.encode(raw)
        }

        fn signature_text(payload: &[u8]) -> String {
            let key = signing_key();
            let sig = key.sign(payload).to_bytes();
            let mut raw = vec![b'E', b'd'];
            raw.extend_from_slice(&KEYNUM);
            raw.extend_from_slice(&sig);
            // The global signature covers the file signature followed by the
            // trusted comment text (without its "trusted comment: " prefix),
            // exactly as minisign produces it.
            let trusted = "timestamp:1700000000\tfile:fixture.bin";
            let mut global_input = Vec::with_capacity(sig.len() + trusted.len());
            global_input.extend_from_slice(&sig);
            global_input.extend_from_slice(trusted.as_bytes());
            let global = key.sign(&global_input).to_bytes();
            format!(
                "untrusted comment: test signature\n{}\ntrusted comment: {trusted}\n{}\n",
                BASE64.encode(raw),
                BASE64.encode(global)
            )
        }

        fn verify_with_fixture_key(data: &[u8], sig_text: &str) -> Result<(), String> {
            let public_key = minisign_verify::PublicKey::from_base64(&pubkey_base64())
                .map_err(|error| format!("fixture key invalid: {error}"))?;
            let signature = minisign_verify::Signature::decode(sig_text)
                .map_err(|error| format!("fixture signature invalid: {error}"))?;
            public_key
                .verify(data, &signature, true)
                .map_err(|error| format!("verify failed: {error}"))
        }

        #[test]
        fn genuine_signature_verifies() {
            assert!(verify_with_fixture_key(PAYLOAD, &signature_text(PAYLOAD)).is_ok());
        }

        #[test]
        fn tampered_payload_is_rejected() {
            let mut tampered = PAYLOAD.to_vec();
            tampered[0] ^= 0xff;
            assert!(verify_with_fixture_key(&tampered, &signature_text(PAYLOAD)).is_err());
        }

        #[test]
        fn tampered_signature_is_rejected() {
            let mut sig = signature_text(PAYLOAD).into_bytes();
            let middle = sig.len() / 2;
            sig[middle] = if sig[middle] == b'A' { b'B' } else { b'A' };
            let text = String::from_utf8(sig).unwrap();
            assert!(verify_with_fixture_key(PAYLOAD, &text).is_err());
        }

        #[test]
        fn malformed_signature_is_rejected() {
            assert!(verify_with_fixture_key(PAYLOAD, "not a signature").is_err());
            // The production entry point rejects garbage the same way.
            assert!(verify_signature(PAYLOAD, "not a signature").is_err());
        }
    }

    /// Build a synthetic tar.zst in memory and run the real staged-install
    /// path against a temporary prefix.
    #[test]
    fn staged_install_replaces_prefix_files() {
        use std::io::Write;

        fn append_bytes(
            builder: &mut tar::Builder<impl Write>,
            path: &str,
            data: &[u8],
        ) {
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder.append_data(&mut header, path, data).unwrap();
        }

        let version = "9.9.9";
        let top = format!("colemak-dh-tutor-{version}-x86_64");
        let mut tar_data = Vec::new();
        {
            let encoder = zstd::stream::write::Encoder::new(&mut tar_data, 3).unwrap();
            let mut builder = tar::Builder::new(encoder);
            append_bytes(
                &mut builder,
                &format!("{top}/bin/colemak-dh-tutor"),
                b"new-binary",
            );
            append_bytes(
                &mut builder,
                &format!("{top}/share/applications/colemak-dh-tutor.desktop"),
                b"[Desktop Entry]\nExec=colemak-dh-tutor %U\n",
            );
            append_bytes(
                &mut builder,
                &format!("{top}/share/icons/hicolor/128x128/apps/colemak-dh-tutor.png"),
                b"new-icon",
            );
            let encoder = builder.into_inner().unwrap();
            encoder.finish().unwrap();
        }

        let staging = tempfile::tempdir().unwrap();
        let decoder = zstd::stream::read::Decoder::new(&tar_data[..]).unwrap();
        tar::Archive::new(decoder).unpack(staging.path()).unwrap();

        let prefix = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(prefix.path().join("bin")).unwrap();
        std::fs::write(prefix.path().join("bin/colemak-dh-tutor"), b"old-binary").unwrap();

        install_staged_tree(staging.path(), prefix.path(), version).unwrap();

        assert_eq!(
            std::fs::read(prefix.path().join("bin/colemak-dh-tutor")).unwrap(),
            b"new-binary"
        );
        let desktop = std::fs::read_to_string(
            prefix.path().join("share/applications/colemak-dh-tutor.desktop"),
        )
        .unwrap();
        let bindir = prefix.path().join("bin");
        assert!(
            desktop.contains(&format!("Exec={}/colemak-dh-tutor %U", bindir.to_string_lossy())),
            "desktop Exec must point at the install prefix, got: {desktop}"
        );
        assert_eq!(
            std::fs::read(
                prefix
                    .path()
                    .join("share/icons/hicolor/128x128/apps/colemak-dh-tutor.png")
            )
            .unwrap(),
            b"new-icon"
        );
    }

    #[test]
    fn staged_install_rejects_wrong_version_dir() {
        let staging = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(staging.path().join("colemak-dh-tutor-1.0.0-x86_64/bin")).unwrap();
        let prefix = tempfile::tempdir().unwrap();
        assert!(install_staged_tree(staging.path(), prefix.path(), "9.9.9").is_err());
    }
}
