#!/usr/bin/env node
/**
 * Build `latest.json` for the Tauri updater from downloaded release artifacts.
 *
 * Scans a flat directory of release files, pairs each update payload with its
 * detached `.sig` signature (both produced by signed CI builds), and writes
 * the static manifest the app's updater endpoint serves:
 *
 *   windows-x86_64 -> NSIS .exe (+ .exe.sig)
 *   darwin-aarch64  -> .app.tar.gz (+ .sig)
 *   darwin-x86_64   -> .app.tar.gz (+ .sig)
 *   linux-x86_64    -> ~/.local .tar.zst (+ .sig, verified by the app itself;
 *                      the stock Tauri updater only handles AppImage on Linux)
 *
 * Usage:
 *   node scripts/build-updater-manifest.mjs --dir release-artifacts \
 *     --repo owner/repo --tag v0.1.2 --out release-artifacts/latest.json
 *   node scripts/build-updater-manifest.mjs --self-test
 */
import { readdirSync, readFileSync, writeFileSync, mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, basename } from 'node:path';
import assert from 'node:assert/strict';

function parseArgs(argv) {
  const args = {};
  for (let i = 0; i < argv.length; i += 1) {
    if (argv[i].startsWith('--')) {
      const key = argv[i].slice(2);
      const next = argv[i + 1];
      if (next && !next.startsWith('--')) {
        args[key] = next;
        i += 1;
      } else {
        args[key] = true;
      }
    }
  }
  return args;
}

function readVersion() {
  const root = new URL('../package.json', import.meta.url);
  return JSON.parse(readFileSync(root, 'utf8')).version;
}

function findExactlyOne(files, description, predicate) {
  const matches = files.filter(predicate);
  if (matches.length !== 1) {
    throw new Error(
      `Expected exactly one ${description}, found ${matches.length}` +
        (matches.length ? `: ${matches.join(', ')}` : '. Did all platform jobs upload?'),
    );
  }
  return matches[0];
}

function readSignature(files, dir, payload) {
  const sigName = `${payload}.sig`;
  if (!files.includes(sigName)) {
    console.warn(`warning: missing signature for ${payload}; clients will reject this update.`);
    return '';
  }
  return readFileSync(join(dir, sigName), 'utf8').trim();
}

export function buildManifest({ files, version, repo, tag }) {
  const isSig = (name) => name.endsWith('.sig');
  const windowsPayload = findExactlyOne(
    files,
    'Windows NSIS installer',
    (name) => name.endsWith('-setup.exe') && !isSig(name),
  );
  const linuxPayload = findExactlyOne(
    files,
    'Linux ~/.local tarball',
    (name) => name.startsWith('colemak-dh-tutor-') && name.endsWith('.tar.zst') && !isSig(name),
  );
  const macArchives = files.filter((name) => name.endsWith('.app.tar.gz') && !isSig(name));
  if (macArchives.length !== 2) {
    throw new Error(`Expected exactly two macOS .app.tar.gz archives, found ${macArchives.length}.`);
  }
  const armArchive = findExactlyOne(macArchives, 'Apple Silicon updater archive', (name) =>
    name.includes('aarch64'),
  );
  const intelArchive = findExactlyOne(macArchives, 'Intel updater archive', (name) =>
    name.includes('x64') && !name.includes('aarch64'),
  );

  const assetUrl = (name) =>
    `https://github.com/${repo}/releases/download/${tag}/${encodeURIComponent(name)}`;

  return {
    version,
    notes: `Colemak-DH Tutor ${tag}: see https://github.com/${repo}/releases/tag/${tag} for release notes.`,
    pub_date: new Date().toISOString(),
    platforms: {
      'windows-x86_64': {
        url: assetUrl(windowsPayload),
        signature: '',
        __payload: windowsPayload,
      },
      'darwin-aarch64': {
        url: assetUrl(armArchive),
        signature: '',
        __payload: armArchive,
      },
      'darwin-x86_64': {
        url: assetUrl(intelArchive),
        signature: '',
        __payload: intelArchive,
      },
      'linux-x86_64': {
        url: assetUrl(linuxPayload),
        signature: '',
        __payload: linuxPayload,
      },
    },
  };
}

