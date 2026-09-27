#!/usr/bin/env node
/**
 * Keep validation builds green when the updater signing key is unavailable.
 *
 * The Tauri bundler hard-errors when `plugins.updater.pubkey` is configured
 * but `TAURI_SIGNING_PRIVATE_KEY` is missing. Fork and Dependabot builds have
 * no secrets, so this script strips the updater section (and artifact
 * generation) from a COPY of the workflow's checkout before building. Builds
 * with the secret are untouched and stay fully signed.
 *
 * Usage:
 *   TAURI_SIGNING_PRIVATE_KEY=... node scripts/ensure-updater-config.mjs [--config src-tauri/tauri.conf.json]
 *   node scripts/ensure-updater-config.mjs --self-test
 */
import { readFileSync, writeFileSync, mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import assert from 'node:assert/strict';

function defaultConfigPath() {
  return new URL('../src-tauri/tauri.conf.json', import.meta.url);
}

export function ensureUpdaterConfig({ key, configPath }) {
  const raw = readFileSync(configPath, 'utf8');
  if (key && key.trim() !== '') {
    console.log('Updater signing key present; keeping signed updater config.');
    return false;
  }
  const config = JSON.parse(raw);
  if (config.plugins) {
    delete config.plugins.updater;
    if (Object.keys(config.plugins).length === 0) delete config.plugins;
  }
  if (config.bundle) delete config.bundle.createUpdaterArtifacts;
  writeFileSync(configPath, `${JSON.stringify(config, null, 2)}\n`);
  console.log('No updater signing key; stripped updater config for an unsigned validation build.');
  return true;
}

function selfTest() {
  const dir = mkdtempSync(join(tmpdir(), 'updater-config-'));
  try {
    const sample = {
      productName: 'App',
      bundle: { active: true, createUpdaterArtifacts: true },
      plugins: { updater: { pubkey: 'abc', endpoints: ['https://example.invalid/x.json'] } },
    };
    const configPath = join(dir, 'tauri.conf.json');
    writeFileSync(configPath, JSON.stringify(sample));

    // Key present: untouched.
    const before = readFileSync(configPath, 'utf8');
    assert.equal(ensureUpdaterConfig({ key: 'secret', configPath }), false);
    assert.equal(readFileSync(configPath, 'utf8'), before);

    // Key missing: updater section and artifact flag removed.
    assert.equal(ensureUpdaterConfig({ key: '', configPath }), true);
    const stripped = JSON.parse(readFileSync(configPath, 'utf8'));
    assert.ok(!stripped.plugins?.updater);
    assert.ok(!('createUpdaterArtifacts' in stripped.bundle));
    assert.equal(stripped.productName, 'App');

    // Missing env entirely behaves like missing key.
    writeFileSync(configPath, JSON.stringify(sample));
    assert.equal(ensureUpdaterConfig({ key: undefined, configPath }), true);
    console.log('Updater config self-test passed.');
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

function main() {
  const argv = process.argv.slice(2);
  if (argv.includes('--self-test')) {
    selfTest();
    return;
  }
  const configIndex = argv.indexOf('--config');
  const configPath =
    configIndex === -1 ? defaultConfigPath() : join(process.cwd(), argv[configIndex + 1]);
  ensureUpdaterConfig({ key: process.env.TAURI_SIGNING_PRIVATE_KEY, configPath });
}

main();