function attachSignatures(manifest, files, dir) {
  for (const entry of Object.values(manifest.platforms)) {
    entry.signature = readSignature(files, dir, entry.__payload);
    delete entry.__payload;
  }
  return manifest;
}

function selfTest() {
  const dir = mkdtempSync(join(tmpdir(), 'updater-manifest-'));
  try {
    const version = '0.9.9';
    const payloads = [
      'Colemak-DH Tutor_0.9.9_x64-setup.exe',
      'Colemak-DH Tutor_0.9.9_aarch64.app.tar.gz',
      'Colemak-DH Tutor_0.9.9_x64.app.tar.gz',
      'colemak-dh-tutor-0.9.9-x86_64.tar.zst',
    ];
    for (const name of payloads) {
      writeFileSync(join(dir, name), 'payload');
      writeFileSync(join(dir, `${name}.sig`), `sig-for-${basename(name)}`);
    }
    const files = readdirSync(dir);
    const manifest = attachSignatures(
      buildManifest({ files, version, repo: 'owner/repo', tag: 'v0.9.9' }),
      files,
      dir,
    );
    assert.equal(manifest.version, '0.9.9');
    assert.ok(Date.parse(manifest.pub_date) > 0);
    assert.match(manifest.notes, /owner\/repo\/releases\/tag\/v0\.9\.9/);
    const platforms = manifest.platforms;
    assert.deepEqual(Object.keys(platforms).sort(), [
      'darwin-aarch64',
      'darwin-x86_64',
      'linux-x86_64',
      'windows-x86_64',
    ]);
    // Spaces in asset names must be URL-encoded; arch must route correctly.
    assert.equal(
      platforms['windows-x86_64'].url,
      'https://github.com/owner/repo/releases/download/v0.9.9/Colemak-DH%20Tutor_0.9.9_x64-setup.exe',
    );
    assert.match(platforms['darwin-aarch64'].url, /aarch64\.app\.tar\.gz$/);
    assert.match(platforms['darwin-x86_64'].url, /x64\.app\.tar\.gz$/);
    assert.match(platforms['linux-x86_64'].url, /colemak-dh-tutor-0\.9\.9-x86_64\.tar\.zst$/);
    for (const entry of Object.values(platforms)) {
      assert.match(entry.signature, /^sig-for-/);
    }
    // Missing platform payloads fail loudly instead of shipping half a manifest.
    const noIntel = [
      'Colemak-DH Tutor_0.9.9_x64-setup.exe',
      'Colemak-DH Tutor_0.9.9_aarch64.app.tar.gz',
      'Colemak-DH Tutor_0.9.9_universal.app.tar.gz',
      'colemak-dh-tutor-0.9.9-x86_64.tar.zst',
    ];
    assert.throws(
      () => buildManifest({ files: noIntel, version, repo: 'o/r', tag: 'v0.9.9' }),
      /Intel updater archive/,
    );
    assert.throws(
      () => buildManifest({ files: files.filter((n) => !n.endsWith('.tar.zst')), version, repo: 'o/r', tag: 'v0.9.9' }),
      /Linux/,
    );
    console.log('Updater manifest self-test passed.');
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args['self-test']) {
    selfTest();
    return;
  }
  const dir = args.dir;
  const repo = args.repo;
  const tag = args.tag;
  const out = args.out;
  if (!dir || !repo || !tag || !out) {
    console.error('Usage: build-updater-manifest.mjs --dir DIR --repo owner/repo --tag vX.Y.Z --out latest.json');
    process.exit(2);
  }
  const version = readVersion();
  if (tag !== `v${version}`) {
    throw new Error(`Tag ${tag} does not match package version ${version}.`);
  }
  const files = readdirSync(dir);
  const manifest = attachSignatures(buildManifest({ files, version, repo, tag }), files, dir);
  writeFileSync(out, `${JSON.stringify(manifest, null, 2)}\n`);
  console.log(`Wrote ${out} for version ${version}.`);
}

main();
